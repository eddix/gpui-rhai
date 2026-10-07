//! A container that holds focus itself shows the 2px focus frame over its edge
//! (#118): an overlay panel with focus on the panel, not on a control inside.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, rgba,
};
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME};

/// A focus color no other part of the theme uses.
const RING: u32 = 0x00ff_00ff;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/button" as button;
import "components/dialog" as dialog;
import "components/menu" as menu;
import "components/sheet" as sheet;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800)).height(px(600))) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .theme_token_overrides(ThemeTokenOverrides {
                colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(RING))]),
                ..Default::default()
            })
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
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("frame", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("frame"), host.clone(), window, cx)
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
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

/// Quads painted with a visible border in the focus color: (border width, width, height).
fn frames(visual: &mut VisualTestContext) -> Vec<(f32, f32, f32)> {
    let color: gpui::Hsla = rgba(RING).into();
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_widths.left.0 > 0.0 && quad.border_color == color)
            .map(|quad| {
                (
                    quad.border_widths.left.0,
                    quad.bounds.size.width.0,
                    quad.bounds.size.height.0,
                )
            })
            .collect()
    })
}

fn panel_width(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str) -> f32 {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    #[allow(clippy::cast_possible_truncation)]
    tree.nodes()
        .find(|node| node.role == role)
        .and_then(|node| node.geometry)
        .map(|geometry| geometry.visual.width as f32)
        .unwrap_or_else(|| panic!("no {role} node"))
}

#[gpui::test]
fn a_dialog_panel_holding_focus_shows_the_frame(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "confirm", open: true, title: "Confirm",
            content: text("Continue?"),
            actions: [button::Button(#{ key: "ok", text: "OK", on_click: Fn("noop") })] })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    let found = frames(&mut visual);
    println!("panel {width}, frames {found:?}");
    let scale = visual.update(|window, _| window.scale_factor());
    assert!(
        found
            .iter()
            .any(|(border, w, _)| (*border - 2.0 * scale).abs() < 0.01
                && (*w - width * scale).abs() < 0.5),
        "the panel holds focus: a 2px frame the panel's width, got {found:?}"
    );
    // GPUI paints a border over the children, so the hairline must turn the focus
    // color too, or it would cover the frame's outer pixel.
    assert!(
        found.iter().any(
            |(border, w, _)| (*border - scale).abs() < 0.01 && (*w - width * scale).abs() < 0.5
        ),
        "the panel's own hairline takes the focus color, got {found:?}"
    );

    // Tab moves focus to the button: the button shows its own frame, the panel none.
    visual.simulate_keystrokes("tab");
    settle(&mut visual);
    let found = frames(&mut visual);
    println!("after tab {found:?}");
    assert!(
        !found
            .iter()
            .any(|(_, w, _)| (*w - width * scale).abs() < 0.5),
        "focus inside the panel: no panel frame, got {found:?}"
    );
    assert!(!found.is_empty(), "the focused button shows its frame");
}

#[gpui::test]
fn a_sheet_panel_holding_focus_shows_the_frame(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"sheet::Sheet(#{ key: "inspector", open: true, side: "end", title: "Inspector",
            content: text("Details"), initial_focus: "panel" })"#,
    );
    let width = panel_width(&mut visual, &view, "dialog");
    let scale = visual.update(|window, _| window.scale_factor());
    let found = frames(&mut visual);
    println!("panel {width}, frames {found:?}");
    assert!(
        found
            .iter()
            .any(|(border, w, _)| (*border - 2.0 * scale).abs() < 0.01
                && (*w - width * scale).abs() < 0.5),
        "the sheet holds focus: a 2px frame the panel's width, got {found:?}"
    );
}
