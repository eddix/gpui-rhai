use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiNode, UiNodeKind};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{
    Host, Mount, bounds, filled_including_rounded_clips, live_fills, node_texts, snapshot,
};

fn by_key(node: &UiNode, key: &str) -> usize {
    let here = usize::from(node.key().is_some_and(|k| k.as_str() == key));
    here + match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().map(|child| by_key(child, key)).sum()
        }
        _ => 0,
    }
}

fn label_in_step(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
) -> gpui_rhai::GeometryBounds {
    let r = bounds(v, view, "treeitem", name);
    snapshot(v, view)
        .find_by_role_and_name("text", name)
        .filter_map(|n| n.geometry.map(|g| g.visual))
        .find(|g| g.y >= r.y - 1. && g.y + g.height <= r.y + r.height + 1.)
        .unwrap_or_else(|| panic!("step's text must be inside its row: {name}"))
}

#[gpui::test]
fn cost_waterfall_aligns_siblings_and_uses_one_grid_at_all_widths(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light"] {
        let (mut v, view) = gallery(cx, "cost_waterfall", theme);
        v.simulate_resize(size(px(1470.), px(1120.)));
        ready(&mut v, &view);
        let leaf = label_in_step(&mut v, &view, "Draft a plan");
        let group = label_in_step(&mut v, &view, "Metadata scout");
        assert!(
            (leaf.x - group.x).abs() < 0.6,
            "sibling text must start together: {leaf:?}, {group:?}"
        );
        click(
            &mut v,
            &view,
            "button",
            "Most expensive: Summarize evidence",
        );
        let child = label_in_step(&mut v, &view, "Summarize evidence");
        assert!(
            (child.x - group.x - 14.).abs() < 0.6,
            "one authored tree indent: {group:?}, {child:?}"
        );
        for narrow in [false, true] {
            if narrow {
                click(&mut v, &view, "button", "Narrow ledger");
            }
            let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
            assert_eq!(
                by_key(&root, "ledger-grid"),
                1,
                "one shared data-grid plane"
            );
            assert_eq!(by_key(&root, "grid"), 0, "no per-row duplicate grid");
            let r = bounds(&mut v, &view, "treeitem", "Usage credit");
            assert!(
                (r.height - if narrow { 46. } else { 32. }).abs() < 0.6,
                "{theme}/{narrow}: {r:?}"
            );
            let border = v.update(|_, cx| {
                view.theme_snapshot(cx)
                    .unwrap()
                    .variant
                    .tokens
                    .color("border")
                    .unwrap()
                    .as_rgba_hex()
            });
            let card = bounds(&mut v, &view, "group", "Cost waterfall");
            let rules: Vec<_> = live_fills(&mut v, border)
                .into_iter()
                .filter(|q| {
                    q.width <= 1.5
                        && q.height >= 25.
                        && f64::from(q.x) >= card.x
                        && f64::from(q.right()) <= card.x + card.width
                        && f64::from(q.y) >= card.y
                        && f64::from(q.bottom()) <= card.y + card.height + 1.
                })
                .collect();
            if narrow {
                let first = bounds(&mut v, &view, "treeitem", "Draft a plan");
                assert!(
                    rules
                        .iter()
                        .any(|q| (q.height - 30.).abs() < 0.6
                            && f64::from(q.bottom()) <= first.y + 1.),
                    "narrow axis grid stops before the step rows: {rules:?}, {first:?}"
                );
            } else {
                assert!(
                    rules.iter().any(|q| q.height > 96.),
                    "the wide grid paints continuous lines across rows: {rules:?}"
                );
            }
        }
    }
}

#[gpui::test]
fn cost_waterfall_rtl_tree_maps_arrow_direction_only_once(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "cost_waterfall", "modern-dark");
    v.simulate_resize(size(px(1350.), px(1020.)));
    v.update(|_, cx| view.select_locale("ar", cx)).unwrap();
    ready(&mut v, &view);
    click(&mut v, &view, "treeitem", "Metadata scout");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(false)
    );
    v.simulate_keystrokes("left");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(true)
    );
    node(&mut v, &view, "treeitem", "Summarize evidence");
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(false)
    );
}

#[gpui::test]
fn cost_waterfall_maximum_ledger_mounts_under_the_native_budget(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(r#"
        import "patterns/cost_waterfall" as cw;
        fn view(ctx){
            let steps=[];
            for i in 0..192{steps.push(#{id:"step:"+i.to_string(),name:"Step "+i.to_string(),kind:"llm",status:"done",
                cost_micros:500,tokens_in:800,tokens_out:25,duration_ms:22});}
            column([cw::CostWaterfall(#{key:"ceiling",run:"max",steps:steps,mode:"tokens",max_height:240})])
                .with_style(style().width(px(780)))
        }
    "#)
        .registry("patterns/cost_waterfall")
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Total cost $0.10");
    assert!(bounds(&mut v, &view, "tree", "Run steps").height <= 240.5);
}

#[gpui::test]
fn cost_waterfall_rollup_and_top_inspector_keep_native_keyboard_focus(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "cost_waterfall", "modern-light");
    v.simulate_resize(size(px(1260.0), px(960.0)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Total cost −$0.009");
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut words = Vec::new();
    node_texts(&root, &mut words);
    assert!(words.iter().any(|s| s == "$0.025") && words.iter().any(|s| s == "−$0.025"));
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(false)
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("treeitem", "Summarize evidence")
            .next()
            .is_none()
    );

    click(
        &mut v,
        &view,
        "button",
        "Most expensive: Summarize evidence",
    );
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(true)
    );
    node(&mut v, &view, "region", "Summarize evidence details");
    node(&mut v, &view, "status", "Total cost −$0.009");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("region", "Summarize evidence details")
            .next()
            .is_none()
    );
    v.simulate_keystrokes("left left");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "treeitem", "Metadata scout").expanded,
        Some(false)
    );
    v.simulate_keystrokes("right down enter");
    ready(&mut v, &view);
    node(&mut v, &view, "region", "Summarize evidence details");
    node(&mut v, &view, "status", "Total cost −$0.009");
}

#[gpui::test]
fn cost_waterfall_controlled_mode_and_narrow_ledger_keep_true_amounts(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "cost_waterfall", theme);
        v.simulate_resize(size(px(1230.0), px(925.0)));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Refuse next measure");
        click(&mut v, &view, "button", "Tokens");
        assert_eq!(node(&mut v, &view, "button", "Cost").pressed, Some(true));
        node(
            &mut v,
            &view,
            "status",
            "The Host kept the previous measure",
        );
        click(&mut v, &view, "button", "Tokens");
        assert_eq!(node(&mut v, &view, "button", "Tokens").pressed, Some(true));
        node(&mut v, &view, "status", "Total cost −$0.009");
        click(&mut v, &view, "button", "Grow live cost");
        node(&mut v, &view, "status", "Total cost $0.01");
        click(&mut v, &view, "button", "Narrow ledger");
        let card = bounds(&mut v, &view, "group", "Cost waterfall");
        let row = bounds(&mut v, &view, "treeitem", "Usage credit");
        assert!(
            card.width <= 350.5 && row.x + row.width <= card.x + card.width + 0.5,
            "{theme}: {card:?} {row:?}"
        );
        click(&mut v, &view, "button", "Empty ledger");
        node(&mut v, &view, "status", "Total cost –");
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("treeitem", "Usage credit")
                .next()
                .is_none()
        );
    }
}

#[gpui::test]
fn cost_waterfall_live_bar_animates_in_native_frames(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "cost_waterfall".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("cost-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1260.0), px(940.0)));
    clock.advance(Duration::from_millis(600));
    ready(&mut v, &view);
    let width = |v: &mut VisualTestContext| {
        let row = bounds(v, &view, "treeitem", "Write the answer");
        filled_including_rounded_clips(v, 0xf5ee_fdff)
            .into_iter()
            .filter(|q| {
                (q.bounds.height - 16.0).abs() <= 0.75
                    && f64::from(q.bounds.y) > row.y
                    && f64::from(q.bounds.y + q.bounds.height) < row.y + row.height
            })
            .max_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x))
            .expect("native bar to the right of the running step's mark")
            .bounds
            .width
    };
    let before = width(&mut v);
    click(&mut v, &view, "button", "Grow live cost");
    assert!((width(&mut v) - before).abs() < 1.0);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(90));
    ready(&mut v, &view);
    let middle = width(&mut v);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    clock.advance(Duration::from_millis(620));
    ready(&mut v, &view);
    let end = width(&mut v);
    assert!(
        before + 1.0 < middle && middle + 1.0 < end,
        "{before} → {middle} → {end}"
    );
}
