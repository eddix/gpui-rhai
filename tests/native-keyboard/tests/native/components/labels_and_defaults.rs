//! Labels and defaults of the official components follow their schema docs: a loading Button
//! shows and announces its loading text at the idle width, Avatar derives uppercased initials
//! by grapheme, Spinner turns on the theme's `ambient` duration unless given a speed, and the
//! Select and Combobox clear buttons share the localized `common.clear` name.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px};
use gpui_rhai::*;
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE,
    TOKEN_BASE_SOURCE, ZH_CN_LOCALE,
};

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

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main.to_owned())]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([
                ("en.rhai".into(), EN_LOCALE.to_owned()),
                ("zh_cn.rhai".into(), ZH_CN_LOCALE.to_owned()),
                ("ar.rhai".into(), AR_LOCALE.to_owned()),
            ])
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("labels", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("labels"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    visual.run_until_parked();
    visual.executor().advance_clock(Duration::from_millis(32));
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
}

fn last_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
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

fn names(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str) -> Vec<String> {
    nodes(visual, view)
        .into_iter()
        .filter(|node| node.role == role)
        .map(|node| node.name)
        .collect()
}

fn bounds_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> Vec<GeometryBounds> {
    nodes(visual, view)
        .into_iter()
        .filter(|node| node.role == role && node.name == name)
        .map(|node| node.geometry.unwrap().visual)
        .collect()
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    match bounds_of(visual, view, role, name).as_slice() {
        [bounds] => *bounds,
        other => panic!("expected one {role} named {name}, found {other:?}"),
    }
}

fn press(visual: &mut VisualTestContext, at: GeometryBounds) {
    #[allow(clippy::cast_possible_truncation)]
    let position = gpui::point(
        px((at.x + at.width / 2.0) as f32),
        px((at.y + at.height / 2.0) as f32),
    );
    visual.simulate_click(position, gpui::Modifiers::none());
    settle(visual);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.test_id.as_deref() == Some("log"))
        .unwrap()
        .name
        .trim()
        .to_owned()
}

#[gpui::test]
fn a_loading_button_shows_and_announces_its_loading_text(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/button" as button;
fn view(ctx) { row([button::Button(#{ text: "Deploy", loading: true, loading_text: "Deploying" })]) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(names(&mut visual, &view, "button"), ["Deploying"]);
    // The idle label only holds the width: it is neither painted nor announced as text.
    assert_eq!(names(&mut visual, &view, "text"), ["Deploying"]);
}

#[gpui::test]
fn a_loading_button_keeps_the_width_of_its_idle_label(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/button" as button;
fn view(ctx) { column([
    row([button::Button(#{ text: "Publish release" })]),
    row([button::Button(#{ text: "Publish release", loading: true, loading_text: "Saving" })]),
    row([button::Button(#{ text: "Working on it" })]),
    row([button::Button(#{ text: "Go", loading: true, loading_text: "Working on it" })]),
    row([button::Button(#{ text: "Save", loading: true })]),
    row([button::Button(#{ text: "Save" })]),
]) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let idle = bounds(&mut visual, &view, "button", "Publish release");
    let shorter = bounds(&mut visual, &view, "button", "Saving");
    assert!(
        (idle.width - shorter.width).abs() < 0.5 && (idle.height - shorter.height).abs() < 0.5,
        "idle {idle:?}, loading {shorter:?}"
    );
    let text = bounds(&mut visual, &view, "text", "Saving");
    let idle_text = bounds(&mut visual, &view, "text", "Publish release");
    assert!(
        text.width + 10.0 < idle_text.width,
        "{text:?} {idle_text:?}"
    );
    assert!(
        ((text.x + text.width / 2.0) - (shorter.x + shorter.width / 2.0)).abs() < 1.0,
        "the loading text is centered: {text:?} in {shorter:?}"
    );
    // A wider loading text widens the button to fit, as the idle button with that text.
    let wider = bounds_of(&mut visual, &view, "button", "Working on it");
    assert_eq!(wider.len(), 2, "{wider:?}");
    assert!((wider[0].width - wider[1].width).abs() < 0.5, "{wider:?}");
    // Without a loading text the label stays as it is.
    let plain = bounds_of(&mut visual, &view, "button", "Save");
    assert_eq!(plain.len(), 2, "{plain:?}");
    assert!((plain[0].width - plain[1].width).abs() < 0.5, "{plain:?}");
}

#[gpui::test]
fn a_loading_button_ignores_clicks(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/button" as button;
fn state_schema() { #{ fields: #{
    log: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } } } } }
fn deploy(ctx, payload) { ctx.set_state("log", `${ctx.get_state("log")} deploy`); }
fn cancel(ctx, payload) { ctx.set_state("log", `${ctx.get_state("log")} cancel`); }
fn view(ctx) { column([
    button::Button(#{ text: "Deploy", loading: true, loading_text: "Deploying", on_click: Fn("deploy") }),
    button::Button(#{ text: "Cancel", on_click: Fn("cancel") }),
    text(ctx.get_state("log")).test_id("log")]) }"#,
    );
    let loading = bounds(&mut visual, &view, "button", "Deploying");
    press(&mut visual, loading);
    let idle = bounds(&mut visual, &view, "button", "Cancel");
    press(&mut visual, idle);
    assert_eq!(log(&mut visual, &view), "cancel");
}

#[gpui::test]
fn avatar_initials_are_the_first_grapheme_of_the_name_uppercased(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        "import \"components/avatar\" as avatar;
fn view(ctx) { row([
    avatar::Avatar(#{ name: \"ada Lovelace\" }),
    avatar::Avatar(#{ name: \"e\u{301}mile Zola\" }),
    avatar::Avatar(#{ name: \"\u{1F469}\u{200D}\u{1F4BB} Dev\" }),
    avatar::Avatar(#{ name: \"Grace Hopper\", initials: \"gh\" }),
]) }",
    );
    assert_eq!(last_error(&mut visual, &view), None);
    // Caller initials are shown as written; only the derived ones are uppercased.
    assert_eq!(
        names(&mut visual, &view, "text"),
        ["A", "E\u{301}", "\u{1F469}\u{200D}\u{1F4BB}", "gh"]
    );
}

#[test]
fn a_spinner_without_speed_turns_on_the_ambient_duration() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/spinner").unwrap(),
        BUNDLED_COMPONENT_SOURCES_BY_ID
            .iter()
            .find(|(id, _)| *id == "components/spinner")
            .unwrap()
            .1
            .to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/spinner_speed.rhai",
            r#"import "components/spinner" as spinner;
fn view(ctx) { column([
    spinner::Spinner(#{ key: "unset", label: "Loading" }),
    spinner::Spinner(#{ key: "cleared", label: "Loading", speed_ms: () }),
    spinner::Spinner(#{ key: "set", label: "Loading", speed_ms: 600 }),
]) }"#,
        )
        .unwrap();
    let context = || {
        UiContext::new(
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "root"),
            Some("main".to_owned()),
            ExecutionPhase::Render,
            BTreeMap::new(),
        )
    };
    let ambient = context().motion_duration("ambient").unwrap();
    let root = engine.render_with_context(&compiled, context()).unwrap();
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("the spinners compose into a column");
    };
    let durations = children
        .iter()
        .map(|spinner| match &spinner.motions()[0] {
            MotionSource::Transition(spec) if spec.property == MotionProperty::Rotate => {
                spec.duration_ms
            }
            other => panic!("a spinner turns with a rotate transition, not {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(durations, [ambient, ambient, 600]);
}

#[gpui::test]
fn select_and_combobox_clear_buttons_share_the_localized_name(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/select" as select;
import "components/combobox" as combobox;
fn view(ctx) { column([
    select::Select(#{ key: "fruit", label: "Fruit", options: [#{ value: "a", label: "Apple" }],
        value: "a", open: false, query: "", clearable: true }),
    combobox::Combobox(#{ key: "tags", label: "Tags", options: [#{ value: "a", label: "Apple" }],
        selected: ["a"], open: false, query: "", clearable: true }),
    select::Select(#{ key: "named", label: "Named", options: [#{ value: "a", label: "Apple" }],
        value: "a", open: false, query: "", clearable: true, clear_label: "Reset fruit" }),
]).with_style(style().width(px(400)).height(px(300))) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(
        names(&mut visual, &view, "button"),
        ["Clear", "Clear", "Reset fruit"]
    );
    visual.update(|_, cx| assert!(view.select_locale("zh-CN", cx).unwrap()));
    settle(&mut visual);
    assert_eq!(
        names(&mut visual, &view, "button"),
        ["清除", "清除", "Reset fruit"]
    );
}
