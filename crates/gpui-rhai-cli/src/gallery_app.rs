use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use gpui_rhai::gpui::prelude::*;
use gpui_rhai::gpui::{
    App, AppContext, Bounds, Context, IntoElement, Render, StatefulInteractiveElement, Styled,
    WeakEntity, Window, WindowBounds, WindowOptions, div, px, rgba, size,
};
use gpui_rhai::{
    AssetData, EmbeddedScriptSource, EmbeddedScriptView, EventResponse, HostSlotRegistry, ModuleId,
    MotionPreference, NativeEvent, NativeHandlerDescriptor, NativeHandlerId, NativeTextDocument,
    RuntimeEngine, ScriptViewConfig, ScriptViewExtension, ScriptViewHandle, ScriptViewHost,
    ValueSchema, install, load_theme_source,
};
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_STORIES,
    BUNDLED_THEME_SOURCES, EN_LOCALE, GALLERY_NAVIGATION_SOURCE, GALLERY_SOURCE_VIEW_SOURCE,
    StoryDefinition, ZH_CN_LOCALE,
};

use super::gallery::{
    GalleryLaunch, host_resident_view, prepare, story_source, view_with_host_slots,
};

const RETAINED_STORY_LIMIT: usize = 8;

#[derive(Clone)]
struct MountedStory {
    primary: ScriptViewHandle,
    dependents: Vec<ScriptViewHandle>,
}

impl MountedStory {
    fn handles(&self) -> impl Iterator<Item = &ScriptViewHandle> {
        std::iter::once(&self.primary).chain(&self.dependents)
    }

    fn resume(&self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        let mut resumed = Vec::new();
        for view in self.dependents.iter().chain([&self.primary]) {
            match view.resume(cx) {
                Ok(true) => resumed.push(view),
                Ok(false) => {}
                Err(error) => {
                    for resumed in resumed.into_iter().rev() {
                        let _ = resumed.suspend(window, cx);
                    }
                    return Err(error.to_string());
                }
            }
        }
        Ok(())
    }

    fn suspend(&self, window: &mut Window, cx: &mut App) -> Result<(), String> {
        let mut failure = None;
        for view in self.handles() {
            if let Err(error) = view.suspend(window, cx)
                && failure.is_none()
            {
                failure = Some(error.to_string());
            }
        }
        failure.map_or(Ok(()), Err)
    }

    fn dispose(&self, cx: &mut App) {
        for view in self.handles() {
            let _ = view.dispose(cx);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewportPreset {
    Auto,
    Compact,
    Regular,
    Wide,
}

impl ViewportPreset {
    const ALL: [Self; 4] = [Self::Auto, Self::Compact, Self::Regular, Self::Wide];

    const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Compact => "Compact",
            Self::Regular => "Regular",
            Self::Wide => "Wide",
        }
    }

    const fn width(self) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Compact => Some(520.0),
            Self::Regular => Some(800.0),
            Self::Wide => Some(1_120.0),
        }
    }
}

fn mount_gallery_story(
    launch: &GalleryLaunch,
    generation: u64,
    host: &ScriptViewHost,
    window: &mut Window,
    cx: &mut App,
) -> Result<MountedStory, String> {
    if launch.story == "apps/host-embedding" {
        let resident_host =
            ScriptViewHost::new(format!("gallery-resident-window-{generation}"), cx)
                .map_err(|error| error.to_string())?;
        let resident = host_resident_view(launch)?
            .prepare()
            .map_err(|error| error.to_string())?
            .mount(
                ScriptViewConfig::new(format!("gallery-resident-{generation}")),
                resident_host,
                window,
                cx,
            )
            .map_err(|error| error.to_string())?;
        let slots = HostSlotRegistry::new()
            .with_script_view("resident-form", resident.clone())
            .map_err(|error| error.to_string())?;
        let parent = view_with_host_slots(launch, slots)
            .and_then(|view| view.prepare().map_err(|error| error.to_string()))
            .and_then(|prepared| {
                prepared
                    .mount(
                        ScriptViewConfig::new(GalleryApp::view_id(
                            &launch.story,
                            &launch.case,
                            generation,
                        )),
                        host.clone(),
                        window,
                        cx,
                    )
                    .map_err(|error| error.to_string())
            });
        match parent {
            Ok(primary) => Ok(MountedStory {
                primary,
                dependents: vec![resident],
            }),
            Err(error) => {
                let _ = resident.dispose(cx);
                Err(error)
            }
        }
    } else {
        let primary = prepare(launch)?
            .mount(
                ScriptViewConfig::new(GalleryApp::view_id(&launch.story, &launch.case, generation)),
                host.clone(),
                window,
                cx,
            )
            .map_err(|error| error.to_string())?;
        Ok(MountedStory {
            primary,
            dependents: Vec::new(),
        })
    }
}

struct GalleryApp {
    host: ScriptViewHost,
    navigation: ScriptViewHandle,
    source_view: ScriptViewHandle,
    launch: GalleryLaunch,
    current_key: String,
    views: BTreeMap<String, MountedStory>,
    visits: Vec<String>,
    error: Option<String>,
    next_generation: u64,
    search: String,
    source_revision: u64,
    category: Option<String>,
    viewport: ViewportPreset,
    motion_preference: MotionPreference,
}

impl GalleryApp {
    fn key(story: &str, case: &str) -> String {
        format!("{story}::{case}")
    }

    fn view_id(story: &str, case: &str, generation: u64) -> String {
        format!(
            "gallery-{}-{}-{generation}",
            story.replace('/', "-"),
            case.replace('/', "-")
        )
    }

    fn new(launch: GalleryLaunch, window: &mut Window, cx: &mut Context<Self>) -> Self {
        install(cx);
        let host =
            ScriptViewHost::new("gallery-window", cx).expect("Gallery host identity is valid");
        let key = Self::key(&launch.story, &launch.case);
        let view = mount_gallery_story(&launch, 1, &host, window, cx)
            .expect("validated initial Gallery story mounts");
        let navigation = prepare_navigation(
            &launch,
            NavigationExtension {
                owner: cx.entity().downgrade(),
            },
        )
        .and_then(|prepared| {
            prepared
                .mount(
                    ScriptViewConfig::new("gallery-navigation"),
                    host.clone(),
                    window,
                    cx,
                )
                .map_err(|error| error.to_string())
        })
        .expect("Gallery navigation mounts");
        let source_view = prepare_source_view(
            &launch,
            SourceExtension {
                document: NativeTextDocument::new(
                    "gallery_source",
                    1,
                    Arc::<str>::from(story_source(&launch).expect("initial story source resolves")),
                )
                .expect("Gallery source document is valid"),
            },
        )
        .and_then(|prepared| {
            prepared
                .mount(
                    ScriptViewConfig::new("gallery-source"),
                    host.clone(),
                    window,
                    cx,
                )
                .map_err(|error| error.to_string())
        })
        .expect("Gallery source view mounts");
        Self {
            host,
            navigation,
            source_view,
            launch,
            current_key: key.clone(),
            views: BTreeMap::from([(key.clone(), view)]),
            visits: vec![key],
            error: None,
            next_generation: 2,
            search: String::new(),
            source_revision: 1,
            category: None,
            viewport: ViewportPreset::Auto,
            motion_preference: MotionPreference::Normal,
        }
    }

    fn select(&mut self, story: &str, case: &str, window: &mut Window, cx: &mut Context<Self>) {
        let key = Self::key(story, case);
        if key == self.current_key {
            return;
        }
        let candidate = if let Some(view) = self.views.get(&key).cloned() {
            view.resume(window, cx).map(|()| view)
        } else {
            let launch = GalleryLaunch {
                story: story.to_owned(),
                case: case.to_owned(),
                theme: self.launch.theme.clone(),
                locale: self.launch.locale.clone(),
            };
            let generation = self.next_generation;
            self.next_generation = self.next_generation.saturating_add(1);
            mount_gallery_story(&launch, generation, &self.host, window, cx)
        };
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        for view in candidate.handles() {
            if let Err(error) = view.set_motion_preference(self.motion_preference, cx) {
                let _ = candidate.suspend(window, cx);
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        }
        if let Some(current) = self.views.get(&self.current_key)
            && let Err(error) = current.suspend(window, cx)
        {
            let _ = candidate.suspend(window, cx);
            self.error = Some(error);
            cx.notify();
            return;
        }
        self.views.entry(key.clone()).or_insert(candidate);
        self.current_key.clone_from(&key);
        story.clone_into(&mut self.launch.story);
        case.clone_into(&mut self.launch.case);
        self.update_source(cx);
        self.error = None;
        self.visits.retain(|visited| visited != &key);
        self.visits.push(key);
        self.evict(cx);
        cx.notify();
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        let candidate = mount_gallery_story(&self.launch, generation, &self.host, window, cx);
        match candidate {
            Ok(candidate) => {
                for view in candidate.handles() {
                    if let Err(error) = view.set_motion_preference(self.motion_preference, cx) {
                        candidate.dispose(cx);
                        self.error = Some(error.to_string());
                        cx.notify();
                        return;
                    }
                }
                if let Some(previous) = self.views.insert(self.current_key.clone(), candidate) {
                    previous.dispose(cx);
                }
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }

    fn select_theme(&mut self, family: &str, variant: &str, slug: &str, cx: &mut Context<Self>) {
        for mounted in self.views.values() {
            for view in mounted.handles() {
                if let Err(error) = view.select_theme(family, variant, cx) {
                    self.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            }
        }
        for view in [&self.navigation, &self.source_view] {
            if let Err(error) = view.select_theme(family, variant, cx) {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        }
        slug.clone_into(&mut self.launch.theme);
        self.error = None;
        cx.notify();
    }

    fn select_locale(&mut self, locale: &str, cx: &mut Context<Self>) {
        for mounted in self.views.values() {
            for view in mounted.handles() {
                if let Err(error) = view.select_locale(locale, cx) {
                    self.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            }
        }
        for view in [&self.navigation, &self.source_view] {
            if let Err(error) = view.select_locale(locale, cx) {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        }
        locale.clone_into(&mut self.launch.locale);
        self.error = None;
        cx.notify();
    }

    fn select_motion_preference(&mut self, preference: MotionPreference, cx: &mut Context<Self>) {
        for mounted in self.views.values() {
            for view in mounted.handles() {
                if let Err(error) = view.set_motion_preference(preference, cx) {
                    self.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            }
        }
        for view in [&self.navigation, &self.source_view] {
            if let Err(error) = view.set_motion_preference(preference, cx) {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        }
        self.motion_preference = preference;
        self.error = None;
        cx.notify();
    }

    fn select_category(&mut self, category: Option<String>, cx: &mut Context<Self>) {
        self.category = category;
        cx.notify();
    }

    fn select_viewport(&mut self, viewport: ViewportPreset, cx: &mut Context<Self>) {
        self.viewport = viewport;
        cx.notify();
    }

    fn update_source(&mut self, cx: &mut Context<Self>) {
        let source = story_source(&self.launch)
            .unwrap_or_else(|error| format!("Unable to resolve story source: {error}"));
        self.source_revision = self.source_revision.saturating_add(1);
        let result = NativeTextDocument::new(
            "gallery_source",
            self.source_revision,
            Arc::<str>::from(source),
        )
        .map_err(|error| error.to_string())
        .and_then(|document| {
            self.source_view
                .replace_native_text_document("gallery_source", document, cx)
                .map_err(|error| error.to_string())
        });
        if let Err(error) = result {
            self.error = Some(error);
        }
    }

    fn evict(&mut self, cx: &mut Context<Self>) {
        while self.views.len() > RETAINED_STORY_LIMIT {
            let Some(key) = self.visits.first().cloned() else {
                break;
            };
            if key == self.current_key {
                self.visits.rotate_left(1);
                continue;
            }
            self.visits.remove(0);
            if let Some(view) = self.views.remove(&key) {
                view.dispose(cx);
            }
        }
    }

    fn current_view(&self) -> &ScriptViewHandle {
        &self.views[&self.current_key].primary
    }

    fn current_story(&self) -> &'static StoryDefinition {
        BUNDLED_STORIES
            .iter()
            .find(|story| story.id == self.launch.story)
            .expect("selected story belongs to catalog")
    }
}

#[derive(Clone)]
struct NavigationExtension {
    owner: WeakEntity<GalleryApp>,
}

impl ScriptViewExtension for NavigationExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        let owner = self.owner.clone();
        engine
            .register_native_handler(
                NativeHandlerDescriptor::new(
                    NativeHandlerId::parse("gallery.search").map_err(|error| error.to_string())?,
                    BTreeMap::from([("change".to_owned(), ValueSchema::string())]),
                )
                .map_err(|error| error.to_string())?,
                move |event: NativeEvent, runtime, _, app| {
                    let gpui_rhai::UiValue::String(query) = event.payload else {
                        return Err("Gallery search query must be a string".to_owned());
                    };
                    let component = runtime
                        .component_state
                        .paths()
                        .into_iter()
                        .find(|path| runtime.component_state.get(path, "query").is_some())
                        .ok_or_else(|| "Gallery search state is not mounted".to_owned())?;
                    runtime
                        .set_component_state_from_host(
                            &component,
                            "query",
                            gpui_rhai::UiValue::String(query.clone()),
                        )
                        .map_err(|error| error.to_string())?;
                    owner
                        .update(app, |gallery, cx| {
                            gallery.search.clone_from(&query);
                            cx.notify();
                        })
                        .map_err(|error| error.to_string())?;
                    Ok(EventResponse::new())
                },
            )
            .map_err(|error| error.to_string())
    }
}

fn prepare_navigation(
    launch: &GalleryLaunch,
    extension: NavigationExtension,
) -> Result<gpui_rhai::PreparedScriptView, String> {
    let normalized_theme = launch.theme.replace('-', "_");
    let (theme_name, primary_theme) = BUNDLED_THEME_SOURCES
        .iter()
        .copied()
        .find(|(name, _)| name.strip_suffix(".rhai") == Some(normalized_theme.as_str()))
        .ok_or_else(|| format!("unknown Gallery theme `{}`", launch.theme))?;
    let engine = RuntimeEngine::new();
    let selected = load_theme_source(engine.engine(), theme_name, primary_theme)
        .map_err(|error| error.to_string())?;
    let source = GALLERY_NAVIGATION_SOURCE.to_owned();
    let source = format!(
        "{source}\nfn init(ctx) {{ ctx.set_theme({}, {}); ctx.set_locale({}); }}\n",
        serde_json::to_string(&selected.family).map_err(|error| error.to_string())?,
        serde_json::to_string(&selected.name).map_err(|error| error.to_string())?,
        serde_json::to_string(&launch.locale).map_err(|error| error.to_string())?,
    );
    let entry = ModuleId::parse("stories/gallery/navigation").map_err(|error| error.to_string())?;
    let mut modules = BTreeMap::from([(entry.clone(), source)]);
    for &(id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(
            ModuleId::parse(id).map_err(|error| error.to_string())?,
            source.to_owned(),
        );
    }
    EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), primary_theme)
        .theme_sources(
            BUNDLED_THEME_SOURCES
                .iter()
                .filter(|(name, _)| *name != theme_name)
                .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
        )
        .locale_sources([
            ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
            ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
        ])
        .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
            (
                path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                },
            )
        }))
        .extension(extension)
        .prepare()
        .map_err(|error| error.to_string())
}

#[derive(Clone)]
struct SourceExtension {
    document: NativeTextDocument,
}

impl ScriptViewExtension for SourceExtension {
    fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
        runtime
            .native_documents
            .register("gallery_source", self.document.clone())
            .map_err(|error| error.to_string())
    }
}

fn prepare_source_view(
    launch: &GalleryLaunch,
    extension: SourceExtension,
) -> Result<gpui_rhai::PreparedScriptView, String> {
    let normalized_theme = launch.theme.replace('-', "_");
    let (theme_name, primary_theme) = BUNDLED_THEME_SOURCES
        .iter()
        .copied()
        .find(|(name, _)| name.strip_suffix(".rhai") == Some(normalized_theme.as_str()))
        .ok_or_else(|| format!("unknown Gallery theme `{}`", launch.theme))?;
    let engine = RuntimeEngine::new();
    let selected = load_theme_source(engine.engine(), theme_name, primary_theme)
        .map_err(|error| error.to_string())?;
    let source = format!(
        "{}\nfn init(ctx) {{ ctx.set_theme({}, {}); ctx.set_locale({}); }}\n",
        GALLERY_SOURCE_VIEW_SOURCE,
        serde_json::to_string(&selected.family).map_err(|error| error.to_string())?,
        serde_json::to_string(&selected.name).map_err(|error| error.to_string())?,
        serde_json::to_string(&launch.locale).map_err(|error| error.to_string())?,
    );
    let entry = ModuleId::parse("stories/gallery/source").map_err(|error| error.to_string())?;
    let modules = BTreeMap::from([
        (entry.clone(), source),
        (
            ModuleId::parse("components/code_viewer").map_err(|error| error.to_string())?,
            BUNDLED_COMPONENT_SOURCES_BY_ID
                .iter()
                .find(|(id, _)| *id == "components/code_viewer")
                .map(|(_, source)| (*source).to_owned())
                .ok_or_else(|| "CodeViewer source is not bundled".to_owned())?,
        ),
    ]);
    EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), primary_theme)
        .theme_sources(
            BUNDLED_THEME_SOURCES
                .iter()
                .filter(|(name, _)| *name != theme_name)
                .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
        )
        .locale_sources([
            ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
            ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
        ])
        .extension(extension)
        .prepare()
        .map_err(|error| error.to_string())
}

impl Render for GalleryApp {
    #[allow(clippy::too_many_lines)]
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self
            .current_view()
            .theme_snapshot(cx)
            .expect("active Gallery story has a theme")
            .variant;
        let color = |name: &str| rgba(theme.tokens.colors[name].as_rgba_hex());
        let story = self.current_story();
        let current_case = story
            .cases
            .iter()
            .find(|case| case.id == self.launch.case)
            .expect("selected case belongs to the current story");
        let module_summary = if story.module_ids.len() <= 5 {
            story.module_ids.join(", ")
        } else {
            format!("{} public modules", story.module_ids.len())
        };
        let feature_summary = if story.required_features.is_empty() {
            "default".to_owned()
        } else {
            story.required_features.join(", ")
        };
        let needle = self.search.to_lowercase();
        let selected_category = self.category.as_deref();
        let navigation_items = BUNDLED_STORIES
            .iter()
            .filter(|entry| {
                selected_category.is_none_or(|category| entry.category == category)
                    && (needle.is_empty()
                        || entry.title.to_lowercase().contains(&needle)
                        || entry.id.contains(&needle)
                        || entry
                            .module_ids
                            .iter()
                            .any(|module| module.contains(&needle))
                        || entry
                            .keywords
                            .iter()
                            .any(|keyword| keyword.contains(&needle)))
            })
            .map(|entry| {
                let id = entry.id.to_owned();
                let case = entry.cases[0].id.to_owned();
                let selected = entry.id == story.id;
                div()
                    .id(gpui_rhai::gpui::ElementId::Name(
                        format!("story-{}", entry.id).into(),
                    ))
                    .role(gpui_rhai::gpui::Role::Button)
                    .aria_label(format!("Open {}", entry.title))
                    .aria_selected(selected)
                    .w_full()
                    .h(px(28.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(if selected { color("accent") } else { rgba(0) })
                    .bg(if selected {
                        color("selection")
                    } else {
                        rgba(0)
                    })
                    .text_color(if selected {
                        color("text_primary")
                    } else {
                        color("text_muted")
                    })
                    .hover(|style| {
                        style
                            .bg(color("surface_hover"))
                            .text_color(color("text_primary"))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select(&id, &case, window, cx);
                    }))
                    .child(entry.title)
            });
        let category_names = BUNDLED_STORIES
            .iter()
            .map(|story| story.category)
            .collect::<BTreeSet<_>>();
        let all_categories = div()
            .id("gallery-category-all")
            .role(gpui_rhai::gpui::Role::Button)
            .aria_label("Show all story categories")
            .aria_selected(self.category.is_none())
            .px_2()
            .py_1()
            .border_1()
            .border_color(if self.category.is_none() {
                color("accent")
            } else {
                color("border")
            })
            .bg(if self.category.is_none() {
                color("selection")
            } else {
                rgba(0)
            })
            .on_click(cx.listener(|this, _, _, cx| this.select_category(None, cx)))
            .child("All");
        let category_buttons = category_names.into_iter().map(|category| {
            let selected = self.category.as_deref() == Some(category);
            let category_value = category.to_owned();
            div()
                .id(gpui_rhai::gpui::ElementId::Name(
                    format!("gallery-category-{category}").into(),
                ))
                .role(gpui_rhai::gpui::Role::Button)
                .aria_label(format!("Show {category} stories"))
                .aria_selected(selected)
                .px_2()
                .py_1()
                .border_1()
                .border_color(if selected {
                    color("accent")
                } else {
                    color("border")
                })
                .bg(if selected {
                    color("selection")
                } else {
                    rgba(0)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_category(Some(category_value.clone()), cx);
                }))
                .child(category)
        });
        let navigation = self
            .navigation
            .element()
            .expect("Gallery navigation is active");
        let preview = self
            .current_view()
            .flex_item()
            .expect("active Gallery story is renderable");
        let mut preview_surface = div()
            .h_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(preview);
        preview_surface = if let Some(width) = self.viewport.width() {
            preview_surface.w(px(width)).flex_none()
        } else {
            preview_surface.w_full().flex_1()
        };
        let source_view = self
            .source_view
            .flex_item()
            .expect("Gallery source view is active");
        let cases = story.cases.iter().map(|case| {
            let story_id = story.id.to_owned();
            let case_id = case.id.to_owned();
            let selected = case.id == self.launch.case;
            div()
                .id(gpui_rhai::gpui::ElementId::Name(
                    format!("case-{}", case.id).into(),
                ))
                .role(gpui_rhai::gpui::Role::Button)
                .aria_label(format!("Open {} case", case.title))
                .aria_selected(selected)
                .px_2()
                .py_1()
                .border_1()
                .border_color(if selected {
                    color("accent")
                } else {
                    color("border")
                })
                .bg(if selected {
                    color("selection")
                } else {
                    rgba(0)
                })
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select(&story_id, &case_id, window, cx);
                }))
                .child(case.title)
        });
        let reset = div()
            .id("reset-story")
            .role(gpui_rhai::gpui::Role::Button)
            .aria_label("Reset current story")
            .px_2()
            .py_1()
            .border_1()
            .border_color(color("border"))
            .hover(|style| style.bg(color("surface_hover")))
            .on_click(cx.listener(|this, _, window, cx| this.reset(window, cx)))
            .child("Reset story");
        let viewport_buttons = ViewportPreset::ALL.into_iter().map(|viewport| {
            let selected = self.viewport == viewport;
            div()
                .id(gpui_rhai::gpui::ElementId::Name(
                    format!("gallery-viewport-{}", viewport.label().to_lowercase()).into(),
                ))
                .role(gpui_rhai::gpui::Role::Button)
                .aria_label(format!("Use {} story viewport", viewport.label()))
                .aria_selected(selected)
                .px_2()
                .py_1()
                .border_1()
                .border_color(if selected {
                    color("accent")
                } else {
                    color("border")
                })
                .bg(if selected {
                    color("selection")
                } else {
                    rgba(0)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_viewport(viewport, cx);
                }))
                .child(viewport.label())
        });
        let motion_buttons = [
            (MotionPreference::Normal, "Normal"),
            (MotionPreference::Reduced, "Reduced"),
            (MotionPreference::None, "None"),
        ]
        .into_iter()
        .map(|(preference, label)| {
            let selected = self.motion_preference == preference;
            div()
                .id(gpui_rhai::gpui::ElementId::Name(
                    format!("gallery-motion-{}", label.to_lowercase()).into(),
                ))
                .role(gpui_rhai::gpui::Role::Button)
                .aria_label(format!("Use {label} motion preference"))
                .aria_selected(selected)
                .px_2()
                .py_1()
                .border_1()
                .border_color(if selected {
                    color("accent")
                } else {
                    color("border")
                })
                .bg(if selected {
                    color("selection")
                } else {
                    rgba(0)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_motion_preference(preference, cx);
                }))
                .child(label)
        });
        let dark = div()
            .id("gallery-theme-dark")
            .role(gpui_rhai::gpui::Role::Button)
            .aria_label("Use Default Dark theme")
            .px_2()
            .py_1()
            .border_1()
            .border_color(color("border"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.select_theme("Default", "Dark", "default-dark", cx);
            }))
            .child("Dark");
        let light = div()
            .id("gallery-theme-light")
            .role(gpui_rhai::gpui::Role::Button)
            .aria_label("Use Default Light theme")
            .px_2()
            .py_1()
            .border_1()
            .border_color(color("border"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.select_theme("Default", "Light", "default-light", cx);
            }))
            .child("Light");
        let locale_buttons = ["en", "zh-CN", "ar"].into_iter().map(|locale| {
            div()
                .id(gpui_rhai::gpui::ElementId::Name(
                    format!("gallery-locale-{locale}").into(),
                ))
                .role(gpui_rhai::gpui::Role::Button)
                .aria_label(format!("Use {locale} locale"))
                .px_2()
                .py_1()
                .border_1()
                .border_color(color("border"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_locale(locale, cx);
                }))
                .child(locale)
        });
        let error = self.error.as_ref().map(|error| {
            div()
                .p_2()
                .border_1()
                .border_color(color("danger"))
                .text_color(color("danger"))
                .child(error.clone())
        });
        let fixed_viewport = self.viewport != ViewportPreset::Auto;
        let preview_pane = div()
            .id("gallery-preview-viewport")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_x_scroll()
            .child(preview_surface);
        let mut source_pane = div()
            .w(px(360.0))
            .h_full()
            .min_h_0()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(color("border"))
            .child(source_view);
        if fixed_viewport {
            source_pane = source_pane.w_full().h(px(280.0)).border_l_0().border_t_1();
        }
        let mut story_content = div().flex_1().min_h_0().min_w_0().flex();
        if fixed_viewport {
            story_content = story_content.flex_col();
        }
        story_content = story_content.child(preview_pane).child(source_pane);
        self.host.container(
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(color("surface"))
                .text_color(color("text_primary"))
                .child(
                    div()
                        .h(px(58.0))
                        .flex_none()
                        .px_4()
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(color("border"))
                        .child(
                            div().flex().flex_col().child("GPUI RHAI / GALLERY").child(
                                div()
                                    .text_color(color("text_muted"))
                                    .child("Explore · Operations Workbench · live source stories"),
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(format!("{} / {}", theme.family, theme.name))
                                .child(dark)
                                .child(light)
                                .children(locale_buttons),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .child(
                            div()
                                .id("gallery-navigation")
                                .w(px(230.0))
                                .flex_none()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .border_r_1()
                                .border_color(color("border"))
                                .child(div().flex_none().child(navigation))
                                .child(
                                    div()
                                        .flex_none()
                                        .p_2()
                                        .flex()
                                        .flex_wrap()
                                        .gap_1()
                                        .border_b_1()
                                        .border_color(color("border"))
                                        .child(all_categories)
                                        .children(category_buttons),
                                )
                                .child(
                                    div()
                                        .id("gallery-story-list")
                                        .flex_1()
                                        .min_h_0()
                                        .p_2()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .overflow_y_scroll()
                                        .children(navigation_items),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .px_4()
                                        .py_3()
                                        .border_b_1()
                                        .border_color(color("border"))
                                        .child(story.title)
                                        .child(
                                            div()
                                                .text_color(color("text_muted"))
                                                .child(story.purpose),
                                        )
                                        .child(
                                            div().pt_1().text_color(color("text_muted")).child(
                                                format!("Observe: {}", current_case.purpose),
                                            ),
                                        )
                                        .child(
                                            div()
                                                .pt_1()
                                                .flex()
                                                .flex_wrap()
                                                .gap_2()
                                                .text_color(color("text_muted"))
                                                .child(format!("Modules: {module_summary}"))
                                                .child(format!("Source: {}", story.source_module))
                                                .child(format!("Features: {feature_summary}"))
                                                .child(format!(
                                                    "Platforms: {}",
                                                    story.platforms.join(", ")
                                                ))
                                                .child(format!(
                                                    "Tests: {} declared gates",
                                                    story.test_requirements.len()
                                                ))
                                                .child(match story.fixture {
                                                    Some(fixture) => {
                                                        format!("Host fixture: {fixture}")
                                                    }
                                                    None => "Host fixture: none".to_owned(),
                                                })
                                                .child(format!("Docs: {}", story.documentation)),
                                        )
                                        .child(
                                            div()
                                                .pt_2()
                                                .flex()
                                                .flex_wrap()
                                                .items_center()
                                                .gap_2()
                                                .children(cases)
                                                .child(reset),
                                        )
                                        .child(
                                            div()
                                                .pt_2()
                                                .flex()
                                                .flex_wrap()
                                                .items_center()
                                                .gap_1()
                                                .child("Viewport")
                                                .children(viewport_buttons)
                                                .child("Motion")
                                                .children(motion_buttons),
                                        ),
                                )
                                .children(error)
                                .child(story_content),
                        ),
                ),
        )
    }
}

pub fn run(launch: GalleryLaunch) -> Result<(), String> {
    let reported = std::rc::Rc::new(std::cell::RefCell::new(None));
    let error = std::rc::Rc::clone(&reported);
    gpui_rhai::gpui_platform::application().run(move |cx: &mut App| {
        install(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1180.0), px(820.0)), cx);
        if let Err(open_error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            move |window, cx| cx.new(|cx| GalleryApp::new(launch, window, cx)),
        ) {
            *error.borrow_mut() = Some(open_error.to_string());
            cx.quit();
        }
        cx.activate(true);
    });
    match reported.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
