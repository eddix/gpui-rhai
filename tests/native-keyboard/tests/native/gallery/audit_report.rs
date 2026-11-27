//! Strict findings stay visible, even when their count does not change.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::MotionPreference;
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use crate::support::{
    Host, Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of,
};

const PROFILE: &str = r#"fn profile(){#{name:"report",audit:["low-contrast-text"]}}"#;

#[gpui::test]
fn equal_counts_with_new_color_evidence_refresh_the_gallery_report(cx: &mut TestAppContext) {
    let (mut visual,view)=Mount::new(r#"
        fn state_schema(){#{fields:#{report:#{schema:#{type:"string"},"default":#{type:"string",value:"unreported"}}}}}
        fn init(ctx){ctx.register_action("gallery.audit",Fn("reported"));}
        fn reported(ctx,value){ctx.set_state("report",value.report);}
        fn view(ctx){column([
            text("Palette role").with_style(style().font_size(px(13)).text_color(theme_color("field.placeholder")).background(theme_color("surface.field"))),
            text(ctx.get_state("report")).test_id("report").with_style(style().text_color(rgb(0xffffff)).background(rgb(0)))
        ]).with_style(style().width(px(600)).height(px(400)))}
    "#).theme(gpui_rhai_registry::MODERN_LIGHT_THEME).motion(MotionPreference::None)
        .with(|v|v.profile_source(PROFILE).theme_sources([("modern_dark.rhai".to_owned(),gpui_rhai_registry::MODERN_DARK_THEME.to_owned())])).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let mut cached = None;
    visual
        .update(|window, cx| acceptance::report_audit(&view, &mut cached, window, cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 3);
    let first = text_of(&mut visual, &view, "report");
    assert!(
        first.contains("Modern / Light") && first.contains("#b5b5b5ff"),
        "{first}"
    );
    assert_eq!(cached.as_ref().unwrap().findings.len(), 1);
    let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
    visual
        .update(|window, cx| acceptance::report_audit(&view, &mut cached, window, cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        visual.update(|_, cx| view.committed_revision(cx).unwrap()),
        revision,
        "identical evidence must not republish"
    );
    visual
        .update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 3);
    visual
        .update(|window, cx| acceptance::report_audit(&view, &mut cached, window, cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(cached.as_ref().unwrap().findings.len(), 1);
    let second = text_of(&mut visual, &view, "report");
    assert!(
        second.contains("Modern / Dark") && second.contains("#5c5c63ff"),
        "{second}"
    );
    assert_ne!(first, second);
    assert!(second.contains("No findings are suppressed"));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn failed_audit_publication_does_not_advance_the_retry_cache(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new("fn view(ctx){text(\"No audit action\")}").mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let mut cached = None;
    for _ in 0..2 {
        assert!(
            visual
                .update(|window, cx| acceptance::report_audit(&view, &mut cached, window, cx))
                .is_err()
        );
        assert!(cached.is_none());
    }
}

#[gpui::test]
fn the_real_gallery_opens_a_copyable_report_without_erasing_contrast_risks(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "select".to_owned(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("audit-report", "audit-report"),
        true,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 4);
    let mut cached = None;
    visual
        .update(|window, cx| acceptance::report_audit(&view, &mut cached, window, cx))
        .unwrap();
    settle(&mut visual, Step::Frame, 3);
    let actual = cached.as_ref().unwrap();
    assert!(
        actual.findings.iter().any(|f| f.contrast.is_some()),
        "empty-field risk positive control"
    );
    let link = bounds(&mut visual, &view, "button", "Open audit report");
    click_center(&mut visual, link);
    settle(&mut visual, Step::Frame, 4);
    bounds(&mut visual, &view, "dialog", "Audit findings");
    bounds(&mut visual, &view, "document", "Audit evidence");
    // Publishing diagnostic text must not turn the underlying palette risk
    // into a pass or recursively invent another report on every revision.
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert!(findings.iter().any(|f| f.contrast.is_some()));
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::Frame, 4);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("dialog", "Audit findings")
            .next()
            .is_none()
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
