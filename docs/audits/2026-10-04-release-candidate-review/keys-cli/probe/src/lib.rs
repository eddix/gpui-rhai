#![cfg(test)]
#![allow(dead_code,unused_imports)]
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
fn target_stop(ctx,p){add(ctx,"S");event_response().stop()}
fn target_immediate(ctx,p){add(ctx,"I");event_response().stop_immediate()}
fn payload_capture(ctx,p){add(ctx,`C${p}`);event_response()}
fn payload_target(ctx,p){add(ctx,`T${p}`);event_response()}
fn host_action(ctx,p){add(ctx,"A");event_response().stop()}
fn init(ctx){ctx.register_action("audit.host",Fn("host_action"));}
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
                include_str!("../../../../../../registry/components/input.rhai").into(),
            ),
            (
                ModuleId::parse("components/drag_source").unwrap(),
                include_str!("../../../../../../registry/components/drag_source.rhai").into(),
            ),
        ])),
        include_str!("../../../../../../registry/themes/default_dark.rhai"),
    )
    .key_binding(KeyBindingSpec::new("ctrl-k",ActionId::parse("audit.host").unwrap(),None).unwrap())
    .prepare()
    .unwrap();
    let window = cx.add_window(move |window, cx| {
        let domain = ScriptViewHost::new("keys", cx).unwrap();
        domain.bind_keys(prepared.key_bindings().iter().cloned(),cx).unwrap();
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
fn stop_and_stop_immediate_preserve_distinct_same_phase_semantics(cx:&mut TestAppContext){
    for (phase,first,expected) in [("on","target_stop","ST"),("on","target_immediate","I"),
        ("on_capture","target_stop","ST"),("on_capture","target_immediate","I")] {
        let target=format!(r#"box([text("Target").{phase}("key:x",Fn("{first}"))
            .{phase}("key:X",Fn("child_target")).on_bubble("key:x",Fn("child_bubble"))])
            .tab_stop(false).on_bubble("key:x",Fn("parent_bubble"))"#);
        let window=mount(cx,&target);let mut visual=VisualTestContext::from_window(*window,cx);
        visual.update(|window,cx|window.focus_next(cx));key(&mut visual,"x",Modifiers::none(),None);
        let trace=named_text(cx,window,"trace");eprintln!("{phase}/{first}: {trace}");
        assert_eq!(trace,expected);cleanup(cx,window);
    }
}

#[gpui::test]
fn normalized_entry_aliases_share_declared_payload_across_phases(cx:&mut TestAppContext){
    let target=r#"text("Target").on_capture("key:A",Fn("payload_capture"))
        .on("key:a",Fn("payload_target")).on_key_value("A",Fn("h"),7)"#;
    let window=mount(cx,target);let mut visual=VisualTestContext::from_window(*window,cx);
    visual.update(|window,cx|window.focus_next(cx));key(&mut visual,"a",Modifiers::none(),None);
    assert_eq!(named_text(cx,window,"trace"),"C7T7");assert_eq!(status(cx,window),"7:0:");cleanup(cx,window);
}

#[gpui::test]
fn disabled_ancestor_does_not_run_key_capture_or_bubble(cx:&mut TestAppContext){
    let target=r#"box([text("Target").on("key:x",Fn("child_target"))]).disabled(true).tab_stop(false)
        .on_capture("key:x",Fn("parent_capture")).on_bubble("key:x",Fn("parent_bubble"))"#;
    let window=mount(cx,target);let mut visual=VisualTestContext::from_window(*window,cx);
    visual.update(|window,cx|window.focus_next(cx));key(&mut visual,"x",Modifiers::none(),None);
    assert_eq!(named_text(cx,window,"trace"),"T");cleanup(cx,window);
}

#[gpui::test]
fn matched_host_chord_precedes_raw_node_capture(cx:&mut TestAppContext){
    for (handler,plain,combined) in [("child_capture","C","CA"),("stopped_capture","P","PA")] {
        let target=format!("text(\"Target\").on_capture(\"key:k\",Fn(\"{handler}\"))");
        let window=mount(cx,&target);let mut visual=VisualTestContext::from_window(*window,cx);
        visual.update(|window,cx|window.focus_next(cx));
        key(&mut visual,"k",Modifiers::none(),None);
        assert_eq!(named_text(cx,window,"trace"),plain,"control: unbound raw key reaches node capture");
        key(&mut visual,"k",Modifiers{control:true,..Default::default()},None);
        let trace=named_text(cx,window,"trace");eprintln!("plain k then ctrl-k/{handler}: {trace}");
        assert_eq!(trace,combined,"matched Host action precedes raw key capture, even its stop handler");cleanup(cx,window);
    }
}
