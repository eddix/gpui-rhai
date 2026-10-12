//! A window drag area moves the window only when the Host allows it and the
//! press is not taken by a focusable control inside it. The test platform's
//! window move panics with `not implemented`, which is how these tests observe
//! it.

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::ScriptViewHandle;

use crate::support::{Mount, Step, bounds, settle};

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
    let (mut visual, view) = Mount::components(MAIN)
        .config(move |config| config.window_drag_areas(allowed))
        .id("window-drag")
        .mount(cx);
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
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
