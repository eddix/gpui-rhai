//! Region `scroll: true` draws ScrollArea's overlay scrollbar (#121): dragging along the
//! right edge scrolls the body, as it does a ScrollArea with the same content and size
//! (the control); the wheel precondition shows the body is a viewport.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

const ROWS: &str = r#"
fn rows() {
    let rows = [];
    for index in 0..12 {
        rows.push(text(`Row ${index}`).with_key(`${index}`)
            .with_style(style().height(px(28)).flex_shrink(false).items_center()));
    }
    column(rows).with_style(style().width(relative(1.0)))
}
"#;

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let mut modules = BTreeMap::new();
    let tables = [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
    ];
    for (id, source) in tables.iter().flat_map(|t| t.iter()) {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"
import "components/scroll_area" as scroll_area;
import "layouts/region" as region;
{ROWS}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(400)).height(px(400))) }}
"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("scroll", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("scroll"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    (visual, view)
}

fn visual_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> (f64, f64, f64, f64) {
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let g = snapshot
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .geometry
            .unwrap()
            .visual;
        (g.x, g.y, g.width, g.height)
    })
}

/// Hover the viewport, then drag along its right edge from top to bottom (where an overlay
/// scrollbar thumb sits). Returns Row 11's y before and after.
fn drag_right_edge(
    cx: &mut TestAppContext,
    view_body: &str,
    viewport_role: &str,
    viewport_name: &str,
) -> (f64, f64) {
    let (mut visual, view) = mount(cx, view_body);
    let (x, y, w, h) = visual_of(&mut visual, &view, viewport_role, viewport_name);
    let before = visual_of(&mut visual, &view, "text", "Row 11").1;
    // 24px in from the ends: a Region's body (the viewport) starts below its 12px inset,
    // so this is on the thumb in both, not on the viewport's top edge.
    let start = point(px((x + w - 6.0) as f32), px((y + 24.0) as f32));
    let end = point(px((x + w - 6.0) as f32), px((y + h - 24.0) as f32));
    visual.simulate_mouse_move(start, None, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let after = visual_of(&mut visual, &view, "text", "Row 11").1;
    eprintln!("{viewport_name}: Row 11 y {before} -> {after}");
    (before, after)
}

/// Wheel over the viewport. Returns Row 11's y before and after.
fn wheel(
    cx: &mut TestAppContext,
    view_body: &str,
    viewport_role: &str,
    viewport_name: &str,
) -> (f64, f64) {
    let (mut visual, view) = mount(cx, view_body);
    let (x, y, w, h) = visual_of(&mut visual, &view, viewport_role, viewport_name);
    let before = visual_of(&mut visual, &view, "text", "Row 11").1;
    visual.simulate_event(ScrollWheelEvent {
        position: point(px((x + w / 2.0) as f32), px((y + h / 2.0) as f32)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-200.0))),
        ..ScrollWheelEvent::default()
    });
    visual.run_until_parked();
    let after = visual_of(&mut visual, &view, "text", "Row 11").1;
    eprintln!("{viewport_name} wheel: Row 11 y {before} -> {after}");
    (before, after)
}

const REGION: &str = r#"box([region::Region(#{ label: "Logs", scroll: true, body: rows() })])
            .with_style(style().width(px(280)).height(px(112)).flex_col())"#;

#[gpui::test]
fn scroll_area_body_has_a_draggable_scrollbar(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (before, after) = drag_right_edge(
        cx,
        r#"scroll_area::ScrollArea(#{ key: "logs", label: "Logs", width: px(280), height: px(112), content: rows() })"#,
        "region",
        "Logs",
    );
    assert!(
        after < before,
        "ScrollArea: dragging the scrollbar should scroll (control)"
    );
}

#[gpui::test]
fn region_scroll_body_is_a_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (before, after) = wheel(cx, REGION, "region", "Logs");
    assert!(
        after < before,
        "Region scroll: true should scroll on the wheel (it does; precondition)"
    );
}

#[gpui::test]
fn region_scroll_body_has_a_draggable_scrollbar(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (before, after) = drag_right_edge(cx, REGION, "region", "Logs");
    assert!(
        after < before,
        "Region scroll: true: dragging the scrollbar should scroll the body; Row 11 stayed at {before}"
    );
}
