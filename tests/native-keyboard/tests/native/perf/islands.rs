//! Cache islands keep what GPUI tracks per element across the frames that
//! change them.

use gpui::{Modifiers, MouseButton, TestAppContext, px, size};

use crate::support::{Mount, Step, bounds, center, settle, text_of};

/// A press on the page adds a hot node to the block of rows, which takes the
/// block out of its island; the row pressed keeps its click.
const PRESS_REPLANS: &str = r#"fn state_schema(){#{fields:#{
    pressed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    clicked:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn press(ctx,payload){ctx.set_state("pressed",true);}
fn noop(ctx,payload){()}
fn pick(ctx,payload){ctx.set_state("clicked",payload);}
fn rows(ctx){
    let result=[];
    for i in 0..12{result.push(row([text(`Row ${i}`),text("detail")]).with_key(`r${i}`)
        .accessibility_role("button").accessibility_label(`Row ${i}`).on_click_value(Fn("pick"),`r${i}`));}
    if ctx.get_state("pressed"){result.push(text("hot").with_key("hot").on("pointer_down",Fn("noop")));}
    result
}
fn view(ctx){
    column([
        text(ctx.get_state("clicked")).test_id("clicked"),
        column(rows(ctx)).with_key("block"),
    ]).with_style(style().width(px(360))).on("pointer_down",Fn("press"))
}"#;

#[gpui::test]
fn a_press_that_replans_the_islands_keeps_its_click(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = Mount::new(PRESS_REPLANS).mount(cx);
    visual.simulate_resize(size(px(500.0), px(500.0)));
    settle(&mut visual, Step::Tick, 2);
    let statistics = visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap();
    assert!(statistics.island_misses > 0, "the block is no island");
    let row = center(bounds(&mut visual, &view, "button", "Row 3"));
    visual.simulate_mouse_down(row, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Tick, 2);
    visual.simulate_mouse_up(row, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(text_of(&mut visual, &view, "clicked"), "r3");
}
