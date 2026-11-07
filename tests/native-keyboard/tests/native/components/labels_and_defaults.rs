//! Labels and defaults of the official components follow their schema docs: a loading Button
//! shows and announces its loading text and a spinner (in place of its prefix, or before the
//! label), IconButton swaps its icon for the spinner, Avatar derives uppercased initials
//! by grapheme, Spinner turns on the theme's `ambient` duration unless given a speed, and the
//! Select and Combobox clear buttons share the localized `common.clear` name.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE, ZH_CN_LOCALE};

use crate::support::{Mount, Step, click_center, last_error, nodes, registry_source, text_of};

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .locale("en.rhai", EN_LOCALE)
        .locale("zh_cn.rhai", ZH_CN_LOCALE)
        .locale("ar.rhai", AR_LOCALE)
        .motion(MotionPreference::None)
        .id("labels")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Tick, 1);
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
    click_center(visual, at);
    settle(visual);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
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
fn a_loading_button_adds_a_spinner_to_the_width_of_its_idle_label(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/button" as button;
import "components/icon_button" as icon_button;
fn icon() { image_source(asset("app/icons/check"))
    .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))) }
fn view(ctx) { column([
    row([button::Button(#{ text: "Publish release" })]),
    row([button::Button(#{ text: "Publish release", loading: true, loading_text: "Saving" })]),
    row([button::Button(#{ text: "Working on it" })]),
    row([button::Button(#{ text: "Go", loading: true, loading_text: "Working on it" })]),
    row([button::Button(#{ text: "Save", loading: true })]),
    row([button::Button(#{ text: "Save" })]),
    row([button::Button(#{ text: "Sync", prefix: icon() })]),
    row([button::Button(#{ text: "Sync", loading: true, prefix: icon() })]),
    row([icon_button::IconButton(#{ label: "Refresh", icon: image_source(asset("app/icons/check")) })]),
    row([icon_button::IconButton(#{ label: "Refresh", loading: true, icon: image_source(asset("app/icons/check")) })]),
]) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let spinners = bounds_of(&mut visual, &view, "presentation", "");
    let icon_slot = |bounds: &GeometryBounds| bounds.width > 10.0 && bounds.width < 20.0;
    let slots: Vec<_> = spinners.iter().filter(|bounds| icon_slot(bounds)).collect();
    assert_eq!(
        slots.len(),
        5,
        "one spinner per loading button: {spinners:?}"
    );
    let slot = slots[0].width;

    // The spinner and the gap after it are what loading adds; the label keeps the wider of
    // the idle and loading texts, centered.
    let idle = bounds(&mut visual, &view, "button", "Publish release");
    let shorter = bounds(&mut visual, &view, "button", "Saving");
    let added = shorter.width - idle.width;
    assert!(
        added > slot && added < slot + 12.0,
        "idle {idle:?}, loading {shorter:?}"
    );
    assert!(
        (idle.height - shorter.height).abs() < 0.5,
        "{idle:?} {shorter:?}"
    );
    let wider = bounds_of(&mut visual, &view, "button", "Working on it");
    assert_eq!(wider.len(), 2, "{wider:?}");
    assert!(
        (wider[1].width - wider[0].width - added).abs() < 0.5,
        "{wider:?}"
    );
    let plain = bounds_of(&mut visual, &view, "button", "Save");
    assert_eq!(plain.len(), 2, "{plain:?}");
    assert!(
        (plain[0].width - plain[1].width - added).abs() < 0.5,
        "{plain:?}"
    );
    // The spinner sits before the label.
    let text = bounds(&mut visual, &view, "text", "Saving");
    assert!(
        slots[0].x + slots[0].width <= text.x,
        "{:?} {text:?}",
        slots[0]
    );

    // In place of a prefix icon, or of an IconButton's icon, the width holds.
    for name in ["Sync", "Refresh"] {
        let pair = bounds_of(&mut visual, &view, "button", name);
        assert_eq!(pair.len(), 2, "{pair:?}");
        assert!(
            (pair[0].width - pair[1].width).abs() < 0.5,
            "{name}: {pair:?}"
        );
    }
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
        registry_source("components/spinner").to_owned(),
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
