use std::time::{Duration, Instant};

use gpui::{
    ClipboardItem, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{
    AutomationCommand, ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue,
    VirtualCollectionMetrics,
};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, snapshot};

fn mount(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut v, view) = gallery(cx, "collaboration_stream", theme);
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    (v, view)
}

fn button(v: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    click(v, view, "button", label);
}

#[gpui::test]
fn collaboration_stream_uses_the_own_quote_surface_only_in_clean(cx: &mut TestAppContext) {
    let source = r#"
        import "patterns/collaboration_stream" as stream;
        fn view(ctx){stream::CollaborationStream(#{key:"stream",session:"quote-paper",
            agents:[#{id:"scribe",name:"Scribe"},#{id:"atlas",name:"Atlas"}],max_height:320,
            messages:[
                #{id:"question",from:"scribe",to:["you"],kind:"question",text:"Can we ship?"},
                #{id:"aside",from:"atlas",to:["you"],kind:"share",text:"Checks passed."},
                #{id:"reply",from:"you",to:["scribe"],kind:"reply",text:"Yes.",reply_to:"question"}
            ]})}
    "#;
    for (theme, paper) in [
        (gpui_rhai_registry::MODERN_LIGHT_THEME, 0x0000000b),
        (gpui_rhai_registry::MODERN_DARK_THEME, 0xffffff0d),
        (gpui_rhai_registry::CLEAN_LIGHT_THEME, 0xffffffff),
        (gpui_rhai_registry::CLEAN_DARK_THEME, 0x1c1c1fff),
    ] {
        let (mut v, view) = Mount::new(source)
            .registry("patterns/collaboration_stream")
            .theme(theme)
            .motion(MotionPreference::None)
            .mount(cx);
        ready(&mut v, &view);
        let subject = bounds(&mut v, &view, "button", "Show message from Scribe");
        assert!(
            crate::support::live_fills(&mut v, paper)
                .iter()
                .any(|rect| (f64::from(rect.x) - subject.x).abs() <= 1.
                    && (f64::from(rect.y) - subject.y).abs() <= 1.
                    && (f64::from(rect.width) - subject.width).abs() <= 1.
                    && (f64::from(rect.height) - subject.height).abs() <= 1.),
            "own quoted message did not paint its expected surface {paper:08x}"
        );
    }
}

fn participant_strip_fixture(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let script = r#"
        import "patterns/collaboration_stream" as stream;
        fn state_schema(){#{fields:#{
            reorder:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            wide:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
        }}}
        fn reorder(ctx,event){ctx.set_state("reorder",true);}
        fn widen(ctx,event){ctx.set_state("wide",true);}
        fn view(ctx){
            let agents=[#{id:"atlas",name:"Atlas"},#{id:"scribe",name:"Scribe"},#{id:"team/🧪",name:"Chemistry"}];
            if ctx.get_state("reorder"){agents.insert(0,#{id:"newcomer",name:"Newcomer"});}
            let messages=[#{id:"1",from:"atlas",to:["all"],kind:"share",text:"Plan"},
                #{id:"2",from:"scribe",to:["all"],kind:"share",text:"Draft"},
                #{id:"3",from:"team/🧪",to:["all"],kind:"share",text:"Independent check"}];
            column([
                text("Reorder").accessibility_role("button").on_click(Fn("reorder")),
                text("Widen").accessibility_role("button").on_click(Fn("widen")),
                stream::CollaborationStream(#{key:"stream",session:"roster-shape",title:"Test participants",
                    agents:agents,messages:messages,max_height:240})
                    .with_style(style().width(px(if ctx.get_state("wide"){600}else{190})))
            ]).with_style(style().items_start().gap(px(10)))
        }
    "#;
    let (mut v, view) = Mount::new(script)
        .registry("patterns/collaboration_stream")
        .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(MotionPreference::None)
        .mount(cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1280.), px(900.)));
    ready(&mut v, &view);
    (v, view)
}

fn strip_mark(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    id: &str,
) -> (Vec<i32>, Option<gpui_rhai::ColorValue>) {
    use gpui_rhai::{UiNode, UiNodeKind};
    fn find<'a>(n: &'a UiNode, k: &str) -> Option<&'a UiNode> {
        if n.key().is_some_and(|key| key.as_str() == k) {
            return Some(n);
        }
        match n.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                children.iter().find_map(|child| find(child, k))
            }
            _ => None,
        }
    }
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let strip = find(&root, "agents").unwrap();
    let tab = find(strip, id).unwrap();
    let UiNodeKind::Box { children } = tab.kind() else {
        panic!("expected a native tab")
    };
    let mark = &children[0];
    let UiNodeKind::Box { children } = mark.kind() else {
        panic!("expected a mark wrapper")
    };
    let UiNodeKind::Svg { source } = children[0].kind() else {
        panic!("expected fallback SVG")
    };
    let coordinate = |xml: &str, name: &str| {
        let start = format!("{name}='");
        let value = xml
            .split_once(&start)
            .unwrap()
            .1
            .split_once(char::from(39_u8))
            .unwrap()
            .0;
        value.parse::<f64>().unwrap()
    };
    let positions = source
        .as_str()
        .split("<rect ")
        .skip(1)
        .map(|dot| {
            let x = ((coordinate(dot, "x") - 2.) / 4.5).round() as i32;
            let y = ((coordinate(dot, "y") - 2.) / 4.5).round() as i32;
            3 * y + x
        })
        .collect();
    (positions, mark.style().base.text_color.clone())
}

#[gpui::test]
fn collaboration_stream_fallback_shapes_follow_business_id_not_roster_order(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = participant_strip_fixture(cx);
    let atlas = strip_mark(&mut v, &view, "atlas");
    let scribe = strip_mark(&mut v, &view, "scribe");
    let chemistry = strip_mark(&mut v, &view, "team/🧪");
    assert_eq!(atlas.0, (0..9).collect::<Vec<_>>());
    assert_eq!(scribe.0, [0, 2, 6, 7, 8]);
    assert_eq!(chemistry.0, [3, 4, 5, 6, 8]);
    button(&mut v, &view, "Reorder");
    let after = strip_mark(&mut v, &view, "atlas");
    assert_ne!(
        atlas.1, after.1,
        "the authored palette follows roster order"
    );
    assert_eq!(atlas.0, after.0);
    assert_eq!(scribe.0, strip_mark(&mut v, &view, "scribe").0);
    assert_eq!(chemistry.0, strip_mark(&mut v, &view, "team/🧪").0);
}

fn writing_dots(visual: &mut VisualTestContext) -> Vec<(f32, f32, f32)> {
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let ink = gpui::rgb(0x0060e2);
            let mut dots = Vec::new();
            crate::support::walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, pose| {
                    for quad in &scene.quads {
                        let Some(color) = quad.background.as_solid().map(gpui::Rgba::from) else {
                            continue;
                        };
                        let rect = crate::support::logical_paint_rect(quad.bounds, pose, scale);
                        if (rect.width - 3.).abs() <= 0.5
                            && (rect.height - 3.).abs() <= 0.5
                            && (color.r - ink.r).abs() < 0.002
                            && (color.g - ink.g).abs() < 0.002
                            && (color.b - ink.b).abs() < 0.002
                        {
                            assert!((quad.corner_radii.top_left.0 / scale - 0.75).abs() < 0.01);
                            dots.push((rect.x, rect.y, color.a));
                        }
                    }
                },
            );
            dots.sort_by(|(x, y, _), (u, v, _)| y.total_cmp(v).then_with(|| x.total_cmp(u)));
            dots
        })
    })
}

#[gpui::test]
fn collaboration_stream_writing_marks_twinkle_in_diagonals_on_the_runtime_clock(
    cx: &mut TestAppContext,
) {
    let script = r#"
        import "patterns/collaboration_stream" as stream;
        fn state_schema(){#{fields:#{writing:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn toggle(ctx,event){ctx.set_state("writing",!ctx.get_state("writing"));}
        fn view(ctx){
            column([
                text("Toggle writing").accessibility_role("button").on_click(Fn("toggle")),
                stream::CollaborationStream(#{key:"stream",session:"wave",title:"Participants",
                    agents:[#{id:"scribe",name:"Scribe"}],max_height:240,
                    messages:[#{id:"one",from:"scribe",to:["all"],kind:"share",
                        text:"Published result.",typing:ctx.get_state("writing")}]})
            ])
        }
    "#;
    let nearly = |left: f32, right: f32| assert!((left - right).abs() < 0.02, "{left} != {right}");
    for (theme, preference) in [
        (
            gpui_rhai_registry::MODERN_LIGHT_THEME,
            MotionPreference::Normal,
        ),
        (
            gpui_rhai_registry::CLEAN_DARK_THEME,
            MotionPreference::Normal,
        ),
        (
            gpui_rhai_registry::MODERN_DARK_THEME,
            MotionPreference::Reduced,
        ),
        (
            gpui_rhai_registry::CLEAN_LIGHT_THEME,
            MotionPreference::None,
        ),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let runtime_clock = clock.clock();
        let (mut v, view) = Mount::new(script)
            .registry("patterns/collaboration_stream")
            .theme(theme)
            .motion(preference)
            .with(move |view| view.runtime_clock(runtime_clock.clone()))
            .mount(cx);
        v.update(|window, _| window.activate_window());
        ready(&mut v, &view);
        assert!(
            writing_dots(&mut v).is_empty(),
            "quiet marks use the static vector"
        );
        button(&mut v, &view, "Toggle writing");
        let first = writing_dots(&mut v);
        assert_eq!(
            first.len(),
            9,
            "each cell in the writing mark has native paint"
        );
        for (index, &(x, y, alpha)) in first.iter().enumerate() {
            let positions = [0.0, 4.5, 9.0];
            assert!((x - first[0].0 - positions[index % 3]).abs() <= 0.5);
            assert!((y - first[0].1 - positions[index / 3]).abs() <= 0.5);
            let on = [0, 2, 6, 7, 8].contains(&index);
            nearly(
                alpha,
                if preference == MotionPreference::Normal || on {
                    0.78
                } else {
                    0.78 * 0.16
                },
            );
        }
        v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        clock.advance(Duration::from_millis(600));
        ready(&mut v, &view);
        let first_trough = writing_dots(&mut v);
        clock.advance(Duration::from_millis(480));
        ready(&mut v, &view);
        let final_trough = writing_dots(&mut v);
        if preference == MotionPreference::Normal {
            nearly(first_trough[0].2, 0.78 * 0.2);
            nearly(first_trough[1].2, first_trough[3].2);
            nearly(first_trough[2].2, first_trough[4].2);
            nearly(first_trough[4].2, first_trough[6].2);
            assert!(first_trough[8].2 > 0.78 * 0.90);
            nearly(final_trough[8].2, 0.78 * 0.2);
            assert!(final_trough[0].2 > 0.78 * 0.90);
        } else {
            assert_eq!(first_trough, first);
            assert_eq!(final_trough, first);
        }
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty()
        );
        if preference == MotionPreference::Normal {
            let writing = node(&mut v, &view, "button", "Scribe: Writing…");
            v.update(|_, cx| {
                assert!(
                    view.set_motion_preference(MotionPreference::Reduced, cx)
                        .unwrap()
                )
            });
            ready(&mut v, &view);
            let still = writing_dots(&mut v);
            assert_eq!(still.len(), 9);
            for (index, &(_, _, opacity)) in still.iter().enumerate() {
                let on = [0, 2, 6, 7, 8].contains(&index);
                nearly(opacity, if on { 0.78 } else { 0.78 * 0.16 });
            }
            v.update(|_, cx| {
                assert!(
                    view.set_motion_preference(MotionPreference::Normal, cx)
                        .unwrap()
                )
            });
            ready(&mut v, &view);
            let restarted = writing_dots(&mut v);
            clock.advance(Duration::from_millis(400));
            ready(&mut v, &view);
            assert_ne!(writing_dots(&mut v), restarted, "restored motion advances");
            let unchanged = node(&mut v, &view, "button", "Scribe: Writing…");
            assert_eq!(unchanged.id, writing.id);
            assert_eq!(unchanged.geometry, writing.geometry);
        }
        button(&mut v, &view, "Toggle writing");
        assert!(
            writing_dots(&mut v).is_empty(),
            "finishing restores the quiet mark"
        );
        node(&mut v, &view, "button", "Scribe: Published result.");
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn collaboration_stream_tab_fades_follow_the_native_horizontal_boundaries(cx: &mut TestAppContext) {
    let (mut v, view) = participant_strip_fixture(cx);
    let edges = |v: &mut VisualTestContext| {
        v.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                let mut edges = (false, false);
                crate::support::walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::default(),
                    &mut |scene, _| {
                        for group in &scene.isolated_groups {
                            let ramps = group.alpha_ramps();
                            if ramps[4] == 1. {
                                edges.0 |= ramps[1] > 0.;
                                edges.1 |= ramps[3] > 0.;
                            }
                        }
                    },
                );
                edges
            })
        })
    };
    assert_eq!(edges(&mut v), (false, true));
    button(&mut v, &view, "Everyone");
    v.simulate_keystrokes("end");
    ready(&mut v, &view);
    let focus_is = |v: &mut VisualTestContext, label: &str| {
        let target = node(v, &view, "button", label).id;
        let path = v.update(|window, cx| view.debug_frame_digest(window, cx).unwrap().focus_path);
        assert_eq!(path.first(), Some(&target));
    };
    focus_is(&mut v, "Chemistry");
    // Keyboard reveal is minimal; it can leave the viewport's trailing
    // padding outside. Check the physical endpoint with a clamped wheel.
    let wheel = |v: &mut VisualTestContext, dx| {
        let strip = bounds(v, &view, "toolbar", "Test participants");
        v.simulate_event(ScrollEvent {
            position: crate::support::center(strip),
            delta: ScrollDelta::Pixels(point(px(dx), px(0.))),
            ..Default::default()
        });
        ready(v, &view);
    };
    wheel(&mut v, -900.);
    assert_eq!(edges(&mut v), (true, false));
    v.simulate_keystrokes("home");
    ready(&mut v, &view);
    focus_is(&mut v, "Everyone");
    wheel(&mut v, 900.);
    assert_eq!(edges(&mut v), (false, true));
    button(&mut v, &view, "Widen");
    let strip = bounds(&mut v, &view, "toolbar", "Test participants");
    let last = bounds(&mut v, &view, "button", "Chemistry");
    assert!(
        strip.width > 400. && last.x + last.width <= strip.x + strip.width,
        "wide strip must physically fit the tab: {strip:?} {last:?}"
    );
    let ramps = v.update(|window, _| {
        window.visual_probe_with_painted_scene(|scene| {
            let mut result = Vec::new();
            crate::support::walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, _| {
                    result.extend(
                        scene
                            .isolated_groups
                            .iter()
                            .map(|group| group.alpha_ramps())
                            .filter(|r| r[4] == 1.),
                    );
                },
            );
            result
        })
    });
    assert_eq!(
        edges(&mut v),
        (false, false),
        "{strip:?} {last:?} ramps={ramps:?}"
    );
}

#[gpui::test]
fn collaboration_stream_toolbar_preserves_each_designs_own_content_inset(cx: &mut TestAppContext) {
    for (theme, strip_dx, first_dx) in [("modern-light", 6., 8.), ("clean-dark", -8., -8.)] {
        let (mut v, view) = mount(cx, theme);
        let frame = bounds(&mut v, &view, "region", "Agent chat");
        let strip = bounds(&mut v, &view, "toolbar", "Agent chat");
        let first = bounds(&mut v, &view, "button", "Everyone");
        assert!(
            (strip.x - frame.x - strip_dx).abs() <= 0.5,
            "{theme}: {frame:?} {strip:?}"
        );
        assert!(
            (first.x - frame.x - first_dx).abs() <= 0.5,
            "{theme}: {frame:?} {first:?}"
        );
        button(&mut v, &view, "Everyone");
        v.simulate_keystrokes("end");
        ready(&mut v, &view);
        let expected = node(&mut v, &view, "button", "Scribe").id;
        let focus = v.update(|window, cx| view.debug_frame_digest(window, cx).unwrap().focus_path);
        assert_eq!(focus.first(), Some(&expected));
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn collaboration_stream_participant_ink_follows_hover_and_selection(cx: &mut TestAppContext) {
    use crate::support::{center, glyphs_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("stream_participant_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, muted, hover, selected) in [
        ("modern-dark", 0x7c7c82ff, 0xa1a1a6ff, 0xedededff),
        ("clean-dark", 0xffffff73, 0xffffff9e, 0xffffffe6),
    ] {
        let (mut v, view) = mount(&mut real, theme);
        let atlas = bounds(&mut v, &view, "button", "Atlas");
        let away = point(px(8.), px(8.));
        v.simulate_mouse_move(away, None, gpui::Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, atlas, muted) >= 4,
            "{theme}: resting"
        );
        v.simulate_mouse_move(center(atlas), None, gpui::Modifiers::none());
        ready(&mut v, &view);
        assert!(glyphs_with_ink(&mut v, atlas, hover) >= 4, "{theme}: hover");
        assert_eq!(
            glyphs_with_ink(&mut v, atlas, muted),
            0,
            "{theme}: stale ink"
        );
        v.simulate_mouse_move(away, None, gpui::Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, atlas, muted) >= 4,
            "{theme}: restored"
        );
        button(&mut v, &view, "Atlas");
        assert!(
            glyphs_with_ink(&mut v, atlas, selected) >= 4,
            "{theme}: selected"
        );
        assert_eq!(node(&mut v, &view, "button", "Atlas").pressed, Some(true));
    }
}

#[gpui::test]
fn collaboration_stream_recipient_summary_fits_without_changing_send_targets(
    cx: &mut TestAppContext,
) {
    let script = r#"
        import "patterns/collaboration_stream" as stream;
        fn state_schema(){#{fields:#{
            tight:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            sent:#{schema:#{type:"string"},"default":#{type:"string",value:""}}
        }}}
        fn narrow(ctx,event){ctx.set_state("tight",true);}
        fn edited(ctx,message){}
        fn sent(ctx,message){
            let ids="";for id in message.to{if ids!=""{ids+=",";}ids+=id;}
            ctx.set_state("sent",ids);
        }
        fn view(ctx){
            let agents=[#{id:"a",name:"Orchard"},#{id:"b",name:"Bayfront"},#{id:"c",name:"Cedar"},
                #{id:"d",name:"Delta"},#{id:"e",name:"Echo"}];
            let card=stream::CollaborationStream(#{key:"chat",session:"recipient-fixture",agents:agents,
                messages:[],max_height:140,draft:"@Orchard @Bayfront @Cedar @Delta @Echo please review",
                on_change:Fn("edited"),on_send:Fn("sent")});
            column([
                text("Narrow").with_key("narrow").accessibility_role("button").on_click(Fn("narrow")),
                card.with_style(style().width(px(if ctx.get_state("tight"){220}else{600}))),
                text(ctx.get_state("sent")).with_key("sent")
            ]).with_style(style().items_start().gap(px(12)))
        }
    "#;
    let (mut v, view) = Mount::new(script)
        .registry("patterns/collaboration_stream")
        .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut v, &view);
    let full = "to Orchard, Bayfront, Cedar, Delta, Echo";
    let group = node(&mut v, &view, "group", full);
    let visible = |tree: &gpui_rhai::AccessibilityTree, words: &str| {
        tree.find_by_role_and_name("text", words)
            .any(|node| node.geometry.is_some())
    };
    let tree = snapshot(&mut v, &view);
    assert!(visible(&tree, "Orchard") && visible(&tree, "Bayfront") && visible(&tree, "+3"));
    assert!(
        !visible(&tree, "Cedar"),
        "there are at most three visual cells after the prefix"
    );
    let send = bounds(&mut v, &view, "button", "Send message");
    click(&mut v, &view, "button", "Narrow");
    let smaller = node(&mut v, &view, "group", full);
    assert_eq!(smaller.id, group.id);
    let tree = snapshot(&mut v, &view);
    let people = ["Orchard", "Bayfront"]
        .iter()
        .filter(|name| visible(&tree, name))
        .count();
    assert!(people < 2, "a narrow viewport omits an optional recipient");
    assert!(
        visible(&tree, &format!("+{}", 5 - people)),
        "the measured summary absorbs both kinds of omission"
    );
    assert!((bounds(&mut v, &view, "button", "Send message").width - send.width).abs() <= 0.5);
    click(&mut v, &view, "button", "Send message");
    node(&mut v, &view, "text", "a,b,c,d,e");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn collaboration_stream_composer_uses_the_authored_height_range(cx: &mut TestAppContext) {
    for theme in ["clean-light", "modern-dark"] {
        let (mut v, view) = mount(cx, theme);
        let label = "Message the agents";
        click(&mut v, &view, "text_field", label);
        let before = node(&mut v, &view, "text_field", label);
        let focus = v.update(|window, cx| window.focused(cx)).unwrap();
        assert!((before.geometry.unwrap().visual.height - 40.).abs() < 1.);
        let draft = (1..=12)
            .map(|n| format!("Line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        v.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(draft.clone())));
        v.simulate_keystrokes("cmd-v");
        ready(&mut v, &view);
        let overflow = node(&mut v, &view, "text_field", label);
        let rect = overflow.geometry.unwrap().visual;
        assert!((rect.height - 120.).abs() < 1., "{theme}: {rect:?}");
        let inner_shadow = v.update(|window, _| {
            use crate::support::{logical_paint_rect, walk_live_paint};
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut found = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::default(),
                    &mut |scene, pose| {
                        found |= scene.shadows.iter().any(|shadow| {
                            let area = logical_paint_rect(shadow.element_bounds, pose, scale);
                            [
                                (area.x, rect.x),
                                (area.y, rect.y),
                                (area.width, rect.width),
                                (area.height, rect.height),
                            ]
                            .iter()
                            .all(|(actual, expected)| (f64::from(*actual) - expected).abs() <= 1.)
                        });
                    },
                );
                found
            })
        });
        assert!(
            !inner_shadow,
            "{theme}: the editable box must not get a second focus shadow"
        );
        assert_eq!(overflow.id, before.id);
        assert_eq!(
            v.update(|window, cx| window.focused(cx)),
            Some(focus.clone())
        );
        let send = bounds(&mut v, &view, "button", "Send message");
        assert!(send.y >= rect.y + rect.height - 0.5);
        v.update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "gallery.set_scale".into(),
                    payload: Some(UiValue::String("150".into())),
                },
                window,
                cx,
            )
        })
        .unwrap();
        ready(&mut v, &view);
        let scaled = bounds(&mut v, &view, "text_field", label);
        assert!((scaled.height - 180.).abs() < 1., "{theme}: {scaled:?}");
        assert_eq!(node(&mut v, &view, "text_field", label).id, before.id);
        v.simulate_keystrokes("cmd-a");
        v.simulate_input("A");
        ready(&mut v, &view);
        assert!((bounds(&mut v, &view, "text_field", label).height - 60.).abs() < 1.);
        assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
        v.simulate_keystrokes("enter");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Message proposed");
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn collaboration_stream_attachment_and_reply_paint_the_body_ink_in_all_presets(
    cx: &mut TestAppContext,
) {
    use crate::support::{center, glyphs_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("stream_attachment_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, body, active) in [
        ("modern-light", 0x5f5f5fff, 0x171717ff),
        ("modern-dark", 0xa1a1a6ff, 0xedededff),
        ("clean-light", 0x0000009c, 0x000000e0),
        ("clean-dark", 0xffffff9e, 0xffffffe6),
    ] {
        let (mut v, view) = mount(&mut real, theme);
        let reply = bounds(&mut v, &view, "button", "Reply to Atlas");
        let file = bounds(&mut v, &view, "button", "Open checks.csv");
        assert!((file.height - 24.).abs() < 1., "{theme}: {file:?}");
        assert!(glyphs_with_ink(&mut v, reply, body) >= 4, "{theme}: Reply");
        assert!(
            glyphs_with_ink(&mut v, file, body) >= 6,
            "{theme}: attachment"
        );

        v.simulate_mouse_move(center(file), None, gpui::Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, file, active) >= 6,
            "{theme}: attachment hover"
        );
        assert_eq!(glyphs_with_ink(&mut v, file, body), 0, "{theme}: stale ink");

        let rules = gpui_rhai::AuditRules::from_ids(["text-edge-misaligned"]).unwrap();
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert!(
            !findings.iter().any(|f| f.path.contains("/author-mark/")),
            "{findings:#?}"
        );
        button(&mut v, &view, "Open checks.csv");
        node(&mut v, &view, "dialog", "checks.csv");
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn glyph_band_count(scene: &gpui::Scene) -> usize {
    let count: usize = scene
        .isolated_groups
        .iter()
        .map(|g| {
            let ramps = g.alpha_ramps();
            let hit = ramps[4] == 1.
                && ramps[1] > 0.
                && ramps[3] > 0.
                && !g.source().monochrome_sprites.is_empty();
            usize::from(hit) + glyph_band_count(g.source())
        })
        .sum();
    #[cfg(feature = "gpui-fast")]
    let count = count
        + scene
            .layers
            .frames
            .iter()
            .flat_map(|layer| layer.content.scenes())
            .map(glyph_band_count)
            .sum::<usize>();
    count
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn thinking_labels_share_glyph_paint_and_end_when_the_host_finishes_or_stops(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let source = r#"
        import "components/thinking_loader" as loader;
        import "components/thinking_state" as steps;
        fn state_schema(){#{fields:#{done:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn finish(c,e){c.set_state("done",true);}
        fn view(c){
            let done=c.get_state("done");
            let status=if done{"stopped"}else{"working"};
            let duration=c.motion_duration("thinking_sweep");
            column([
                text("Sweep "+duration.to_string()).accessibility_role("status"),
                loader::ThinkingLoader(#{key:"loader",phases:["Reading the proposal"],done:done,done_label:"Done reading"}),
                steps::ThinkingState(#{key:"steps",label:"Review in progress",status:status,steps:[
                    #{id:"active",label:"Validate source",state:"running"}
                ]}),
                text("Finish").accessibility_role("button").on_click(Fn("finish"))
            ]).with_style(style().width(px(420)).gap(px(8)))
        }
    "#;
    for (theme, duration) in [
        (gpui_rhai_registry::MODERN_LIGHT_THEME, 1800),
        (gpui_rhai_registry::CLEAN_LIGHT_THEME, 1600),
    ] {
        let mut native = TestAppContext::build_with_text_system(
            cx.dispatcher.clone(),
            Some("thinking_glyph_sweep"),
            std::sync::Arc::new(apple::AppleTextSystem::new()),
        );
        let clock = ManualRuntimeClock::new(Instant::now());
        let binding = clock.clock();
        let (mut v, view) = Mount::components(source)
            .theme(theme)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(binding))
            .mount(&mut native);
        v.update(|w, _| w.activate_window());
        ready(&mut v, &view);
        node(&mut v, &view, "status", &format!("Sweep {duration}"));
        clock.advance(Duration::from_millis(768));
        ready(&mut v, &view);
        let count = |v: &mut VisualTestContext| {
            v.update(|w, _| w.visual_probe_with_painted_scene(glyph_band_count))
        };
        assert_eq!(
            count(&mut v),
            3,
            "loader, header, and active step each paint one band"
        );
        button(&mut v, &view, "Finish");
        clock.advance(Duration::from_millis(680));
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Done reading");
        node(&mut v, &view, "button", "Stopped");
        assert_eq!(count(&mut v), 0);
    }
}

fn metrics(v: &mut VisualTestContext, view: &ScriptViewHandle) -> VirtualCollectionMetrics {
    v.update(|_, cx| view.take_performance_snapshot(cx).unwrap())
        .virtual_collections
        .into_iter()
        .find(|m| m.item_count >= 6)
        .unwrap()
}

#[gpui::test]
fn collaboration_stream_only_follows_the_tail_while_the_reader_is_there(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "modern-light");
    let initial = metrics(&mut v, &view);
    assert!(initial.is_following_tail);
    assert_eq!(initial.item_count, 6, "paint insets cannot become messages");
    let viewport = bounds(&mut v, &view, "list", "Collaboration messages");
    let last = bounds(
        &mut v,
        &view,
        "button",
        "Atlas: Should we keep the quick-start example?",
    );
    assert!(viewport.y + viewport.height - last.y - last.height >= 20.);
    #[cfg(feature = "vendor-gpui")]
    assert!(v.update(|window, _| window.visual_probe_with_painted_scene(has_vertical_edges)));
    let list = v
        .debug_bounds("virtual-list:transcript:editorial:0")
        .unwrap();
    v.simulate_event(ScrollEvent {
        position: point(list.left() + px(40.), list.top() + px(6.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(340.))),
        ..Default::default()
    });
    ready(&mut v, &view);
    let reading = metrics(&mut v, &view);
    assert!(!reading.is_following_tail);
    button(&mut v, &view, "Append message");
    let appended = metrics(&mut v, &view);
    assert!(!appended.is_following_tail);
    assert_eq!(appended.item_count, 7);
    assert_eq!(appended.scroll_item, reading.scroll_item);
    button(&mut v, &view, "1 new ↓");
    assert!(metrics(&mut v, &view).is_following_tail);
    node(
        &mut v,
        &view,
        "button",
        "Scribe: Review update 1: the example works in a clean workspace.",
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn has_vertical_edges(scene: &gpui::Scene) -> bool {
    let drawn = scene.isolated_groups.iter().any(|group| {
        let ramps = group.alpha_ramps();
        (ramps[1] > 0. && ramps[3] > 0. && ramps[4] == 0.) || has_vertical_edges(group.source())
    });
    #[cfg(feature = "gpui-fast")]
    let drawn = drawn
        || scene
            .layers
            .frames
            .iter()
            .flat_map(|layer| layer.content.scenes())
            .any(has_vertical_edges);
    drawn
}

#[cfg(feature = "vendor-gpui")]
pub(super) fn glyph_band_center(scene: &gpui::Scene) -> Option<f32> {
    for g in &scene.isolated_groups {
        let ramps = g.alpha_ramps();
        if ramps[4] == 1.
            && ramps[1] > 0.
            && ramps[3] > 0.
            && !g.source().monochrome_sprites.is_empty()
        {
            return Some(g.transform().translation[0] + ramps[2]);
        }
        if let Some(child) = glyph_band_center(g.source()) {
            return Some(child);
        }
    }
    #[cfg(feature = "gpui-fast")]
    return scene
        .layers
        .frames
        .iter()
        .flat_map(|layer| layer.content.scenes())
        .find_map(glyph_band_center);
    #[cfg(not(feature = "gpui-fast"))]
    None
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn collaboration_stream_glyph_band_uses_the_native_clock_and_respects_motion_policy(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut fonts = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("stream_glyph_sweep"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let cx = &mut fonts;
    for (key, preference) in [
        ("normal", MotionPreference::Normal),
        ("reduced", MotionPreference::Reduced),
        ("disabled", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "collaboration_stream".into(),
            theme: "clean-dark".into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(preference)
        .prepare()
        .unwrap();
        let (win, view) =
            super::mount_prepared(cx, prepared, (key, "gallery"), false, |host, view| Host {
                host,
                view,
            });
        let mut visual = VisualTestContext::from_window(*win, cx);
        visual.update(|w, _| w.activate_window());
        visual.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut visual, &view);
        button(&mut visual, &view, "Show typing");
        let writing = "Scribe: Writing…";
        let first = node(&mut visual, &view, "button", writing);
        let sample = |v: &mut VisualTestContext| {
            v.update(|w, _| w.visual_probe_with_painted_scene(glyph_band_center))
        };
        assert!(sample(&mut visual).is_none(), "no highlight at launch");
        clock.advance(Duration::from_millis(640));
        ready(&mut visual, &view);
        let left = sample(&mut visual);
        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        clock.advance(Duration::from_millis(280));
        ready(&mut visual, &view);
        let right = sample(&mut visual);
        if preference == MotionPreference::Normal {
            assert!(
                right.unwrap() > left.unwrap() + 10.,
                "actual band advances across glyphs"
            );
        } else {
            assert!(
                left.is_none() && right.is_none(),
                "motion preference suppresses the band"
            );
        }
        let same = node(&mut visual, &view, "button", writing);
        assert_eq!(same.id, first.id);
        assert_eq!(same.geometry, first.geometry);
        assert!(
            visual
                .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty(),
            "motion frames do not evaluate Rhai"
        );
    }
}

#[gpui::test]
fn collaboration_stream_keeps_a_rejected_reply_and_targets_by_participant_id(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = mount(cx, "clean-dark");
    button(&mut v, &view, "Refuse next send");
    button(&mut v, &view, "Reply to Atlas");
    v.simulate_input("@Scribe Keep the example.");
    ready(&mut v, &view);
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Message proposed");
    assert!(node(&mut v, &view, "button", "Send message").disabled);
    v.executor().advance_clock(Duration::from_millis(550));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Send declined by Host");
    assert!(!node(&mut v, &view, "button", "Send message").disabled);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "to Atlas, Scribe"),
        "reply recipient and typed mention must remain after rejection"
    );
    button(&mut v, &view, "Send message");
    v.executor().advance_clock(Duration::from_millis(550));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Reply accepted by Host: atlas");
    node(&mut v, &view, "button", "You: @Scribe Keep the example.");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Reply to Atlas")
            .next()
            .is_none()
    );
    click(&mut v, &view, "text_field", "Message the agents");
    v.simulate_input("New topic");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "to Everyone")
    );
}

#[gpui::test]
fn collaboration_stream_quote_and_attachment_work_after_narrow_reflow(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "modern-dark");
    let id = node(
        &mut v,
        &view,
        "button",
        "Atlas: Should we keep the quick-start example?",
    )
    .id;
    button(&mut v, &view, "Narrow stream");
    assert_eq!(
        node(
            &mut v,
            &view,
            "button",
            "Atlas: Should we keep the quick-start example?"
        )
        .id,
        id
    );
    button(&mut v, &view, "Show message from You");
    assert!(!metrics(&mut v, &view).is_following_tail);
    node(
        &mut v,
        &view,
        "button",
        "You: Check the release guide before we share it.",
    );
    button(&mut v, &view, "Jump to latest");
    button(&mut v, &view, "Open review/");
    node(&mut v, &view, "dialog", "review/");
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "notes.md")
    );
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("dialog", "review/")
            .next()
            .is_none()
    );
    button(&mut v, &view, "Open review/");
    button(&mut v, &view, "Close preview");
    button(&mut v, &view, "Open checks.csv");
    node(&mut v, &view, "group", "Table preview");
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "Modern, Clean")
    );
}

#[gpui::test]
fn collaboration_stream_completes_a_mention_before_sending(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "clean-light");
    click(&mut v, &view, "text_field", "Message the agents");
    v.simulate_input("@Scr");
    ready(&mut v, &view);
    node(&mut v, &view, "button", "Address Scribe");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.name == "to Scribe")
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Message proposed")
            .next()
            .is_none()
    );
    v.simulate_input("Please review.");
    ready(&mut v, &view);
    v.simulate_keystrokes("enter");
    v.executor().advance_clock(Duration::from_millis(550));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Message accepted by Host: scribe");
}

#[gpui::test]
fn collaboration_stream_keyboard_reaches_a_previously_unrealized_message(cx: &mut TestAppContext) {
    let (mut v, view) = mount(cx, "clean-light");
    button(
        &mut v,
        &view,
        "Atlas: Should we keep the quick-start example?",
    );
    v.simulate_keystrokes("home");
    ready(&mut v, &view);
    assert!(!metrics(&mut v, &view).is_following_tail);
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    let selected = node(
        &mut v,
        &view,
        "button",
        "You: Check the release guide before we share it.",
    );
    assert_eq!(selected.pressed, Some(true));
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
