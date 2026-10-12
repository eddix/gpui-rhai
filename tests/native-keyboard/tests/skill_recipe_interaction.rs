//! The recipes in the gpui-rhai skill, used past their first frame. `check` lints every call
//! in a recipe but runs only the first frame, so the branches a user reaches by pressing (a
//! form's review, confirm and toast, a help popover, a table's paging) never ran in a test
//! (PR #111 review, U5). Each recipe is mounted as an agent copies it from the installed
//! skill; every text field gets text, then every enabled control is pressed once, the last
//! one in the semantics tree first, so what a press opens (a dialog, a panel, a menu) is
//! pressed before the page under it. No press may leave the view failed.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px};
use gpui_rhai::*;
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, BUNDLED_SKILL_FILES,
    BUNDLED_THEME_SOURCES, DEFAULT_THEME, EN_LOCALE, TOKEN_BASE_SOURCE, ZH_CN_LOCALE,
};

/// Roles a press acts on.
const PRESSABLE: &[&str] = &[
    "button", "checkbox", "switch", "radio", "tab", "menuitem", "option", "combobox", "link",
];
/// Presses per recipe at most; every recipe needs far fewer.
const MAX_PRESSES: usize = 200;
const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 900.0;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let element = if self.view.state() == ScriptViewState::Active {
            self.view.element().unwrap()
        } else {
            gpui::div().into_any_element()
        };
        self.host.container(element)
    }
}

/// (title, Rhai source) of every recipe, as the installed skill has them.
fn recipes() -> Vec<(String, String)> {
    let page = BUNDLED_SKILL_FILES
        .iter()
        .find(|(path, _)| *path == "gpui-rhai/references/recipes.md")
        .expect("the skill bundles its recipes")
        .1;
    page.split("\n## ")
        .skip(1)
        .map(|section| {
            let title = section.lines().next().unwrap().trim().to_owned();
            let source = section
                .split("```rhai\n")
                .nth(1)
                .and_then(|block| block.split("```").next())
                .unwrap_or_else(|| panic!("recipe {title} has no Rhai block"));
            (title, source.to_owned())
        })
        .collect()
}

fn mount(cx: &mut TestAppContext, source: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), source.to_owned())]);
    for (id, module) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(BUNDLED_CHART_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*module).to_owned());
    }
    // An installed project serves the registry icons under `app/`; the components ask for
    // them without it.
    let assets = BUNDLED_ASSET_SOURCES.iter().flat_map(|(path, svg)| {
        let id = path.trim_end_matches(".svg");
        [id.to_owned(), format!("app/{id}")].map(|id| {
            (
                id,
                AssetData {
                    mime_type: "image/svg+xml".into(),
                    bytes: svg.as_bytes().to_vec(),
                },
            )
        })
    });
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .theme_sources(
                BUNDLED_THEME_SOURCES
                    .iter()
                    .map(|(file, theme)| ((*file).to_owned(), (*theme).to_owned())),
            )
            .locale_sources([
                ("en.rhai".into(), EN_LOCALE.to_owned()),
                ("zh_cn.rhai".into(), ZH_CN_LOCALE.to_owned()),
                ("ar.rhai".into(), AR_LOCALE.to_owned()),
            ])
            .motion_preference(MotionPreference::None)
            .asset_sources(assets)
            .prepare()
            .unwrap_or_else(|error| panic!("the recipe prepares: {error}"));
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("recipe", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("recipe"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(px(WIDTH), px(HEIGHT)));
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn nodes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<AccessibilityNode> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .cloned()
            .collect()
    })
}

fn failure(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
}

/// The middle of a node that is on screen and can take a press.
fn target(node: &AccessibilityNode) -> Option<gpui::Point<gpui::Pixels>> {
    let bounds = node.geometry?.visual;
    let (x, y) = (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    #[allow(clippy::cast_possible_truncation)]
    (bounds.width > 0.0
        && bounds.height > 0.0
        && (0.0..f64::from(WIDTH)).contains(&x)
        && (0.0..f64::from(HEIGHT)).contains(&y))
    .then(|| gpui::point(px(x as f32), px(y as f32)))
}

struct Run {
    pressed: Vec<String>,
    typed: Vec<String>,
    /// Every name the semantics tree showed at some point.
    seen: BTreeSet<String>,
    failures: Vec<String>,
}

fn exercise(cx: &mut TestAppContext, title: &str, source: &str) -> Run {
    let (mut visual, view) = mount(cx, source);
    let mut run = Run {
        pressed: Vec::new(),
        typed: Vec::new(),
        seen: BTreeSet::new(),
        failures: Vec::new(),
    };
    if let Some(error) = failure(&mut visual, &view) {
        run.failures.push(format!("{title}: first frame: {error}"));
        return run;
    }
    // Text first, so a form's validation lets its submit through.
    let fields: Vec<_> = nodes(&mut visual, &view)
        .into_iter()
        .filter(|node| node.role == "text_field" && !node.disabled && !node.read_only)
        .filter_map(|node| Some((node.name.clone(), target(&node)?)))
        .collect();
    for (name, at) in fields {
        visual.simulate_click(at, gpui::Modifiers::none());
        settle(&mut visual);
        visual.simulate_input("Ada");
        settle(&mut visual);
        run.typed.push(name.clone());
        if let Some(error) = failure(&mut visual, &view) {
            run.failures
                .push(format!("{title}: typing into {name:?}: {error}"));
            return run;
        }
    }
    let mut done = BTreeSet::new();
    for _ in 0..MAX_PRESSES {
        let tree = nodes(&mut visual, &view);
        run.seen.extend(tree.iter().map(|node| node.name.clone()));
        let next = tree.iter().rev().find_map(|node| {
            let key = (node.role.clone(), node.name.clone());
            (PRESSABLE.contains(&node.role.as_str()) && !node.disabled && !done.contains(&key))
                .then(|| Some((key, target(node)?)))
                .flatten()
        });
        let Some((key, at)) = next else { break };
        visual.simulate_click(at, gpui::Modifiers::none());
        settle(&mut visual);
        run.pressed.push(format!("{} {:?}", key.0, key.1));
        if let Some(error) = failure(&mut visual, &view) {
            run.failures
                .push(format!("{title}: pressing {} {:?}: {error}", key.0, key.1));
            return run;
        }
        done.insert(key);
    }
    run.seen
        .extend(nodes(&mut visual, &view).into_iter().map(|node| node.name));
    run
}

#[gpui::test]
fn every_recipe_survives_pressing_every_control(cx: &mut TestAppContext) {
    let recipes = recipes();
    assert_eq!(recipes.len(), 4, "the skill has four recipes");
    let mut failures = Vec::new();
    let mut form_saved = false;
    for (title, source) in &recipes {
        let run = exercise(cx, title, source);
        println!(
            "{title}: typed {:?}; pressed {} controls: {:?}",
            run.typed,
            run.pressed.len(),
            run.pressed
        );
        assert!(
            run.pressed.len() >= 3 || !run.failures.is_empty(),
            "{title}: pressed only {:?}",
            run.pressed
        );
        if title.starts_with("Form") {
            form_saved = run.seen.contains("Profile saved");
        }
        failures.extend(run.failures);
    }
    assert!(failures.is_empty(), "{failures:#?}");
    // The branch the review named: Review opens the dialog, Confirm shows the toast.
    assert!(form_saved, "the form recipe never reached its toast");
}
