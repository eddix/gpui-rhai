//! Intrinsic overflow cohorts use real layout and the existing presented semantic tree.

use crate::support::{Mount, Step, bounds, filled, last_error, nodes, settle};
use gpui::{TestAppContext, px, size};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

#[gpui::test]
fn overflow_zero_summary_participates_in_the_preferred_native_width(cx: &mut TestAppContext) {
    let source = r#"
        fn cell(label,w){
            box([text(label)]).accessibility_role("group").accessibility_label(label)
                .with_style(style().width(px(w)).height(px(20)))
        }
        fn fits(){
            [cell("Keep",30),cell("Optional",20).with_style(style().overflow_priority(0)),
             cell("Full",18).overflow_summary_for(0),cell("Reduced",17).overflow_summary_for(1)]
        }
        fn view(ctx){column([
            row(fits()).with_style(style().overflow_row().gap(px(4)))
                .accessibility_role("group").accessibility_label("Natural"),
            row(fits()).with_style(style().overflow_row().gap(px(4)).width(px(75)))
                .accessibility_role("group").accessibility_label("Limited")
        ]).with_style(style().items_start())}
    "#;
    let (mut v, view) = Mount::components(source).mount(cx);
    settle(&mut v, Step::Frame, 4);
    let natural = bounds(&mut v, &view, "group", "Natural");
    assert!((natural.width - 76.).abs() <= 0.5, "{natural:?}");
    let tree = crate::support::snapshot(&mut v, &view);
    for (row, expected) in [
        ("Natural", vec!["Keep", "Optional", "Full"]),
        ("Limited", vec!["Keep", "Reduced"]),
    ] {
        let parent = tree.find_by_role_and_name("group", row).next().unwrap();
        let labels: Vec<_> = parent
            .children
            .iter()
            .filter_map(|id| tree.node(*id))
            .filter(|n| n.geometry.is_some())
            .map(|n| n.name.as_str())
            .collect();
        assert_eq!(labels, expected, "{row}");
    }
    assert_eq!(last_error(&mut v, &view), None);
}

#[gpui::test]
fn overflow_rows_replay_only_the_visible_semantic_subset_in_cache_islands(cx: &mut TestAppContext) {
    use crate::support::{Lockstep, by_test_id, dispatch};
    let source = r#"
        fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
        fn bump(ctx,event){ctx.set_state("count",ctx.get_state("count")+1);}
        fn view(ctx){let cells=[];
            for index in 0..12 {cells.push(box([text(`Cell ${index}`)]).with_key(`${index}`)
                .accessibility_role("group").accessibility_label(`Cell ${index}`)
                .with_style(style().width(px(50)).height(px(24)).overflow_priority(if index==0||index==11{65535}else{0}))
                .with_style(if index==0||index==11{style().overflow_required()}else{style()}));}
            column([text(`${ctx.get_state("count")}`),text("Bump").test_id("bump").on_click(Fn("bump")),
                row(cells).with_key("cells").with_style(style().overflow_row().width(relative(1.0)).gap(px(4)))
            ]).with_style(style().width(relative(1.0)))
        }
    "#;
    let build = || Mount::components(source).config(|config| config.host_sets_width(true));
    let mut sides = Lockstep::mount(
        cx,
        build().config(|config| config.render_islands(false)),
        build(),
        size(px(900.0), px(300.0)),
    );
    for width in [900.0, 350.0, 900.0] {
        sides.act(|visual, view| {
            visual.simulate_resize(size(px(width), px(300.0)));
            settle(visual, Step::Frame, 4);
            assert_eq!(
                nodes(visual, view).iter().any(|node| node.name == "Cell 1"),
                width > 600.0
            );
        });
        sides.assert_same("resized overflow row");
        for trial in 0..2 {
            let mut stats = Vec::new();
            sides.act(|visual, view| {
                visual.update(|_, cx| view.take_render_statistics(cx).unwrap());
                dispatch(visual, view, by_test_id("bump"), "click").unwrap();
                settle(visual, Step::Frame, 3);
                stats.push(visual.update(|_, cx| view.take_render_statistics(cx).unwrap()));
            });
            sides.assert_same("replayed overflow row");
            let cached = stats.last().unwrap();
            assert_eq!(cached.island_drift, 0);
            // The first commit warms/promotes the measured island; assert an
            // actual replay on the next one, as the shared island tests do.
            if cfg!(feature = "gpui-pre") && trial == 1 {
                assert!(cached.island_hits > 0, "{cached:?}");
            }
        }
    }
}

#[gpui::test]
fn overflow_cohorts_keep_logical_identity_and_restore_interactive_state(cx: &mut TestAppContext) {
    let source = r#"
        import "components/input" as input;
        fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
        fn changed(ctx,value){ctx.set_state("value",value);}
        fn noop(ctx,event){}
        fn view(ctx){column([
            row([
                text("Before").accessibility_role("button").accessibility_label("Before").on_click(Fn("noop"))
                    .with_style(style().width(px(60)).height(px(32))),
                input::Input(#{key:"field",label:"Field",value:ctx.get_state("value"),on_change:Fn("changed")})
                    .with_style(style().width(px(160)).overflow_priority(0)),
                text("After").accessibility_role("button").accessibility_label("After").on_click(Fn("noop"))
                    .with_style(style().width(px(60)).height(px(32)))
            ]).with_style(style().overflow_row().width(relative(1.0)).gap(px(8))),
            text(ctx.get_state("value")).test_id("value")
        ])}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(500.0), px(200.0)));
    settle(&mut visual, Step::Frame, 4);
    let field = bounds(&mut visual, &view, "text_field", "Field");
    crate::support::click_center(&mut visual, field);
    visual.simulate_input("hello");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "value"),
        "hello"
    );
    visual.simulate_resize(size(px(180.0), px(200.0)));
    settle(&mut visual, Step::Frame, 3);
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.role == "text_field")
    );
    visual.simulate_input("X");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        crate::support::text_of(&mut visual, &view, "value"),
        "hello"
    );
    visual.simulate_resize(size(px(500.0), px(200.0)));
    settle(&mut visual, Step::Frame, 3);
    let field = bounds(&mut visual, &view, "text_field", "Field");
    crate::support::click_center(&mut visual, field);
    visual.simulate_keystrokes("cmd-z");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(crate::support::text_of(&mut visual, &view, "value"), "");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn a_captured_gesture_cannot_commit_after_its_cohort_is_omitted(cx: &mut TestAppContext) {
    for narrow in [320, 140] {
        let source = r#"
        import "components/draggable" as draggable;
        fn state_schema(){#{fields:#{commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            width:#{schema:#{type:"integer"},"default":#{type:"integer",value:500}}}}}
        fn moved(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
        fn shrink(ctx,event){ctx.set_state("width",__WIDTH__);}
        fn view(ctx){column([
            row([box([]).with_style(style().width(px(40)).height(px(20))),
                draggable::Draggable(#{key:"card",label:"Card",position:#{x:0,y:0},threshold:0,on_move:Fn("moved"),
                    content:text("Move").with_style(style().width(px(100)).height(px(40)))})
                    .with_style(style().width(px(240)).height(px(80)).overflow_priority(0))
            ]).with_style(style().overflow_row().width(px(ctx.get_state("width"))).gap(px(8))),
            text("Shrink").test_id("shrink").on_click(Fn("shrink")),
            text(`${ctx.get_state("commits")}`).test_id("commits")
        ])}
    "#.replace("__WIDTH__", &narrow.to_string());
        let (mut visual, view) = Mount::components(source).mount(cx);
        visual.simulate_resize(size(px(500.0), px(200.0)));
        settle(&mut visual, Step::Frame, 4);
        let object = bounds(&mut visual, &view, "text", "Move");
        let at = crate::support::center(object);
        visual.simulate_mouse_down(at, gpui::MouseButton::Left, gpui::Modifiers::none());
        settle(&mut visual, Step::Frame, 2);
        visual.simulate_mouse_move(
            at + gpui::point(px(20.0), px(0.0)),
            Some(gpui::MouseButton::Left),
            gpui::Modifiers::none(),
        );
        settle(&mut visual, Step::Frame, 2);
        let preview = bounds(&mut visual, &view, "text", "Move");
        assert!(
            preview.x > object.x + 19.0,
            "the gesture must actually be captured: {object:?} -> {preview:?}"
        );
        // Change only the row's budget through the same Host path in both cases;
        // do not let a platform-window resize cancel the gesture for us.
        crate::support::dispatch(
            &mut visual,
            &view,
            crate::support::by_test_id("shrink"),
            "click",
        )
        .unwrap();
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "Card"),
            narrow > 200
        );
        visual.simulate_mouse_up(
            at + gpui::point(px(50.0), px(0.0)),
            gpui::MouseButton::Left,
            gpui::Modifiers::none(),
        );
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            crate::support::text_of(&mut visual, &view, "commits"),
            if narrow > 200 { "1" } else { "0" }
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn overflow_priority_cohorts_and_box_alignment_work_in_both_directions(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        let source = r#"
            fn init(ctx){ctx.set_locale("__LOCALE__");}
            fn cell(name,priority){box([]).with_key(name).accessibility_role("group").accessibility_label(name)
                .with_style(style().width(px(40)).height(px(20)).merge(if priority<0{style()}else{style().overflow_priority(priority)}))}
            fn view(ctx){row([cell("A",-1),cell("B",0),cell("C",0),cell("D",1)])
                .with_style(style().overflow_row().width(relative(1.0)).gap(px(8)))}
        "#.replace("__LOCALE__", if rtl {"ar"} else {"en"});
        let (mut visual, view) = Mount::components(source)
            .locale("en.rhai", EN_LOCALE)
            .locale("ar.rhai", AR_LOCALE)
            .mount(cx);
        visual.simulate_resize(size(px(130.0), px(200.0)));
        settle(&mut visual, Step::Frame, 4);
        let a = bounds(&mut visual, &view, "group", "A");
        let d = bounds(&mut visual, &view, "group", "D");
        assert!(
            !nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "B" || node.name == "C")
        );
        assert_eq!(a.x < d.x, !rtl);
        assert!(((d.x - a.x).abs() - 48.0).abs() < 0.1);
        visual.simulate_resize(size(px(500.0), px(200.0)));
        settle(&mut visual, Step::Frame, 3);
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "B")
        );
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "C")
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn explicitly_hidden_cells_take_no_gap_and_invisible_cells_still_measure(cx: &mut TestAppContext) {
    let source = r#"
        fn cell(label){box([]).accessibility_role("group").accessibility_label(label)
            .with_style(style().width(px(40)).height(px(20)))}
        fn view(ctx){row([cell("A"),cell("Hidden").with_style(style().hidden()),
            cell("B"),cell("Invisible").with_style(style().invisible()),cell("C")])
            .with_style(style().overflow_row().width(relative(1.0)).gap(px(8)))}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.simulate_resize(size(px(500.0), px(200.0)));
    settle(&mut visual, Step::Frame, 4);
    let a = bounds(&mut visual, &view, "group", "A");
    let b = bounds(&mut visual, &view, "group", "B");
    let c = bounds(&mut visual, &view, "group", "C");
    assert!((b.x - a.x - 48.0).abs() < 0.1);
    assert!((c.x - b.x - 96.0).abs() < 0.1);
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "Hidden" || node.name == "Invisible")
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn overflow_rows_reserve_full_line_height_when_a_tall_cohort_collapses(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx){row([
            box([]).accessibility_role("group").accessibility_label("Required")
                .with_style(style().width(px(20)).height(px(20))),
            box([]).accessibility_role("group").accessibility_label("Tall")
                .with_style(style().width(px(100)).height(px(100)).overflow_priority(0))
        ]).accessibility_role("group").accessibility_label("Line")
            .with_style(style().overflow_row().width(relative(1.0)).gap(px(4)))}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    for width in [200.0, 40.0, 200.0] {
        visual.simulate_resize(size(px(width), px(300.0)));
        settle(&mut visual, Step::Frame, 2);
        let line = bounds(&mut visual, &view, "group", "Line");
        let required = bounds(&mut visual, &view, "group", "Required");
        assert!((line.height - 100.0).abs() < 0.1);
        assert!((required.y - line.y - 40.0).abs() < 0.1);
        assert_eq!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.name == "Tall"),
            width > 100.0
        );
    }
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn overflow_rows_collapse_and_restore_before_paint_without_running_rhai(cx: &mut TestAppContext) {
    let source = r#"
        fn cell(label,width,color,optional){box([]).with_key(label)
            .accessibility_role("group").accessibility_label(label)
            .with_style(style().width(px(width)).height(px(20)).background(color)
                .merge(if optional{style().overflow_priority(0)}else{style()}))}
        fn view(ctx){row([
            cell("Start",40,rgba(0xff00ffff),false),
            cell("Optional",140,rgba(0x00ffffff),true),
            cell("End",40,rgba(0xffff00ff),false)
        ]).accessibility_role("group").accessibility_label("Row")
            .with_style(style().overflow_row().width(relative(1.0)).justify_end()
                .padding(px(10)).border(px(2)).gap(px(8.25)))}
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 4);
    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    for width in [500.0, 260.0, 259.0, 180.0, 500.0] {
        visual.simulate_resize(size(px(width), px(200.0)));
        // One native frame must already make the choice. At the test DPR,
        // authored 8.25px snaps to 8px; the planner must use that same gap.
        settle(&mut visual, Step::Frame, 1);
        let a = bounds(&mut visual, &view, "group", "Start");
        let b = bounds(&mut visual, &view, "group", "End");
        assert!((a.width - 40.0).abs() < 0.1 && (b.width - 40.0).abs() < 0.1);
        assert!(
            (b.x + b.width - f64::from(width) + 12.0).abs() < 0.1,
            "{width}: {a:?} {b:?}"
        );
        assert!(
            (a.y - 12.0).abs() < 0.1 && (b.y - a.y).abs() < 0.1,
            "{a:?} {b:?}"
        );
        let optional = nodes(&mut visual, &view)
            .into_iter()
            .find(|node| node.name == "Optional");
        if width >= 260.0 {
            let optional = optional.unwrap().geometry.unwrap().visual;
            assert!((optional.x - a.x - a.width - 8.0).abs() < 0.1);
            assert!(!filled(&mut visual, 0x00ff_ffff).is_empty());
        } else {
            assert!(optional.is_none());
            assert!(filled(&mut visual, 0x00ff_ffff).is_empty());
            assert!((b.x - a.x - a.width - 8.0).abs() < 0.1);
        }
        assert!(
            filled(&mut visual, 0xff00_ffff)
                .iter()
                .any(|quad| (f64::from(quad.bounds.x) - a.x).abs() < 0.1
                    && (f64::from(quad.bounds.y) - a.y).abs() < 0.1)
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
            .timings
            .is_empty()
    );
    let frames = visual.update(|window, cx| window.simulate_next_frame(cx));
    assert_eq!(frames, 0, "settled intrinsic rows must become idle");
}
