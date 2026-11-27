use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, center, snapshot};

const PRICE: &str = "Changed price Pro plan";
const PAGE: &str = "Updated pricing page";
const CDN: &str = "Refreshed previews Product cards";
const MAIL: &str = "Sent launch email Subscribers";
const QUOTE: &str = "Updated quote template";

fn described(v: &mut VisualTestContext, view: &ScriptViewHandle, who: &str, phrase: &str) -> bool {
    node(v, view, "listitem", who).description.contains(phrase)
}
fn wait(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(650));
    ready(v, view);
}
fn absent(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> bool {
    snapshot(v, view)
        .find_by_role_and_name("button", name)
        .next()
        .is_none()
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[cfg(target_os = "macos")]
fn grayscale_samples(v: &mut VisualTestContext) -> Vec<(f32, f32)> {
    fn collect(scene: &gpui::Scene, samples: &mut Vec<(f32, f32)>) {
        for group in &scene.isolated_groups {
            if group.grayscale() > 0.0 {
                samples.push((group.grayscale(), group.opacity()));
            }
            collect(group.source(), samples);
        }
        #[cfg(feature = "gpui-fast")]
        for layer in &scene.layers.frames {
            if let Some(child) = layer.content.scene() {
                collect(child, samples);
            }
        }
    }
    v.update(|window, _| {
        window.visual_probe_with_painted_scene(|scene| {
            let mut samples = Vec::new();
            collect(scene, &mut samples);
            samples.sort_by(|a, b| a.0.total_cmp(&b.0));
            samples
        })
    })
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[cfg(target_os = "macos")]
fn grayscale_marks(v: &mut VisualTestContext) -> usize {
    let samples = grayscale_samples(v);
    for (gray, opacity) in &samples {
        assert_eq!(gray.to_bits(), 1.0f32.to_bits());
        assert_eq!(opacity.to_bits(), 0.5f32.to_bits());
    }
    samples.len()
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[cfg(target_os = "macos")]
#[gpui::test]
fn gallery_agent_ledger_staggers_accepted_marks_and_fades_on_restore(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "agent_ledger".into(),
        theme: "modern-dark".into(),
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
        ("ledger-gray-clock", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1240.), px(1040.)));
    ready(&mut v, &view);
    assert_eq!(grayscale_marks(&mut v), 1);
    let bounds_before = bounds(&mut v, &view, "listitem", PAGE);
    click(&mut v, &view, "button", &format!("Undo: {PRICE}"));
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert_eq!(grayscale_marks(&mut v), 1);
    clock.advance(Duration::from_millis(650));
    wait(&mut v, &view);
    node(&mut v, &view, "status", "Undone: 3 changes");
    assert_eq!(grayscale_marks(&mut v), 1, "no elapsed animation time");
    for (elapsed, expected) in [(60, 2), (100, 3), (100, 4)] {
        clock.advance(Duration::from_millis(elapsed));
        ready(&mut v, &view);
        let samples = grayscale_samples(&mut v);
        assert_eq!(samples.len(), expected, "{samples:?}");
        for (gray, opacity) in &samples {
            assert!((opacity - (1.0 - 0.5 * gray)).abs() < 0.0001);
        }
    }
    let mid = grayscale_samples(&mut v);
    assert!(mid[0].0 > 0.0 && mid[0].0 < 0.5, "{mid:?}");
    click(&mut v, &view, "button", &format!("Redo: {PRICE}"));
    // The demo Host itself acknowledges after 460ms on the same clock.
    // Independent Rust and Metal fixtures reverse before a track settles.
    clock.advance(Duration::from_millis(650));
    wait(&mut v, &view);
    node(&mut v, &view, "status", "Restored: 3 changes");
    assert_eq!(
        grayscale_marks(&mut v),
        4,
        "restoration starts at the settled undo"
    );
    clock.advance(Duration::from_millis(120));
    ready(&mut v, &view);
    let restoring = grayscale_samples(&mut v);
    assert_eq!(restoring.len(), mid.len());
    assert!(restoring[0].0 < 0.9);
    clock.advance(Duration::from_millis(230));
    ready(&mut v, &view);
    assert_eq!(grayscale_marks(&mut v), 1);
    assert_eq!(bounds(&mut v, &view, "listitem", PAGE), bounds_before);
}

#[gpui::test]
fn gallery_agent_ledger_previews_transitive_and_irreversible_dependents_without_moving_rows(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_ledger", "modern-light");
    v.simulate_resize(size(px(1240.), px(1030.)));
    ready(&mut v, &view);
    assert!(absent(&mut v, &view, &format!("Undo: {MAIL}")));
    assert!(described(&mut v, &view, QUOTE, "Undone"));
    node(&mut v, &view, "status", "6 changes · 1 undone");
    let before = bounds(&mut v, &view, "listitem", PAGE);
    let price = bounds(&mut v, &view, "button", &format!("Undo: {PRICE}"));
    v.simulate_mouse_move(center(price), None, Modifiers::none());
    ready(&mut v, &view);
    assert!(described(&mut v, &view, PAGE, "Will also be undone"));
    assert!(described(&mut v, &view, CDN, "Will also be undone"));
    assert!(described(
        &mut v,
        &view,
        MAIL,
        "Irreversible; will stay applied"
    ));
    assert!(!described(&mut v, &view, QUOTE, "Will also be undone"));
    assert!(!described(&mut v, &view, PRICE, "Undone"));
    assert_eq!(bounds(&mut v, &view, "listitem", PAGE), before);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn gallery_agent_ledger_pending_and_refused_batches_preserve_state_and_redo_is_exact(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_ledger", "clean-light");
    v.simulate_resize(size(px(1240.), px(1040.)));
    ready(&mut v, &view);
    #[cfg(target_os = "macos")]
    assert_eq!(grayscale_marks(&mut v), 1, "initially undone quote");
    click(&mut v, &view, "button", "Refuse next ledger action");
    click(&mut v, &view, "button", &format!("Undo: {PRICE}"));
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert!(!described(&mut v, &view, PRICE, "Undone"));
    wait(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host refused the ledger action.",
    );
    assert!(absent(&mut v, &view, &format!("Redo: {PRICE}")));
    #[cfg(target_os = "macos")]
    assert_eq!(
        grayscale_marks(&mut v),
        1,
        "refused proposal changes no paint"
    );
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    let focus = v
        .update(|window, cx| window.focused(cx))
        .expect("undo retained focus");
    wait(&mut v, &view);
    node(&mut v, &view, "status", "Undone: 3 changes");
    node(&mut v, &view, "status", "6 changes · 4 undone");
    #[cfg(target_os = "macos")]
    assert_eq!(grayscale_marks(&mut v), 4, "accepted batch");
    for id in [PRICE, PAGE, CDN] {
        assert!(described(&mut v, &view, id, "Undone"));
    }
    assert!(!described(&mut v, &view, MAIL, "Undone"));
    assert!(absent(&mut v, &view, &format!("Redo: {QUOTE}")));
    assert!(described(
        &mut v,
        &view,
        QUOTE,
        "Restore prerequisites first"
    ));
    assert_eq!(
        v.update(|window, cx| window.focused(cx)),
        Some(focus.clone())
    );
    v.update(|_, cx| view.select_theme("Modern", "Dark", cx))
        .unwrap();
    ready(&mut v, &view);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    wait(&mut v, &view);
    node(&mut v, &view, "status", "Restored: 3 changes");
    node(&mut v, &view, "status", "6 changes · 1 undone");
    #[cfg(target_os = "macos")]
    assert_eq!(grayscale_marks(&mut v), 1, "accepted restore");
    assert!(!described(&mut v, &view, PRICE, "Undone"));
    assert!(described(&mut v, &view, QUOTE, "Undone"));
    node(&mut v, &view, "button", &format!("Redo: {QUOTE}"));
}

#[gpui::test]
fn gallery_agent_ledger_rejects_an_undo_ticket_after_the_snapshot_changes(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_ledger", "clean-dark");
    v.simulate_resize(size(px(1240.), px(1040.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", &format!("Undo: {PRICE}"));
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    click(&mut v, &view, "button", "Record follow-up");
    wait(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The ledger changed before this request settled.",
    );
    node(&mut v, &view, "status", "7 changes · 1 undone");
    assert!(!described(&mut v, &view, PRICE, "Undone"));
    assert!(!described(&mut v, &view, CDN, "Undone"));
    assert!(absent(&mut v, &view, &format!("Redo: {PRICE}")));
}

#[gpui::test]
fn gallery_agent_ledger_narrow_rtl_keeps_its_text_column_during_preview(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_ledger", "clean-dark");
    v.simulate_resize(size(px(750.), px(1040.)));
    v.update(|_, cx| view.select_locale("ar", cx)).unwrap();
    ready(&mut v, &view);
    let before = bounds(&mut v, &view, "text", "Sent launch email");
    let undo = bounds(&mut v, &view, "button", &format!("تراجع: {PRICE}"));
    assert!(undo.x < before.x, "{before:?} {undo:?}");
    click(&mut v, &view, "button", "Preview price cascade");
    assert_eq!(bounds(&mut v, &view, "text", "Sent launch email"), before);
    assert!(described(&mut v, &view, MAIL, "سيبقى مطبّقًا"));
}

#[gpui::test]
fn gallery_agent_ledger_new_entry_grows_in_without_an_initial_layout_jump(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "agent_ledger".into(),
        theme: "modern-dark".into(),
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
        ("ledger-growth", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1160.), px(1030.)));
    ready(&mut v, &view);
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let row = |v: &mut VisualTestContext| bounds(v, &view, "listitem", "Recorded release notes").y;
    let before = row(&mut v);
    click(&mut v, &view, "button", "Record follow-up");
    node(&mut v, &view, "status", "The Host recorded another change.");
    let start = row(&mut v);
    clock.advance(Duration::from_millis(110));
    ready(&mut v, &view);
    let middle = row(&mut v);
    clock.advance(Duration::from_millis(650));
    ready(&mut v, &view);
    let end = row(&mut v);
    assert!(
        (before - start).abs() < 1.0,
        "initial jump: {before} → {start}"
    );
    assert!(
        middle > start + 2.0 && middle < end - 2.0 && end - before > 45.0,
        "{before} {start} {middle} {end}"
    );
}

#[gpui::test]
fn agent_ledger_checks_all_sixty_four_graph_bits_and_fails_cycles_before_presentation(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = Mount::components(r#"
        import "patterns/agent_ledger" as l;
        fn sink(ctx,event){}
        fn view(ctx){
            let did_block=false;
            let bad=#{run:"bad",entries:[
                #{id:"a",action:"A",system:"Test",at_ms:0,depends_on:["b"]},
                #{id:"b",action:"B",system:"Test",at_ms:1,depends_on:["a"]}
            ]};
            try{l::graph(bad);}catch{did_block=true;}
            let entries=[];
            for i in 0..64{
                let deps=[];
                for j in max(0,i-4)..i{deps.push(j.to_string());}
                entries.push(#{id:i.to_string(),action:"Change "+i.to_string(),system:"Test",at_ms:i*1000,depends_on:deps});
            }
            column([text(if did_block{"Cycle rejected"}else{"Cycle escaped"}),
                l::AgentLedger(#{key:"ledger",run:"dense/r",revision:0,now_ms:75000,entries:entries,point_at:"0",on_action:Fn("sink")})])
        }
    "#).registry("patterns/agent_ledger").theme(gpui_rhai_registry::CLEAN_DARK_THEME).motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    node(&mut v, &view, "text", "Cycle rejected");
    assert!(described(&mut v, &view, "Change 63", "Will also be undone"));
    let root = node(&mut v, &view, "button", "Undo: Change 0");
    assert!(root.description.contains("64 changes"));
    let parent = bounds(&mut v, &view, "listitem", "Change 63").y;
    assert!(parent < bounds(&mut v, &view, "listitem", "Change 62").y);
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
