//! Decorative motion and customization must share the native resize owner.

use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};

use crate::support::{
    Mount, ReplaceOnce, Step, bounds, by_test_id, dispatch, filled, last_error, settle,
};

const MARK: u32 = 0xff00_ffff;

fn source(orientation: &str, extra: &str) -> String {
    format!(
        r#"import "components/split_pane" as split;
import "app/grip_support" as palette;
fn custom_grip(ctx, detail) {{
    box([]).with_style(style().width(px(4)).height(px(20)))
        .signal_style(detail.state, #{{drag:style().background(palette::ink())}})
        .progress_motion(motion_signal(detail.state,["drag"],
            motion_transition("height",20.0,44.0,#{{duration_ms:80,easing:"linear"}})))
}}
fn view(ctx) {{
    split::SplitPane(#{{key:"panels",label:"Panels",size:0.5,orientation:"{orientation}",
        min_start:0.0,min_end:0.0,start:text("Start"),end:text("End"),
        part_styles:#{{grip_content:style().background(rgba({MARK:#x}))}},
        {extra}
    }}).with_style(style().width(px(420)).height(px(180)))
}}
"#
    )
}

fn mount(
    cx: &mut TestAppContext,
    orientation: &str,
    preference: MotionPreference,
    extra: &str,
) -> (VisualTestContext, ScriptViewHandle, ManualRuntimeClock) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(source(orientation, extra))
        .module("app/grip_support", "fn ink() { rgba(0x00ffffff) }")
        .with(move |view| view.runtime_clock(clock))
        .motion(preference)
        .id(&format!("grip-{orientation}-{preference:?}"))
        .mount(cx);
    settle(&mut visual, Step::Refresh, 5);
    (visual, view, manual)
}

fn length(visual: &mut VisualTestContext, horizontal: bool) -> f32 {
    let quads = filled(visual, MARK);
    assert_eq!(quads.len(), 1, "one decoration: {quads:?}");
    if horizontal {
        quads[0].bounds.height
    } else {
        quads[0].bounds.width
    }
}

fn pointer_at_separator(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> gpui::Point<gpui::Pixels> {
    let b = bounds(visual, view, "separator", "Panels");
    point(
        px((b.x + b.width / 2.0) as f32),
        px((b.y + b.height / 2.0) as f32),
    )
}

#[gpui::test]
fn default_grip_grows_and_reverses_through_intermediate_native_frames(cx: &mut TestAppContext) {
    for orientation in ["horizontal", "vertical"] {
        let horizontal = orientation == "horizontal";
        let (mut visual, view, manual) = mount(cx, orientation, MotionPreference::Normal, "");
        let initial = bounds(&mut visual, &view, "separator", "Panels");
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        assert!((length(&mut visual, horizontal) - 24.0).abs() < 0.5);
        let at = pointer_at_separator(&mut visual, &view);
        visual.simulate_mouse_move(at, None, Modifiers::none());
        settle(&mut visual, Step::Refresh, 3);
        assert!(
            (length(&mut visual, horizontal) - 24.0).abs() < 0.5,
            "must not jump at activation"
        );
        manual.advance(Duration::from_millis(50));
        settle(&mut visual, Step::Refresh, 3);
        let middle = length(&mut visual, horizontal);
        assert!(
            middle > 24.5 && middle < 39.5,
            "must present an intermediate length, got {middle}"
        );
        visual.simulate_mouse_move(point(px(20.0), px(20.0)), None, Modifiers::none());
        settle(&mut visual, Step::Refresh, 3);
        assert!(
            (length(&mut visual, horizontal) - middle).abs() < 0.5,
            "reverse from current sample"
        );
        manual.advance(Duration::from_millis(160));
        settle(&mut visual, Step::Refresh, 3);
        assert!((length(&mut visual, horizontal) - 24.0).abs() < 0.5);
        assert_eq!(
            bounds(&mut visual, &view, "separator", "Panels"),
            initial,
            "decoration must not resize the interaction lane"
        );
        assert_eq!(last_error(&mut visual, &view), None);
        let timings = visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings;
        assert!(
            timings.is_empty(),
            "native hover/motion must not execute Rhai: {timings:?}"
        );
    }
}

#[gpui::test]
fn reduced_and_none_grip_motion_snap_to_current_state(cx: &mut TestAppContext) {
    for preference in [MotionPreference::Reduced, MotionPreference::None] {
        let (mut visual, view, _) = mount(cx, "horizontal", preference, "");
        assert!((length(&mut visual, true) - 24.0).abs() < 0.5);
        let at = pointer_at_separator(&mut visual, &view);
        visual.simulate_mouse_move(at, None, Modifiers::none());
        settle(&mut visual, Step::Refresh, 4);
        assert!((length(&mut visual, true) - 40.0).abs() < 0.5);
        visual.simulate_mouse_move(point(px(20.0), px(20.0)), None, Modifiers::none());
        settle(&mut visual, Step::Refresh, 4);
        assert!((length(&mut visual, true) - 24.0).abs() < 0.5);
    }
}

#[gpui::test]
fn custom_factory_keeps_its_state_styles_and_native_motion(cx: &mut TestAppContext) {
    let (mut visual, view, manual) = mount(
        cx,
        "horizontal",
        MotionPreference::Normal,
        r#"handle_renderer: ctx.node_factory(Fn("custom_grip")),"#,
    );
    let at = pointer_at_separator(&mut visual, &view);
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(40));
    settle(&mut visual, Step::Refresh, 3);
    let decoration = filled(&mut visual, 0x00ff_ffff);
    assert_eq!(
        decoration.len(),
        1,
        "factory's own state style must not be overwritten"
    );
    assert!(
        (decoration[0].bounds.height - 32.0).abs() < 1.0,
        "{decoration:?}"
    );
    assert!((length(&mut visual, true) - 32.0).abs() < 1.0);
    assert_eq!(last_error(&mut visual, &view), None);
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
}

#[gpui::test]
fn static_replacement_and_no_grip_keep_separator_semantics(cx: &mut TestAppContext) {
    let (mut visual, view, _) = mount(
        cx,
        "vertical",
        MotionPreference::Normal,
        "handle:box([]).with_style(style().width(px(36)).height(px(6))),",
    );
    assert!((length(&mut visual, false) - 36.0).abs() < 0.5);
    assert_eq!(last_error(&mut visual, &view), None);
    let (mut visual, view, _) = mount(cx, "horizontal", MotionPreference::Normal, "grip:false,");
    assert!(filled(&mut visual, MARK).is_empty());
    assert!(bounds(&mut visual, &view, "separator", "Panels").height > 100.0);
}

#[gpui::test]
fn imported_component_factory_preserves_private_helpers_imports_and_user_curry(
    cx: &mut TestAppContext,
) {
    let factory = source(
        "horizontal",
        r#"handle_renderer: ctx.node_factory(Fn("custom_grip").curry(6.0)),"#,
    )
    .replace_once(
        "fn custom_grip(ctx, detail)",
        "private fn custom_grip(width, ctx, detail)",
    )
    .replace_once(
        "style().width(px(4)).height(px(20))",
        "style().width(px(width)).height(px(20))",
    )
    .replace_once("fn view(ctx)", "fn render_Shell(ctx, props)");
    let component = format!(
        r#"{factory}
define_component(#{{metadata:#{{id:"app/shell", "export":"Shell",version:"0.2.2",
runtime_api:#{{min_inclusive:3,max_exclusive:4}},dependencies:[],capabilities:#{{}}}},
schema:#{{props:#{{key:#{{schema:#{{type:"string"}},required:true,sensitive:false}}}},
state:#{{fields:#{{}}}},events:#{{}},slots:#{{}},parts:["root"]}},render:Fn("render_Shell")}});
fn Shell(props) {{ render_component("app/shell",props) }}
"#
    );
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(
        r#"import "app/shell" as shell;
fn view(ctx){shell::Shell(#{key:"shell"})}"#,
    )
    .module("app/shell", component)
    .module("app/grip_support", "fn ink(){rgba(0x00ffffff)}")
    .with(move |view| view.runtime_clock(clock))
    .motion(MotionPreference::Normal)
    .id("imported-grip-factory")
    .mount(cx);
    settle(&mut visual, Step::Refresh, 5);
    let at = pointer_at_separator(&mut visual, &view);
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(40));
    settle(&mut visual, Step::Refresh, 3);
    let quads = filled(&mut visual, 0x00ff_ffff);
    assert_eq!(quads.len(), 1);
    assert!(
        (quads[0].bounds.width - 6.0).abs() < 0.5,
        "user curry must remain intact: {quads:?}"
    );
    assert!(
        (quads[0].bounds.height - 32.0).abs() < 1.0,
        "native factory motion: {quads:?}"
    );
    assert_eq!(last_error(&mut visual, &view), None);
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
}

#[gpui::test]
fn same_named_factories_in_helper_modules_do_not_reuse_the_wrong_decoration(
    cx: &mut TestAppContext,
) {
    let main = r#"import "components/split_pane" as split;
import "app/a" as a; import "app/b" as b;
fn state_schema(){#{fields:#{choice:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
fn choose(ctx,p){ctx.set_state("choice",!ctx.get_state("choice"));}
fn view(ctx){column([
 text("Choose").test_id("choose").on_click(Fn("choose")),
 split::SplitPane(#{key:"panels",label:"Panels",size:0.5,min_start:0.0,min_end:0.0,
 start:text("A"),end:text("B"),handle_renderer:if ctx.get_state("choice"){b::factory(ctx)}else{a::factory(ctx)}})
 .with_style(style().width(px(420)).height(px(180)))])}
"#;
    let helper = |color| {
        format!(
            r#"fn factory(ctx){{ctx.node_factory(Fn("decoration"))}}
private fn decoration(ctx,detail){{box([]).with_style(style().width(px(6)).height(px(20)).background(rgba({color:#x})))}}"#
        )
    };
    let (mut visual, view) = Mount::components(main)
        .module("app/a", helper(0xff00_ffff_u32))
        .module("app/b", helper(0x00ff_ffff_u32))
        .id("factory-context-reuse")
        .mount(cx);
    settle(&mut visual, Step::Refresh, 4);
    assert_eq!(filled(&mut visual, 0xff00_ffff).len(), 1);
    dispatch(&mut visual, &view, by_test_id("choose"), "click").unwrap();
    settle(&mut visual, Step::Refresh, 4);
    assert!(filled(&mut visual, 0xff00_ffff).is_empty());
    assert_eq!(filled(&mut visual, 0x00ff_ffff).len(), 1);
    assert_eq!(last_error(&mut visual, &view), None);
}
