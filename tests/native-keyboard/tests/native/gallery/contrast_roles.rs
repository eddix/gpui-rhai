use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{AuditFinding, AuditRules, GeometryBounds, ScriptViewHandle, ThemeVariant};

use super::conversation::{click, gallery, node, ready};
use crate::support::{PresetRisks, bounds, glyphs_with_ink, live_fills, snapshot};

fn real(cx: &TestAppContext, name: &'static str) -> TestAppContext {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some(name),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    )
}

fn report(v: &mut VisualTestContext, view: &ScriptViewHandle) -> (ThemeVariant, Vec<AuditFinding>) {
    let policy = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    v.update(|_, cx| {
        (
            view.theme_snapshot(cx).unwrap().variant,
            view.composition_audit_with(&policy, cx).unwrap(),
        )
    })
}

fn record(risks: &mut PresetRisks, theme: &ThemeVariant, findings: &[AuditFinding], suffix: &str) {
    let finding = findings
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .unwrap_or_else(|| {
            panic!(
                "{} / {}: expected warning for {suffix}: {findings:#?}",
                theme.family, theme.name
            )
        });
    assert!(risks.record(theme, finding), "{finding:#?}");
    let mut other = finding.clone();
    other.path.push_str("/unreviewed");
    assert!(!risks.record(theme, &other));
    let mut other = finding.clone();
    other.contrast.as_mut().unwrap().background_srgb = Some([0.8; 3]);
    assert!(!risks.record(theme, &other));
}

#[gpui::test]
fn cost_waterfall_task_and_run_status_keep_distinct_painted_roles(cx: &mut TestAppContext) {
    let mut app = real(cx, "cost_header_roles");
    let (mut v, view) = gallery(&mut app, "cost_waterfall", "modern-light");
    v.simulate_resize(size(px(1320.), px(1000.)));
    ready(&mut v, &view);
    let id = node(&mut v, &view, "status", "Live").id;
    let mut risks = PresetRisks::new();
    for (family, appearance, task_ink, live_ink, live_bg, height) in [
        ("Modern", "Light", 0x8a8a8aff, 0x2456d6ff, 0xffffffff, 28.),
        ("Modern", "Dark", 0x7c7c82ff, 0x8ab4ffff, 0x232326ff, 28.),
        ("Clean", "Light", 0x00000073, 0x006dcbf2, 0x008ff519, 20.),
        ("Clean", "Dark", 0xffffff73, 0x70b8ffff, 0x0077ff3a, 20.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let task = bounds(&mut v, &view, "text", "Evidence review");
        let status = bounds(&mut v, &view, "status", "Live");
        assert_eq!(node(&mut v, &view, "status", "Live").id, id);
        assert!(
            glyphs_with_ink(&mut v, task, task_ink) >= 9,
            "{family}/{appearance}: task {:?} has incorrect ink",
            task
        );
        assert!(
            glyphs_with_ink(&mut v, status, live_ink) >= 3,
            "{family}/{appearance}: wrong status ink"
        );
        assert!(
            (status.height - height).abs() < 0.75,
            "{family}/{appearance}: {status:?}"
        );
        assert!(
            live_fills(&mut v, live_bg).iter().any(|q| [
                f64::from(q.x) - status.x,
                f64::from(q.y) - status.y,
                f64::from(q.width) - status.width,
                f64::from(q.height) - status.height,
            ]
            .iter()
            .all(|d| d.abs() < 0.75)),
            "{family}/{appearance}: missing status backing"
        );
        let (stock, findings) = report(&mut v, &view);
        if family == "Modern" || appearance == "Light" {
            record(&mut risks, &stock, &findings, "/header/title-group/task");
        }
        for finding in findings
            .iter()
            .filter(|f| f.path.ends_with("/header/run-state/label"))
        {
            assert!(risks.record(&stock, finding), "{finding:?}");
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn breakdown_chart_unit_uses_its_own_muted_ink(cx: &mut TestAppContext) {
    let mut app = real(cx, "breakdown_unit");
    let (mut v, view) = gallery(&mut app, "breakdown_chart", "modern-light");
    v.simulate_resize(size(px(1350.), px(1000.)));
    let mut risks = PresetRisks::new();
    for (family, appearance, ink, line) in [
        ("Modern", "Light", 0x8f8f8fff, 20.),
        ("Modern", "Dark", 0x8f8f8fff, 20.),
        ("Clean", "Light", 0x00000073, 16.),
        ("Clean", "Dark", 0xffffff75, 16.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let total = snapshot(&mut v, &view)
            .nodes()
            .find(|n| n.role == "status" && n.name.starts_with("Visible total"))
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        let unit = bounds(&mut v, &view, "text", "tasks");
        assert!(unit.x >= total.x && unit.x + unit.width <= total.x + total.width + 0.5);
        assert!(
            glyphs_with_ink(&mut v, unit, ink) >= 4,
            "{family}/{appearance}: unit ink"
        );
        assert!(
            (unit.height - line).abs() < 0.75,
            "{family}/{appearance}: {unit:?}"
        );
        if appearance == "Light" {
            let (stock, findings) = report(&mut v, &view);
            record(
                &mut risks,
                &stock,
                &findings,
                "/workload/heading/total/unit",
            );
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn conversation_model_fallback_is_legible_inside_composer(cx: &mut TestAppContext) {
    let mut app = real(cx, "inline_model_fallback");
    let (mut v, view) = gallery(&mut app, "scene.messages", "modern-light");
    v.simulate_resize(size(px(1280.), px(1000.)));
    ready(&mut v, &view);
    let auto = node(&mut v, &view, "button", "Auto");
    let first = auto.geometry.unwrap().visual;
    let monogram = GeometryBounds {
        width: 30.,
        ..first
    };
    assert!(
        glyphs_with_ink(&mut v, monogram, 0x171717ff) >= 1,
        "missing strong fallback glyph: {monogram:?}"
    );
    let (_, findings) = report(&mut v, &view);
    assert!(
        !findings
            .iter()
            .any(|f| f.path.ends_with("/response-model/trigger/model-icon/0")),
        "{findings:?}"
    );
    click(&mut v, &view, "button", "Auto");
    node(&mut v, &view, "option", "Quick");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert_eq!(node(&mut v, &view, "button", "Auto").id, auto.id);
}

#[gpui::test]
fn affine_table_keeps_the_authored_header_risk_when_rotated_or_straight(cx: &mut TestAppContext) {
    let mut app = real(cx, "affine_header_proof");
    let (mut v, view) = gallery(&mut app, "foundations.affine", "modern-light");
    v.simulate_resize(size(px(1260.), px(1020.)));
    ready(&mut v, &view);
    let mut risks = PresetRisks::new();
    for transformed in [true, false] {
        if !transformed {
            click(&mut v, &view, "button", "Straighten specimens");
        }
        let (stock, findings) = report(&mut v, &view);
        for id in ["task", "state"] {
            record(
                &mut risks,
                &stock,
                &findings,
                &format!("/affine-table/affine-table-header/{id}/action/0"),
            );
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn recommendation_confidence_keeps_its_source_ink_and_opaque_paper(cx: &mut TestAppContext) {
    let mut app = real(cx, "recommendation_confidence");
    let (mut v, view) = gallery(&mut app, "smart_recommendation", "modern-light");
    v.simulate_resize(size(px(1250.), px(1050.)));
    ready(&mut v, &view);
    let (stock, findings) = report(&mut v, &view);
    let mut risks = PresetRisks::new();
    record(
        &mut risks,
        &stock,
        &findings,
        "/frame/case/foot/0/confidence/1",
    );
    risks.assert_reported();
}
