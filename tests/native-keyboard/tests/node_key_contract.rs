//! Real Rhai registration and GPUI focus-path dispatch, not AST-only probes.
use gpui::{
    Context, IntoElement, KeyDownEvent, Keystroke, Modifiers, MouseButton, Render, TestAppContext,
    VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::collections::BTreeMap;

struct Host {
    domain: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.domain.container(self.view.element().unwrap())
    }
}

const SOURCE: &str = r#"
import "components/input" as input;
import "components/drag_source" as drag_source;
fn state_schema() { #{ fields: #{
    keys: #{schema:#{type:"integer"}, "default":#{type:"integer",value:0}},
    clicks: #{schema:#{type:"integer"}, "default":#{type:"integer",value:0}},
    typed: #{schema:#{type:"string"}, "default":#{type:"string",value:""}},
    trace: #{schema:#{type:"string"}, "default":#{type:"string",value:""}},
    drag: #{schema:#{type:"string"}, "default":#{type:"string",value:"ready"}}
} } }
fn h(ctx,p) { let n = if p == () { 1 } else { p }; ctx.set_state("keys",ctx.get_state("keys")+n); }
fn clicked(ctx,p) { ctx.set_state("clicks",ctx.get_state("clicks")+1); }
fn changed(ctx,p) { ctx.set_state("typed",p); }
fn add(ctx,tag) { ctx.set_state("trace",ctx.get_state("trace")+tag); }
fn parent_capture(ctx,p) { add(ctx,"P"); event_response() }
fn stopped_capture(ctx,p) { add(ctx,"P"); event_response().stop() }
fn child_capture(ctx,p) { add(ctx,"C"); event_response() }
fn child_target(ctx,p) { add(ctx,"T"); event_response() }
fn child_bubble(ctx,p) { add(ctx,"B"); event_response() }
fn parent_bubble(ctx,p) { add(ctx,"Q"); event_response() }
fn drag_ended(ctx,p) { ctx.set_state("drag",if p.cancelled {"cancelled"} else {"finished"}); }
fn view(ctx) { column([
    TARGET,
    input::Input(#{key:"input",label:"Editable",value:ctx.get_state("typed"),on_change:Fn("changed")}),
    text(`${ctx.get_state("keys")}:${ctx.get_state("clicks")}:${ctx.get_state("typed")}`).accessibility_role("status"),
    text(ctx.get_state("trace")).test_id("trace"),
    text(ctx.get_state("drag")).test_id("drag-status")
]).with_style(style().width(px(300))) }
"#;

fn mount(cx: &mut TestAppContext, target: &str) -> WindowHandle<Host> {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, SOURCE.replace("TARGET", target)),
            (
                ModuleId::parse("components/input").unwrap(),
                include_str!("../../../registry/components/input.rhai").into(),
            ),
            (
                ModuleId::parse("components/drag_source").unwrap(),
                include_str!("../../../registry/components/drag_source.rhai").into(),
            ),
        ])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let window = cx.add_window(move |window, cx| {
        let domain = ScriptViewHost::new("keys", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("keys"), domain.clone(), window, cx)
            .unwrap();
        Host { domain, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    window
}

fn status(cx: &mut TestAppContext, window: WindowHandle<Host>) -> String {
    window
        .update(cx, |host, _, cx| {
            host.view
                .accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|n| n.role == "status")
                .unwrap()
                .name
                .clone()
        })
        .unwrap()
}

fn named_text(cx: &mut TestAppContext, window: WindowHandle<Host>, id: &str) -> String {
    window
        .update(cx, |host, _, cx| {
            host.view
                .accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.test_id.as_deref() == Some(id))
                .unwrap()
                .name
                .clone()
        })
        .unwrap()
}

fn key(visual: &mut VisualTestContext, name: &str, modifiers: Modifiers, key_char: Option<&str>) {
    visual.simulate_event(KeyDownEvent {
        keystroke: Keystroke {
            key: name.into(),
            key_char: key_char.map(Into::into),
            modifiers,
        },
        is_held: false,
        prefer_character_input: false,
    });
    visual.run_until_parked();
}

fn cleanup(cx: &mut TestAppContext, window: WindowHandle<Host>) {
    window
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn every_entry_dispatches_normalized_named_and_literal_punctuation_keys(cx: &mut TestAppContext) {
    for method in ["on_key_value", "on", "on_capture", "on_bubble"] {
        for (registered, delivered) in [
            ("?", "?"),
            ("/", "/"),
            ("[", "["),
            ("]", "]"),
            ("-", "-"),
            ("=", "="),
            ("Escape", "escape"),
        ] {
            let name = serde_json::to_string(&if method == "on_key_value" {
                registered.into()
            } else {
                format!("key:{registered}")
            })
            .unwrap();
            let payload = if method == "on_key_value" { ",7" } else { "" };
            let target = format!("text(\"Target\").{method}({name},Fn(\"h\"){payload})");
            let window = mount(cx, &target);
            let mut visual = VisualTestContext::from_window(*window, cx);
            visual.update(|window, cx| window.focus_next(cx));
            key(&mut visual, "f12", Modifiers::none(), None);
            assert_eq!(status(cx, window), "0:0:");
            key(&mut visual, delivered, Modifiers::none(), None);
            assert_eq!(
                status(cx, window),
                if method == "on_key_value" {
                    "7:0:"
                } else {
                    "1:0:"
                },
                "{method}, {registered}"
            );
            cleanup(cx, window);
        }
    }
}

#[gpui::test]
fn raw_key_matching_ignores_modifiers_but_not_key_char(cx: &mut TestAppContext) {
    let window = mount(
        cx,
        "text(\"Target\").on(\"key:/\",Fn(\"h\")).on(\"key:?\",Fn(\"h\"))",
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.focus_next(cx));
    key(
        &mut visual,
        "/",
        Modifiers {
            control: true,
            ..Default::default()
        },
        None,
    );
    assert_eq!(status(cx, window), "1:0:");
    key(
        &mut visual,
        "q",
        Modifiers {
            alt: true,
            ..Default::default()
        },
        Some("?"),
    );
    assert_eq!(
        status(cx, window),
        "1:0:",
        "key_char is not a raw-key shortcut"
    );
    key(&mut visual, "?", Modifiers::none(), Some("?"));
    assert_eq!(status(cx, window), "2:0:");
    cleanup(cx, window);
}

#[gpui::test]
fn enter_and_space_click_only_when_their_key_handler_is_absent(cx: &mut TestAppContext) {
    for explicit in [false, true] {
        let target = if explicit {
            "text(\"Target\").on_click(Fn(\"clicked\")).on(\"key:Enter\",Fn(\"h\"))"
        } else {
            "text(\"Target\").on_click(Fn(\"clicked\"))"
        };
        let window = mount(cx, target);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.focus_next(cx));
        key(&mut visual, "enter", Modifiers::none(), None);
        assert_eq!(status(cx, window), if explicit { "1:0:" } else { "0:1:" });
        key(&mut visual, "space", Modifiers::none(), None);
        assert_eq!(status(cx, window), if explicit { "1:1:" } else { "0:2:" });
        cleanup(cx, window);
    }
}

#[gpui::test]
fn unrelated_native_text_focus_and_ime_commits_do_not_activate_node_shortcuts(
    cx: &mut TestAppContext,
) {
    let window = mount(
        cx,
        "text(\"Target\").on(\"key:?\",Fn(\"h\")).on(\"key:/\",Fn(\"h\"))",
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.focus_next(cx));
    key(&mut visual, "?", Modifiers::none(), None);
    assert_eq!(
        status(cx, window),
        "1:0:",
        "focused shortcut control is a positive control"
    );
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_input("?/[]-=中文");
    visual.run_until_parked();
    // dispatch_keystroke uses GPUI's with_simulated_ime path for Unicode.
    // This verifies native text/committed IME input, not OS preedit UI.
    assert_eq!(status(cx, window), "1:0:?/[]-=中文");
    cleanup(cx, window);
}

#[gpui::test]
fn disabled_key_control_is_not_a_tab_target_or_click_fallback(cx: &mut TestAppContext) {
    let window = mount(
        cx,
        "text(\"Target\").disabled(true).on(\"key:?\",Fn(\"h\")).on_click(Fn(\"clicked\"))",
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_input("?");
    key(&mut visual, "enter", Modifiers::none(), None);
    assert_eq!(status(cx, window), "0:0:?");
    cleanup(cx, window);
}

#[gpui::test]
fn native_key_phases_follow_the_focus_path_and_stop_prevents_later_phases(cx: &mut TestAppContext) {
    for stop in [false, true] {
        let capture = if stop {
            "stopped_capture"
        } else {
            "parent_capture"
        };
        let target = format!(
            r#"
            box([text("Target").on_capture("key:?",Fn("child_capture"))
                .on("key:?",Fn("child_target")).on_bubble("key:?",Fn("child_bubble"))])
                .tab_stop(false).on_capture("key:?",Fn("{capture}"))
                .on_bubble("key:?",Fn("parent_bubble"))
        "#
        );
        let window = mount(cx, &target);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.focus_next(cx));
        key(&mut visual, "?", Modifiers::none(), None);
        assert_eq!(
            named_text(cx, window, "trace"),
            if stop { "P" } else { "PCTBQ" }
        );
        cleanup(cx, window);
    }
}

#[gpui::test]
fn explicit_capture_and_bubble_enter_replace_click_fallback(cx: &mut TestAppContext) {
    for method in ["on_capture", "on_bubble"] {
        let target =
            format!("text(\"Target\").on_click(Fn(\"clicked\")).{method}(\"key:Enter\",Fn(\"h\"))");
        let window = mount(cx, &target);
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, cx| window.focus_next(cx));
        key(&mut visual, "enter", Modifiers::none(), None);
        assert_eq!(
            status(cx, window),
            "1:0:",
            "{method} must not additionally click"
        );
        cleanup(cx, window);
    }
}

#[gpui::test]
fn active_gesture_escape_owner_precedes_script_key_capture(cx: &mut TestAppContext) {
    let target = r#"box([drag_source::DragSource(#{key:"drag",label:"Source",
        source_id:"source",payload_type:"card",payload:#{id:"a"},on_drag_end:Fn("drag_ended"),
        content:text("Card").with_style(style().width(px(150)).height(px(40)))})])
        .tab_stop(false).on_capture("key:Escape",Fn("h"))"#;
    let window = mount(cx, target);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.focus_next(cx));
    key(&mut visual, "escape", Modifiers::none(), None);
    assert_eq!(
        status(cx, window),
        "1:0:",
        "without an owner the ancestor capture runs"
    );
    let bounds = window
        .update(cx, |host, _, cx| {
            host.view
                .accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", "Source")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
        })
        .unwrap();
    let start = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    let moved = point(start.x + px(20.0), start.y + px(10.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(moved, MouseButton::Left, Modifiers::none());
    visual.run_until_parked();
    key(&mut visual, "escape", Modifiers::none(), None);
    visual.run_until_parked();
    assert_eq!(named_text(cx, window, "drag-status"), "cancelled");
    assert_eq!(
        status(cx, window),
        "1:0:",
        "global gesture owner must consume before script capture"
    );
    visual.simulate_mouse_up(moved, MouseButton::Left, Modifiers::none());
    key(&mut visual, "escape", Modifiers::none(), None);
    assert_eq!(
        status(cx, window),
        "2:0:",
        "cancelled owner must relinquish Escape"
    );
    cleanup(cx, window);
}
