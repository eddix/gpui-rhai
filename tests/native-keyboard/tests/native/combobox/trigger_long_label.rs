//! A Select / Combobox with a fixed `width` keeps its chevron and clear button inside the
//! trigger however long the text it shows (#136): the selected label, several labels joined
//! in `multiple` mode, or the placeholder. The trigger text is one line that gives way first
//! and ends in an ellipsis; before the fix it was a flex item with the default
//! `min-width: auto`, so it could not shrink below its text and pushed the chevron (and the
//! clear button) past the trigger's end. Seen in oh-my-byted with width 200, size `sm` and
//! the label `Singapore-Central (SGALI)`.
//!
//! The test paints the `indicator` (chevron), `clear`, `value` and `placeholder` parts with a
//! marker background through `part_styles` (colour only, no layout change) and compares the
//! painted quads with the trigger's bounds (role `combobox`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, rgba};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE,
    TOKEN_BASE_SOURCE,
};

const CHEVRON: u32 = 0xff00_ffff;
const VALUE: u32 = 0x00ff_00ff;
const CLEAR: u32 = 0x0000_ffff;
const LONG: &str = "Singapore-Central (SGALI)";

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, main: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main)]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([("en.rhai".into(), EN_LOCALE.to_owned())])
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
        let host = ScriptViewHost::new("select", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("select"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    (visual, view)
}

/// (left, top, right, bottom) in logical pixels.
type Rect = (f64, f64, f64, f64);

fn trigger(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Rect {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("combobox", "Region")
            .next()
            .expect("no combobox named Region")
            .geometry
            .unwrap()
            .visual
    });
    (
        bounds.x,
        bounds.y,
        bounds.x + bounds.width,
        bounds.y + bounds.height,
    )
}

fn painted(visual: &mut VisualTestContext, color: u32) -> Vec<Rect> {
    let fill: gpui::Background = rgba(color).into();
    visual.update(|window, _| {
        let scale = f64::from(window.scale_factor());
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.background == fill)
            .map(|quad| {
                let b = quad.bounds;
                (
                    f64::from(b.origin.x.0) / scale,
                    f64::from(b.origin.y.0) / scale,
                    f64::from((b.origin.x + b.size.width).0) / scale,
                    f64::from((b.origin.y + b.size.height).0) / scale,
                )
            })
            .collect()
    })
}

fn script(component: &str, selection: &str, label: &str, value_style: &str) -> String {
    format!(
        r#"import "components/combobox" as combobox;
import "components/select" as select;
fn view(ctx) {{ column([
    {component}(#{{ key: "region", label: "Region", size: "sm", width: px(200),
        options: [#{{ value: "sg", label: "{label}" }}, #{{ value: "us", label: "US-East" }}],
        {selection}, open: false, query: "",
        part_styles: #{{ indicator: style().background(rgba({CHEVRON:#x})),
            clear: style().background(rgba({CLEAR:#x})),
            placeholder: style(){value_style}.background(rgba({VALUE:#x})),
            value: style(){value_style}.background(rgba({VALUE:#x})) }} }}),
]).with_style(style().width(px(480)).padding(px(16))) }}"#
    )
}

/// The chevron, the trigger text (value or placeholder) and `clears` clear buttons stay
/// inside the trigger, and the text ends before the buttons.
fn assert_contained_with(
    cx: &mut TestAppContext,
    component: &str,
    selection: &str,
    label: &str,
    value_style: &str,
    clears: usize,
) {
    let (mut visual, view) = mount(cx, script(component, selection, label, value_style));
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    let frame = trigger(&mut visual, &view);
    let chevron = painted(&mut visual, CHEVRON);
    let value = painted(&mut visual, VALUE);
    let clear = painted(&mut visual, CLEAR);
    println!(
        "{component} {selection} {label:?}: trigger {frame:?}, chevron {chevron:?}, value {value:?}, clear {clear:?}"
    );
    assert_eq!(chevron.len(), 1, "one chevron quad: {chevron:?}");
    assert_eq!(value.len(), 1, "one value quad: {value:?}");
    assert_eq!(clear.len(), clears, "clear quads: {clear:?}");
    let inside = |r: &Rect| {
        r.0 >= frame.0 - 0.5 && r.2 <= frame.2 + 0.5 && r.1 >= frame.1 - 0.5 && r.3 <= frame.3 + 0.5
    };
    assert!(
        inside(&chevron[0]) && inside(&value[0]) && clear.iter().all(inside),
        "{component} with {label:?}: trigger {frame:?}, chevron {:?} (right edge {:.1} vs {:.1}), value {:?}, clear {clear:?}",
        chevron[0],
        chevron[0].2,
        frame.2,
        value[0]
    );
    let first_button = clear
        .iter()
        .chain(&chevron)
        .map(|r| r.0)
        .fold(f64::MAX, f64::min);
    assert!(
        value[0].2 <= first_button + 0.5,
        "the text ends before the buttons: value {:?}, first button at {first_button:.1}",
        value[0]
    );
}

/// The chevron and the value stay inside the trigger.
fn assert_contained(
    cx: &mut TestAppContext,
    component: &str,
    selection: &str,
    label: &str,
    value_style: &str,
) {
    assert_contained_with(cx, component, selection, label, value_style, 0);
}

#[gpui::test]
fn control_a_short_label_fits(cx: &mut TestAppContext) {
    assert_contained(cx, "select::Select", r#"value: "sg""#, "SG", "");
}

#[gpui::test]
fn a_select_with_a_long_label_keeps_its_chevron_inside(cx: &mut TestAppContext) {
    assert_contained(cx, "select::Select", r#"value: "sg""#, LONG, "");
}

#[gpui::test]
fn a_combobox_with_a_long_label_keeps_its_chevron_inside(cx: &mut TestAppContext) {
    assert_contained(cx, "combobox::Combobox", r#"selected: ["sg"]"#, LONG, "");
}

/// Control for the fix: the same Select with `min_width(0)`, `whitespace_nowrap()` and
/// `text_ellipsis()` on the value through `part_styles`, the 0.2.0 workaround, stays inside.
#[gpui::test]
fn control_a_shrinkable_value_keeps_the_chevron_inside(cx: &mut TestAppContext) {
    assert_contained(
        cx,
        "select::Select",
        r#"value: "sg""#,
        LONG,
        ".min_width(px(0)).whitespace_nowrap().text_ellipsis()",
    );
}

#[gpui::test]
fn a_long_placeholder_keeps_the_chevron_inside(cx: &mut TestAppContext) {
    for component in ["select::Select", "combobox::Combobox"] {
        let selection = if component == "select::Select" {
            r#"value: (), placeholder: "Choose the region that serves this cluster""#
        } else {
            r#"selected: [], placeholder: "Choose the region that serves this cluster""#
        };
        assert_contained(cx, component, selection, LONG, "");
    }
}

#[gpui::test]
fn a_clearable_trigger_with_a_long_label_keeps_both_buttons_inside(cx: &mut TestAppContext) {
    assert_contained_with(
        cx,
        "select::Select",
        r#"value: "sg", clearable: true, clear_label: "Clear region""#,
        LONG,
        "",
        1,
    );
    assert_contained_with(
        cx,
        "combobox::Combobox",
        r#"selected: ["sg"], clearable: true, clear_label: "Clear region""#,
        LONG,
        "",
        1,
    );
}

#[gpui::test]
fn many_labels_in_multiple_mode_keep_the_buttons_inside(cx: &mut TestAppContext) {
    // Two option labels and an unknown stored value, joined into one trigger text.
    assert_contained_with(
        cx,
        "combobox::Combobox",
        r#"mode: "multiple", selected: ["sg", "us", "eu-west-central"], clearable: true"#,
        LONG,
        "",
        1,
    );
}
