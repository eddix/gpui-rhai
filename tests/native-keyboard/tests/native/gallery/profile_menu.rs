use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn profile_setup_uses_authored_spacing_and_a_wider_next_hit_area(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("profile_setup_layout"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "profile_menu", "modern-light");
    v.simulate_resize(size(px(1260.), px(990.)));
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = crate::support::PresetRisks::new();
    for (family, appearance, gap, bottom) in [
        ("Modern", "Light", 7., 18.),
        ("Modern", "Dark", 7., 18.),
        ("Clean", "Light", 8., 6.),
        ("Clean", "Dark", 8., 6.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let count = bounds(&mut v, &view, "text", "2/5");
        let track = bounds(&mut v, &view, "progressbar", "Profile setup");
        let next = bounds(&mut v, &view, "button", "Next: Choose a profile picture");
        let card = bounds(&mut v, &view, "group", "Profile menu");
        assert!(
            (track.y - count.y - count.height - gap).abs() < 0.75,
            "{family}/{appearance}: track gap: {count:?} {track:?}"
        );
        assert!(
            (next.y - track.y - track.height - 6.).abs() < 0.75,
            "{family}/{appearance}: next gap: {track:?} {next:?}"
        );
        assert!(
            (track.x - next.x - 8.).abs() < 0.75 && (next.width - track.width - 16.).abs() < 0.75,
            "{family}/{appearance}: hit area: {track:?} {next:?}"
        );
        assert!(
            (card.y + card.height - next.y - next.height - bottom).abs() < 0.75,
            "{family}/{appearance}: bottom inset: {card:?} {next:?}"
        );
        if appearance == "Light" {
            let (stock, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            for role in [
                "/card/actions/item:apps/badge",
                "/setup/heading/count",
                "/setup/next/words/hint",
            ] {
                let finding = findings
                    .iter()
                    .find(|f| f.path.ends_with(role))
                    .unwrap_or_else(|| panic!("{family}: expected strict warning at {role}"));
                assert!(risks.record(&stock, finding), "{family}: {finding:?}");
                let mut wrong = finding.clone();
                wrong.contrast.as_mut().unwrap().background_srgb = Some([0.9; 3]);
                assert!(!risks.record(&stock, &wrong));
            }
        }
    }
    risks.assert_reported();
    let next = bounds(&mut v, &view, "button", "Next: Choose a profile picture");
    // Click inside the added hit area, not on the text nor the former row.
    crate::support::click_center(
        &mut v,
        gpui_rhai::GeometryBounds {
            x: next.x + 1.,
            y: next.y + next.height / 2. - 1.,
            width: 4.,
            height: 2.,
        },
    );
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "progressbar", "Profile setup").description,
        "3/5"
    );
}

#[gpui::test]
fn profile_menu_keys_route_stable_actions_and_reveal_offscreen_focus(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "profile_menu", "modern-light");
    v.simulate_resize(size(px(1240.0), px(955.0)));
    ready(&mut v, &view);
    click(&mut v, &view, "menuitem", "Settings");
    node(&mut v, &view, "status", "Requested settings");
    v.simulate_keystrokes("up enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Requested logout");
    v.simulate_keystrokes("home down enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Requested docs");
    assert_eq!(node(&mut v, &view, "menuitem", "Apps").description, "New");

    click(&mut v, &view, "button", "Long action list");
    let offscreen = bounds(&mut v, &view, "menuitem", "Account details");
    assert!(node(&mut v, &view, "menuitem", "Workspace admin").disabled);
    click(&mut v, &view, "menuitem", "Preferences");
    v.simulate_keystrokes("down enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Requested saved/1");
    v.simulate_keystrokes("end enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Requested detail/账户");
    assert!(bounds(&mut v, &view, "menuitem", "Account details").y < offscreen.y - 8.0);
    v.simulate_keystrokes("home enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Requested settings");
}

#[gpui::test]
fn profile_menu_keeps_setup_host_owned_and_matches_both_card_widths(cx: &mut TestAppContext) {
    for (theme, width, row_height) in [
        ("modern-dark", 320.0, 36.0),
        ("clean-light", 260.0, 32.0),
        ("clean-dark", 260.0, 32.0),
    ] {
        let (mut v, view) = gallery(cx, "profile_menu", theme);
        v.simulate_resize(size(px(1240.0), px(925.0)));
        ready(&mut v, &view);
        let card = bounds(&mut v, &view, "group", "Profile menu");
        let row = bounds(&mut v, &view, "menuitem", "Apps");
        assert!((card.width - width).abs() < 1.0, "{theme}: {card:?}");
        assert!((row.height - row_height).abs() < 1.0, "{theme}: {row:?}");
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").value,
            Some(UiValue::Integer(40))
        );
        click(&mut v, &view, "button", "Refuse next step");
        click(&mut v, &view, "button", "Next: Choose a profile picture");
        node(&mut v, &view, "status", "The Host kept the existing setup");
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").description,
            "2/5"
        );
        click(&mut v, &view, "button", "Next: Choose a profile picture");
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").description,
            "3/5"
        );

        click(&mut v, &view, "button", "Narrow card");
        assert!(bounds(&mut v, &view, "group", "Profile menu").width <= 208.5);
        click(&mut v, &view, "button", "Next: Add a recovery contact");
        click(&mut v, &view, "button", "Next: Set a passphrase");
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").description,
            "5/5"
        );
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("button", "Next: Set a passphrase")
                .next()
                .is_none()
        );
    }
}

#[gpui::test]
fn profile_menu_setup_starts_at_first_exposure_not_while_mounted_offscreen(
    cx: &mut TestAppContext,
) {
    let source = r#"
        import "patterns/profile_menu" as profile;
        fn state_schema(){#{fields:#{shown:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            steps:#{schema:#{type:"integer"},"default":#{type:"integer",value:3}}}}}
        fn show(ctx,event){ctx.set_state("shown",true);handled()}
        fn advance(ctx,event){ctx.set_state("steps",min(5,ctx.get_state("steps")+1));handled()}
        fn view(ctx){
            let shown=ctx.get_state("shown")==true;
            column([text("Expose card").accessibility_role("button")
                .with_style(style().width(px(180)).height(px(28))).on_click(Fn("show")),
              text("Advance setup").accessibility_role("button")
                .with_style(style().width(px(180)).height(px(28))).on_click(Fn("advance")),
              column([box([]).with_key("spacer").with_style(style().height(px(if shown{0}else{500})).flex_shrink(false)),
                profile::ProfileMenu(#{key:"one",name:"Ada",email:"ada@example.test",items:[],
                  setup:#{done:ctx.get_state("steps"),total:5}})]).with_key("clip")
                .with_style(style().min_height(px(0)).height(px(260)).width(px(340)).overflow_y_scroll())])
        }
    "#;
    let clock = ManualRuntimeClock::new(Instant::now());
    let runtime = clock.clock();
    let theme = gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .find(|(name, _)| *name == "modern_light.rhai")
        .unwrap()
        .1;
    let (mut v, view) = Mount::new(source)
        .registry("patterns/profile_menu")
        .theme(theme)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(runtime))
        .mount(cx);
    let painted_fill = |v: &mut VisualTestContext| {
        crate::support::live_fills(v, 0x4fb0_feff)
            .into_iter()
            .find(|rect| rect.height > 0.0 && rect.height <= 5.0)
    };
    let fill = |v: &mut VisualTestContext| painted_fill(v).map_or(0.0, |r| r.width);
    clock.advance(Duration::from_millis(750));
    ready(&mut v, &view);
    assert!(fill(&mut v) < 1.0);
    let clipped = node(&mut v, &view, "group", "Profile menu")
        .geometry
        .unwrap()
        .clip;
    assert!(
        clipped.is_some_and(|rect| rect.width == 0.0 || rect.height == 0.0),
        "{clipped:?}"
    );
    click(&mut v, &view, "button", "Expose card");
    let track = bounds(&mut v, &view, "progressbar", "Profile setup");
    assert!(track.y < 260.0 && track.width > 150.0, "{track:?}");
    if !gpui_rhai::EXTENDED_RENDERING {
        let exposed = f64::from(fill(&mut v));
        assert!(
            (exposed - track.width * 0.6).abs() < 1.5,
            "{exposed}, {track:?}"
        );
        click(&mut v, &view, "button", "Advance setup");
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").value,
            Some(UiValue::Integer(80))
        );
        let accepted = f64::from(fill(&mut v));
        assert!(
            (accepted - track.width * 0.8).abs() < 1.5,
            "{accepted}, {track:?}"
        );
        return;
    }
    assert!(
        fill(&mut v) < 1.0,
        "first exposure must not jump to an offscreen-completed fill"
    );
    clock.advance(Duration::from_millis(85));
    ready(&mut v, &view);
    assert!(
        fill(&mut v) < 1.0,
        "visible before the authored delay ended"
    );
    click(&mut v, &view, "button", "Advance setup");
    assert_eq!(
        node(&mut v, &view, "progressbar", "Profile setup").value,
        Some(UiValue::Integer(80))
    );
    clock.advance(Duration::from_millis(64));
    ready(&mut v, &view);
    assert!(fill(&mut v) < 1.0, "visible at t=149ms");
    clock.advance(Duration::from_millis(65));
    ready(&mut v, &view);
    let painted = painted_fill(&mut v).expect("fill should appear after the delay");
    assert!(
        (f64::from(painted.x) - track.x).abs() < 1.5,
        "the reveal must start at the leading edge: {painted:?}, {track:?}"
    );
    let first = f64::from(painted.width);
    assert!(
        first > 8.0 && first < track.width * 0.4,
        "accepted progress must not restart the reveal at t=85ms: {first}"
    );
    clock.advance(Duration::from_millis(300));
    ready(&mut v, &view);
    let middle = f64::from(fill(&mut v));
    clock.advance(Duration::from_millis(537));
    ready(&mut v, &view);
    let end = f64::from(fill(&mut v));
    assert!(
        middle > first + 8.0 && middle < end - 5.0 && (end - track.width * 0.8).abs() < 1.5,
        "{first} → {middle} → {end}; track: {track:?}"
    );
    click(&mut v, &view, "button", "Advance setup");
    assert_eq!(
        node(&mut v, &view, "progressbar", "Profile setup").value,
        Some(UiValue::Integer(100))
    );
    assert!(
        (f64::from(fill(&mut v)) - track.width).abs() < 1.5,
        "accepted updates must not replay or wait for the entrance"
    );
}

#[gpui::test]
fn profile_menu_reduced_and_no_motion_present_accepted_progress_without_delay(
    cx: &mut TestAppContext,
) {
    let theme = gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .find(|(name, _)| *name == "modern_light.rhai")
        .unwrap()
        .1;
    for preference in [MotionPreference::Reduced, MotionPreference::None] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let runtime = clock.clock();
        let (mut v, view) = Mount::new(
            r#"import "patterns/profile_menu" as profile;
fn view(ctx) { profile::ProfileMenu(#{key:"one",name:"Ada",email:"ada@example.test",
   items:[],setup:#{done:3,total:5}}) }"#,
        )
        .registry("patterns/profile_menu")
        .theme(theme)
        .motion(preference)
        .with(move |view| view.runtime_clock(runtime))
        .mount(cx);
        ready(&mut v, &view);
        let track = bounds(&mut v, &view, "progressbar", "Profile setup");
        let rect = crate::support::live_fills(&mut v, 0x4fb0_feff)
            .into_iter()
            .find(|r| (f64::from(r.y) - track.y).abs() < 1.)
            .unwrap_or_else(|| panic!("{preference:?}: the progress fill stayed hidden"));
        assert!((f64::from(rect.width) - track.width * 0.6).abs() < 1.5);
        assert_eq!(
            node(&mut v, &view, "progressbar", "Profile setup").value,
            Some(UiValue::Integer(60))
        );
    }
}
