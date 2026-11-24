use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasScene, ManualRuntimeClock, MotionPreference, PrimitiveValue,
    ScriptViewHandle, UiNode, UiNodeKind, UiValue,
};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, snapshot};

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(520));
    ready(visual, view);
}

fn layer<'a>(n: &'a UiNode, key: &str) -> Option<(&'a UiNode, &'a CanvasScene)> {
    match n.kind() {
        UiNodeKind::Canvas { scene } if n.key().is_some_and(|k| k.as_str() == key) => {
            Some((n, scene))
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| layer(child, key))
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(child) => layer(child, key),
            PrimitiveValue::Nodes(children) => children.iter().find_map(|child| layer(child, key)),
            _ => None,
        }),
        _ => None,
    }
}

fn replay(v: &mut VisualTestContext, view: &ScriptViewHandle, slot: usize) -> UiValue {
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    layer(&root, &format!("pulse-{slot}"))
        .unwrap()
        .0
        .attributes()["motion_replay_key"]
        .clone()
}

fn heat(v: &mut VisualTestContext, view: &ScriptViewHandle) -> f64 {
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (_, canvas) = layer(&root, "wires").unwrap();
    let CanvasCommand::Path { stroke, .. } = canvas
        .commands()
        .iter()
        .find(|c| c.key() == "edge-4")
        .unwrap()
    else {
        panic!("expected a standing search edge")
    };
    stroke.as_ref().unwrap().1
}

#[gpui::test]
fn gallery_agent_graph_tracks_accepted_call_phases_without_replaying_on_resize(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_graph", "modern-light");
    v.simulate_resize(size(px(1270.), px(1140.)));
    later(&mut v, &view);
    let initial = replay(&mut v, &view, 0);
    let quiet = heat(&mut v, &view);
    let before = bounds(&mut v, &view, "button", "Workspace search");
    click(&mut v, &view, "button", "Send search request");
    later(&mut v, &view);
    let send = replay(&mut v, &view, 0);
    assert_ne!(initial, send);
    assert!(heat(&mut v, &view) > quiet);
    let fixed = bounds(&mut v, &view, "button", "Workspace search");
    assert_eq!(
        before, fixed,
        "accepted calls must not relocate their endpoints"
    );
    v.simulate_resize(size(px(655.), px(1220.)));
    later(&mut v, &view);
    assert_eq!(replay(&mut v, &view, 0), send);
    let resized = bounds(&mut v, &view, "button", "Workspace search");
    assert_ne!(resized.x, fixed.x);
    let second = replay(&mut v, &view, 1);
    click(&mut v, &view, "button", "Return search reply");
    later(&mut v, &view);
    assert_ne!(replay(&mut v, &view, 1), second);
    let reply = replay(&mut v, &view, 1);
    click(&mut v, &view, "button", "Advance past heat window");
    assert_eq!(replay(&mut v, &view, 1), reply);
    assert!((heat(&mut v, &view) - quiet).abs() < f64::EPSILON);
    node(&mut v, &view, "text", "Calls/min: 0");
}

#[gpui::test]
fn gallery_agent_graph_native_hit_and_inspector_respect_controlled_selection(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_graph", "clean-dark");
    v.simulate_resize(size(px(1180.), px(1130.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next selection");
    click(&mut v, &view, "button", "Workspace search");
    node(&mut v, &view, "status", "The Host kept the old selection.");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("region", "Connections for Workspace search")
            .next()
            .is_none()
    );
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Selected search/资料");
    node(&mut v, &view, "region", "Connections for Workspace search");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Closed the inspector.");
    v.simulate_keystrokes("home enter");
    ready(&mut v, &view);
    node(&mut v, &view, "region", "Connections for Coordinator");
    node(
        &mut v,
        &view,
        "listitem",
        "→ Review assistant · Review the draft · Pending",
    );
}

#[gpui::test]
fn agent_graph_standing_pairs_do_not_collide_with_separator_in_business_ids(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/agent_graph" as graph;
        fn view(ctx){
            graph::AgentGraph(#{key:"graph",run:"identity",revision:0,now_ms:0,nodes:[
                #{id:"a|b",kind:"agent",name:"First"},#{id:"c",kind:"tool",name:"Second"},
                #{id:"a",kind:"agent",name:"Third"},#{id:"b|c",kind:"tool",name:"Fourth"}],
                edges:[#{from:"a|b",to:"c"},#{from:"a",to:"b|c"},#{from:"c",to:"a|b"}]})
        }
        "#,
    )
    .registry("patterns/agent_graph")
    .mount(cx);
    v.simulate_resize(size(px(850.), px(740.)));
    later(&mut v, &view);
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    assert_eq!(layer(&root, "wires").unwrap().1.commands().len(), 2);
    let stage = bounds(&mut v, &view, "group", "Communication graph");
    let target = bounds(&mut v, &view, "button", "First");
    assert!(target.width >= 44.0 && target.height >= 44.0);
    assert!(target.x >= stage.x && target.x + target.width <= stage.x + stage.width);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn painted_head(visual: &mut VisualTestContext) -> f64 {
    visual.update(|window, _| {
        let scale = f64::from(window.scale_factor());
        window.visual_probe_with_painted_scene(|scene| {
            scene
                .paths
                .iter()
                .filter_map(|path| {
                    let color = path.color.as_solid()?.to_rgb();
                    (color.r > 0.8 && color.b > 0.8 && color.g < 0.2 && color.a > 0.96)
                        .then_some(f64::from(path.bounds.center().x.0) / scale)
                })
                .next()
                .expect("expected a presented magenta pulse head")
        })
    })
}

#[gpui::test]
fn agent_graph_dense_narrow_layout_keeps_every_native_target_separate(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/agent_graph" as graph;
        fn view(ctx){
            let nodes=[];let edges=[];
            for i in 0..24{
                nodes.push(#{id:"n"+i.to_string(),kind:if i<10{"agent"}else{"tool"},name:"Node "+i.to_string()});
            }
            for i in 0..23{
                edges.push(#{from:nodes[i].id,to:nodes[i+1].id});
            }
            for i in 0..22{
                edges.push(#{from:nodes[i].id,to:nodes[i+2].id});
            }
            for i in 0..19{
                edges.push(#{from:nodes[i].id,to:nodes[i+3].id});
            }
            graph::AgentGraph(#{key:"graph",run:"dense",revision:0,now_ms:0,nodes:nodes,edges:edges})
        }
        "#,
    )
    .registry("patterns/agent_graph")
    .mount(cx);
    v.simulate_resize(size(px(330.), px(1100.)));
    later(&mut v, &view);
    let stage = bounds(&mut v, &view, "group", "Communication graph");
    let rects: Vec<_> = (0..24)
        .map(|i| bounds(&mut v, &view, "button", &format!("Node {i}")))
        .collect();
    for (i, r) in rects.iter().enumerate() {
        assert!(r.width >= 44.0 && r.height >= 44.0);
        assert!(r.x >= stage.x && r.x + r.width <= stage.x + stage.width);
        for q in &rects[i + 1..] {
            assert!(
                r.x + r.width <= q.x
                    || q.x + q.width <= r.x
                    || r.y + r.height <= q.y
                    || q.y + q.height <= r.y,
                "dense target overlap: {r:?} and {q:?}"
            );
        }
    }
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    assert_eq!(layer(&root, "wires").unwrap().1.commands().len(), 64);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn agent_graph_canvas_head_resamples_route_at_same_native_motion_time(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut v, view) = Mount::new(
        r#"
        fn state_schema(){#{fields:#{active:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            wide:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn start(ctx,e){ctx.set_state("active",true);handled()}
        fn move_route(ctx,e){ctx.set_state("wide",true);handled()}
        fn trigger(label,action){
            text(label).with_key(label).accessibility_role("button").accessibility_label(label).on_click(action)
        }
        fn view(ctx){
            let far=if ctx.get_state("wide"){210.0}else{110.0};
            let route=canvas_scene([canvas_stroke_path("guide",[path_move(10,21),path_line(far,21)],1,rgba(0x00000000))])
                .path_pulse("guide",4,rgb(0xff00ff),false,false);
            column([trigger("Start pulse",Fn("start")),trigger("Move route",Fn("move_route")),
                canvas(route).with_key("pulse").with_style(style().width(px(240)).height(px(50)))
                    .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:800,easing:"linear"}))
                    .motion_on_change(if ctx.get_state("active"){"active"}else{"rest"})])
        }
        "#,
    )
    .with(move |view| view.runtime_clock(clock))
    .motion(MotionPreference::Normal)
    .mount(cx);
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Start pulse");
    manual.advance(Duration::from_millis(200));
    later(&mut v, &view);
    let old = painted_head(&mut v);
    click(&mut v, &view, "button", "Move route");
    let changed = painted_head(&mut v);
    assert!(
        (changed - old - 25.0).abs() < 3.0,
        "same quarter-progress should advance 25px on the longer route: {old} → {changed}"
    );
    manual.advance(Duration::from_millis(200));
    later(&mut v, &view);
    let later = painted_head(&mut v);
    assert!((later - changed - 50.0).abs() < 3.0);
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
