use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::acceptance::action;
use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, node_texts, snapshot};

fn content(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut out = Vec::new();
    node_texts(&root, &mut out);
    // Check the editable draft bytes, without the non-editable inline link.
    for text in &mut out {
        if let Some(body) = text.strip_suffix(" \u{200b}Review quote") {
            text.truncate(body.len());
        }
    }
    out
}

fn underline_at_link(v: &mut VisualTestContext, area: gpui_rhai::GeometryBounds) -> f32 {
    use crate::support::{logical_paint_rect, walk_live_paint};
    v.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut found = None;
            walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, pose| {
                    for line in &scene.underlines {
                        let rect = logical_paint_rect(line.bounds, pose, scale);
                        let x = f64::from(rect.x + rect.width / 2.);
                        if x > area.x
                            && x < area.x + area.width
                            && f64::from(rect.y) > area.y
                            && f64::from(rect.y) < area.y + area.height + 2.
                        {
                            found = Some(line.color.a);
                        }
                    }
                },
            );
            found.expect("the live link underline paints inside its projected range")
        })
    })
}

#[gpui::test]
#[cfg(all(target_os = "macos", feature = "vendor-gpui"))]
fn email_draft_hover_uses_native_underline_motion_without_rerunning_rhai(cx: &mut TestAppContext) {
    use crate::support::center;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{Modifiers, point};
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_link_hover"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "email_draft".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        &mut real,
        prepared,
        ("email-hover", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, &real);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1260.), px(960.)));
    clock.advance(Duration::from_millis(600));
    ready(&mut v, &view);
    let link = bounds(&mut v, &view, "link", "Review quote");
    let idle = underline_at_link(&mut v, link);
    assert!((idle - 89. / 255.).abs() < 0.01);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();

    v.simulate_mouse_move(center(link), None, Modifiers::none());
    ready(&mut v, &view);
    assert!((underline_at_link(&mut v, link) - idle).abs() < 0.01);
    clock.advance(Duration::from_millis(80));
    ready(&mut v, &view);
    let middle = underline_at_link(&mut v, link);
    assert!(middle > idle + 0.05 && middle < 0.995, "{idle} -> {middle}");
    let away = point(px(8.), px(14.));
    v.simulate_mouse_move(away, None, Modifiers::none());
    ready(&mut v, &view);
    assert!((underline_at_link(&mut v, link) - middle).abs() < 0.01);
    clock.advance(Duration::from_millis(55));
    ready(&mut v, &view);
    let return_sample = underline_at_link(&mut v, link);
    assert!(return_sample > idle + 0.01 && return_sample < middle - 0.01);
    v.simulate_mouse_move(center(link), None, Modifiers::none());
    ready(&mut v, &view);
    assert!((underline_at_link(&mut v, link) - return_sample).abs() < 0.01);
    clock.advance(Duration::from_millis(170));
    ready(&mut v, &view);
    assert!(underline_at_link(&mut v, link) >= 0.99);
    assert_eq!(bounds(&mut v, &view, "link", "Review quote"), link);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    v.simulate_mouse_move(away, None, Modifiers::none());
    ready(&mut v, &view);
    clock.advance(Duration::from_millis(200));
    ready(&mut v, &view);
    assert!((underline_at_link(&mut v, link) - idle).abs() < 0.01);
    click(&mut v, &view, "link", "Review quote");
    node(
        &mut v,
        &view,
        "status",
        "Requested review-quote in the Host",
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("text_field", "Email text")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn adjacent_equal_underline_styles_keep_independent_hover_owners(cx: &mut TestAppContext) {
    use crate::support::{Mount, center};
    use gpui::Modifiers;
    let (mut v, view) = Mount::new(r#"
        fn state_schema() {
            #{fields:#{selected:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}
        }
        fn select_link(name,ctx,event) { ctx.set_state("selected", name); }
        fn view(ctx){
            let a=span("first").color(rgb(0x222222)).underline(1.0,rgba(0x22222244)).underline_hover(0,"linear");
            let b=span("second").color(rgb(0x222222)).underline(1.0,rgba(0x22222244)).underline_hover(0,"linear");
            column([
                inline_prose(text([a,b]).with_style(style().font_size(px(16)).line_height(px(22))),[
                    #{start:0,end:5,node:box([]).with_key("first").accessibility_role("link").accessibility_label("First")
                        .on_click(Fn("select_link").curry("First"))},
                    #{start:5,end:11,node:box([]).with_key("second").accessibility_role("link").accessibility_label("Second")
                        .on_click(Fn("select_link").curry("Second"))}
                ]),
                text("Selected " + ctx.get_state("selected")).accessibility_role("status")
            ])
        }
    "#).motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    let [first, second] = ["First", "Second"].map(|name| bounds(&mut v, &view, "link", name));
    for (name, target, other) in [("First", first, second), ("Second", second, first)] {
        if gpui_rhai::EXTENDED_RENDERING {
            v.simulate_mouse_move(center(target), None, Modifiers::none());
            ready(&mut v, &view);
            assert!(underline_at_link(&mut v, target) > 0.99);
            assert!((underline_at_link(&mut v, other) - 68. / 255.).abs() < 0.01);
        }
        click(&mut v, &view, "link", name);
        node(&mut v, &view, "status", &format!("Selected {name}"));
    }
}

#[gpui::test]
#[cfg(feature = "vendor-gpui")]
fn wrapped_link_hover_ignores_the_blank_area_of_its_semantic_union(cx: &mut TestAppContext) {
    use crate::support::{Mount, center};
    use gpui::Modifiers;
    let (mut v, view) = Mount::new(r#"
        fn view(ctx){
            let label="One\nThis line is much longer";
            let decorated=span(label).color(rgb(0x222222))
                .underline(1.0,rgba(0x22222244)).underline_hover(0,"linear");
            inline_prose(text([decorated]).with_style(style().font_size(px(16)).line_height(px(24))),[
                #{start:0,end:grapheme_count(label),node:box([]).with_key("link")
                    .accessibility_role("link").accessibility_label("Wrapped")}
            ])
        }
    "#).motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    let link = bounds(&mut v, &view, "link", "Wrapped");
    assert!(link.width > 90. && link.height >= 47., "{link:?}");
    let event = |x, y| {
        center(gpui_rhai::GeometryBounds {
            x,
            y,
            width: 0.,
            height: 0.,
        })
    };
    v.simulate_mouse_move(
        event(link.x + link.width - 3., link.y + 10.),
        None,
        Modifiers::none(),
    );
    ready(&mut v, &view);
    assert!((underline_at_link(&mut v, link) - 68. / 255.).abs() < 0.01);
    for (dx, dy) in [(5., 10.), (65., 35.)] {
        v.simulate_mouse_move(event(link.x + dx, link.y + dy), None, Modifiers::none());
        ready(&mut v, &view);
        let mut front = link;
        front.height = 24.;
        let mut second = front;
        second.y += 24.;
        assert!(underline_at_link(&mut v, front) > 0.99);
        assert!(underline_at_link(&mut v, second) > 0.99);
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_inline_quote_shares_a_line_and_does_not_start_editing(cx: &mut TestAppContext) {
    use crate::support::click_center;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_draft_inline_quote"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "email_draft", "clean-light");
    v.simulate_resize(size(px(1260.), px(960.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Short");
    let paragraph = snapshot(&mut v, &view)
        .nodes()
        .find(|n| {
            n.role == "text"
                && n.name.starts_with("Hi Jonas, your Growth")
                && n.name.ends_with("Review quote")
        })
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let quote = bounds(&mut v, &view, "link", "Review quote");
    assert!(quote.width > 5. && quote.width < paragraph.width / 2.);
    assert!(
        quote.y >= paragraph.y && quote.y + quote.height <= paragraph.y + paragraph.height + 1.,
        "an inline link must be within the shaped paragraph: {paragraph:?} {quote:?}"
    );
    click(&mut v, &view, "link", "Review quote");
    node(
        &mut v,
        &view,
        "status",
        "Requested review-quote in the Host",
    );
    let link_focus = v.update(|window, cx| window.focused(cx)).unwrap();
    v.simulate_keystrokes("tab");
    ready(&mut v, &view);
    assert_ne!(
        v.update(|window, cx| window.focused(cx)),
        Some(link_focus.clone())
    );
    v.simulate_keystrokes("shift-tab");
    ready(&mut v, &view);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(link_focus));
    v.update(|window, cx| {
        view.automate(
            gpui_rhai::AutomationCommand::Dispatch {
                locator: gpui_rhai::AutomationLocator::RoleName {
                    role: "button".into(),
                    name: "Refuse next send".into(),
                },
                event: "click".into(),
                payload: None,
            },
            window,
            cx,
        )
        .unwrap()
    });
    ready(&mut v, &view);
    node(&mut v, &view, "status", "The next send will be declined");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "Requested review-quote in the Host",
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("text_field", "Email text")
            .next()
            .is_none()
    );
    click_center(
        &mut v,
        gpui_rhai::GeometryBounds {
            x: paragraph.x + 3.,
            y: paragraph.y + 2.,
            width: 2.,
            height: 2.,
        },
    );
    ready(&mut v, &view);
    let editor = node(&mut v, &view, "text_field", "Email text");
    assert!(
        matches!(editor.value, Some(UiValue::String(ref text)) if text.ends_with("Quote for changes:"))
    );
}

#[gpui::test]
fn email_draft_keeps_amount_and_reference_in_the_same_reading_voice(cx: &mut TestAppContext) {
    let mixed_type = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
    for (theme, line) in [("modern-dark", 22.), ("clean-light", 24.)] {
        let (mut v, view) = gallery(cx, "email_draft", theme);
        v.simulate_resize(size(px(1260.), px(960.)));
        for (scale, factor) in [("100", 1.), ("125", 1.25), ("150", 1.5)] {
            action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            let id = bounds(&mut v, &view, "text", "SUB-1182");
            let amount = bounds(&mut v, &view, "text", "$8,900");
            assert!(
                (id.height - line * factor).abs() <= 1.
                    && (id.height - amount.height).abs() <= 1.
                    && (id.y - amount.y).abs() <= 1.,
                "{theme}/{scale}: {id:?} {amount:?}"
            );
            let findings = v.update(|_, cx| view.composition_audit_with(&mixed_type, cx).unwrap());
            assert!(
                !findings.iter().any(|f| f.path.contains("/email/")),
                "{theme}/{scale}: {findings:?}"
            );
        }
    }
}

#[gpui::test]
fn email_draft_tone_and_edit_buttons_share_the_reference_height_in_every_preset(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "email_draft", theme);
        v.simulate_resize(size(px(1260.), px(960.)));
        for density in ["comfortable", "compact"] {
            action(
                &mut v,
                &view,
                "gallery.set_density",
                UiValue::String(density.into()),
            );
            for (scale, expected) in [("100", 28.), ("150", 42.)] {
                action(
                    &mut v,
                    &view,
                    "gallery.set_scale",
                    UiValue::String(scale.into()),
                );
                ready(&mut v, &view);
                let tone = bounds(&mut v, &view, "button", "Formal");
                let edit = bounds(&mut v, &view, "button", "Edit text");
                let context = format!("{theme}/{density}/{scale}");
                assert!(
                    (tone.height - expected).abs() <= 0.5,
                    "{context}: tone {tone:?}"
                );
                assert!(
                    (edit.height - expected).abs() <= 0.5,
                    "{context}: edit {edit:?}"
                );
                assert!(
                    (edit.width - expected).abs() <= 0.5,
                    "{context}: edit {edit:?}"
                );
            }
        }
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_tone_ink_follows_native_hover_and_selection(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{Modifiers, point};
    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email-draft-tone-ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, idle, active) in [
        ("modern-light", 0x8f8f8fff, 0x171717ff),
        ("modern-dark", 0x8f8f8fff, 0xedededff),
        ("clean-light", 0x0000009c, 0x000000e0),
        ("clean-dark", 0xffffff9e, 0xffffffe6),
    ] {
        let (mut visual, view) = gallery(&mut native, "email_draft", theme);
        visual.simulate_resize(size(px(1260.), px(960.)));
        ready(&mut visual, &view);
        let friendly = bounds(&mut visual, &view, "button", "Friendly");
        let short = bounds(&mut visual, &view, "button", "Short");
        assert!(
            glyphs_with_ink(&mut visual, friendly, active) >= 5,
            "{theme}: the initial pressed tone must actually paint the active ink"
        );
        assert!(
            glyphs_with_ink(&mut visual, short, idle) >= 4,
            "{theme}: at rest"
        );
        visual.simulate_mouse_move(crate::support::center(short), None, Modifiers::none());
        ready(&mut visual, &view);
        assert!(
            glyphs_with_ink(&mut visual, short, active) >= 4,
            "{theme}: on hover"
        );
        assert_eq!(
            node(&mut visual, &view, "button", "Short").pressed,
            Some(false)
        );
        click(&mut visual, &view, "button", "Short");
        visual.simulate_mouse_move(point(px(4.), px(9.)), None, Modifiers::none());
        ready(&mut visual, &view);
        let short = bounds(&mut visual, &view, "button", "Short");
        let friendly = bounds(&mut visual, &view, "button", "Friendly");
        assert_eq!(
            node(&mut visual, &view, "button", "Short").pressed,
            Some(true)
        );
        assert!(
            glyphs_with_ink(&mut visual, short, active) >= 4,
            "{theme}: stays active without hover"
        );
        assert!(
            glyphs_with_ink(&mut visual, friendly, idle) >= 5,
            "{theme}: former choice returns to idle"
        );
    }
}

#[gpui::test]
fn email_draft_native_editor_and_recipient_route_the_confirmed_receipt(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "email_draft", "modern-light");
    v.simulate_resize(size(px(1260.0), px(960.0)));
    ready(&mut v, &view);
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s.starts_with("Hi Jonas,"))
    );
    click(&mut v, &view, "button", "Recipient");
    node(&mut v, &view, "option", "Lena Hart");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("option", "Lena Hart")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Recipient");
    click(&mut v, &view, "option", "Finance team");
    click(&mut v, &view, "button", "Formal");
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s.starts_with("Dear Finance team,"))
    );
    assert_eq!(node(&mut v, &view, "button", "Formal").pressed, Some(true));
    click(&mut v, &view, "link", "Review quote");
    node(
        &mut v,
        &view,
        "status",
        "Requested review-quote in the Host",
    );
    click(&mut v, &view, "button", "Remove Renewal-quote.pdf");
    node(&mut v, &view, "button", "Attach Renewal-quote.pdf");
    click(&mut v, &view, "button", "Refuse next send");
    click(&mut v, &view, "button", "Edit text");
    let mac = cfg!(target_os = "macos");
    v.simulate_keystrokes(if mac { "cmd-a" } else { "ctrl-a" });
    v.simulate_input("Hi Finance team,\n已核对");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "text_field", "Email text").value,
        Some(UiValue::String("Hi Finance team,\n已核对".into()))
    );
    v.simulate_keystrokes(if mac { "cmd-enter" } else { "ctrl-enter" });
    ready(&mut v, &view);
    assert!(node(&mut v, &view, "button", "Sending").disabled);
    v.executor().advance_clock(Duration::from_millis(900));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "The Host declined the send");
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s == "Hi Finance team,\n已核对")
    );
    click(&mut v, &view, "button", "Recipient");
    click(&mut v, &view, "option", "Lena Hart");
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s == "Hi Lena Hart,\n已核对")
    );
    click(&mut v, &view, "button", "Recipient");
    click(&mut v, &view, "option", "Finance team");
    click(&mut v, &view, "button", "Send email");
    v.executor().advance_clock(Duration::from_millis(900));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Sent to Finance team");
    node(
        &mut v,
        &view,
        "status",
        "Simulated send to finance/采购 without a file",
    );
    click(&mut v, &view, "button", "Rename saved contact");
    node(&mut v, &view, "status", "The Host updated the address book");
    node(&mut v, &view, "status", "Sent to Finance team");
    click(&mut v, &view, "button", "Undo");
    node(&mut v, &view, "status", "The Host restored the draft");
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s == "Hi Finance team,\n已核对")
    );
    click(&mut v, &view, "button", "Send email");
    click(&mut v, &view, "button", "New draft");
    v.executor().advance_clock(Duration::from_millis(950));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "New local draft");
    assert!(
        content(&mut v, &view)
            .iter()
            .any(|s| s.starts_with("Hi Jonas,"))
    );
    node(&mut v, &view, "button", "Remove Renewal-quote.pdf");
}

#[gpui::test]
fn email_draft_receipt_and_restore_retarget_the_same_native_height(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "email_draft".into(),
        theme: "clean-light".into(),
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
        ("draft-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1240.0), px(960.0)));
    ready(&mut v, &view);
    let h = |v: &mut VisualTestContext| bounds(v, &view, "group", "Email draft").height;
    let before = h(&mut v);
    click(&mut v, &view, "button", "Discard");
    node(&mut v, &view, "status", "Draft discarded");
    if !gpui_rhai::EXTENDED_RENDERING {
        assert!(h(&mut v) < before - 10.0);
        click(&mut v, &view, "button", "Undo");
        assert!((h(&mut v) - before).abs() < 1.5);
        return;
    }
    assert!((h(&mut v) - before).abs() < 1.5);
    clock.advance(Duration::from_millis(80));
    ready(&mut v, &view);
    let middle = h(&mut v);
    assert!(middle < before - 10.0);
    click(&mut v, &view, "button", "Undo");
    assert!(
        (h(&mut v) - middle).abs() < 1.5,
        "restore must start at the displayed height"
    );
    clock.advance(Duration::from_millis(450));
    ready(&mut v, &view);
    assert!((h(&mut v) - before).abs() < 1.5);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn email_draft_recipient_shell_follows_painted_hover_and_open_state(cx: &mut TestAppContext) {
    use crate::support::{center, live_fills, logical_paint_rect, walk_live_paint};
    use gpui::{Modifiers, point};
    for (theme, height, idle, hover, open, edge) in [
        (
            "modern-light",
            26.,
            Some(0xf5f5f5ff),
            0xe6e6e6db,
            Some(0xf5f5f5ff),
            0xebebebff,
        ),
        (
            "modern-dark",
            26.,
            Some(0x2a2a2dff),
            0x38383adb,
            Some(0x2a2a2dff),
            0xffffff14,
        ),
        (
            "clean-light",
            28.,
            None,
            0x00000008,
            Some(0x00000008),
            0x0000000f,
        ),
        (
            "clean-dark",
            28.,
            None,
            0xffffff0a,
            Some(0xffffff0a),
            0xffffff14,
        ),
    ] {
        let (mut view_ctx, view) = gallery(cx, "email_draft", theme);
        view_ctx.simulate_resize(size(px(1260.), px(960.)));
        ready(&mut view_ctx, &view);
        let recipient = node(&mut view_ctx, &view, "button", "Recipient");
        let area = recipient.geometry.unwrap().visual;
        assert!((area.height - height).abs() < 1., "{theme}: {area:?}");
        let same_bounds = |rect: crate::support::Rect| {
            [
                (rect.x, area.x),
                (rect.y, area.y),
                (rect.width, area.width),
                (rect.height, area.height),
            ]
            .iter()
            .all(|(a, b)| (f64::from(*a) - b).abs() < 1.)
        };
        let filled =
            |v: &mut VisualTestContext, ink| live_fills(v, ink).into_iter().any(same_bounds);
        let inset = view_ctx.update(|window, _| {
            let scale = window.scale_factor();
            let expected = gpui::Hsla::from(gpui::rgba(edge));
            window.visual_probe_with_painted_scene(|scene| {
                let mut matches = false;
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::default(),
                    &mut |scene, pose| {
                        matches |= scene.shadows.iter().any(|shadow| {
                            shadow.inset == 1
                                && (shadow.color.a - expected.a).abs() < 0.004
                                && (shadow.color.l - expected.l).abs() < 0.004
                                && same_bounds(logical_paint_rect(
                                    shadow.element_bounds,
                                    pose,
                                    scale,
                                ))
                        });
                    },
                );
                matches
            })
        });
        assert!(inset, "{theme}: no matching inset edge at {area:?}");
        assert!(!filled(&mut view_ctx, hover), "{theme}: idle hover fill");
        if let Some(ink) = idle {
            assert!(filled(&mut view_ctx, ink), "{theme}: idle fill");
        }

        view_ctx.simulate_mouse_move(center(area), None, Modifiers::none());
        ready(&mut view_ctx, &view);
        assert!(filled(&mut view_ctx, hover), "{theme}: hover fill");
        click(&mut view_ctx, &view, "button", "Recipient");
        node(&mut view_ctx, &view, "option", "Lena Hart");
        view_ctx.simulate_mouse_move(point(px(4.), px(9.)), None, Modifiers::none());
        ready(&mut view_ctx, &view);
        let expanded = node(&mut view_ctx, &view, "button", "Recipient");
        assert_eq!(expanded.id, recipient.id);
        assert_eq!(expanded.expanded, Some(true));
        assert_eq!(expanded.geometry.unwrap().visual, area);
        if let Some(ink) = open {
            assert!(
                filled(&mut view_ctx, ink),
                "{theme}: open fill away from pointer"
            );
        }
        view_ctx.simulate_keystrokes("escape");
        ready(&mut view_ctx, &view);
        let collapsed = node(&mut view_ctx, &view, "button", "Recipient");
        assert_eq!(collapsed.id, recipient.id);
        assert_eq!(collapsed.expanded, Some(false));
        assert!(
            !filled(&mut view_ctx, hover),
            "{theme}: Escape must clear open/hover fill"
        );
    }
}

#[gpui::test]
fn email_draft_audit_uses_the_painted_paper_behind_its_grid_content(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, live_fills};
    let (mut v, view) = gallery(cx, "email_draft", "modern-light");
    v.simulate_resize(size(px(1260.), px(960.)));
    ready(&mut v, &view);
    let to = bounds(&mut v, &view, "label", "to");
    let tone = bounds(&mut v, &view, "button", "Short");
    assert!(
        live_fills(&mut v, 0xffffffff).iter().any(|paper| {
            paper.width > 340.
                && paper.width < 570.
                && paper.height > 80.
                && f64::from(paper.x) < to.x
                && f64::from(paper.right()) > to.x + to.width
                && f64::from(paper.y) < to.y
                && f64::from(paper.bottom()) > to.y + to.height
                && f64::from(paper.x) < tone.x
                && f64::from(paper.right()) > tone.x + tone.width
                && f64::from(paper.bottom()) > tone.y + tone.height
        }),
        "{to:?}"
    );
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let (theme, findings) = v.update(|_, cx| {
        (
            view.theme_snapshot(cx).unwrap().variant,
            view.composition_audit_with(&rules, cx).unwrap(),
        )
    });
    assert!(
        !findings
            .iter()
            .any(|f| f.path.ends_with("/draft-grid/meta/to/0")),
        "{findings:#?}"
    );
    // The account belongs to the tray; only the recipient is a chip.
    // The attachment's transparent row belongs to the white paper.
    for (part, paper, ratio) in [
        ("/email/record/1", 240., 2.84),
        ("/trigger/1", 245., 2.97),
        ("/draft-grid/meta/file/1", 255., 3.23),
        ("/actions/tones/tone:short/0", 255., 3.23),
        ("/actions/tones/tone:formal/0", 255., 3.23),
    ] {
        let evidence = findings
            .iter()
            .find(|f| {
                f.path.ends_with(part)
                    && (part != "/trigger/1" || f.path.contains("/draft-grid/meta/to/recipient:"))
            })
            .and_then(|f| f.contrast.as_ref())
            .unwrap_or_else(|| panic!("missing contrast evidence for {part}: {findings:#?}"));
        assert!(!evidence.conservative, "{part}: {evidence:?}");
        assert!(
            evidence
                .background_srgb
                .unwrap()
                .into_iter()
                .all(|c| (c - paper / 255.).abs() < 0.001),
            "{part}: {evidence:?}"
        );
        assert!(
            (evidence.ratio - ratio).abs() < 0.015,
            "{part}: {evidence:?}"
        );
    }
    let mut policy = PresetRisks::new();
    let mut exact = 0;
    for finding in findings.iter().filter(|f| f.path.contains("/sized/email/")) {
        assert!(policy.record(&theme, finding), "{finding:#?}");
        exact += 1;
    }
    assert_eq!(exact, 8);
    policy.assert_reported();
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_link_keeps_separate_glyph_and_underline_ink(cx: &mut TestAppContext) {
    use crate::support::{glyphs_with_ink, logical_paint_rect, walk_live_paint};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_draft_link_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, glyph, stroke, width) in [
        ("modern-light", 0x404040ff, 0x40404059, 1.5),
        ("modern-dark", 0xc4c4c4ff, 0xc4c4c459, 1.5),
        ("clean-light", 0x000000ff, 0x00000045, 1.0),
        ("clean-dark", 0xffffffff, 0xffffff4d, 1.0),
    ] {
        let (mut v, view) = gallery(&mut real, "email_draft", theme);
        v.simulate_resize(size(px(1260.), px(960.)));
        ready(&mut v, &view);
        let link = bounds(&mut v, &view, "link", "Review quote");
        assert!(
            glyphs_with_ink(&mut v, link, glyph) >= 8,
            "{theme}: {link:?}"
        );
        let expected = gpui::rgba(stroke);
        let mut matched = false;
        v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::default(),
                    &mut |scene, pose| {
                        for line in &scene.underlines {
                            let ink = line.color.to_rgb();
                            if [
                                ink.r - expected.r,
                                ink.g - expected.g,
                                ink.b - expected.b,
                                ink.a - expected.a,
                            ]
                            .into_iter()
                            .any(|delta| delta.abs() > 1.0 / 255.0)
                            {
                                continue;
                            }
                            let area = logical_paint_rect(line.bounds, pose, scale);
                            matched |= (line.thickness.0 - (width * scale).round()).abs() <= 0.01
                                && f64::from(area.x) >= link.x - 1.
                                && f64::from(area.right()) <= link.x + link.width + 1.
                                && f64::from(area.y) >= link.y
                                && f64::from(area.y) <= link.y + link.height + 1.;
                        }
                    },
                );
            });
        });
        assert!(
            matched,
            "{theme}: no underline in the link's presented range"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_account_and_address_keep_their_distinct_native_ink(cx: &mut TestAppContext) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_draft_account_and_address_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, account_ink, address_ink) in [
        ("modern-light", 0x8f8f8fff, 0x8f8f8fff),
        ("modern-dark", 0x8f8f8fff, 0x8f8f8fff),
        ("clean-light", 0x0000009c, 0x00000073),
        ("clean-dark", 0xffffff9e, 0xffffff75),
    ] {
        let (mut v, view) = gallery(&mut real, "email_draft", theme);
        v.simulate_resize(size(px(1260.), px(960.)));
        ready(&mut v, &view);
        let address = if theme.starts_with("clean") {
            "jonas@example.test"
        } else {
            "(jonas@example.test)"
        };
        for (label, ink, line_height) in [
            ("Northwind Logistics", account_ink, 16.),
            (address, address_ink, 20.),
        ] {
            let area = bounds(&mut v, &view, "text", label);
            assert!(
                (area.height - line_height).abs() <= 1.,
                "{theme}: {label}: {area:?}"
            );
            assert!(
                glyphs_with_ink(&mut v, area, ink) >= 12,
                "{theme}: {label} must paint at {ink:#010x} in {area:?}"
            );
        }
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_attachment_keeps_the_paper_and_routes_both_actions(cx: &mut TestAppContext) {
    use crate::support::{center, glyphs_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui::{Modifiers, point};

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_draft_file_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, ink, old_chip, remove_size, attach_size, hover, active) in [
        (
            "modern-light",
            0x8f8f8fff,
            0xf5f5f5ff,
            20.,
            26.,
            0xf5f5f5ff,
            0x171717ff,
        ),
        (
            "modern-dark",
            0x8f8f8fff,
            0x2a2a2dff,
            20.,
            26.,
            0xffffff0d,
            0xedededff,
        ),
        (
            "clean-light",
            0x0000009c,
            0x0000000f,
            24.,
            28.,
            0x00000008,
            0x000000e0,
        ),
        (
            "clean-dark",
            0xffffff9e,
            0xffffff14,
            24.,
            28.,
            0xffffff0a,
            0xffffffe6,
        ),
    ] {
        let (mut v, view) = gallery(&mut real, "email_draft", theme);
        v.simulate_resize(size(px(1260.), px(960.)));
        ready(&mut v, &view);
        let area = bounds(&mut v, &view, "text", "Renewal-quote.pdf");
        assert!(glyphs_with_ink(&mut v, area, ink) >= 8, "{theme}: {area:?}");
        assert!(
            !live_fills(&mut v, old_chip).iter().any(|chip| {
                f64::from(chip.x) <= area.x
                    && f64::from(chip.right()) >= area.x + area.width
                    && f64::from(chip.y) <= area.y
                    && f64::from(chip.bottom()) >= area.y + area.height
            }),
            "{theme}: the attached file must not paint a chip behind its name"
        );
        let remove = bounds(&mut v, &view, "button", "Remove Renewal-quote.pdf");
        assert!(
            (remove.width - remove_size).abs() <= 0.5 && (remove.height - remove_size).abs() <= 0.5,
            "{theme}: {remove:?}"
        );
        v.simulate_mouse_move(center(remove), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(
            live_fills(&mut v, hover).iter().any(|fill| {
                (f64::from(fill.x) - remove.x).abs() <= 0.5
                    && (f64::from(fill.y) - remove.y).abs() <= 0.5
                    && (f64::from(fill.width) - remove.width).abs() <= 0.5
            }),
            "{theme}: the remove action did not paint its hover wash"
        );
        click(&mut v, &view, "button", "Remove Renewal-quote.pdf");
        v.simulate_mouse_move(point(px(4.), px(9.)), None, Modifiers::none());
        ready(&mut v, &view);
        let attach = bounds(&mut v, &view, "button", "Attach Renewal-quote.pdf");
        assert!(
            (attach.height - attach_size).abs() <= 0.5,
            "{theme}: {attach:?}"
        );
        assert!(
            glyphs_with_ink(&mut v, attach, ink) >= 8,
            "{theme}: reattach rest ink"
        );
        v.simulate_mouse_move(center(attach), None, Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, attach, active) >= 8,
            "{theme}: reattach hover ink"
        );
        click(&mut v, &view, "button", "Attach Renewal-quote.pdf");
        node(&mut v, &view, "button", "Remove Renewal-quote.pdf");
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn email_draft_switches_action_bands_without_restarting_the_editor_or_request(
    cx: &mut TestAppContext,
) {
    use crate::support::glyphs_with_ink;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("email_draft_action_bands"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "email_draft", "modern-dark");
    v.simulate_resize(size(px(1260.), px(960.)));
    ready(&mut v, &view);
    let y = |v: &mut VisualTestContext, role, title| {
        let area = bounds(v, &view, role, title);
        area.y + area.height / 2.
    };
    let send = node(&mut v, &view, "button", "Send email").id;
    let tone = node(&mut v, &view, "button", "Formal").id;
    let hint = bounds(&mut v, &view, "text", "Cmd/Ctrl+Enter to send");
    assert!(glyphs_with_ink(&mut v, hint, 0x8f8f8fff) > 10);
    assert!((y(&mut v, "button", "Send email") - y(&mut v, "button", "Formal")).abs() <= 1.);
    assert!(y(&mut v, "text", "Cmd/Ctrl+Enter to send") > y(&mut v, "button", "Send email"));

    click(&mut v, &view, "button", "Edit text");
    let editor = node(&mut v, &view, "text_field", "Email text").id;
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    let (select, submit) = if cfg!(target_os = "macos") {
        ("cmd-a", "cmd-enter")
    } else {
        ("ctrl-a", "ctrl-enter")
    };
    v.simulate_keystrokes(select);
    v.simulate_input("Hi Jonas,\n已确认");
    ready(&mut v, &view);
    v.update(|_, cx| assert!(view.select_theme("Clean", "Light", cx).unwrap()));
    ready(&mut v, &view);
    assert_eq!(node(&mut v, &view, "button", "Send email").id, send);
    assert_eq!(node(&mut v, &view, "button", "Formal").id, tone);
    let body = node(&mut v, &view, "text_field", "Email text");
    assert_eq!(body.id, editor);
    assert_eq!(
        body.value,
        Some(UiValue::String("Hi Jonas,\n已确认".into()))
    );
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    let formal_y = y(&mut v, "button", "Formal");
    let hint_y = y(&mut v, "text", "Cmd/Ctrl+Enter to send");
    let send_y = y(&mut v, "button", "Send email");
    let review_y = y(&mut v, "text", "Ready for review");
    let hint = bounds(&mut v, &view, "text", "Cmd/Ctrl+Enter to send");
    assert!(glyphs_with_ink(&mut v, hint, 0x00000045) > 10);
    assert!(
        (formal_y - hint_y).abs() <= 1.,
        "tone {formal_y}, hint {hint_y}"
    );
    assert!(
        (send_y - review_y).abs() <= 1.,
        "send {send_y}, review {review_y}"
    );

    v.simulate_keystrokes(submit);
    ready(&mut v, &view);
    let sending = node(&mut v, &view, "button", "Sending");
    assert_eq!(sending.id, send);
    assert!(sending.disabled);
    v.update(|_, cx| assert!(view.select_theme("Modern", "Light", cx).unwrap()));
    ready(&mut v, &view);
    assert_eq!(node(&mut v, &view, "button", "Sending").id, send);
    v.executor().advance_clock(Duration::from_millis(900));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "Sent to Jonas Weber · Renewal-quote.pdf",
    );
}

#[gpui::test]
fn email_draft_recipient_fits_the_clipped_header_at_large_reading_sizes(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "email_draft", "modern-dark");
    for scale in ["175", "200"] {
        for (family, appearance) in [("Modern", "Dark"), ("Clean", "Light")] {
            v.update(|_, cx| view.select_theme(family, appearance, cx))
                .unwrap();
            super::acceptance::action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            let id = node(&mut v, &view, "button", "Recipient").id;
            for narrow in [false, true] {
                if narrow {
                    click(&mut v, &view, "button", "Narrow draft");
                }
                let recipient = node(&mut v, &view, "button", "Recipient");
                assert_eq!(recipient.id, id);
                let bounds = recipient.geometry.unwrap();
                let card = crate::support::bounds(&mut v, &view, "group", "Email draft");
                assert!(
                    bounds.clip.is_none_or(|clip| {
                        clip.x <= bounds.visual.x + 1.
                            && clip.x + clip.width + 1. >= bounds.visual.x + bounds.visual.width
                    }),
                    "{scale}/{family}/{appearance}/{narrow}: {bounds:?}, card {card:?}"
                );
                let bad = v.update(|_, cx| view.debug_unreachable_controls(cx).unwrap());
                assert!(
                    !bad.iter().any(|f| [
                        "Recipient",
                        "Remove Renewal-quote.pdf",
                        "Formal",
                        "Discard",
                        "Edit text",
                        "Send email"
                    ]
                    .iter()
                    .any(|control| f.contains(control))),
                    "{scale}/{family}/{appearance}/{narrow}: {bad:?}, recipient {bounds:?}, card {card:?}"
                );
                assert!(recipient.description.contains("@example.test"));
                click(&mut v, &view, "button", "Recipient");
                node(&mut v, &view, "option", "Lena Hart");
                v.simulate_keystrokes("escape");
                ready(&mut v, &view);
                assert_eq!(node(&mut v, &view, "button", "Recipient").id, id);
            }
            click(&mut v, &view, "button", "Wide draft");
        }
    }
}

#[gpui::test]
fn email_draft_preset_sizes_and_absent_recipients_keep_actions_bounded(cx: &mut TestAppContext) {
    for (theme, width) in [
        ("modern-dark", 520.0),
        ("clean-light", 480.0),
        ("clean-dark", 480.0),
    ] {
        let (mut v, view) = gallery(cx, "email_draft", theme);
        v.simulate_resize(size(px(1260.0), px(940.0)));
        ready(&mut v, &view);
        let wide = bounds(&mut v, &view, "group", "Email draft");
        assert!((wide.width - width).abs() < 1.0, "{theme}: {wide:?}");
        click(&mut v, &view, "button", "Narrow draft");
        let narrow = bounds(&mut v, &view, "group", "Email draft");
        let send = bounds(&mut v, &view, "button", "Send email");
        assert!(
            narrow.width <= 320.5 && send.x + send.width <= narrow.x + narrow.width + 0.5,
            "{theme}: {narrow:?} {send:?}"
        );
        click(&mut v, &view, "button", "Remove recipients");
        assert!(node(&mut v, &view, "button", "Send email").disabled);
        assert!(content(&mut v, &view).iter().any(|s| s == "No recipient"));
        click(&mut v, &view, "button", "Discard");
        node(&mut v, &view, "status", "Draft discarded");
        assert!(bounds(&mut v, &view, "group", "Email draft").height < narrow.height);
        click(&mut v, &view, "button", "Undo");
        click(&mut v, &view, "button", "Restore recipients");
        assert!(!node(&mut v, &view, "button", "Send email").disabled);
    }
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
