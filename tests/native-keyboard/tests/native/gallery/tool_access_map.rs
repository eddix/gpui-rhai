use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn tool_access_map_preserves_distinct_tab_geometry_and_secondary_ink(cx: &mut TestAppContext) {
    use crate::support::{center, glyphs_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("tool_map_body_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, body, active, height) in [
        ("modern-light", 0x6b6b6bff, 0x171717ff, 30.),
        ("modern-dark", 0xa1a1a6ff, 0xedededff, 30.),
        ("clean-light", 0x0000009c, 0x000000e0, 28.),
        ("clean-dark", 0xffffff9e, 0xffffffe6, 28.),
    ] {
        let (mut v, view) = gallery(&mut real, "tool_access_map", theme);
        v.simulate_resize(size(px(1370.), px(1020.)));
        ready(&mut v, &view);
        let tab = bounds(&mut v, &view, "tab", "Reviewer");
        let agent = bounds(&mut v, &view, "group", "Planner");
        let tool = bounds(&mut v, &view, "button", "Read documents");
        assert!((tab.height - height).abs() < 1., "{theme}: {tab:?}");
        assert!(glyphs_with_ink(&mut v, tab, body) >= 6, "{theme}: tab");
        assert!(glyphs_with_ink(&mut v, agent, body) >= 12, "{theme}: task");
        assert!(
            glyphs_with_ink(&mut v, tool, body) >= 1,
            "{theme}: tool glyph"
        );
        let mixed_type = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
        let findings = v.update(|_, cx| view.composition_audit_with(&mixed_type, cx).unwrap());
        assert!(
            !findings.iter().any(|f| f.path.contains("/agent-card/")),
            "{theme}: {findings:?}"
        );

        v.simulate_mouse_move(center(tab), None, gpui::Modifiers::none());
        ready(&mut v, &view);
        assert!(
            glyphs_with_ink(&mut v, tab, active) >= 6,
            "{theme}: tab hover"
        );
        assert_eq!(node(&mut v, &view, "tab", "Reviewer").selected, Some(false));
        assert!(node(&mut v, &view, "button", "Publish draft").disabled);
    }
}

#[gpui::test]
fn tool_access_map_reflows_and_keeps_blocked_tools_noninteractive(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light"] {
        let (mut v, view) = gallery(cx, "tool_access_map", theme);
        v.simulate_resize(size(px(1370.), px(1020.)));
        ready(&mut v, &view);
        let agent = bounds(&mut v, &view, "group", "Planner");
        let search = bounds(&mut v, &view, "button", "Search library");
        let blocked = bounds(&mut v, &view, "button", "Publish draft");
        assert!(search.x + search.width < agent.x);
        assert!(agent.x + agent.width < blocked.x);
        assert!(node(&mut v, &view, "button", "Publish draft").disabled);
        assert_eq!(
            node(&mut v, &view, "button", "Publish draft").description,
            "No access"
        );

        click(&mut v, &view, "button", "Narrow map");
        let narrow_agent = bounds(&mut v, &view, "group", "Planner");
        let narrow_search = bounds(&mut v, &view, "button", "Search library");
        assert!(narrow_agent.y + narrow_agent.height <= narrow_search.y);
        assert!(narrow_agent.width < 390.);
        click(&mut v, &view, "tab", "Planner");
        v.simulate_keystrokes("right");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Showing reviewer");
        assert!(node(&mut v, &view, "button", "Search library").disabled);
        assert!(!node(&mut v, &view, "button", "Build outline").disabled);
    }
}

#[gpui::test]
fn tool_access_map_inspects_call_history_and_keeps_replay_with_the_host(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "tool_access_map", "modern-light");
    v.simulate_resize(size(px(1370.), px(1020.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Search library");
    node(&mut v, &view, "region", "Calls to Search library");
    click(&mut v, &view, "option", "Find archived diagrams");
    v.simulate_keystrokes("down");
    ready(&mut v, &view);
    assert!(node(&mut v, &view, "option", "Find setup notes").selected == Some(true));
    click(&mut v, &view, "button", "Refuse next replay");
    click(&mut v, &view, "button", "Replay");
    node(&mut v, &view, "status", "Replay refused by Host");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("option", "Replay receipt")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Edit input");
    node(&mut v, &view, "text_field", "Input to replay with");
    let select_all = if cfg!(target_os = "macos") {
        "cmd-a"
    } else {
        "ctrl-a"
    };
    v.simulate_keystrokes(select_all);
    v.simulate_input("{broken");
    ready(&mut v, &view);
    assert!(node(&mut v, &view, "button", "Run").disabled);
    v.simulate_keystrokes(select_all);
    v.simulate_input(r#"{"query":"new headings","limit":2}"#);
    ready(&mut v, &view);
    assert!(!node(&mut v, &view, "button", "Run").disabled);
    click(&mut v, &view, "button", "Run");
    node(&mut v, &view, "status", "Edited replay request accepted");
    node(&mut v, &view, "option", "Replay receipt");
    click(&mut v, &view, "button", "Back to the map");
    node(&mut v, &view, "tabpanel", "Tool access map");
}

#[gpui::test]
fn tool_access_map_running_and_stale_draft_cannot_replay(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "tool_access_map", "clean-dark");
    v.simulate_resize(size(px(1370.), px(1020.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Read documents");
    assert!(node(&mut v, &view, "button", "Replay").disabled);
    click(&mut v, &view, "button", "Finish reader");
    assert!(!node(&mut v, &view, "button", "Replay").disabled);
    click(&mut v, &view, "button", "Edit input");
    node(&mut v, &view, "text_field", "Input to replay with");
    click(&mut v, &view, "button", "Finish reader");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("text_field", "Input to replay with")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Replay");
    node(&mut v, &view, "status", "Replay request accepted");
    click(&mut v, &view, "tab", "Reviewer");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("region", "Calls to Read documents")
            .next()
            .is_none()
    );
    node(&mut v, &view, "tabpanel", "Tool access map");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn tool_access_map_running_wire_moves_natively_without_rerunning_rhai(cx: &mut TestAppContext) {
    fn pulse_x(scene: &gpui::Scene, scale: f32) -> Option<f32> {
        for path in &scene.paths {
            let Some(color) = path.color.as_solid() else {
                continue;
            };
            let c = color.to_rgb();
            let w = path.bounds.size.width.0 / scale;
            if (4.6..5.4).contains(&w)
                && (0.26..0.36).contains(&c.r)
                && (0.65..0.75).contains(&c.g)
                && c.b > 0.95
                && c.a > 0.9
            {
                return Some(path.bounds.center().x.0 / scale);
            }
        }
        #[cfg(feature = "gpui-fast")]
        for layer in scene
            .layers
            .frames
            .iter()
            .filter(|layer| layer.tile_size == 0)
        {
            if let Some(x) = layer
                .content
                .scenes()
                .find_map(|scene| pulse_x(scene, scale))
            {
                return Some(x);
            }
        }
        None
    }
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "tool_access_map".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("access-map", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1370.), px(1020.)));
    ready(&mut v, &view);
    let pulse = |v: &mut VisualTestContext| {
        v.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| pulse_x(scene, window.scale_factor()))
        })
        .expect("running wire paints a signal")
    };
    let agent = bounds(&mut v, &view, "group", "Planner");
    let reader = bounds(&mut v, &view, "button", "Read documents");
    clock.advance(Duration::from_millis(180));
    ready(&mut v, &view);
    let first = pulse(&mut v);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(585));
    ready(&mut v, &view);
    let next = pulse(&mut v);
    assert!(
        first <= agent.x as f32 && next < first - 8.0 && next >= (reader.x + reader.width) as f32,
        "wire didn't travel toward the running tool: {first} -> {next}"
    );
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
