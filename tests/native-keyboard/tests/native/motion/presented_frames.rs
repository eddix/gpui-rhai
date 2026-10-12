//! Motion in presented frames: inherited motion groups, the motion catalog story, the
//! Host's `none` policy, canvas morph hit testing and timeline rebinding.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{Host, Mount, bounds, by_test_id, dispatch, last_error, node_texts, snapshot};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ScriptViewConfig, ScriptViewHost};
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare as prepare_gallery};

#[gpui::test]
fn inherited_motion_group_survives_incremental_component_update(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
                define_component(#{
                    metadata: #{ id: "components/group_probe", "export": "GroupProbe",
                        version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{} },
                    schema: #{ props: #{ key: #{ schema: #{ type: "string" },
                        required: true, sensitive: false } }, state: #{ fields: #{ count: #{
                        schema: #{ type: "integer" },
                        "default": #{ type: "integer", value: 0 } } } },
                        events: #{}, slots: #{}, parts: ["root"] },
                    render: Fn("render_probe"),
                });
                fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
                fn render_probe(ctx, props) {
                    text(`count:${ctx.get_state("count")}`)
                        .with_key("child").test_id("group-child")
                        .accessibility_role("button")
                        .accessibility_label(`count:${ctx.get_state("count")}`)
                        .with_style(style().width(px(100)).height(px(30)))
                        .on_click(Fn("increment"))
                        .shared_layout("item").layout_motion(100, "linear")
                }
                fn view(ctx) {
                    motion_group("outer", [render_component(
                        "components/group_probe", #{ key: "probe" })])
                }
            "#,
    )
    .ids("motion-group-window", "motion-group-view")
    .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    visual.run_until_parked();
    dispatch(&mut visual, &view, by_test_id("group-child"), "click").unwrap();
    visual.run_until_parked();

    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "count:1")
            .next()
            .is_some(),
        "incremental component output did not retain its inherited motion group"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn motion_catalog_story_mounts_and_renders_a_real_frame(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = prepare_gallery(&GalleryLaunch {
        story: "motion/catalog".to_owned(),
        ..GalleryLaunch::default()
    })
    .expect("prepare the official Motion story");
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("motion-gallery-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("motion-gallery-view"),
                host.clone(),
                window,
                cx,
            )
            .expect("mount the official motion gallery");
        *captured_for_window.borrow_mut() = Some(view.clone());
        Host { host, view }
    });

    cx.refresh().expect("render the first gallery frame");
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    assert!(
        visual.update(|_, cx| view.root(cx).unwrap().is_some()),
        "the mounted gallery must retain a rendered root"
    );
    let initial = visual.update(|_, cx| {
        let root = view.root(cx).unwrap().unwrap();
        let mut texts = Vec::new();
        node_texts(&root, &mut texts);
        texts
    });
    assert!(
        initial.iter().position(|text| text == "One")
            < initial.iter().position(|text| text == "Three")
    );
    for id in ["reorder", "play"] {
        dispatch(&mut visual, &view, by_test_id(id), "click").unwrap();
    }
    visual.run_until_parked();
    let changed = visual.update(|_, cx| {
        let root = view.root(cx).unwrap().unwrap();
        let mut texts = Vec::new();
        node_texts(&root, &mut texts);
        texts
    });
    assert!(
        changed.iter().position(|text| text == "Three")
            < changed.iter().position(|text| text == "One")
    );
    assert!(changed.iter().any(|text| text == "Timeline: playing"));
}

#[gpui::test]
fn host_none_policy_snaps_layout_motion_in_the_presented_frame(cx: &mut TestAppContext) {
    let manual = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(
        r#"
                fn state_schema(){#{fields:#{x:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
                fn move_card(ctx, payload) { ctx.set_state("x", 100); }
                fn init(ctx) { ctx.register_action("card.move", Fn("move_card")); }
                fn view(ctx) {
                    column([
                        text("Card").with_key("card").test_id("card")
                            .accessibility_role("button").accessibility_label("Card")
                            .layout_motion(1000, "linear")
                            .with_style(style().width(px(100)).height(px(30))
                                .margin_left(px(ctx.get_state("x"))))
                    ])
                }
            "#,
    )
    .with(move |view| view.runtime_clock(clock))
    .motion(gpui_rhai::MotionPreference::None)
    .ids("none-motion-window", "none-motion-view")
    .mount(cx);
    cx.refresh().unwrap();

    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Action {
                    id: "card.move".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    cx.refresh().unwrap();

    let changed = card_geometry(&mut visual, &view);
    assert_eq!(changed.layout.x, 100.0);
    assert_eq!(changed.visual.x, 100.0);

    manual.advance(std::time::Duration::from_millis(500));
    cx.refresh().unwrap();
    let later = card_geometry(&mut visual, &view);
    assert_eq!(
        later.visual.x, 100.0,
        "None must not retain a hidden animation"
    );
}

#[gpui::test]
fn canvas_morph_hit_testing_follows_the_presented_path(cx: &mut TestAppContext) {
    let source = r#"
        fn state_schema(){#{fields:#{hit:#{schema:#{type:"string"},
            "default":#{type:"string",value:"unseen"}}}}}
        fn hit(ctx,event) {
            ctx.set_state("hit", if event.canvas_key == () { "none" } else { event.canvas_key });
        }
        fn view(ctx) {
            column([
                canvas(canvas_scene([
                    canvas_morph_stroke_path("shape",
                        [path_move(20.0,20.0),path_line(180.0,20.0)],
                        [path_move(20.0,80.0),path_line(180.0,80.0)],
                        10.0, theme_color("accent"))
                ])).with_key("canvas")
                    .accessibility_role("button").accessibility_label("Canvas")
                    .with_style(style().width(px(200)).height(px(100)))
                    .on("pointer_down", Fn("hit"))
                    .motion(motion_transition("path_progress",1.0,1.0,
                        #{duration_ms:1000,easing:"linear"})),
                text(ctx.get_state("hit")).with_key("result")
                    .accessibility_role("status")
                    .accessibility_label(ctx.get_state("hit"))
            ])
        }
    "#;
    let manual = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(source)
        .with(move |view| view.runtime_clock(clock))
        .ids("canvas-morph-window", "canvas-morph-view")
        .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let bounds = bounds(&mut visual, &view, "button", "Canvas");
    visual.simulate_click(
        point(px((bounds.x + 100.0) as f32), px((bounds.y + 80.0) as f32)),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("status", "shape")
            .next()
            .is_some()
    );
    visual.simulate_click(
        point(px((bounds.x + 100.0) as f32), px((bounds.y + 20.0) as f32)),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("status", "none")
            .next()
            .is_some()
    );
}

#[gpui::test]
fn timeline_rebinds_a_replaced_child_in_presented_frames(cx: &mut TestAppContext) {
    let source = r#"
        fn state_schema(){#{fields:#{switched:#{schema:#{type:"bool"},
            "default":#{type:"bool",value:false}}}}}
        fn replace_target(ctx,event){ctx.set_state("switched",true);}
        fn repaint(ctx,event){}
        fn init(ctx){
            ctx.register_action("audit.replace",Fn("replace_target"));
            ctx.register_action("audit.repaint",Fn("repaint"));
        }
        fn view(ctx){
            let child=if ctx.get_state("switched"){box([])}else{text("Title")};
            column([child.with_key("title").accessibility_role("button")
                .accessibility_label("Title")
                .with_style(style().width(px(100)).height(px(30)))])
                .with_key("panel").timeline(motion_timeline("intro",
                    motion_track("title",motion_transition("width",100.0,200.0,
                        #{duration_ms:1000,easing:"linear"})),#{}))
        }
    "#;
    let manual = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(source)
        .with(move |view| view.runtime_clock(clock))
        .ids("timeline-target-window", "timeline-target-view")
        .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut widths = vec![bounds(&mut visual, &view, "button", "Title").width];
    for id in ["audit.replace", "audit.repaint"] {
        manual.advance(std::time::Duration::from_millis(250));
        visual
            .update(|window, cx| {
                view.automate(
                    gpui_rhai::AutomationCommand::Action {
                        id: id.to_owned(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        visual.run_until_parked();
        cx.refresh().unwrap();
        visual.run_until_parked();
        widths.push(bounds(&mut visual, &view, "button", "Title").width);
    }
    assert!(widths[2] > 110.0, "presented widths: {widths:?}");
}

fn card_geometry(
    visual: &mut VisualTestContext,
    view: &gpui_rhai::ScriptViewHandle,
) -> gpui_rhai::ElementGeometry {
    snapshot(visual, view)
        .find_by_role_and_name("button", "Card")
        .next()
        .unwrap()
        .geometry
        .unwrap()
}
