use std::time::{Duration, Instant};

use gpui::{ClipboardItem, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    AutomationCommand, ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue,
};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, by_role, click_center, dispatch, snapshot};

fn choose(v: &mut VisualTestContext, view: &ScriptViewHandle, trigger: &str, option: &str) {
    click(v, view, "combobox", trigger);
    click(v, view, "option", option);
}

fn edit(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, value: &str) {
    click(v, view, "spinbutton", name);
    v.simulate_keystrokes("cmd-a");
    v.simulate_input(value);
    ready(v, view);
    v.simulate_keystrokes("enter");
    ready(v, view);
}

fn replace_phase(v: &mut VisualTestContext, view: &ScriptViewHandle, text: &str) {
    v.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(text.into())));
    v.simulate_keystrokes("cmd-a cmd-v");
    ready(v, view);
}

#[gpui::test]
fn billing_schedule_date_summary_and_labels_use_secondary_body_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("billing_date_label"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, ink) in [
        ("modern-light", 0x404040ff),
        ("modern-dark", 0xc4c4c4ff),
        ("clean-light", 0x0000009c),
        ("clean-dark", 0xffffff9e),
    ] {
        let (mut v, view) = gallery(&mut real, "billing_schedule", theme);
        v.simulate_resize(size(px(1400.), px(1040.)));
        ready(&mut v, &view);
        for (label, glyphs) in [("Start date", 8), ("Duration", 8), ("Milestone", 8)] {
            let rect = bounds(&mut v, &view, "text", label);
            assert!(
                crate::support::glyphs_with_ink(&mut v, rect, ink) >= glyphs,
                "{theme}/{label}: date label paints secondary text, not the muted caption"
            );
        }
    }
}

#[gpui::test]
fn billing_schedule_phase_fits_text_and_preserves_the_native_editor_when_wrapping(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "billing_schedule", theme);
        v.simulate_resize(size(px(1280.), px(1100.)));
        ready(&mut v, &view);
        let date = "Jan 1, 2026";
        click(&mut v, &view, "button", "Host-controlled quote");
        click(&mut v, &view, "text_field", "Phase name");
        let initial = node(&mut v, &view, "text_field", "Phase name").id;
        let focus = v.update(|window, cx| window.focused(cx)).unwrap();
        v.simulate_keystrokes("cmd-a");
        v.simulate_input("Q");
        ready(&mut v, &view);
        let short = bounds(&mut v, &view, "text_field", "Phase name");
        let date_short = bounds(&mut v, &view, "text", date);
        assert!(
            short.width < 80. && (short.y - date_short.y).abs() < 12.,
            "{theme}: short phase shares the date line: {short:?} {date_short:?}"
        );
        let text = "Integration and validation";
        replace_phase(&mut v, &view, text);
        let longer = bounds(&mut v, &view, "text_field", "Phase name");
        let date_longer = bounds(&mut v, &view, "text", date);
        assert!(
            longer.width > short.width + 70. && date_longer.x > date_short.x + 70.,
            "{theme}: growing text moves the date rail: {short:?} {longer:?} {date_short:?} {date_longer:?}"
        );
        dispatch(
            &mut v,
            &view,
            by_role("button", "Refuse next edit"),
            "click",
        )
        .unwrap();
        // Paste delivers one proposal; simulating separate physical keys
        // would ask the Host to accept the rest after refusing the first.
        replace_phase(&mut v, &view, "Refused");
        let field = node(&mut v, &view, "text_field", "Phase name");
        assert_eq!(field.value, Some(UiValue::String(text.into())));
        assert_eq!(field.geometry.unwrap().visual, longer);
        assert_eq!(field.id, initial);
        assert_eq!(
            v.update(|window, cx| window.focused(cx)),
            Some(focus.clone())
        );

        let text = "Build-out and live validation · 阶段名称保持可编辑";
        replace_phase(&mut v, &view, text);
        dispatch(&mut v, &view, by_role("button", "Narrow schedule"), "click").unwrap();
        ready(&mut v, &view);
        let field = node(&mut v, &view, "text_field", "Phase name");
        let narrow = field.geometry.unwrap().visual;
        let date_narrow = bounds(&mut v, &view, "text", date);
        assert!(
            narrow.width <= 348. && date_narrow.y > narrow.y + narrow.height - 1.,
            "{theme}: narrow phase and date must not overlap: {narrow:?} {date_narrow:?}"
        );
        assert_eq!(field.id, initial);
        assert_eq!(
            v.update(|window, cx| window.focused(cx)),
            Some(focus.clone())
        );
        assert_eq!(field.value, Some(UiValue::String(text.into())));
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
        assert_eq!(node(&mut v, &view, "text_field", "Phase name").id, initial);
        assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
        v.simulate_keystrokes("end");
        v.simulate_input("!");
        ready(&mut v, &view);
        assert_eq!(
            node(&mut v, &view, "text_field", "Phase name").value,
            Some(UiValue::String(format!("{text}!")))
        );
    }
}

#[gpui::test]
fn billing_schedule_reflows_at_the_reading_scale_without_clipping_or_remounting(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "billing_schedule", theme);
        v.simulate_resize(size(px(1280.), px(1160.)));
        ready(&mut v, &view);
        let line = "Set-up fee";
        let field = "Set-up fee price";
        let timing = "Set-up fee billing timing";
        let remove = "Remove Set-up fee";
        let position = bounds(&mut v, &view, "spinbutton", field);
        let initial = node(&mut v, &view, "spinbutton", field).id;
        let timing_id = node(&mut v, &view, "combobox", timing).id;
        let remove_id = node(&mut v, &view, "button", remove).id;
        click_center(&mut v, position);
        ready(&mut v, &view);
        let focus = v
            .update(|window, cx| window.focused(cx))
            .expect("editor has focus");
        v.simulate_keystrokes("cmd-a");
        v.simulate_input("1234.56");
        ready(&mut v, &view);
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
        let parent = bounds(&mut v, &view, "row", line);
        let table = bounds(&mut v, &view, "table", "Billing lines");
        for (role, label, expected_id) in [
            ("spinbutton", field, initial),
            ("combobox", timing, timing_id),
            ("button", remove, remove_id),
        ] {
            let control = node(&mut v, &view, role, label);
            let child = control.geometry.unwrap().visual;
            assert_eq!(control.id, expected_id, "{theme}: {label}");
            assert!(
                child.width > 12. && child.height > 12.,
                "{theme}: {label} {child:?}"
            );
            assert!(
                child.x >= parent.x - 0.5
                    && child.x + child.width <= parent.x + parent.width + 0.5
                    && child.x >= table.x - 0.5
                    && child.x + child.width <= table.x + table.width + 0.5,
                "{theme}: {label} {child:?} must fit row {parent:?} and table {table:?}"
            );
        }
        let price = bounds(&mut v, &view, "spinbutton", field);
        let cadence = bounds(&mut v, &view, "combobox", "Set-up fee repeats");
        assert!(
            price.y >= cadence.y + cadence.height - 0.5,
            "{theme}: price moves below cadence"
        );
        assert_eq!(
            v.update(|window, cx| window.focused(cx)),
            Some(focus),
            "{theme}"
        );
        assert!(
            v.update(|_, cx| view.debug_unreachable_controls(cx).unwrap())
                .is_empty(),
            "{theme}"
        );
        v.simulate_keystrokes("enter");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "First invoice $1,234.56");
        choose(&mut v, &view, timing, "Billed in arrears");
        node(&mut v, &view, "status", "First invoice $0.00");
        click(&mut v, &view, "button", remove);
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("row", line)
                .next()
                .is_none(),
            "{theme}"
        );
    }
}

#[gpui::test]
fn billing_schedule_quotes_products_discount_minimum_and_arrears(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "billing_schedule", "modern-light");
    v.simulate_resize(size(px(1280.), px(1000.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "First invoice $4,999.00");
    choose(&mut v, &view, "Duration", "3 months");
    choose(
        &mut v,
        &view,
        "Add product",
        "Platform licence · $1,200.00 · Monthly",
    );
    node(&mut v, &view, "status", "First invoice $6,199.00");
    node(&mut v, &view, "status", "Phase total (3 months) $8,599.00");
    click(&mut v, &view, "button", "Add discount");
    node(&mut v, &view, "status", "First invoice $5,579.10");
    node(&mut v, &view, "status", "Phase total (3 months) $7,739.10");
    click(&mut v, &view, "button", "Add minimum");
    node(
        &mut v,
        &view,
        "status",
        "Phase total (3 months) $10,000.00 · minimum applies",
    );
    assert!(node(&mut v, &view, "button", "Add discount").disabled);
    choose(
        &mut v,
        &view,
        "Platform licence billing timing",
        "Billed in arrears",
    );
    node(&mut v, &view, "status", "First invoice $4,499.10");
    click(&mut v, &view, "button", "Remove Minimum commitment");
    node(&mut v, &view, "status", "Phase total (3 months) $7,739.10");
    choose(&mut v, &view, "Platform licence repeats", "Quarterly");
    node(&mut v, &view, "status", "Phase total (3 months) $5,579.10");
    click(&mut v, &view, "button", "Add minimum");
    click(&mut v, &view, "button", "Narrow schedule");
    let table = bounds(&mut v, &view, "table", "Billing lines");
    let first = bounds(&mut v, &view, "row", "Set-up fee");
    let second = bounds(&mut v, &view, "row", "Platform licence");
    let invoice = bounds(&mut v, &view, "status", "First invoice $4,499.10");
    let total = bounds(
        &mut v,
        &view,
        "status",
        "Phase total (3 months) $10,000.00 · minimum applies",
    );
    assert!(
        second.y >= first.y + first.height - 0.5,
        "{first:?} {second:?}"
    );
    assert!(
        total.y > invoice.y && total.x + total.width <= table.x + table.width + 18.0,
        "narrow total must fit its container: {total:?} {table:?}"
    );
}

#[gpui::test]
fn billing_schedule_number_editor_survives_rejected_proposal_and_native_resize(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "billing_schedule", theme);
        v.simulate_resize(size(px(1280.), px(960.)));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Host-controlled quote");
        click(&mut v, &view, "button", "Refuse next edit");
        edit(&mut v, &view, "Set-up fee price", "1234.56");
        node(&mut v, &view, "status", "The Host kept the previous quote");
        node(&mut v, &view, "status", "First invoice $4,999.00");
        let key = node(&mut v, &view, "spinbutton", "Set-up fee price").id;
        let wide = bounds(&mut v, &view, "table", "Billing lines");
        let focus = v.update(|window, cx| window.focused(cx)).unwrap();
        v.simulate_keystrokes("cmd-a");
        v.simulate_input("1234.56");
        ready(&mut v, &view);
        v.simulate_resize(size(px(720.), px(960.)));
        ready(&mut v, &view);
        assert!(bounds(&mut v, &view, "table", "Billing lines").width < wide.width);
        assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
        assert_eq!(
            node(&mut v, &view, "spinbutton", "Set-up fee price").id,
            key
        );
        node(&mut v, &view, "status", "First invoice $4,999.00");
        v.simulate_keystrokes("enter");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "First invoice $1,234.56");
        edit(&mut v, &view, "Set-up fee price", "1.005");
        node(&mut v, &view, "status", "First invoice $1,234.56");
    }
}

#[gpui::test]
fn billing_schedule_uses_the_sdk_calendar_and_preserves_a_real_month_end(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "billing_schedule", "clean-light");
    v.simulate_resize(size(px(1280.), px(960.)));
    ready(&mut v, &view);
    click(&mut v, &view, "combobox", "Start date");
    let tree = snapshot(&mut v, &view);
    let day = tree
        .nodes()
        .find(|n| n.role == "gridcell" && n.value == Some(UiValue::String("2026-01-31".into())))
        .expect("the opened calendar includes January 31");
    click_center(&mut v, day.geometry.unwrap().visual);
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "combobox", "Start date").value,
        Some(UiValue::String("2026-01-31".into()))
    );
    choose(&mut v, &view, "Duration", "1 month");
    let mut texts = Vec::new();
    let root = v.update(|_, cx| view.root(cx).unwrap()).unwrap();
    crate::support::node_texts(&root, &mut texts);
    assert!(texts.iter().any(|s| s == "Feb 27, 2026"), "{texts:?}");
    click(&mut v, &view, "button", "Remove Set-up fee");
    node(&mut v, &view, "status", "First invoice $0.00");
}

#[gpui::test]
fn billing_schedule_64_lines_keep_unique_editors_and_bounded_work(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(r#"
        import "patterns/billing_schedule" as billing;
        fn view(ctx){
            let lines=[];
            for i in 0..64{
                lines.push(#{id:"item/"+i.to_string(),name:"Item "+i.to_string(),kind:"product",
                    price_minor:12500,repeat:"monthly",timing:if i%2==0{"advance"}else{"arrears"}});
            }
            billing::BillingSchedule(#{key:"all",schedule:"maximum",draft:#{phase:"Course",start:"2026-01-01",duration:"12m",lines:lines}})
        }
    "#)
    .registry("patterns/billing_schedule")
    .bundled_assets()
    .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
    .motion(MotionPreference::None)
    .mount(cx);
    ready(&mut v, &view);
    let tree = snapshot(&mut v, &view);
    assert_eq!(tree.nodes().filter(|n| n.role == "spinbutton").count(), 64);
    node(&mut v, &view, "status", "First invoice $4,000.00");
    node(
        &mut v,
        &view,
        "status",
        "Phase total (12 months) $96,000.00",
    );
    assert!(node(&mut v, &view, "button", "Add minimum").disabled);
}

#[gpui::test]
fn billing_schedule_native_height_motion_never_keeps_removed_editors_interactive(
    cx: &mut TestAppContext,
) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "billing_schedule".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) = super::mount_prepared(
        cx,
        prepared,
        ("billing-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1250.), px(945.)));
    clock.advance(Duration::from_millis(450));
    ready(&mut v, &view);
    let first_invoice_y =
        |v: &mut VisualTestContext| bounds(v, &view, "status", "First invoice $4,999.00").y;
    let start = first_invoice_y(&mut v);
    click(&mut v, &view, "button", "Add minimum");
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(82));
    ready(&mut v, &view);
    let intermediate = first_invoice_y(&mut v);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    clock.advance(Duration::from_millis(350));
    ready(&mut v, &view);
    let end = first_invoice_y(&mut v);
    assert!(
        start + 1.0 < intermediate && intermediate + 1.0 < end,
        "{start} → {intermediate} → {end}"
    );
    click(&mut v, &view, "button", "Remove Minimum commitment");
    assert!(
        !snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.role == "spinbutton" && n.name == "Minimum commitment")
    );
    node(&mut v, &view, "status", "Phase total $4,999.00");
    clock.advance(Duration::from_millis(400));
    ready(&mut v, &view);
    assert!((first_invoice_y(&mut v) - start).abs() <= 0.5);
}
