//! Native input through the shipped Gallery, not a prepare-only assertion.

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{AccessibilityNode, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use crate::support::{
    Host, Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of,
};

pub(super) fn gallery(
    cx: &mut TestAppContext,
    page: &str,
    theme: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: page.into(),
        theme: theme.into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) =
        super::mount_prepared(cx, prepared, ("gallery", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    ready(&mut visual, &view);
    (visual, view)
}

pub(super) fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 4);
    assert_eq!(last_error(visual, view), None);
}

/// Set the card's measured outer width, independent of the Host test
/// chrome. Checking exact thresholds against the requested window width
/// would accidentally include space that the card can never use.
pub(super) fn resize_to_group_width(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    width: f32,
    height: f32,
) {
    visual.simulate_resize(size(px(width), px(height)));
    ready(visual, view);
    let measured = bounds(visual, view, "group", name).width;
    if (measured - f64::from(width)).abs() > 0.25 {
        #[allow(clippy::cast_possible_truncation)]
        let accommodated = width + (f64::from(width) - measured) as f32;
        visual.simulate_resize(size(px(accommodated), px(height)));
        ready(visual, view);
    }
    let actual = bounds(visual, view, "group", name);
    assert!(
        (actual.width - f64::from(width)).abs() <= 0.25,
        "{name}: {actual:?} vs {width}"
    );
}

pub(super) fn click(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) {
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    ready(visual, view);
}

pub(super) fn node(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> AccessibilityNode {
    let tree = snapshot(visual, view);
    tree.find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("missing {role} {name}"))
        .clone()
}

#[gpui::test]
fn gallery_message_actions_copy_feedback_and_versions(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "message_actions", "modern-light");
    let second = text_of(&mut visual, &view, "message-preview");
    click(&mut visual, &view, "button", "Copy response");
    let clipboard = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(clipboard.as_deref(), Some(second.as_str()));
    click(&mut visual, &view, "button", "Previous version");
    assert_ne!(text_of(&mut visual, &view, "message-preview"), second);
    assert_eq!(
        node(&mut visual, &view, "status", "Version 1 of 2").name,
        "Version 1 of 2"
    );
    click(&mut visual, &view, "button", "Bad response");
    click(&mut visual, &view, "button", "Other");
    click(&mut visual, &view, "text_field", "Describe the issue");
    visual.simulate_input("check the examples");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Send feedback");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Feedback received")
    );
    click(&mut visual, &view, "button", "Read aloud");
    let playback = visual
        .update(|window, cx| window.focused(cx))
        .expect("playback owns focus before the old feedback closes");
    visual
        .executor()
        .advance_clock(std::time::Duration::from_millis(1600));
    ready(&mut visual, &view);
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(playback)
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Describe the issue")
    );
    click(&mut visual, &view, "button", "Regenerate response");
    assert_eq!(
        node(&mut visual, &view, "status", "Version 3 of 3").name,
        "Version 3 of 3"
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Describe the issue")
    );
    assert_eq!(
        node(&mut visual, &view, "button", "Stop reading").pressed,
        Some(true)
    );
}

#[gpui::test]
fn reaction_bar_projects_rejected_and_accepted_proposals_without_duplicate_buttons(
    cx: &mut TestAppContext,
) {
    let source = r#"
        import "components/reaction_bar" as bar;
        fn state_schema(){#{fields:#{
            allow:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            next:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn react(ctx,proposal){ctx.set_state("n",ctx.get_state("n")+1);
            if ctx.get_state("allow"){ctx.set_state("next",proposal);}}
        fn allow(ctx,payload){ctx.set_state("allow",true);}
        fn view(ctx){let result=ctx.get_state("next");
            column([
                bar::ReactionBar(#{key:"bar",actions:["like","like","share"],
                    reactions:if result==(){[#{id:"like",count:1249,active:false}]}else{[result]},on_react:Fn("react")}),
                text("Accept changes").accessibility_role("button").on_click(Fn("allow")),
                text(ctx.get_state("n").to_string()).test_id("invocations")
            ])}
    "#;
    let (mut visual, view) = Mount::components(source)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
        .mount(cx);
    ready(&mut visual, &view);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Like")
            .count(),
        1
    );
    click(&mut visual, &view, "button", "Like");
    let rejected = node(&mut visual, &view, "button", "Like");
    assert_eq!(rejected.pressed, Some(false));
    assert!(rejected.description.contains("1,249"));
    click(&mut visual, &view, "button", "Accept changes");
    click(&mut visual, &view, "button", "Like");
    let accepted = node(&mut visual, &view, "button", "Like");
    assert_eq!(accepted.pressed, Some(true));
    assert!(accepted.description.contains("1,250"));
    assert_eq!(text_of(&mut visual, &view, "invocations"), "2");
}

#[gpui::test]
fn gallery_model_picker_switch_settings_keyboard_select_and_retains_draft(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "model_picker", "clean-light");
    visual.simulate_resize(size(px(650.), px(660.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "text_field", "Prompt");
    visual.simulate_input("release draft");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Auto");
    assert!(node(&mut visual, &view, "option", "Offline").disabled);
    click(&mut visual, &view, "switch", "Extended thinking");
    click(&mut visual, &view, "radio", "High");
    assert_eq!(
        node(&mut visual, &view, "radio", "High").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut visual, &view, "switch", "Web search");
    assert_eq!(
        node(&mut visual, &view, "switch", "Web search").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut visual, &view, "option", "Studio");
    click(&mut visual, &view, "button", "Studio");
    visual.simulate_keystrokes("home enter");
    ready(&mut visual, &view);
    let selected = node(&mut visual, &view, "button", "Auto");
    assert!(!selected.expanded.unwrap_or(false));
    assert_eq!(
        node(&mut visual, &view, "text_field", "Prompt").value,
        Some(UiValue::String("release draft".into()))
    );
}

#[gpui::test]
fn gallery_cited_answer_opens_inline_source_and_copies_literal_quote(cx: &mut TestAppContext) {
    use gpui::{Modifiers, MouseButton, point};
    let (mut visual, view) = gallery(cx, "cited_answer", "modern-light");
    let body = bounds(&mut visual, &view, "document", "Cited answer");
    let position = point(px((body.x + 2.0) as f32), px((body.y + 10.0) as f32));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("tab enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Build log").expanded,
        Some(true)
    );
    // The excerpt prepares independently; clicking a previously measured
    // action while that native content is changing can hit an obsolete row.
    let mut ready_excerpt = false;
    for _ in 0..20 {
        let target = bounds(&mut visual, &view, "document", "Evidence excerpt");
        click_center(&mut visual, target);
        visual.simulate_keystrokes("alt-cmd-c");
        let contents = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        if contents.is_some_and(|text| text.starts_with("The&#32;first&#32;attempt")) {
            ready_excerpt = true;
            break;
        }
        ready(&mut visual, &view);
    }
    assert!(
        ready_excerpt,
        "the actual excerpt document must finish preparing"
    );
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("pending quote copy".into()))
    });
    click(&mut visual, &view, "button", "Copy quote");
    let clipboard = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    let quote = clipboard.unwrap();
    assert!(
        quote.starts_with("“The first attempt failed")
            && quote.ends_with("(https://example.com/build)")
    );
    click(&mut visual, &view, "button", "Open source");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Source requested (not opened): https://example.com/build")
    );
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Build log").expanded,
        Some(false)
    );
    visual.simulate_keystrokes("right enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Runbook").expanded,
        Some(true)
    );
}

#[gpui::test]
fn gallery_ai_answer_projects_host_stop_and_follow_up(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "ai_answer", "clean-light");
    click(&mut visual, &view, "button", "Publish first chunk");
    node(&mut visual, &view, "status", "Writing answer…");
    click(&mut visual, &view, "button", "Stop answer");
    node(&mut visual, &view, "status", "Stopped");
    node(&mut visual, &view, "document", "Answer");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Show the retry protocol")
    );
    click(&mut visual, &view, "button", "Complete response");
    click(&mut visual, &view, "button", "Show the retry protocol");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role != "button" && n.name == "Show the retry protocol")
    );
}

#[gpui::test]
fn cited_answer_hover_and_preview_do_not_reprepare_body(cx: &mut TestAppContext) {
    use gpui_rhai::{DocumentRuntimeConfig, RuntimeEngine, ScriptViewExtension};
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone)]
    struct Count(Rc<RefCell<Option<DocumentRuntimeConfig>>>);
    impl ScriptViewExtension for Count {
        fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
            self.0.replace(Some(engine.document_runtime_config()));
            Ok(())
        }
    }
    let script = r#"
        import "components/cited_answer" as cited;
        fn view(ctx) { cited::CitedAnswer(#{key:"answer",
            segments:[#{text:"The 中文 claim cites the report.",cite:["report/7"]}],
            sources:[#{id:"report/7",title:"Reports",site:"Report",url:"local:7"}]}) }
    "#;
    let stats = Rc::new(RefCell::new(None));
    let probe = Count(stats.clone());
    let (mut visual, view) = Mount::components(script)
        .with(move |input| input.extension(probe))
        .mount(cx);
    ready(&mut visual, &view);
    let count = || {
        stats
            .borrow()
            .as_ref()
            .unwrap()
            .markdown_preparation_stats()
            .jobs_started
    };
    let before = count();
    assert!(
        before > 0,
        "the positive control must really prepare Markdown"
    );
    click(&mut visual, &view, "button", "Report");
    node(&mut visual, &view, "region", "Source preview");
    click(&mut visual, &view, "button", "Close preview");
    assert_eq!(
        count(),
        before,
        "annotation-only updates cannot restart preparation"
    );
}

#[gpui::test]
fn gallery_thinking_state_expands_sources_and_preserves_work_after_collapse(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "thinking_state", "clean-light");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Release runbook")
    );
    click(&mut visual, &view, "button", "+2 more");
    click(&mut visual, &view, "button", "Release runbook");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Source requested (not opened): https://example.com/runbook")
    );
    click(&mut visual, &view, "button", "Thinking");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "button" && n.name == "Release runbook")
    );
    click(&mut visual, &view, "button", "Thinking");
    node(&mut visual, &view, "button", "Release runbook");
    click(&mut visual, &view, "button", "Publish next step");
    node(&mut visual, &view, "listitem", "Summarize the result");
    click(&mut visual, &view, "button", "Complete work");
    node(&mut visual, &view, "listitem", "Finished thinking");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "button" && n.name.starts_with("Worked for "))
    );
}

#[gpui::test]
fn thinking_loader_local_preview_cannot_override_host_phase_or_completion(cx: &mut TestAppContext) {
    use std::time::Duration;
    let source = r#"
        import "components/thinking_loader" as loader;
        fn state_schema(){#{fields:#{
            done:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
        }}}
        fn finish(ctx,payload){ ctx.set_state("done",true); }
        fn view(ctx){column([
            loader::ThinkingLoader(#{key:"local",phases:["First step","Next step"],
                autoplay:true,repeat:true,interval_ms:500,done:ctx.get_state("done"),done_label:"Delivered"}),
            loader::ThinkingLoader(#{key:"controlled",phases:["Pinned","Do not enter"],phase:0,
                autoplay:true,repeat:true,interval_ms:500}),
            text("Finish").accessibility_role("button").on_click(Fn("finish")),
        ])}
    "#;
    let (mut visual, view) = Mount::components(source)
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "First step");
    visual.executor().advance_clock(Duration::from_millis(520));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Next step");
    node(&mut visual, &view, "status", "Pinned");
    click(&mut visual, &view, "button", "Finish");
    visual.executor().advance_clock(Duration::from_secs(4));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Delivered");
    node(&mut visual, &view, "status", "Pinned");
}

#[gpui::test]
fn thinking_loader_animates_constrained_width_without_rerunning_rhai(cx: &mut TestAppContext) {
    use gpui_rhai::ManualRuntimeClock;
    use std::time::{Duration, Instant};
    let source = r#"
        import "components/thinking_loader" as loader;
        fn state_schema(){#{fields:#{
            phase:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn next(ctx,payload){ctx.set_state("phase",1);}
        fn view(ctx){column([
            loader::ThinkingLoader(#{key:"work",phases:["Ready","Searching the rollout logs"],phase:ctx.get_state("phase")}),
            text("Next").accessibility_role("button").on_click(Fn("next"))
        ]).with_style(style().width(px(230)).items_start())}
    "#;
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::components(source)
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock))
        .mount(cx);
    ready(&mut visual, &view);
    let first = bounds(&mut visual, &view, "status", "Ready").width;
    click(&mut visual, &view, "button", "Next");
    let target = "Searching the rollout logs";
    let start = bounds(&mut visual, &view, "status", target).width;
    assert!((first - start).abs() < 0.5, "first={first}, start={start}");
    let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
    manual.advance(Duration::from_millis(150));
    settle(&mut visual, Step::Refresh, 4);
    let mid = bounds(&mut visual, &view, "status", target).width;
    manual.advance(Duration::from_millis(350));
    settle(&mut visual, Step::Refresh, 4);
    let end = bounds(&mut visual, &view, "status", target).width;
    assert!(
        first < mid && mid < end && end <= 230.5,
        "{first}, {mid}, {end}"
    );
    assert_eq!(
        visual.update(|_, cx| view.committed_revision(cx).unwrap()),
        revision
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_run_controls_routes_submission_queue_steer_and_recovery(cx: &mut TestAppContext) {
    use std::time::Duration;
    let (mut visual, view) = gallery(cx, "run_controls", "modern-light");
    click(&mut visual, &view, "button", "Decline next input");
    click(&mut visual, &view, "text_field", "Message");
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "run-host-note"),
        "Request declined. Draft preserved."
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("Investigate the stalled rollout.".into()))
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("running"));
    let draft = "Steer or queue a message";
    click(&mut visual, &view, "text_field", draft);
    visual.simulate_input("next task");
    visual.simulate_keystrokes("alt-enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "listitem", "next task");
    assert_eq!(
        node(&mut visual, &view, "text_field", draft).value,
        Some(UiValue::String(String::new()))
    );
    visual.simulate_input("inspect");
    visual.simulate_keystrokes("shift-enter");
    visual.simulate_input("edge host");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", draft).value,
        Some(UiValue::String("inspect\nedge host".into()))
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "inspect\nedge host")
    );
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    assert_ne!(
        text_of(&mut visual, &view, "run-host-progress"),
        "running · 0"
    );
    visual.simulate_input("remain in draft");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    let paused = text_of(&mut visual, &view, "run-host-progress");
    assert!(paused.starts_with("stopped"));
    visual.executor().advance_clock(Duration::from_secs(2));
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "run-host-progress"), paused);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("remain in draft".into()))
    );
    click(&mut visual, &view, "button", "Resume");
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("running"));
    click(&mut visual, &view, "button", "Simulate disconnect");
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("error"));
    click(&mut visual, &view, "button", "Start queued run");
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("running"));
    assert_eq!(
        node(&mut visual, &view, "text_field", draft).value,
        Some(UiValue::String("remain in draft".into()))
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "listitem" && n.name == "next task")
    );
}

#[gpui::test]
fn gallery_run_controls_policy_is_controlled_and_escape_closes_menu_first(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "run_controls", "clean-light");
    click(&mut visual, &view, "button", "Send");
    click(&mut visual, &view, "button", "Permissions: Ask first");
    visual.simulate_keystrokes("end enter");
    ready(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "run-host-note"),
        "The demo Host declined full access."
    );
    click(&mut visual, &view, "button", "Permissions: Ask first");
    visual.simulate_keystrokes("down enter");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Permissions: Auto-edit");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Permissions: Auto-edit").expanded,
        Some(false)
    );
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("running"));
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(text_of(&mut visual, &view, "run-host-progress").starts_with("stopped"));
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn textarea_alt_submit_does_not_commit_marked_input(cx: &mut TestAppContext) {
    verify_textarea_alternate_composition(cx, "alt_enter", "alt-enter");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn textarea_modifier_alt_submit_does_not_commit_marked_input(cx: &mut TestAppContext) {
    verify_textarea_alternate_composition(
        cx,
        "mod_enter",
        if cfg!(target_os = "macos") {
            "cmd-enter"
        } else {
            "ctrl-enter"
        },
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn verify_textarea_alternate_composition(cx: &mut TestAppContext, mode: &str, key: &str) {
    let source = r#"
        import "components/textarea" as area;
        fn state_schema(){#{fields:#{
            draft:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
            receipt:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}},
        }}}
        fn change(ctx,value){ctx.set_state("draft",value);}
        fn received(label,ctx,value){ctx.set_state("receipt",label+":"+value);}
        fn escape(ctx,payload){ctx.set_state("receipt","ancestor escape");handled()}
        fn view(ctx){column([
            area::Textarea(#{key:"editor",label:"Alternate",value:ctx.get_state("draft"),
                on_change:Fn("change"),submit_key:"enter",on_submit:Fn("received").curry("main"),
                alt_submit_key:"ALTERNATE_MODE",on_alt_submit:Fn("received").curry("alternate")}),
            text(ctx.get_state("receipt")).test_id("receipt")
        ]).on_key_value("escape",Fn("escape"),())}
    "#
    .replace("ALTERNATE_MODE", mode);
    let (mut visual, view) = Mount::components(source).mount(cx);
    ready(&mut visual, &view);
    click(&mut visual, &view, "text_field", "Alternate");
    let mut ime = visual
        .update(|window, _| window.visual_probe_take_input())
        .expect("native focused editor");
    ime.replace_and_mark_text_in_range(None, "中文", Some(2..2));
    visual.update(|window, _| window.visual_probe_restore_input(ime));
    ready(&mut visual, &view);
    visual.simulate_keystrokes(&format!("{key} enter escape"));
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "receipt"), "none");
    let mut ime = visual
        .update(|window, _| window.visual_probe_take_input())
        .expect("retained native editor");
    assert_eq!(ime.marked_text_range(), Some(0..2));
    ime.unmark_text();
    visual.update(|window, _| window.visual_probe_restore_input(ime));
    ready(&mut visual, &view);
    visual.simulate_keystrokes(key);
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "receipt"), "alternate:中文");
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "receipt"), "main:中文");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "receipt"), "ancestor escape");
}

#[gpui::test]
fn run_controls_preserves_paused_log_and_resets_follow_for_a_new_run(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollEvent, point};
    let source = r#"
        import "patterns/run_controls" as run;
        fn state_schema(){#{fields:#{
            count:#{schema:#{type:"integer"},"default":#{type:"integer",value:320}},
            run:#{schema:#{type:"string"},"default":#{type:"string",value:"first/1"}},
        }}}
        fn append(ctx,payload){ctx.set_state("count",ctx.get_state("count")+50);}
        fn restart(ctx,payload){ctx.set_state("run","second/2");ctx.set_state("count",1);}
        fn view(ctx){
            let text_value="";for i in 0..ctx.get_state("count"){text_value+="The accepted answer has more lines. ";}
            column([
                run::RunControls(#{key:"runner",run_id:ctx.get_state("run"),draft:"",status:"running",
                    prompt:"Look at the complete log.",answer:text_value}),
                text("Append").accessibility_role("button").on_click(Fn("append")),
                text("New run").accessibility_role("button").on_click(Fn("restart"))
            ]).with_style(style().width(px(500)).min_width(px(0)))
        }
    "#;
    let component = gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID
        .iter()
        .find(|(id, _)| *id == "patterns/run_controls")
        .unwrap()
        .1;
    let (mut visual, view) = Mount::components(source)
        .module("patterns/run_controls", component)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    let metrics = |visual: &mut VisualTestContext| {
        visual
            .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
            .virtual_collections[0]
            .clone()
    };
    let initial = metrics(&mut visual);
    assert!(
        initial.item_count > 3 && initial.is_following_tail && initial.viewport_height > 0.0,
        "{initial:?}"
    );
    let log = visual.debug_bounds("virtual-list:run-log:first/1").unwrap();
    let pos = point(
        log.origin.x + log.size.width / 2.0,
        log.origin.y + log.size.height / 2.0,
    );
    visual.simulate_event(ScrollEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(160.))),
        ..Default::default()
    });
    ready(&mut visual, &view);
    let reading = metrics(&mut visual);
    assert!(
        !reading.is_following_tail,
        "before {initial:?}; after wheel {reading:?}"
    );
    node(&mut visual, &view, "button", "Jump to latest");
    click(&mut visual, &view, "button", "Append");
    let appended = metrics(&mut visual);
    assert!(!appended.is_following_tail, "after append {appended:?}");
    node(&mut visual, &view, "button", "Jump to latest");
    click(&mut visual, &view, "button", "Jump to latest");
    assert!(metrics(&mut visual).is_following_tail);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Jump to latest")
    );
    visual.simulate_event(ScrollEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(160.))),
        ..Default::default()
    });
    ready(&mut visual, &view);
    assert!(!metrics(&mut visual).is_following_tail);
    node(&mut visual, &view, "button", "Jump to latest");
    click(&mut visual, &view, "button", "New run");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Jump to latest")
    );
}

#[gpui::test]
fn gallery_prompt_library_retains_literal_variables_and_only_fills_the_draft(
    cx: &mut TestAppContext,
) {
    use gpui::{Modifiers, MouseButton, point};
    let (mut visual, view) = gallery(cx, "prompt_library", "modern-light");
    assert!(node(&mut visual, &view, "button", "Use prompt").disabled);
    click(&mut visual, &view, "button", "Copy prompt");
    let copied = visual
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .unwrap();
    assert_eq!(
        copied,
        "Explain the rollout to {audience}. Keep the scope to edge hosts."
    );
    let preview = bounds(&mut visual, &view, "document", "Prompt preview");
    let pos = point(px((preview.x + 2.) as f32), px((preview.y + 8.) as f32));
    visual.simulate_mouse_down(pos, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(pos, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("tab enter");
    ready(&mut visual, &view);
    visual.simulate_input("{scope}");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "audience").value,
        Some(UiValue::String("{scope}".into()))
    );
    click(&mut visual, &view, "option", "Write meeting notes");
    assert!(node(&mut visual, &view, "button", "Use prompt").disabled);
    click(&mut visual, &view, "option", "Summarize rollout");
    assert_eq!(
        node(&mut visual, &view, "text_field", "audience").value,
        Some(UiValue::String("{scope}".into()))
    );
    // The preview still uses the actual asynchronous Markdown reader after
    // a template switch; its final line height changes the footer position.
    let mut prepared = false;
    for _ in 0..20 {
        let preview = bounds(&mut visual, &view, "document", "Prompt preview");
        click_center(&mut visual, preview);
        visual.simulate_keystrokes("alt-cmd-c");
        if visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .is_some_and(|raw| raw.contains("gpui-prompt:1"))
        {
            prepared = true;
            break;
        }
        ready(&mut visual, &view);
    }
    assert!(prepared, "accepted template must have a prepared preview");
    assert!(!node(&mut visual, &view, "button", "Use prompt").disabled);
    click(&mut visual, &view, "button", "Use prompt");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Prompt draft").value,
        Some(UiValue::String(
            "Explain the rollout to {scope}. Keep the scope to edge hosts.".into()
        ))
    );
    assert!(node(&mut visual, &view, "button", "Send").disabled);
    click(&mut visual, &view, "text_field", "audience");
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("Operators");
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Prompt draft").value,
        Some(UiValue::String(
            "Explain the rollout to Operators. Keep the scope to edge hosts.".into()
        ))
    );
}

#[gpui::test]
fn gallery_prompt_library_reflows_without_replacing_the_active_variable_editor(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "prompt_library", "clean-light");
    click(&mut visual, &view, "text_field", "audience");
    visual.simulate_input("Ops");
    ready(&mut visual, &view);
    let focus = visual
        .update(|window, cx| window.focused(cx))
        .expect("variable input owns focus");
    visual.simulate_resize(size(px(580.), px(750.)));
    ready(&mut visual, &view);
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
    visual.simulate_input(" team");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "audience").value,
        Some(UiValue::String("Ops team".into()))
    );
}

#[gpui::test]
fn gallery_integration_picker_only_toggles_accepted_selection_and_preserves_group_until_search(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "integration_picker", "modern-light");
    let before = bounds(&mut visual, &view, "option", "Observability").y;
    click(&mut visual, &view, "switch", "Connect Observability");
    assert_eq!(
        node(&mut visual, &view, "switch", "Disconnect Observability").checked,
        Some(UiValue::Bool(true))
    );
    assert!(
        (bounds(&mut visual, &view, "option", "Observability").y - before).abs() < 0.5,
        "connection doesn't reorder the current results"
    );
    click(&mut visual, &view, "text_field", "Search integrations");
    visual.simulate_input("logs.workspace");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Connected");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "option" && n.name == "Secure Archive")
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("archive.workspace");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "switch", "Connect Secure Archive");
    // Selection and toggle are separate transactions when a query has
    // hidden the accepted selection.
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Host declined the archive request.")
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "switch", "Connect Secure Archive").checked,
        Some(UiValue::Bool(false))
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Host declined the archive request.")
    );
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Search integrations").value,
        Some(UiValue::String(String::new()))
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Close requested.")
    );
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Close requested.")
    );
}

#[gpui::test]
fn gallery_integration_settings_validates_and_retries_without_erasing_drafts(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "integration_settings", "modern-light");
    let edit = |visual: &mut VisualTestContext, text: &str| {
        click(visual, &view, "text_field", "Site address");
        visual.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.into())));
        visual.simulate_keystrokes("cmd-a cmd-v");
        ready(visual, &view);
    };
    edit(&mut visual, "https://shop.example/settings");
    let site = node(&mut visual, &view, "text_field", "Site address");
    assert_eq!(
        site.value,
        Some(UiValue::String("https://shop.example/settings".into()))
    );
    assert!(site.invalid, "{site:?}");
    assert!(node(&mut visual, &view, "button", "Save").disabled);
    edit(&mut visual, "https://München.de/");
    click(&mut visual, &view, "combobox", "Time zone");
    click(&mut visual, &view, "text_field", "Time zone");
    visual.simulate_input("tok");
    ready(&mut visual, &view);
    click(&mut visual, &view, "option", "Tokyo");
    assert!(!node(&mut visual, &view, "button", "Save").disabled);
    click(&mut visual, &view, "button", "Decline next save");
    click(&mut visual, &view, "button", "Save");
    node(&mut visual, &view, "button", "Saving…");
    visual
        .executor()
        .advance_clock(std::time::Duration::from_millis(500));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Save declined by the demo Host.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Site address").value,
        Some(UiValue::String("https://München.de/".into()))
    );
    click(&mut visual, &view, "button", "Save");
    visual
        .executor()
        .advance_clock(std::time::Duration::from_millis(500));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Site address").value,
        Some(UiValue::String("xn--mnchen-3ya.de".into()))
    );
    assert!(
        text_of(&mut visual, &view, "integration-accepted")
            .contains("revision 2: xn--mnchen-3ya.de · Asia/Tokyo")
    );
}

#[gpui::test]
fn gallery_integration_settings_conflict_pause_and_discard(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "integration_settings", "clean-dark");
    click(&mut visual, &view, "text_field", "Site address");
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("draft.example");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Change site on Host");
    assert!(node(&mut visual, &view, "button", "Save").disabled);
    click(&mut visual, &view, "text_field", "Site address");
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("edited.example");
    ready(&mut visual, &view);
    assert!(node(&mut visual, &view, "button", "Save").disabled);
    click(&mut visual, &view, "switch", "Storefront sync");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "text_field" && n.name == "Site address")
    );
    click(&mut visual, &view, "switch", "Storefront sync");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Site address").value,
        Some(UiValue::String("edited.example".into()))
    );
    click(&mut visual, &view, "button", "Keep my edits");
    assert!(!node(&mut visual, &view, "button", "Save").disabled);
    click(&mut visual, &view, "button", "Discard");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Site address").value,
        Some(UiValue::String("host.example".into()))
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "button" && n.name == "Save")
    );
}

#[gpui::test]
fn gallery_integration_settings_has_a_separate_host_key_workflow(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "integration_picker", "clean-light");
    click(&mut visual, &view, "button", "Explore integration");
    node(&mut visual, &view, "region", "Observability settings");
    click(&mut visual, &view, "button", "Generate a new API key");
    visual
        .executor()
        .advance_clock(std::time::Duration::from_millis(500));
    ready(&mut visual, &view);
    node(&mut visual, &view, "group", "API key stored");
    click(&mut visual, &view, "button", "Show key");
    node(&mut visual, &view, "button", "Hide key");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name.contains("demo-only-key-1")
                || n.value == Some(UiValue::String("demo-only-key-1".into())))
    );
    click(&mut visual, &view, "button", "Copy key");
    let clipboard = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(clipboard.as_deref(), Some("demo-only-key-1"));
    assert!(
        text_of(&mut visual, &view, "integration-accepted").contains("revision 1: shop.example")
    );
    click(&mut visual, &view, "switch", "Observability sync");
    click(&mut visual, &view, "switch", "Observability sync");
    node(&mut visual, &view, "button", "Show key");
    click(&mut visual, &view, "button", "Back to integrations");
    node(&mut visual, &view, "option", "Observability");
}

#[gpui::test]
fn gallery_comment_thread_keeps_drafts_until_accepted_and_expands_details(cx: &mut TestAppContext) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "comment_thread", "modern-light");
    let detail = "The previous configuration is still active.";
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == detail)
    );
    click(&mut visual, &view, "button", "Reviewed two notes");
    assert_eq!(
        node(&mut visual, &view, "button", "Reviewed two notes").expanded,
        Some(true)
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == detail)
    );
    click(&mut visual, &view, "button", "Reviewed two notes");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == detail)
    );

    click(&mut visual, &view, "button", "Decline next comment");
    click(&mut visual, &view, "text_field", "Reply");
    visual.simulate_input("Please add the incident ID.");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert!(node(&mut visual, &view, "button", "Send reply").disabled);
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the comment. Draft retained.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Reply").value,
        Some(UiValue::String("Please add the incident ID.".into()))
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "group" && n.name == "Jules")
    );

    click(&mut visual, &view, "button", "Send reply");
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Reply").value,
        Some(UiValue::String(String::new()))
    );
    assert!(node(&mut visual, &view, "button", "Send reply").disabled);
    visual.executor().advance_clock(Duration::from_millis(850));
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Reply ready");
    click(&mut visual, &view, "button", "Close comment");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "region" && n.name == "Review comment")
    );
    click(&mut visual, &view, "button", "Show comments");
    node(&mut visual, &view, "region", "Review comment");
}

#[gpui::test]
fn comment_thread_keeps_the_native_reader_anchor_when_host_appends(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollEvent, point};

    let source = r#"
        import "patterns/comment_thread" as conversation;
        fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:64}}}}}
        fn append(ctx,event){ctx.set_state("count",ctx.get_state("count")+1);handled()}
        fn view(ctx){
            let messages=[];
            for i in 0..ctx.get_state("count"){
                messages.push(#{id:`message/${i}`,author:`Reviewer ${i}`,text:`Comment ${i} about the release.`});
            }
            column([
                text("Append a message").accessibility_role("button").on_click(Fn("append")),
                conversation::CommentThread(#{key:"review",messages:messages,draft:"",max_height:px(204)})
            ]).with_style(style().width(px(400)))
        }
    "#;
    let (mut visual, view) = Mount::components(source)
        .modules(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    let metrics = |visual: &mut VisualTestContext| {
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .virtual_collections[0]
            .clone()
    };
    assert!(metrics(&mut visual).is_following_tail);
    let list = visual
        .debug_bounds("virtual-list:transcript")
        .expect("native transcript");
    visual.simulate_event(ScrollEvent {
        position: point(list.left() + px(40.), list.top() + px(48.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(300.))),
        ..Default::default()
    });
    settle(&mut visual, Step::Tick, 10);
    let before = metrics(&mut visual);
    assert!(!before.is_following_tail);
    click(&mut visual, &view, "button", "Append a message");
    let after = metrics(&mut visual);
    assert_eq!(after.scroll_item, before.scroll_item);
    assert!(!after.is_following_tail);
    click(&mut visual, &view, "button", "Jump to latest");
    settle(&mut visual, Step::Tick, 10);
    assert!(metrics(&mut visual).is_following_tail);
    node(&mut visual, &view, "group", "Reviewer 64");
}

#[gpui::test]
fn gallery_comments_panel_filters_sorts_and_proposes_only_authorized_edit(cx: &mut TestAppContext) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "comments_panel", "clean-dark");
    let groups = |visual: &mut VisualTestContext| {
        snapshot(visual, &view)
            .nodes()
            .filter(|n| {
                n.role == "group"
                    && ["Tess Rivera", "Riley Kim", "Morgan Lee", "Sam Lee"]
                        .contains(&n.name.as_str())
            })
            .map(|n| n.name.clone())
            .collect::<Vec<_>>()
    };
    click(&mut visual, &view, "tab", "Questions");
    assert_eq!(groups(&mut visual), ["Riley Kim"]);
    click(&mut visual, &view, "button", "Resolve comment");
    assert_eq!(
        node(&mut visual, &view, "button", "Reopen comment").pressed,
        Some(true)
    );
    click(&mut visual, &view, "button", "Reopen comment");
    assert_eq!(
        node(&mut visual, &view, "button", "Resolve comment").pressed,
        Some(false)
    );
    click(&mut visual, &view, "tab", "Edits, 1");
    assert_eq!(groups(&mut visual).len(), 2);
    assert_eq!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Run edit")
            .count(),
        1
    );
    click(&mut visual, &view, "button", "Run edit");
    assert!(node(&mut visual, &view, "button", "Running…").disabled);
    visual.executor().advance_clock(Duration::from_millis(550));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Edit applied");
    click(&mut visual, &view, "button", "Undo");
    node(&mut visual, &view, "button", "Run edit");
    node(&mut visual, &view, "status", "The demo change was undone.");

    click(&mut visual, &view, "tab", "All");
    assert!(
        bounds(&mut visual, &view, "group", "Riley Kim").y
            > bounds(&mut visual, &view, "group", "Morgan Lee").y
    );
    click(&mut visual, &view, "button", "Sort comments");
    click(&mut visual, &view, "menuitem", "Oldest first");
    assert!(
        bounds(&mut visual, &view, "group", "Riley Kim").y
            < bounds(&mut visual, &view, "group", "Morgan Lee").y
    );
    click(&mut visual, &view, "tab", "Questions");
    click(&mut visual, &view, "text_field", "Add a comment");
    visual.simulate_input("Test noted.");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter");
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Comment added.");
    assert_eq!(node(&mut visual, &view, "tab", "All").selected, Some(true));
    assert!(
        bounds(&mut visual, &view, "group", "Jules").y
            < bounds(&mut visual, &view, "group", "Morgan Lee").y
    );
}

#[gpui::test]
fn gallery_writing_assistant_streams_accepted_text_and_reset_cancels_old_answer(
    cx: &mut TestAppContext,
) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "writing_assistant", "modern-light");
    click(&mut visual, &view, "button", "Polish my email");
    node(&mut visual, &view, "status", "Writing…");
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Writing…");
    visual.executor().advance_clock(Duration::from_millis(500));
    ready(&mut visual, &view);
    let partial = snapshot(&mut visual, &view);
    let reply = partial.nodes().find(|n| {
        n.name.starts_with("Here’s a first draft.") && !n.name.contains("before Friday.")
    });
    assert!(
        reply.is_some(),
        "{:?}",
        partial
            .nodes()
            .map(|node| &node.name)
            .filter(|name| name.contains("draft"))
            .collect::<Vec<_>>()
    );
    let reply_bounds = reply.unwrap().geometry.unwrap().visual;
    let input_y = bounds(&mut visual, &view, "text_field", "Message").y;
    assert!(
        (0.0..65.0).contains(&(input_y - reply_bounds.y - reply_bounds.height)),
        "A short transcript should sit above the composer: {reply_bounds:?}, {input_y}"
    );
    click(&mut visual, &view, "text_field", "Message");
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            "Next draft while writing".into(),
        ));
    });
    visual.simulate_keystrokes("cmd-v");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("Next draft while writing".into()))
    );
    assert!(node(&mut visual, &view, "button", "Send").disabled);
    click(&mut visual, &view, "button", "New chat");
    visual.executor().advance_clock(Duration::from_secs(4));
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "What should we write?");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name.contains("Here’s a first draft."))
    );
}

#[gpui::test]
fn gallery_writing_assistant_native_submit_and_rejection_keep_multiline_draft(
    cx: &mut TestAppContext,
) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "writing_assistant", "clean-dark");
    visual.simulate_resize(size(px(620.), px(740.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next message");
    click(&mut visual, &view, "text_field", "Message");
    let height = bounds(&mut visual, &view, "text_field", "Message").height;
    visual.simulate_input("A short draft");
    visual.simulate_keystrokes("shift-enter");
    visual.simulate_input("A second line");
    ready(&mut visual, &view);
    assert!(bounds(&mut visual, &view, "text_field", "Message").height > height);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("A short draft\nA second line".into()))
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Writing…");
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the message. Draft retained.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("A short draft\nA second line".into()))
    );
    click(&mut visual, &view, "button", "Send");
    visual.executor().advance_clock(Duration::from_millis(380));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String(String::new()))
    );
}

#[gpui::test]
fn gallery_writing_assistant_dictation_preview_never_sends_and_old_lease_dies(
    cx: &mut TestAppContext,
) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "writing_assistant", "clean-light");
    click(&mut visual, &view, "text_field", "Message");
    visual.simulate_input("Original draft");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Dictate");
    node(&mut visual, &view, "button", "Stop dictation");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "text_field" && n.name == "Message")
    );
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "writing-dictation-preview"),
        "Write a friendly"
    );
    click(&mut visual, &view, "button", "Stop dictation");
    visual.executor().advance_clock(Duration::from_secs(2));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String("Write a friendly".into()))
    );
    node(&mut visual, &view, "heading", "What should we write?");
    click(&mut visual, &view, "button", "Dictate");
    click(&mut visual, &view, "button", "New chat");
    visual.executor().advance_clock(Duration::from_secs(2));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Message").value,
        Some(UiValue::String(String::new()))
    );
    node(&mut visual, &view, "button", "Dictate");
}

#[gpui::test]
fn gallery_approval_card_rejection_review_revision_and_keyboard(cx: &mut TestAppContext) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "approval_card", "modern-light");
    assert_eq!(
        node(&mut visual, &view, "button", "Approve")
            .key_shortcuts
            .as_deref(),
        Some(if cfg!(target_os = "macos") {
            "cmd-enter"
        } else {
            "ctrl-enter"
        })
    );
    click(&mut visual, &view, "button", "Decline next proposal");
    click(&mut visual, &view, "checkbox", "Always allow this action");
    assert_eq!(
        node(&mut visual, &view, "checkbox", "Always allow this action").checked,
        Some(UiValue::Bool(true))
    );
    visual.simulate_keystrokes("cmd-enter");
    ready(&mut visual, &view);
    assert!(node(&mut visual, &view, "button", "Processing…").disabled);
    visual.executor().advance_clock(Duration::from_millis(700));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host did not accept the proposal.",
    );
    assert_eq!(
        node(&mut visual, &view, "checkbox", "Always allow this action").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut visual, &view, "button", "Edit");
    assert_eq!(
        node(&mut visual, &view, "checkbox", "Always allow this action").checked,
        Some(UiValue::Bool(false))
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Revised release summary")
    );
    click(&mut visual, &view, "checkbox", "Always allow this action");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Denied");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "button" && n.name == "Approve")
    );
}

#[gpui::test]
fn gallery_approval_card_soft_clock_pauses_for_hover_and_focus(cx: &mut TestAppContext) {
    use gpui::{Modifiers, point};
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "approval_card", "modern-dark");
    let read = |visual: &mut VisualTestContext| {
        snapshot(visual, &view)
            .nodes()
            .find(|n| n.name.starts_with("Expires in "))
            .unwrap()
            .name
            .clone()
    };
    let card = bounds(&mut visual, &view, "group", "Record a release note");
    visual.simulate_mouse_move(
        point(px((card.x + 35.) as f32), px((card.y + 22.) as f32)),
        None,
        Modifiers::none(),
    );
    ready(&mut visual, &view);
    let hovered = read(&mut visual);
    visual.executor().advance_clock(Duration::from_secs(3));
    ready(&mut visual, &view);
    assert_eq!(read(&mut visual), hovered);
    click(&mut visual, &view, "checkbox", "Always allow this action");
    visual.simulate_mouse_move(point(px(12.), px(12.)), None, Modifiers::none());
    ready(&mut visual, &view);
    let focused = read(&mut visual);
    visual.executor().advance_clock(Duration::from_secs(3));
    ready(&mut visual, &view);
    assert_eq!(read(&mut visual), focused);
    click(&mut visual, &view, "button", "Decline next proposal");
    visual.executor().advance_clock(Duration::from_millis(1330));
    ready(&mut visual, &view);
    assert_ne!(read(&mut visual), focused);
}

#[gpui::test]
fn gallery_approval_card_replacement_cannot_accept_old_receipt(cx: &mut TestAppContext) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "approval_card", "clean-dark");
    click(&mut visual, &view, "button", "Approve");
    node(&mut visual, &view, "button", "Processing…");
    click(&mut visual, &view, "button", "New review request");
    visual.executor().advance_clock(Duration::from_secs(2));
    ready(&mut visual, &view);
    assert!(!node(&mut visual, &view, "button", "Approve").disabled);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "status" && n.name == "Approved")
    );
    click(&mut visual, &view, "button", "Approve");
    visual.executor().advance_clock(Duration::from_millis(720));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Approved");
}

#[gpui::test]
fn gallery_permission_request_timed_receipt_revoke_and_destructive_guard(cx: &mut TestAppContext) {
    use std::time::Duration;

    let (mut visual, view) = gallery(cx, "permission_request", "clean-light");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Public outline")
    );
    click(&mut visual, &view, "button", "2 more");
    assert_eq!(
        node(&mut visual, &view, "button", "Show less").expanded,
        Some(true)
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Public outline")
    );
    click(&mut visual, &view, "button", "Show less");
    click(&mut visual, &view, "radio", "1 hour");
    assert_eq!(
        node(&mut visual, &view, "radio", "1 hour").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut visual, &view, "button", "Allow for 1 hour");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Allowed temporarily");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "1 hour left")
    );
    click(&mut visual, &view, "button", "Revoke");
    node(&mut visual, &view, "status", "Access revoked");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "The demo grant was revoked.")
    );
    click(&mut visual, &view, "button", "Ask again");
    node(&mut visual, &view, "button", "Allow once");
    click(&mut visual, &view, "radio", "1 hour");
    click(&mut visual, &view, "button", "Allow for 1 hour");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "End demo time window");
    node(&mut visual, &view, "status", "Access expired");
    click(&mut visual, &view, "button", "Use destructive request");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "radio" && n.name == "Always")
    );
    assert!(!node(&mut visual, &view, "button", "Allow once").disabled);
}

#[gpui::test]
fn permission_request_never_falls_back_to_more_durable_scope(cx: &mut TestAppContext) {
    let source = r#"
        import "patterns/permission_request" as access;
        fn submit(ctx,event){throw "No new grant may be proposed for this selection";}
        fn view(ctx){
            access::PermissionRequest(#{key:"access",request:"danger/1",revision:1,agent:"Sample",action:"replace a preview",
                risk:"destructive",scope:"always",scopes:["always","timed"],on_request:Fn("submit")})
                .with_style(style().width(px(380)))
        }
    "#;
    let (mut visual, view) = Mount::components(source)
        .modules(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    assert!(node(&mut visual, &view, "button", "Allow").disabled);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "radio" && n.name == "Always")
    );
    assert_eq!(
        node(&mut visual, &view, "radio", "1 hour").checked,
        Some(UiValue::Bool(false))
    );
}

#[gpui::test]
fn prompt_library_bounds_one_pass_expansion_before_copy_or_use(cx: &mut TestAppContext) {
    let source = r#"
        import "patterns/prompt_library" as library;
        fn use_text(ctx,payload){throw "oversize text must not reach the Host";}
        fn view(ctx){
            let body="";for i in 0..128{body+="{value}";}
            let value="";for i in 0..512{value+="x";}
            library::PromptLibrary(#{key:"bounded",on_use:Fn("use_text"),templates:[
                #{id:"big",title:"Large template",body:body,variables:#{value:#{"default":value}}}
            ]}).with_style(style().width(px(670)))
        }
    "#;
    let (mut visual, view) = Mount::components(source)
        .modules(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    assert!(node(&mut visual, &view, "button", "Use prompt").disabled);
    assert!(node(&mut visual, &view, "button", "Copy prompt").disabled);
    node(
        &mut visual,
        &view,
        "status",
        "Prompt exceeds the editing limit.",
    );
}

#[gpui::test]
fn integration_picker_rejected_selection_never_toggles_hidden_choice(cx: &mut TestAppContext) {
    let source = r#"
        import "patterns/integration_picker" as picker;
        fn state_schema(){#{fields:#{
            seen:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn keep(ctx,id){ctx.set_state("seen",ctx.get_state("seen")+1);}
        fn toggle(ctx,payload){throw "unaccepted selection must not toggle";}
        fn view(ctx){column([
            picker::IntegrationPicker(#{key:"picker",value:"alpha",on_select:Fn("keep"),on_toggle:Fn("toggle"),
                integrations:[
                    #{id:"alpha",name:"Alpha",domain:"alpha.local",description:"First."},
                    #{id:"beta/7",name:"Beta",domain:"beta.local",description:"Second."}
                ]}),
            text(ctx.get_state("seen").to_string()).test_id("seen")
        ]).with_style(style().width(px(675)))}
    "#;
    let (mut visual, view) = Mount::components(source)
        .modules(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut visual, &view);
    click(&mut visual, &view, "text_field", "Search integrations");
    visual.simulate_input("beta.local");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter enter");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "seen"), "2");
    assert_eq!(
        node(&mut visual, &view, "option", "Beta").selected,
        Some(false)
    );
    assert_eq!(
        node(&mut visual, &view, "switch", "Connect Alpha").checked,
        Some(UiValue::Bool(false))
    );
}
