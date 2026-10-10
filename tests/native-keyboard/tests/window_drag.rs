//! A window drag area moves the window only when the Host allows it and the
//! press is not taken by a focusable control inside it. The test platform's
//! window move panics with `not implemented`, which is how these tests observe
//! it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
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

const MAIN: &str = r#"
import "components/title_bar" as title_bar;
import "components/button" as button;
fn state_schema() { #{ fields: #{ clicks: #{ schema: #{ type: "integer" },
    "default": #{ type: "integer", value: 0 } } } } }
fn clicked(ctx, payload) { ctx.set_state("clicks", ctx.get_state("clicks") + 1); }
fn view(ctx) {
    column([
        title_bar::TitleBar(#{ label: "Window", title: "Title", window_drag: true,
            end: [button::Button(#{ text: "Act", on_click: Fn("clicked") })] }),
        text(`clicks ${ctx.get_state("clicks")}`).accessibility_role("status"),
    ])
}
"#;

fn mount(cx: &mut TestAppContext, allowed: bool) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    modules.insert(ModuleId::parse("main").unwrap(), MAIN.to_owned());
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        DEFAULT_THEME,
    )
    .token_base(TOKEN_BASE_SOURCE)
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
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("window-drag", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("window-drag").window_drag_areas(allowed),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    (visual, view)
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| node.role == role && node.name == name)
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no {role} {name}"))
        .visual
}

#[allow(clippy::cast_possible_truncation)]
fn press(visual: &mut VisualTestContext, x: f64, y: f64) {
    let at = point(px(x as f32), px(y as f32));
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
}

#[gpui::test]
fn a_control_inside_the_drag_area_keeps_its_press(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, true);
    let act = bounds(&mut visual, &view, "button", "Act");
    press(
        &mut visual,
        act.x + act.width / 2.0,
        act.y + act.height / 2.0,
    );
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert!(tree.nodes().any(|node| node.name == "clicks 1"));
}

#[gpui::test]
#[should_panic(expected = "not implemented")]
fn the_drag_area_background_moves_the_window(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, true);
    let bar = bounds(&mut visual, &view, "toolbar", "Window");
    let act = bounds(&mut visual, &view, "button", "Act");
    // Between the title and the button: the bar itself.
    press(&mut visual, act.x - 40.0, bar.y + bar.height / 2.0);
}

#[gpui::test]
fn an_embedded_view_cannot_move_the_window_unless_the_host_allows_it(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, false);
    let bar = bounds(&mut visual, &view, "toolbar", "Window");
    let act = bounds(&mut visual, &view, "button", "Act");
    press(&mut visual, act.x - 40.0, bar.y + bar.height / 2.0);
}
