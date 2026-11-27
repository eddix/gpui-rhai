//! Derived solid status paint must not borrow an unrelated action inverse.

use crate::support::{Mount, Step, last_error, settle};
use gpui::TestAppContext;
use gpui_rhai::{AuditRules, MotionPreference};

#[gpui::test]
fn strong_badges_are_readable_on_unrelated_backings_in_every_theme(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_PALETTE_SOURCES)
    {
        let (mut visual,view)=Mount::components(r#"
            import "components/badge" as badge;
            fn view(ctx){let badges=[];for tone in ["neutral","info","success","warning","danger","purple"] {
                badges.push(badge::Badge(#{text:tone,variant:tone,emphasis:"strong"}));
            }column(badges).with_style(style().items_start().gap(px(8)).background(rgb(0xff00ff)))}
        "#).theme(theme).motion(MotionPreference::None).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let findings = visual.update(|_, cx| {
            view.composition_audit_with(&AuditRules::from_ids(["low-contrast-text"]).unwrap(), cx)
                .unwrap()
        });
        assert!(findings.is_empty(), "{name}: {findings:?}");
        assert_eq!(last_error(&mut visual, &view), None, "{name}");
    }
}

#[gpui::test]
fn a_host_badge_override_still_reports_its_unreadable_paint(cx: &mut TestAppContext) {
    let (mut visual,view)=Mount::components(r#"
        import "components/badge" as badge;
        fn view(ctx){badge::Badge(#{text:"Unreadable",variant:"warning",emphasis:"strong",part_styles:#{
            root:style().text_color(rgb(0x345678)).background(rgb(0x345678))}})}
    "#).theme(gpui_rhai_registry::MODERN_LIGHT_THEME).motion(MotionPreference::None).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let findings = visual.update(|_, cx| {
        view.composition_audit_with(&AuditRules::from_ids(["low-contrast-text"]).unwrap(), cx)
            .unwrap()
    });
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].contrast.as_ref().unwrap().ratio, 1.0);
}
