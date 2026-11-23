use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{
    AutomationCommand, LayoutLength, Length, MotionPreference, ScriptViewHandle, UiNode, UiNodeKind,
};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, glyphs_with_ink, live_fills, node_texts, snapshot};

fn content(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut out = Vec::new();
    node_texts(&root, &mut out);
    out
}

fn keyed<'a>(node: &'a UiNode, key: &str) -> Option<&'a UiNode> {
    if node.key().is_some_and(|k| k.as_str() == key) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| keyed(child, key))
        }
        _ => None,
    }
}

#[gpui::test]
fn diff_timeline_legend_and_time_ticks_keep_independent_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("diff_timeline_legend_and_ticks"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut visual, view) = gallery(&mut real, "diff_timeline", "modern-light");
    visual.simulate_resize(size(px(1450.), px(1050.)));
    for (family, appearance, caption, faint, paper) in [
        ("Modern", "Light", 0x8f8f8fff, 0xb5b5b5ff, 0xffffffff),
        ("Modern", "Dark", 0x8b8b92ff, 0x5c5c63ff, 0x19191bff),
        ("Clean", "Light", 0x00000073, 0x0000004d, 0xffffffff),
        ("Clean", "Dark", 0xffffff75, 0xffffff4d, 0x1c1c1eff),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut visual, &view);
        let legend = bounds(&mut visual, &view, "text", "Added");
        assert!(
            crate::support::glyphs_with_ink(&mut visual, legend, caption) >= 5,
            "{family}/{appearance}: legend uses its component's caption ink"
        );
        let plot = bounds(&mut visual, &view, "group", "Edit timeline");
        let ticks = gpui_rhai::GeometryBounds {
            y: plot.y + plot.height,
            height: 24.,
            ..plot
        };
        assert!(
            crate::support::glyphs_with_ink(&mut visual, ticks, faint) >= 4,
            "{family}/{appearance}: time axis uses faint ink"
        );
        let root = bounds(&mut visual, &view, "group", "Diff timeline");
        let full_card = live_fills(&mut visual, paper).iter().any(|paint| {
            (f64::from(paint.x) - root.x).abs() <= 1.
                && (f64::from(paint.y) - root.y).abs() <= 1.
                && (f64::from(paint.width) - root.width).abs() <= 1.
                && (f64::from(paint.height) - root.height).abs() <= 1.
        });
        assert_eq!(
            full_card,
            family == "Clean",
            "{family}/{appearance}: action-independent chart surface"
        );
    }
}

#[gpui::test]
fn diff_timeline_preview_paints_code_and_change_signs_separately(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("diff_timeline_preview_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let source = r#"
        import "patterns/diff_timeline" as dt;
        fn view(ctx){dt::DiffTimeline(#{key:"preview",run:"preview",agent:#{id:"ada",name:"Ada"},
            edits:[#{id:"edit",at:1,file:"src/main.rs",added:1,removed:1,diff:[
                #{kind:"add",text:"return added;"},#{kind:"del",text:"return deleted;"},
                #{kind:"ctx",text:"reused();"}]}]}).with_style(style().width(px(560)))}
    "#;
    for (theme, code, add_sign, del_sign, add_tint, del_tint) in [
        (
            gpui_rhai_registry::MODERN_LIGHT_THEME,
            0x171717ff,
            0x1e8a4fff,
            0xc4320aff,
            0x5fc88a24,
            0xf044381a,
        ),
        (
            gpui_rhai_registry::MODERN_DARK_THEME,
            0xedededff,
            0x7dcf9fff,
            0xff8a7aff,
            0x7dcf9f24,
            0xff6b5f24,
        ),
        (
            gpui_rhai_registry::CLEAN_LIGHT_THEME,
            0x000000e0,
            0x1e8a4fff,
            0xc4320aff,
            0x5fc88a24,
            0xf044381a,
        ),
        (
            gpui_rhai_registry::CLEAN_DARK_THEME,
            0xffffffe6,
            0x7dcf9fff,
            0xff8a7aff,
            0x7dcf9f24,
            0xff6b5f24,
        ),
    ] {
        let (mut v, view) = Mount::new(source)
            .registry("patterns/diff_timeline")
            .theme(theme)
            .motion(MotionPreference::None)
            .mount(&mut real);
        v.update(|window, _| window.activate_window());
        ready(&mut v, &view);
        click(&mut v, &view, "group", "Edit timeline");
        v.simulate_keystrokes("home");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Selected edit");
        for (line, sign_ink, tint) in [
            ("return added;", add_sign, add_tint),
            ("return deleted;", del_sign, del_tint),
        ] {
            let label = bounds(&mut v, &view, "text", line);
            let gutter = gpui_rhai::GeometryBounds {
                x: label.x - 18.,
                width: 18.,
                ..label
            };
            assert!(
                glyphs_with_ink(&mut v, label, code) >= 8,
                "missing code body ink: {line}"
            );
            assert_eq!(
                glyphs_with_ink(&mut v, label, sign_ink),
                0,
                "colored code: {line}"
            );
            assert_eq!(
                glyphs_with_ink(&mut v, gutter, sign_ink),
                1,
                "missing sign: {line}"
            );
            assert!(
                live_fills(&mut v, tint).iter().any(|rect| {
                    f64::from(rect.x) <= gutter.x + 0.5
                        && (f64::from(rect.y) - label.y).abs() <= 0.5
                        && f64::from(rect.right()) >= label.x + label.width - 0.5
                        && f64::from(rect.bottom()) >= label.y + label.height - 0.5
                }),
                "missing tinted code row: {line} at {label:?}, {tint:08x}"
            );
        }
        node(&mut v, &view, "text", "reused();");
    }
}

#[gpui::test]
fn diff_timeline_reserves_a_real_count_lane_and_paints_paired_semantic_amounts(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("diff_timeline_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, chip_height) in [
        (gpui_rhai_registry::MODERN_DARK_THEME, 22.),
        (gpui_rhai_registry::CLEAN_DARK_THEME, 20.),
    ] {
        count_lane_case(&mut real, theme, chip_height);
    }
}

fn count_lane_case(real: &mut TestAppContext, theme: &str, chip_height: f64) {
    let (mut v, view) = Mount::new(r#"
        import "patterns/diff_timeline" as dt;
        fn state_schema(){#{fields:#{full:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            narrow:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn init(ctx){ctx.register_action("timeline.full",Fn("full"));ctx.register_action("timeline.narrow",Fn("narrow"));}
        fn full(ctx,p){ctx.set_state("full",true);}
        fn narrow(ctx,p){ctx.set_state("narrow",true);}
        fn view(ctx){
            let edits=[
                #{id:"a",at:1,file:"src/a_much_longer_directory/engine.rs",added:9,removed:1},
                #{id:"b",at:2,file:"src/help.rs",added:2,removed:4},
                #{id:"c",at:60,file:"src/a_much_longer_directory/engine.rs",added:999990,removed:444}
            ];
            dt::DiffTimeline(#{key:"precision",run:"precision",agent:#{id:"ada",name:"Ada"},context:"Review",
                edits:edits,elapsed:if ctx.get_state("full"){70.0}else{5.0}})
                .with_style(style().width(if ctx.get_state("narrow"){px(360)}else{px(580)}))
        }
    "#)
    .registry("patterns/diff_timeline")
    .theme(theme)
    .motion(MotionPreference::None)
    .mount(real);
    v.simulate_resize(size(px(1040.), px(800.)));
    ready(&mut v, &view);
    let widths = |v: &mut VisualTestContext| {
        v.update(|_, cx| {
            let root = view.root(cx).unwrap().unwrap();
            [
                "file:src/a_much_longer_directory/engine.rs",
                "file:src/help.rs",
            ]
            .map(|name| {
                let counts = keyed(keyed(&root, name).unwrap(), "counts").unwrap();
                let Some(LayoutLength::Definite(Length::Pixels(width))) = counts.style().base.width
                else {
                    panic!(
                        "count lane must use the shaped digit advance: {:?}",
                        counts.style().base.width
                    );
                };
                width
            })
        })
    };
    let row = |v: &mut VisualTestContext| {
        bounds(
            v,
            &view,
            "listitem",
            "src/a_much_longer_directory/engine.rs",
        )
    };
    let amount = |v: &mut VisualTestContext, name: &str| {
        let r = row(v);
        snapshot(v, &view)
            .find_by_role_and_name("text", name)
            .filter_map(|node| node.geometry.map(|g| g.visual))
            .find(|g| g.y >= r.y - 1. && g.y + g.height <= r.y + r.height + 1.)
            .unwrap_or_else(|| panic!("the row owns its separate amount {name}"))
    };
    let early = widths(&mut v);
    assert!(
        early[0] > 70. && early[0] < 160. && (early[0] - early[1]).abs() < 0.1,
        "{early:?}"
    );
    let positive = amount(&mut v, "+9");
    let negative = amount(&mut v, "−1");
    assert!(
        positive.x + positive.width < negative.x,
        "{positive:?} and {negative:?}"
    );
    assert!(glyphs_with_ink(&mut v, positive, 0x7dcf9fff) > 0);
    assert!(glyphs_with_ink(&mut v, negative, 0xff8a7aff) > 0);
    let totals = bounds(&mut v, &view, "group", "Change totals");
    assert!(
        live_fills(&mut v, 0x7dcf9f24).iter().any(|q| {
            (f64::from(q.height) - chip_height).abs() < 0.6
                && f64::from(q.x) >= totals.x - 1.
                && f64::from(q.right()) <= totals.x + totals.width + 1.
                && f64::from(q.y) >= totals.y - 1.
                && f64::from(q.bottom()) <= totals.y + totals.height + 1.
        }),
        "the header owns a tinted {chip_height}px status chip"
    );
    assert!(content(&mut v, &view).iter().all(|s| s != "+999,999"));
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: "timeline.full".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    ready(&mut v, &view);
    let full = widths(&mut v);
    assert!(
        (early[0] - full[0]).abs() < 0.1 && (early[1] - full[1]).abs() < 0.1,
        "{early:?} -> {full:?}"
    );
    let bigger = amount(&mut v, "+999,999");
    assert!(bigger.width > positive.width && bigger.x >= row(&mut v).x);
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: "timeline.narrow".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    ready(&mut v, &view);
    let narrow = widths(&mut v);
    assert!(
        narrow[0] < full[0] && (narrow[0] - narrow[1]).abs() < 0.1,
        "{full:?} -> {narrow:?}"
    );
    let title = bounds(&mut v, &view, "heading", "Ada · Review");
    let totals = bounds(&mut v, &view, "group", "Change totals");
    assert!(
        totals.y >= title.y + title.height - 1.,
        "narrow totals move below the title"
    );
    assert!(content(&mut v, &view).iter().any(|s| s == "engine.rs"));
}

#[gpui::test]
fn diff_timeline_full_ledger_remains_within_the_runtime_budget(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(r#"
        import "patterns/diff_timeline" as dt;
        fn view(ctx){
            let edits=[];
            for i in 0..192{
                edits.push(#{id:"e"+i.to_string(),at:i*6,file:"src/part"+i.to_string()+".rs",added:2,removed:1});
            }
            column([dt::DiffTimeline(#{key:"limits",run:"limits",agent:#{id:"test",name:"Test"},edits:edits,max_rows:4})])
                .with_style(style().width(px(590)))
        }
    "#)
        .registry("patterns/diff_timeline")
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "group", "Change totals").description,
        "+384 −192 across 192 files"
    );
    assert!(bounds(&mut v, &view, "list", "Changed files").height <= 129.0);
}

#[gpui::test]
fn diff_timeline_pointer_and_keyboard_inspect_the_same_ledger(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "diff_timeline", "modern-light");
    v.simulate_resize(size(px(1240.0), px(990.0)));
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "group", "Change totals").description,
        "+97 −59 across 4 files"
    );
    click(&mut v, &view, "button", "Early edits");
    assert_eq!(
        node(&mut v, &view, "group", "Change totals").description,
        "+4 −47 across 2 files"
    );
    let semantics = snapshot(&mut v, &view);
    assert_eq!(
        semantics.find_by_role_and_name("text", "Net −43").count(),
        1
    );
    assert_eq!(semantics.find_by_role_and_name("text", "−43").count(), 0);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "src/api.rs")
            .next()
            .is_none()
    );

    let chart = bounds(&mut v, &view, "group", "Edit timeline");
    v.simulate_mouse_move(
        point(
            px((chart.x + chart.width * 0.05) as f32),
            px((chart.y + chart.height * 0.62) as f32),
        ),
        None,
        Modifiers::none(),
    );
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "status", "Selected edit").description,
        "0:06 · src/lib.rs · +0 / −38"
    );
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s.contains("pub mod obsolete_adapter;"))
    );

    click(&mut v, &view, "button", "Show full run");
    let full = bounds(&mut v, &view, "group", "Edit timeline");
    assert!((full.width - chart.width).abs() < 1.0);
    click(&mut v, &view, "group", "Edit timeline");
    v.simulate_keystrokes("home right right");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "status", "Selected edit").description,
        "0:35 · src/api.rs · +85 / −12"
    );
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "status", "Selected edit").description,
        "0:35 · src/api.rs · +8 / −0"
    );
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "status", "Selected edit").description,
        "0:52 · assets/preview.png · +0 / −0"
    );
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s == "Binary file changed")
    );
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Selected edit")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn diff_timeline_reserves_lanes_and_follows_live_files_after_resize(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "diff_timeline", theme);
        v.simulate_resize(size(px(1230.0), px(930.0)));
        ready(&mut v, &view);
        let list = bounds(&mut v, &view, "list", "Changed files");
        let first = bounds(&mut v, &view, "listitem", "src/lib.rs");
        let fourth = bounds(&mut v, &view, "listitem", "assets/preview.png");
        assert!(
            first.y >= list.y - 1.0 && fourth.y + fourth.height <= list.y + list.height + 1.0,
            "{theme}: already-visible live row must not scroll past earlier rows"
        );
        click(&mut v, &view, "button", "Early edits");
        let reserved = bounds(&mut v, &view, "list", "Changed files");
        assert!(
            (list.height - reserved.height).abs() < 1.0,
            "{theme}: {list:?} {reserved:?}"
        );
        click(&mut v, &view, "button", "Advance playback");
        click(&mut v, &view, "button", "Advance playback");
        click(&mut v, &view, "button", "Advance playback");
        let editing = node(&mut v, &view, "status", "Editing examples/账户.rhai");
        let tree = snapshot(&mut v, &view);
        let child_bounds = |name: &str| {
            editing
                .children
                .iter()
                .filter_map(|id| tree.node(*id))
                .find(|node| node.name == name)
                .unwrap_or_else(|| panic!("missing live status child {name}"))
                .geometry
                .unwrap()
                .visual
        };
        let prefix = child_bounds("Editing");
        let directory = child_bounds("examples/");
        let filename = child_bounds("账户.rhai");
        assert!(
            (directory.x - (prefix.x + prefix.width) - 4.).abs() <= 0.5,
            "{theme}: {prefix:?} {directory:?}"
        );
        assert!(
            (filename.x - (directory.x + directory.width)).abs() <= 0.5,
            "{theme}: {directory:?} {filename:?}"
        );
        let current = node(&mut v, &view, "listitem", "examples/账户.rhai")
            .geometry
            .unwrap();
        let viewport = bounds(&mut v, &view, "list", "Changed files");
        assert!(
            current.visual.y >= viewport.y - 1.0
                && current.visual.y + current.visual.height <= viewport.y + viewport.height + 1.0,
            "{theme}: {current:?} {viewport:?}"
        );

        click(&mut v, &view, "button", "Narrow timeline");
        let card = bounds(&mut v, &view, "group", "Diff timeline");
        let chart = bounds(&mut v, &view, "group", "Edit timeline");
        assert!(
            card.width <= 350.5
                && chart.x > card.x
                && chart.x + chart.width <= card.x + card.width + 0.5,
            "{theme}: {card:?} {chart:?}"
        );
        click(&mut v, &view, "button", "Empty ledger");
        assert_eq!(
            node(&mut v, &view, "group", "Change totals").description,
            "+0 −0 across 0 files"
        );
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("listitem", "src/lib.rs")
                .next()
                .is_none()
        );
    }
}
