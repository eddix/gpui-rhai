//! Source-space vector graphics must share native drawable geometry with hits.

use gpui::{Modifiers, TestAppContext, point, px};
use gpui_rhai::MotionPreference;

use crate::support::{Mount, Step, bounds, by_test_id, dispatch, last_error, settle, snapshot};

#[gpui::test]
fn canvas_view_box_tracks_asymmetric_insets_rotation_and_relayout(cx: &mut TestAppContext) {
    for angle in [0, 90] {
        let script = r#"
            fn state_schema(){#{fields:#{wide:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
                hit:#{schema:#{type:"string"},"default":#{type:"string",value:"unseen"}}}}}
            fn hit(ctx,event){ctx.set_state("hit",if event.canvas_key==(){"none"}else{event.canvas_key});}
            fn resize(ctx,event){ctx.set_state("wide",true);}
            fn view(ctx){column([
                canvas(canvas_scene([canvas_rect("shape",12,4,8,6,theme_color("accent"))])
                    .view_box(0,0,24,24)).with_key("vector")
                    .with_style(style().width(px(if ctx.get_state("wide"){360}else{240})).height(px(150))
                        .padding_left(px(30)).padding_right(px(10)).padding_top(px(20)).padding_bottom(px(30)).border(px(5)))
                    .motion(motion_transition("rotate",ANGLE,ANGLE,#{duration_ms:1000,easing:"linear"}))
                    .accessibility_role("button").accessibility_label("Vector").on("pointer_down",Fn("hit")),
                text(ctx.get_state("hit")).accessibility_role("status").accessibility_label(ctx.get_state("hit")),
                text("Resize").test_id("resize").on_click(Fn("resize"))
            ])}
        "#.replace("ANGLE", &angle.to_string());
        let (mut visual, view) = Mount::new(script)
            .motion(MotionPreference::None)
            .id(&format!("canvas-view-box-{angle}"))
            .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        for wide in [false, true] {
            if wide {
                dispatch(&mut visual, &view, by_test_id("resize"), "click").unwrap();
                settle(&mut visual, Step::Frame, 3);
            }
            let outer = bounds(&mut visual, &view, "button", "Vector");
            let width = outer.width - 50.0; // border 10 + asymmetric padding 40
            let height = outer.height - 60.0;
            let scale = width.min(height) / 24.0;
            let center = (width / 2.0, height / 2.0);
            let offset = (4.0 * scale, -5.0 * scale); // source (16,7) about (12,12)
            let offset = if angle == 90 {
                (-offset.1, offset.0)
            } else {
                offset
            };
            #[allow(clippy::cast_possible_truncation)]
            let at = point(
                px((outer.x + 35.0 + center.0 + offset.0) as f32),
                px((outer.y + 25.0 + center.1 + offset.1) as f32),
            );
            visual.simulate_click(at, Modifiers::default());
            settle(&mut visual, Step::Frame, 2);
            assert!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("status", "shape")
                    .next()
                    .is_some(),
                "angle {angle}, wide {wide}"
            );
            #[allow(clippy::cast_possible_truncation)]
            let outside = point(px((outer.x + 7.0) as f32), px((outer.y + 7.0) as f32));
            visual.simulate_click(outside, Modifiers::default());
            settle(&mut visual, Step::Frame, 2);
            assert!(
                snapshot(&mut visual, &view)
                    .find_by_role_and_name("status", "none")
                    .next()
                    .is_some()
            );
            assert!(last_error(&mut visual, &view).is_none());
        }
    }
}
