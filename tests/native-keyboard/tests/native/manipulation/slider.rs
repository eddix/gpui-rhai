//! Slider previews a drag natively and commits once, and reverses pointer and arrow
//! semantics in RTL.

use crate::support::{Mount, bounds, palette_texts};
use gpui::{Modifiers, MouseButton, TestAppContext, point, px};
use gpui_rhai_registry::AR_LOCALE;

#[gpui::test]
fn slider_previews_drag_natively_and_commits_once_before_keyboard_steps(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
                    import "components/slider" as slider;
                    fn state_schema() { #{ fields: #{
                        value: #{ schema: #{ type: "number", min: 0.0, max: 100.0 },
                            "default": #{ type: "float", value: 20.0 } },
                        commits: #{ schema: #{ type: "integer", min: 0 },
                            "default": #{ type: "integer", value: 0 } }
                    } } }
                    fn changed(ctx, value) {
                        ctx.set_state("value", value);
                        ctx.set_state("commits", ctx.get_state("commits") + 1);
                    }
                    fn view(ctx) {
                        column([
                            text(`slider:${ctx.get_state("value")}:${ctx.get_state("commits")}`),
                            slider::Slider(#{ key: "volume", label: "Volume",
                                value: ctx.get_state("value"), min: 0.0, max: 100.0,
                                step: 5.0, on_change: Fn("changed") })
                        ]).with_style(style().width(px(320)).padding(px(20)).gap(px(8)))
                    }
                "#,
    )
    .registry("components/slider")
    .bundled_assets()
    .ids("slider-window", "slider-view")
    .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let bounds = bounds(&mut visual, &view, "slider", "Volume");
    let at = |ratio: f64| {
        point(
            px((bounds.x + bounds.width * ratio) as f32),
            px((bounds.y + bounds.height / 2.0) as f32),
        )
    };
    visual.simulate_mouse_down(at(0.2), MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(at(0.4), MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(at(0.6), MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(at(0.8), MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(
        palette_texts(&mut visual, &view).contains(&"slider:20.0:0".to_owned()),
        "pointer moves must remain on the native preview path"
    );

    visual.simulate_mouse_up(at(0.8), MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"slider:80.0:1".to_owned()));

    visual.simulate_keystrokes("right");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"slider:85.0:2".to_owned()));
}

#[gpui::test]
fn slider_reverses_horizontal_pointer_and_arrow_semantics_in_rtl(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
                    import "components/slider" as slider;
                    fn state_schema() { #{ fields: #{
                        value: #{ schema: #{ type: "number", min: 0.0, max: 100.0 },
                            "default": #{ type: "float", value: 50.0 } }
                    } } }
                    fn init(ctx) { ctx.set_locale("ar"); }
                    fn changed(ctx, value) { ctx.set_state("value", value); }
                    fn view(ctx) {
                        column([
                            text(`rtl-slider:${ctx.get_state("value")}`),
                            slider::Slider(#{ key: "rtl-volume", label: "Volume",
                                value: ctx.get_state("value"), min: 0.0, max: 100.0,
                                step: 5.0, on_change: Fn("changed") })
                        ]).with_style(style().width(px(320)).padding(px(20)).gap(px(8)))
                    }
                "#,
    )
    .registry("components/slider")
    .locale("ar.rhai", AR_LOCALE)
    .bundled_assets()
    .ids("rtl-slider-window", "rtl-slider-view")
    .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let bounds = bounds(&mut visual, &view, "slider", "Volume");
    let left = point(
        px((bounds.x + bounds.width * 0.2) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(left, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(left, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"rtl-slider:80.0".to_owned()));

    visual.simulate_keystrokes("right");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"rtl-slider:75.0".to_owned()));
}
