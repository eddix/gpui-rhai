use gpui::{TestAppContext, px, size};

use super::conversation::{click, gallery, node, ready, resize_to_group_width};
use crate::support::{bounds, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn change_summary_uses_preset_wrapping_and_the_actual_content_breakpoint(cx: &mut TestAppContext) {
    use crate::support::{Mount, glyphs_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::MotionPreference;
    use gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES;

    const TITLE: &str = "ReleaseReadinessBeforeRollingProduction";
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("summary_heading"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = Mount::new(
        r#"import "patterns/change_summary" as cards;
fn view(ctx) { column([cards::ChangeSummary(#{
    key:"card", title:"ReleaseReadinessBeforeRollingProduction", period:"This week",
    changes:[#{id:"latency",label:"Latency",change:"Down 14 ms"}],
})]).with_style(style().width(relative(1.0))).env(#{scale:"200"}) }"#,
    )
    .registry("patterns/change_summary")
    .motion(MotionPreference::None)
    .with(|b| {
        b.theme_sources(
            BUNDLED_DESIGN_PRESET_SOURCES
                .iter()
                .map(|&(name, source)| (name.to_owned(), source.to_owned())),
        )
    })
    .mount(&mut real);
    // At 200% the Modern tray has two 12px insets. Clean has no tray.
    for (family, appearance, last_stacked) in [
        ("Modern", "Light", 424.),
        ("Modern", "Dark", 424.),
        ("Clean", "Light", 400.),
        ("Clean", "Dark", 400.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        resize_to_group_width(&mut v, &view, TITLE, last_stacked, 720.);
        let heading = node(&mut v, &view, "heading", TITLE);
        let title = heading.geometry.unwrap().visual;
        let period = bounds(&mut v, &view, "text", "This week");
        assert!(
            period.x >= title.x + title.width,
            "{family}/{appearance}: {title:?} vs {period:?}"
        );
        if family == "Modern" {
            assert!(title.height >= 79., "{family}/{appearance}: {title:?}");
            assert!(
                glyphs_with_ink(&mut v, title, 0x4fb0feff) >= TITLE.len() - 1,
                "{family}/{appearance}: the long word was not fully drawn"
            );
        } else {
            assert!(
                (title.height - 40.).abs() < 1.,
                "{family}/{appearance}: {title:?}"
            );
        }
        let label = bounds(&mut v, &view, "text", "Latency");
        let detail = bounds(&mut v, &view, "text", "Down 14 ms");
        assert!(
            (label.height - 40.).abs() < 1.,
            "{family}/{appearance}: {label:?}"
        );
        assert!(
            detail.y >= label.y + label.height - 0.5,
            "{family}/{appearance}: should stack at {last_stacked}, {label:?} vs {detail:?}"
        );

        resize_to_group_width(&mut v, &view, TITLE, last_stacked + 1., 720.);
        assert_eq!(node(&mut v, &view, "heading", TITLE).id, heading.id);
        let label = bounds(&mut v, &view, "text", "Latency");
        let detail = bounds(&mut v, &view, "text", "Down 14 ms");
        assert!(
            detail.x >= label.x + label.width - 0.5,
            "{family}/{appearance}: should use two columns at {last_stacked}+1, {label:?} vs {detail:?}"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn summary_and_tips_keep_original_contrast_warnings_on_their_painted_headers(
    cx: &mut TestAppContext,
) {
    use crate::support::PresetRisks;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::AuditRules;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("summary_tips_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (page, roles) in [
        (
            "change_summary",
            &[
                "/summary/content/head/title-group/title",
                "/summary/content/head/period",
            ][..],
        ),
        ("tips_panel", &["/tips/head/count"][..]),
    ] {
        let (mut v, view) = gallery(&mut real, page, "modern-light");
        v.simulate_resize(size(px(1260.), px(1000.)));
        for family in ["Modern", "Clean"] {
            v.update(|_, cx| view.select_theme(family, "Light", cx))
                .unwrap();
            ready(&mut v, &view);
            let (stock, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for role in roles {
                let finding = findings.iter().find(|f| f.path.ends_with(role));
                if family == "Clean" && role.ends_with("title") {
                    assert!(finding.is_none(), "{page}, {family}: {finding:?}");
                    continue;
                }
                let finding =
                    finding.unwrap_or_else(|| panic!("{page}, {family}: {role} not reported"));
                assert!(risks.record(&stock, finding), "{finding:?}");
                let mut other = finding.clone();
                other.path.push_str("/unexpected-role");
                assert!(!risks.record(&stock, &other));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn change_summary_preserves_metric_keys_and_reflows_without_hiding_accepted_details(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "change_summary", "modern-light");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    let tree = snapshot(&mut v, &view);
    let initial = tree
        .find_by_role_and_name("listitem", "Request latency")
        .find(|n| n.description.contains("Favourable decrease"))
        .unwrap();
    let id = initial.id;
    assert!(initial.description.contains("Down 14 ms to 62 ms"));
    assert_eq!(
        tree.find_by_role_and_name("listitem", "Request latency")
            .count(),
        2
    );
    let wide_label = bounds(&mut v, &view, "text", "Deploy frequency");
    let wide_detail = bounds(&mut v, &view, "text", "Up to 6 deployments per day");
    assert!(wide_detail.x >= wide_label.x + wide_label.width);
    assert!(
        (wide_detail.y + wide_detail.height / 2. - wide_label.y - wide_label.height / 2.).abs()
            < 4.,
        "{wide_label:?}, {wide_detail:?}"
    );

    click(&mut v, &view, "button", "Narrow card");
    let narrow_label = bounds(&mut v, &view, "text", "Deploy frequency");
    let narrow_detail = bounds(&mut v, &view, "text", "Up to 6 deployments per day");
    assert!(narrow_detail.y >= narrow_label.y + narrow_label.height);
    node(&mut v, &view, "listitem", "跨区域发布成功率");

    click(&mut v, &view, "button", "Change comparison");
    let after = snapshot(&mut v, &view)
        .find_by_role_and_name("listitem", "Request latency")
        .find(|n| n.id == id)
        .unwrap()
        .clone();
    assert!(after.description.contains("Unfavourable increase"));
    assert!(after.description.contains("Up 6 ms to 68 ms"));
    click(&mut v, &view, "button", "Change comparison");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Request latency")
            .next()
            .is_none()
    );
    node(&mut v, &view, "heading", "What's changed");
}
