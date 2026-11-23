use std::time::Duration;

use gpui::{TestAppContext, px, size};
use gpui_rhai::{MotionPreference, UiValue};
use gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES;

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, glyphs_with_ink, live_fills, snapshot};

#[gpui::test]
fn experiment_chips_reproduce_each_presets_independent_ink_and_translucent_wash(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("experiment_chip_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = Mount::new(
        r#"import "patterns/experiment_card" as exp;
fn view(ctx) {
    let nodes=[];
    for tone in ["red","amber","green","blue","violet","gray"]{
        nodes.push(exp::ExperimentCard(#{key:tone,experiment:tone,title:"Experiment "+tone,revision:0,trigger:"click",
            metrics:[#{id:"m",label:"Value"}],
            variants:[#{id:"base",short:"A",name:"Control",control:true,values:#{m:40}},
                      #{id:"trial",short:"B",name:"Trial",values:#{m:60}}],
            note:"Day seven",status:#{label:tone,tone:tone}}));
    }
    column(nodes).with_style(style().width(px(352)).gap(px(8)))
}"#,
    )
    .registry("patterns/experiment_card")
    .bundled_assets()
    .motion(MotionPreference::None)
    .with(|builder| builder.theme_sources(BUNDLED_DESIGN_PRESET_SOURCES.iter().map(|&(name, source)|
        (name.to_owned(), source.to_owned())
    )))
    .mount(&mut real);
    v.simulate_resize(size(px(720.), px(900.)));
    ready(&mut v, &view);
    let chip_id = node(&mut v, &view, "text", "green").id;
    let lead_id = node(&mut v, &view, "text", "B +50.0%").id;
    for (family, mode, card, pairs) in [
        (
            "Modern",
            "Light",
            0xffffffff,
            [
                (0xc4320aff, 0xf044381a),
                (0xb25e00ff, 0xe8b00024),
                (0x1e8a4fff, 0x5fc88a1c),
                (0x0190feff, 0x0190fe17),
                (0x8a35d6ff, 0xa651f01a),
                (0x6b6b72ff, 0x6e6e7817),
            ],
        ),
        (
            "Modern",
            "Dark",
            0x19191bff,
            [
                (0xff8a7aff, 0xf044381a),
                (0xe6c24dff, 0xe8b00024),
                (0x7dcf9fff, 0x5fc88a1c),
                (0x9cd2feff, 0x0190fe17),
                (0xd6baf6ff, 0xa651f01a),
                (0xb4b4bbff, 0x6e6e7817),
            ],
        ),
        (
            "Clean",
            "Light",
            0xffffffff,
            [
                (0xf04438ff, 0xf0443814),
                (0x8f6c00ff, 0xe8b0001a),
                (0x1e8a4fff, 0x5fc88a14),
                (0x0060e2e6, 0x0060e20f),
                (0xa651f0ff, 0xa651f014),
                (0x0000009c, 0x0000000d),
            ],
        ),
        (
            "Clean",
            "Dark",
            0x1c1c1eff,
            [
                (0xff8a7aff, 0xff6b5f24),
                (0xe6c24dff, 0xe6c24d24),
                (0x5fc88aff, 0x7dcf9f24),
                (0x90cdfeff, 0x4d8dff24),
                (0xb98af0ff, 0xb98af02e),
                (0xffffff9e, 0xffffff14),
            ],
        ),
    ] {
        v.update(|_, cx| view.select_theme(family, mode, cx))
            .unwrap();
        ready(&mut v, &view);
        assert_eq!(node(&mut v, &view, "text", "green").id, chip_id);
        assert_eq!(node(&mut v, &view, "text", "B +50.0%").id, lead_id);
        let lead = bounds(&mut v, &view, "text", "B +50.0%");
        assert!(
            (lead.height - 24.).abs() < 0.75,
            "{family}/{mode}: {lead:?}"
        );
        assert!(glyphs_with_ink(&mut v, lead, pairs[2].0) >= 3);
        let frame = bounds(&mut v, &view, "group", "Experiment red");
        assert!(
            live_fills(&mut v, card).iter().any(|quad| {
                let wanted = frame.width - if family == "Modern" { 8. } else { 0. };
                (f64::from(quad.width) - wanted).abs() <= 0.75
            }),
            "{family}/{mode}: missing experiment card paper"
        );
        for (tone, (ink, wash)) in ["red", "amber", "green", "blue", "violet", "gray"]
            .into_iter()
            .zip(pairs)
        {
            let status = bounds(&mut v, &view, "text", tone);
            assert!(
                (status.height - 20.).abs() < 0.75,
                "{family}/{mode}/{tone}: {status:?}"
            );
            assert!(
                glyphs_with_ink(&mut v, status, ink) >= 3,
                "{family}/{mode}/{tone}: ink"
            );
            assert!(
                live_fills(&mut v, wash).iter().any(|paint| [
                    f64::from(paint.x) - status.x,
                    f64::from(paint.y) - status.y,
                    f64::from(paint.width) - status.width,
                    f64::from(paint.height) - status.height,
                ]
                .iter()
                .all(|x| x.abs() <= 0.75)),
                "{family}/{mode}/{tone}: wash"
            );
        }
    }
}

#[gpui::test]
fn experiment_chips_preserve_their_visible_light_preset_audit_warnings(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("experiment_chip_audit"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "experiment_card", "modern-light");
    v.simulate_resize(size(px(1240.), px(960.)));
    let mut risks = crate::support::PresetRisks::new();
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
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
        for role in [
            "titles/1/chip/words",
            "titles/1/1",
            "leader/variant",
            "leader/words",
        ] {
            let tail = format!("/onboarding-card/paper/card/head/{role}");
            let finding = findings
                .iter()
                .find(|finding| finding.path.ends_with(&tail))
                .unwrap_or_else(|| panic!("{family}: missing raw audit warning at {role}"));
            assert!(risks.record(&stock, finding), "{family}: {finding:?}");
            let mut other = finding.clone();
            other.path.push_str("/unrelated");
            assert!(!risks.record(&stock, &other));
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn experiment_card_preserves_metric_ranking_and_confirms_only_the_accepted_candidate(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "experiment_card", "clean-light");
    v.simulate_resize(size(px(1240.), px(960.)));
    ready(&mut v, &view);
    let head = bounds(&mut v, &view, "button", "Onboarding split");
    node(&mut v, &view, "text", "B +20.0%");
    click(&mut v, &view, "button", "Onboarding split");
    assert_eq!(bounds(&mut v, &view, "button", "Onboarding split"), head);
    let id = node(&mut v, &view, "radio", "Quick start").id;
    click(&mut v, &view, "button", "Latency");
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Accepted metric: latency");
    node(&mut v, &view, "text", "C −25.0%");
    let quick = node(&mut v, &view, "radio", "Quick start");
    assert_eq!(quick.id, id);
    assert_eq!(quick.description, "75 ms · −25.0%");

    click(&mut v, &view, "button", "Refuse next request");
    click(&mut v, &view, "radio", "Quick start");
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host kept the current snapshot.",
    );
    assert_eq!(
        node(&mut v, &view, "radio", "Quick start").checked,
        Some(UiValue::Bool(false))
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Choose Quick start")
            .next()
            .is_none()
    );
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Nominated: c");
    assert_eq!(
        node(&mut v, &view, "radio", "Quick start").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut v, &view, "button", "Choose Quick start");
    node(&mut v, &view, "status", "Confirmed c for latency");
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Dark"),
        ("Clean", "Light"),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let selected = node(&mut v, &view, "radio", "Quick start");
        assert_eq!(selected.id, id);
        assert_eq!(selected.checked, Some(UiValue::Bool(true)));
        node(&mut v, &view, "status", "Confirmed c for latency");
    }

    click(&mut v, &view, "button", "Holdout");
    v.executor().advance_clock(Duration::from_millis(350));
    ready(&mut v, &view);
    node(&mut v, &view, "text", "No lift yet");
    assert_eq!(
        node(&mut v, &view, "radio", "Quick start").description,
        "No value · —"
    );
    click(&mut v, &view, "button", "Narrow card");
    assert_eq!(node(&mut v, &view, "radio", "Quick start").id, id);
}
