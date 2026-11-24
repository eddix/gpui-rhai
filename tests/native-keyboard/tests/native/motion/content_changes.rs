//! Content keys use the shared runtime; they do not animate an initial mount.

use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};

use crate::support::{
    ActiveHost, Mount, Step, bounds, by_role, click_center, dispatch, last_error, settle,
};

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn marker_paint(visual: &mut VisualTestContext) -> (f32, f32) {
    fn scan(scene: &gpui::Scene, alpha: f32, sigma: f32, result: &mut (f32, f32)) {
        for color in scene
            .quads
            .iter()
            .filter_map(|quad| quad.background.as_solid())
        {
            if color.s > 0.99 && (color.l - 0.5).abs() < 0.001 && color.h < 0.001 {
                result.0 = result.0.max(alpha * color.a);
                result.1 = result.1.max(sigma);
            }
        }
        for group in &scene.isolated_groups {
            scan(
                group.source(),
                alpha * group.opacity(),
                sigma.max(group.source_blur_sigma()),
                result,
            );
        }
        #[cfg(feature = "gpui-fast")]
        for layer in scene
            .layers
            .frames
            .iter()
            .filter(|layer| layer.tile_size == 0)
        {
            if let Some(child) = layer.content.scene() {
                scan(child, alpha, sigma, result);
            }
        }
    }
    visual.update(|window, _| {
        window.visual_probe_with_painted_scene(|scene| {
            let mut result = (0., 0.);
            scan(scene, 1., 0., &mut result);
            (result.0, result.1 / window.scale_factor())
        })
    })
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn marker_alpha(visual: &mut VisualTestContext) -> f32 {
    marker_paint(visual).0
}

#[gpui::test]
fn number_motion_changes_painted_width_without_script_frames_or_transient_accessibility(
    cx: &mut TestAppContext,
) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(r#"
        fn view(ctx) {
            column([ctx.number_text(50000.0, 0, "$", "", false).with_key("total")
                .motion(motion_transition("number_value", 0.0, 50000.0, #{duration_ms:400,easing:"linear"}))
                .with_style(style().font_size(px(20)).font_feature("tnum",1))])
                .with_style(style().items_start())
        }
    "#).motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock)).mount(cx);
    settle(&mut visual, Step::Refresh, 3);
    let start = bounds(&mut visual, &view, "text", "$50,000").width;
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    manual.advance(Duration::from_millis(100));
    settle(&mut visual, Step::Refresh, 3);
    let middle = bounds(&mut visual, &view, "text", "$50,000").width;
    manual.advance(Duration::from_millis(350));
    settle(&mut visual, Step::Refresh, 3);
    let end = bounds(&mut visual, &view, "text", "$50,000").width;
    assert!(
        start + 10. < middle && middle <= end,
        "{start}, {middle}, {end}"
    );
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn number_motion_projects_one_live_distribution_even_when_retargeted_mid_frame(
    cx: &mut TestAppContext,
) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(r##"
        define_component(#{
            metadata:#{id:"tests/number_lanes","export":"Lanes",version:"0.1.0",
                runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                state:#{fields:#{stage:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},
                events:#{},slots:#{},parts:["root"]},render:Fn("lanes_view")});
        fn next(ctx,event){ctx.set_state("stage",ctx.get_state("stage")+1);}
        fn lane(key,reference,lanes,index,amount,total){
            box([]).with_key(key).with_ref(reference).number_motion_source()
                .accessibility_role("group").accessibility_label(key)
                .with_style(style().height(px(16)).min_width(px(0)).width(relative(amount/total)).flex_shrink(false))
                .motion(motion_transition("number_value",0.0,amount,#{duration_ms:400,easing:"linear"}))
                .sample_numbers(lanes,"width_fraction",index)
        }
        fn lanes_view(ctx,p){
            let stage=ctx.get_state("stage");let a=element_ref("a");let b=element_ref("b");
            let lanes=motion_numbers([a,b]);
            if stage>=3{lanes=lanes.with_capacity(160.0);}
            let weights=if stage==0{[60.0,40.0]}else if stage==1{[60.0,240.0]}else{[30.0,30.0]};
            let total=weights[0]+weights[1];
            column([
                row([lane("first",a,lanes,0,weights[0],total),lane("second",b,lanes,1,weights[1],total)])
                    .with_style(style().width(px(200))),
                ctx.number_text(total,0,"$","",false).sample_numbers(lanes,"sum",0),
                text("Next").accessibility_role("button").accessibility_label("Next").on_click(Fn("next"))
            ]).with_key(p.key)
        }
        fn view(ctx){render_component("tests/number_lanes",#{key:"lanes"})}
    "##).motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock)).mount(cx);
    let first_width = |visual: &mut VisualTestContext| {
        let first = bounds(visual, &view, "group", "first");
        let second = bounds(visual, &view, "group", "second");
        assert!((first.width + second.width - 200.0).abs() < 1.0);
        first.width
    };
    manual.advance(Duration::from_millis(450));
    settle(&mut visual, Step::Refresh, 3);
    assert!((first_width(&mut visual) - 120.0).abs() < 1.0);
    let target = bounds(&mut visual, &view, "button", "Next");
    click_center(&mut visual, target);
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(200));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        crate::support::snapshot(&mut visual, &view)
            .find_by_role_and_name("text", "$300")
            .next()
            .is_some()
    );
    let middle = first_width(&mut visual);
    assert!((middle - 60.0).abs() < 1.0, "{middle}");
    click_center(&mut visual, target);
    settle(&mut visual, Step::Refresh, 3);
    assert!((first_width(&mut visual) - middle).abs() < 1.0);
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    manual.advance(Duration::from_millis(200));
    settle(&mut visual, Step::Refresh, 3);
    let retargeted = first_width(&mut visual);
    assert!(
        (retargeted - 200.0 * 45.0 / 130.0).abs() < 1.0,
        "{retargeted}"
    );
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    // The same sampled lanes can leave a quota remainder rather than
    // stretching to fill. This still uses normal GPUI layout, not a paint mask.
    click_center(&mut visual, target);
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(450));
    settle(&mut visual, Step::Refresh, 3);
    for lane in ["first", "second"] {
        let w = bounds(&mut visual, &view, "group", lane).width;
        assert!((w - 37.5).abs() < 1.0, "{lane}: {w}");
    }
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn fraction_translation_uses_laid_out_axes_and_does_not_move_siblings(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(r#"
        fn view(ctx) { column([
            box([]).with_key("moving").accessibility_role("group").accessibility_label("Moving box")
                .with_style(style().height(px(40)).width(px(90)).background(rgb(0xff0000)))
                .motion(motion_transition("translate_x_fraction",-0.20,0.0,#{duration_ms:200,easing:"linear"}))
                .motion(motion_transition("translate_y_fraction",0.25,0.0,#{duration_ms:200,easing:"linear"})),
            box([]).with_key("sibling").accessibility_role("group").accessibility_label("Sibling box")
                .with_style(style().height(px(20)).width(px(90)))
        ]).with_style(style().padding(px(16)).gap(px(16))) }
    "#).motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock)).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let presented = |visual: &mut VisualTestContext| {
        let moving = crate::support::nodes(visual, &view)
            .into_iter()
            .find(|node| node.name == "Moving box")
            .unwrap()
            .geometry
            .unwrap();
        let paint = crate::support::filled(visual, 0xff00_00ff)
            .first()
            .unwrap()
            .bounds;
        assert!((f64::from(paint.x) - moving.visual.x).abs() < 0.5);
        assert!((f64::from(paint.y) - moving.visual.y).abs() < 0.5);
        moving
    };
    let first = presented(&mut visual);
    let sibling = bounds(&mut visual, &view, "group", "Sibling box");
    assert!((first.visual.x - first.layout.x + 18.).abs() < 0.5);
    assert!((first.visual.y - first.layout.y - 10.).abs() < 0.5);
    manual.advance(Duration::from_millis(100));
    settle(&mut visual, Step::Refresh, 3);
    let half = presented(&mut visual);
    assert!((half.visual.x - half.layout.x + 9.).abs() < 0.5);
    assert!((half.visual.y - half.layout.y - 5.).abs() < 0.5);
    assert_eq!(bounds(&mut visual, &view, "group", "Sibling box"), sibling);
    assert_eq!(last_error(&mut visual, &view), None);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[track_caller]
fn assert_alpha(visual: &mut VisualTestContext, expected: f32) {
    let actual = marker_alpha(visual);
    assert!((actual - expected).abs() < 0.005, "{actual} != {expected}");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn button_label_changes_fade_natively_without_mount_or_reflow_animation(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let loading_alpha = if name.starts_with("clean") {
            0.50
        } else {
            0.55
        };
        for policy in [
            MotionPreference::Normal,
            MotionPreference::Reduced,
            MotionPreference::None,
        ] {
            let manual = ManualRuntimeClock::new(Instant::now());
            let clock = manual.clock();
            let (mut visual, view) = Mount::components(r#"
                import "components/button" as button;
                fn state_schema(){#{fields:#{
                    loading:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
                    moved:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
                }}}
                fn start(ctx,v){ctx.set_state("loading",true);}
                fn finish(ctx,v){ctx.set_state("loading",false);}
                fn reflow(ctx,v){ctx.set_state("moved",true);}
                fn view(ctx){column([
                    button::Button(#{key:"query",text:"Run query",loading:ctx.get_state("loading"),
                        loading_text:"Querying…",on_click:Fn("start"),
                        part_styles:#{label:style().background(rgb(0xff0000))}}),
                    text("Finish").accessibility_role("button").accessibility_label("Finish").on_click(Fn("finish")),
                    text("Reflow").accessibility_role("button").accessibility_label("Reflow").on_click(Fn("reflow"))
                ]).with_style(style().width(px(600)).items_start().gap(px(12)).padding(px(20))
                    .padding_start(px(if ctx.get_state("moved"){64}else{32})))}
            "#).theme(theme).motion(policy)
                .with(move |view| view.runtime_clock(clock)).id("button-content-change").mount(cx);
            settle(&mut visual, Step::Frame, 4);
            assert_alpha(&mut visual, 1.0);
            assert_eq!(
                marker_paint(&mut visual).1,
                0.,
                "no filter on initial mount"
            );
            let button = bounds(&mut visual, &view, "button", "Run query");
            click_center(&mut visual, button);
            settle(&mut visual, Step::Frame, 3);
            bounds(&mut visual, &view, "button", "Querying…");
            assert_alpha(
                &mut visual,
                if policy == MotionPreference::Normal {
                    0.0
                } else {
                    loading_alpha
                },
            );
            let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
            visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
            manual.advance(Duration::from_millis(240));
            settle(&mut visual, Step::Refresh, 3);
            if policy == MotionPreference::Normal {
                let (alpha, sigma) = marker_paint(&mut visual);
                assert!(
                    (0.36..loading_alpha).contains(&alpha),
                    "{name}/{policy:?}: {alpha}"
                );
                let expected_sigma = 3. * (1. - alpha / loading_alpha);
                assert!((0.0..3.).contains(&sigma) && sigma > 0.);
                assert!(
                    (sigma - expected_sigma).abs() < 0.025,
                    "{name}: {sigma} != {expected_sigma}"
                );
            } else {
                assert_alpha(&mut visual, loading_alpha);
                assert_eq!(
                    marker_paint(&mut visual).1,
                    0.,
                    "{name}/{policy:?}: decorative filter"
                );
            }
            assert_eq!(
                visual.update(|_, cx| view.committed_revision(cx).unwrap()),
                revision
            );
            assert!(
                visual
                    .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                    .timings
                    .is_empty()
            );
            manual.advance(Duration::from_millis(240));
            settle(&mut visual, Step::Refresh, 3);
            assert_alpha(&mut visual, loading_alpha);
            assert_eq!(
                marker_paint(&mut visual).1,
                0.,
                "settled labels must not keep a filter"
            );
            let before = bounds(&mut visual, &view, "button", "Querying…");
            dispatch(&mut visual, &view, by_role("button", "Reflow"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            assert_alpha(&mut visual, loading_alpha);
            let after = bounds(&mut visual, &view, "button", "Querying…");
            assert!(
                (after.x - before.x - 32.0).abs() < 0.5,
                "{before:?} {after:?}"
            );
            dispatch(&mut visual, &view, by_role("button", "Finish"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            assert_alpha(
                &mut visual,
                if policy == MotionPreference::Normal {
                    0.0
                } else {
                    1.0
                },
            );
            manual.advance(Duration::from_millis(480));
            settle(&mut visual, Step::Refresh, 4);
            assert_alpha(&mut visual, 1.0);
            let mut scheduled = Vec::new();
            for _ in 0..3 {
                scheduled.push(visual.update(|window, cx| window.simulate_next_frame(cx)));
                visual.run_until_parked();
            }
            assert_eq!(
                scheduled.last(),
                Some(&0),
                "settled labels must idle: {scheduled:?}"
            );
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn button_loading_width_and_following_sibling_use_the_same_constrained_frame(
    cx: &mut TestAppContext,
) {
    for preference in [
        MotionPreference::Normal,
        MotionPreference::Reduced,
        MotionPreference::None,
    ] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::components(r#"
            import "components/button" as button;
            fn state_schema(){#{fields:#{load:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
            fn begin(ctx,v){ctx.set_state("load",true);}
            fn finish(ctx,v){ctx.set_state("load",false);}
            fn view(ctx) {
                row([
                    button::Button(#{key:"query",text:"Go",loading:ctx.get_state("load"),
                        loading_text:"Receiving the most recent query results",on_click:Fn("begin")})
                        .with_style(style().max_width(px(170)).flex_shrink(false)),
                    text("Finish").accessibility_role("button").accessibility_label("Finish").on_click(Fn("finish"))
                ]).with_style(style().width(px(300)).padding(px(12)).gap(px(10)))
            }
        "#).theme(gpui_rhai_registry::MODERN_LIGHT_THEME).motion(preference)
            .with(move |view|view.runtime_clock(clock)).mount(cx);
        settle(&mut visual, Step::Frame, 4);
        let start = bounds(&mut visual, &view, "button", "Go");
        let finish_at =
            |visual: &mut VisualTestContext| bounds(visual, &view, "button", "Finish").x;
        let same_gap =
            |width: f64, sibling: f64| assert!((start.x + width + 10. - sibling).abs() < 1.);
        same_gap(start.width, finish_at(&mut visual));
        click_center(&mut visual, start);
        settle(&mut visual, Step::Frame, 3);
        let label = "Receiving the most recent query results";
        let first = bounds(&mut visual, &view, "button", label);
        same_gap(first.width, finish_at(&mut visual));
        let tween = gpui_rhai::EXTENDED_RENDERING && preference == MotionPreference::Normal;
        if tween {
            assert!(
                (start.width - first.width).abs() < 1.,
                "width jumps at content change: {start:?}/{first:?}"
            );
        } else {
            assert!(
                (first.width - 170.).abs() < 1.,
                "without native size interpolation, present the target: {first:?}"
            );
        }
        let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
        manual.advance(Duration::from_millis(180));
        settle(&mut visual, Step::Refresh, 3);
        let mid = bounds(&mut visual, &view, "button", label);
        same_gap(mid.width, finish_at(&mut visual));
        if tween {
            assert!(mid.width > start.width + 1. && mid.width < 169.5, "{mid:?}");
            dispatch(&mut visual, &view, by_role("button", "Finish"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            let reversed = bounds(&mut visual, &view, "button", "Go");
            assert!(
                (reversed.width - mid.width).abs() < 1.,
                "reverse must begin from the presented width: {reversed:?}/{mid:?}"
            );
            manual.advance(Duration::from_millis(480));
            settle(&mut visual, Step::Refresh, 4);
            let done = bounds(&mut visual, &view, "button", "Go");
            same_gap(done.width, finish_at(&mut visual));
            assert!((done.width - start.width).abs() < 1.);
        } else {
            assert!((mid.width - first.width).abs() < 1.);
            assert_eq!(
                visual.update(|_, cx| view.committed_revision(cx).unwrap()),
                revision
            );
            dispatch(&mut visual, &view, by_role("button", "Finish"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            let done = bounds(&mut visual, &view, "button", "Go");
            same_gap(done.width, finish_at(&mut visual));
            assert!((done.width - start.width).abs() < 1.);
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn button_entry_preserves_host_label_opacity_and_unchanged_loading_labels(cx: &mut TestAppContext) {
    for host_alpha in [false, true] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let source=r#"
            import "components/button" as button;
            fn state_schema(){#{fields:#{loading:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
            fn start(ctx,v){ctx.set_state("loading",true);}
            fn view(ctx){button::Button(#{key:"query",text:"Run query",loading:ctx.get_state("loading"),
                on_click:Fn("start"),part_styles:#{label:style().background(rgb(0xff0000))__ALPHA__}})}
        "#.replace("__ALPHA__",if host_alpha{".opacity(0.8)"}else{""});
        let (mut visual, view) = Mount::components(source)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        assert_alpha(&mut visual, if host_alpha { 0.8 } else { 1.0 });
        let target = bounds(&mut visual, &view, "button", "Run query");
        click_center(&mut visual, target);
        settle(&mut visual, Step::Refresh, 3);
        let expected = if host_alpha { 0.8 } else { 0.55 };
        assert_alpha(&mut visual, expected);
        manual.advance(Duration::from_millis(240));
        settle(&mut visual, Step::Refresh, 3);
        assert_alpha(&mut visual, expected);
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn button_icon_entry_scales_with_the_existing_clock(cx: &mut TestAppContext) {
    for policy in [
        MotionPreference::Normal,
        MotionPreference::Reduced,
        MotionPreference::None,
    ] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::components(r#"
            import "components/button" as button;
            fn state_schema(){#{fields:#{loading:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
            fn start(ctx,v){ctx.set_state("loading",true);}
            fn view(ctx){
                button::Button(#{key:"run", text:"Run query", prefix:text("+"),
                    loading:ctx.get_state("loading"), on_click:Fn("start")})
            }
        "#).motion(policy).with(move |view|view.runtime_clock(clock)).mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        let button = bounds(&mut visual, &view, "button", "Run query");
        fn scale_in_scene(scene: &gpui::Scene) -> Option<f32> {
            for group in &scene.isolated_groups {
                let matrix = group.transform();
                let scale = matrix.rotation_scale[0][0];
                if (0.65..0.999).contains(&scale)
                    && (matrix.rotation_scale[1][1] - scale).abs() < 0.005
                {
                    return Some(scale);
                }
                if let Some(scale) = scale_in_scene(group.source()) {
                    return Some(scale);
                }
            }
            #[cfg(feature = "gpui-fast")]
            for layer in scene
                .layers
                .frames
                .iter()
                .filter(|layer| layer.tile_size == 0)
            {
                if let Some(scale) = layer.content.scenes().find_map(scale_in_scene) {
                    return Some(scale);
                }
            }
            None
        }
        let placement = |visual: &mut VisualTestContext| {
            visual.update(|window, _| window.visual_probe_with_painted_scene(scale_in_scene))
        };
        assert_eq!(placement(&mut visual), None, "no initial mount tween");
        click_center(&mut visual, button);
        settle(&mut visual, Step::Refresh, 3);
        manual.advance(Duration::from_millis(140));
        settle(&mut visual, Step::Refresh, 3);
        if policy == MotionPreference::Normal {
            let scale = placement(&mut visual).expect("rendered icon scale midpoint");
            assert!((0.7..1.).contains(&scale), "{scale}");
        } else {
            assert_eq!(placement(&mut visual), None, "{policy:?} decorative motion");
        }
        assert!(
            (bounds(&mut visual, &view, "button", "Run query").width - button.width).abs() < 0.5
        );
        manual.advance(Duration::from_millis(520));
        settle(&mut visual, Step::Refresh, 3);
        assert_eq!(
            placement(&mut visual),
            None,
            "settled placement is identity"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn content_change_uses_view_active_time_across_a_long_suspend(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (window, view) = Mount::new(r#"
        fn state_schema(){#{fields:#{changed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn change(ctx,v){ctx.set_state("changed",true);}
        fn view(ctx){let value=if ctx.get_state("changed"){"B"}else{"A"};column([
            box([]).with_key("entry").with_style(style().width(px(100)).height(px(24)).background(rgb(0xff0000)))
                .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:1000,easing:"linear"}))
                .motion_on_change(value),
            text("Change").accessibility_role("button").accessibility_label("Change").on_click(Fn("change"))
        ])}
    "#).motion(MotionPreference::Normal).with(move |view|view.runtime_clock(clock))
        .id("content-active-time").mount_root(cx, |host,view|ActiveHost{host,view});
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 3);
    assert_alpha(&mut visual, 1.0);
    dispatch(&mut visual, &view, by_role("button", "Change"), "click").unwrap();
    settle(&mut visual, Step::Frame, 3);
    manual.advance(Duration::from_millis(250));
    settle(&mut visual, Step::Refresh, 3);
    assert_alpha(&mut visual, 0.25);
    assert!(
        visual
            .update(|window, cx| view.suspend(window, cx))
            .unwrap()
    );
    settle(&mut visual, Step::Frame, 3);
    manual.advance(Duration::from_secs(60));
    assert!(visual.update(|_, cx| view.resume(cx)).unwrap());
    settle(&mut visual, Step::Refresh, 3);
    assert_alpha(&mut visual, 0.25);
    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    manual.advance(Duration::from_millis(250));
    settle(&mut visual, Step::Refresh, 3);
    assert_alpha(&mut visual, 0.5);
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
            .timings
            .is_empty()
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn caller_change_modifier_survives_incremental_component_presentation_replay(
    cx: &mut TestAppContext,
) {
    let (mut visual,view)=Mount::new(r#"
        define_component(#{
            metadata:#{id:"tests/change_counter","export":"Counter",version:"0.4.3",
                runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},
                events:#{},slots:#{},parts:["root"]},render:Fn("render_counter")});
        fn increment(ctx,v){ctx.set_state("count",ctx.get_state("count")+1);}
        fn render_counter(ctx,p){text(`Count ${ctx.get_state("count")}`)
            .with_key("counter").with_style(style().width(px(160)).height(px(32)))
            .accessibility_role("button").accessibility_label(`Count ${ctx.get_state("count")}`)
            .on_click(Fn("increment"))}
        fn view(ctx){render_component("tests/change_counter",#{key:"counter"})
            .with_style(style().background(rgb(0xff0000)))
            .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:1000,easing:"linear"}))
            .motion_on_change("caller-value")}
    "#).motion(MotionPreference::Normal).id("caller-change-replay").mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert_alpha(&mut visual, 1.0);
    visual.update(|window, _| window.activate_window());
    let target = bounds(&mut visual, &view, "button", "Count 0");
    click_center(&mut visual, target);
    settle(&mut visual, Step::Frame, 3);
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    bounds(&mut visual, &view, "button", "Count 1");
    assert_alpha(&mut visual, 1.0);
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 3);
    bounds(&mut visual, &view, "button", "Count 2");
    assert_alpha(&mut visual, 1.0);
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
