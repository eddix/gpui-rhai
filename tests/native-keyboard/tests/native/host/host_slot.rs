//! HostSlot: native content a Host mounts inside a script view keeps its own events,
//! text input and ScriptViewHost.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, node_texts, wait_for_view_text};
use gpui::{
    InteractiveElement, IntoElement, Modifiers, ParentElement, StatefulInteractiveElement, Styled,
    TestAppContext, VisualTestContext, div, point, px,
};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, HostSlotRegistry, ModuleId, ScriptViewConfig,
    ScriptViewHost,
};

#[gpui::test]
fn host_slot_renders_native_content_without_leaking_events_to_rhai(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let native_clicks = Rc::new(Cell::new(0_u32));
    let native_clicks_factory = Rc::clone(&native_clicks);
    let slots = HostSlotRegistry::new()
        .with_slot("content", move |_, _| {
            let native_clicks = Rc::clone(&native_clicks_factory);
            Ok(div()
                .id("native-host-slot-content")
                .size_full()
                .on_click(move |_, _, _| native_clicks.set(native_clicks.get() + 1))
                .child("Native host content")
                .into_any_element())
        })
        .unwrap();
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                fn state_schema() { #{ fields: #{ clicks: #{ schema: #{ type: "integer" },
                    "default": #{ type: "integer", value: 0 } } } } }
                fn clicked(ctx, payload) { ctx.set_state("clicks", ctx.get_state("clicks") + 1); }
                fn host_slot(name) { gpui_rhai::HostSlot(#{ key: name, name: name }) }
                fn view(ctx) {
                    column([
                        text(`script:${ctx.get_state("clicks")}`),
                        host_slot("content")
                            .accessibility_role("group").accessibility_label("Host content")
                            .with_style(style().width(px(220)).height(px(120)))
                    ]).on_click(Fn("clicked"))
                }
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .extension(slots)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("host-slot-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("host-slot-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let result = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: gpui_rhai::AutomationLocator::RoleName {
                        role: "group".to_owned(),
                        name: "Host content".to_owned(),
                    },
                },
                window,
                cx,
            )
        })
        .unwrap();
    let gpui_rhai::AutomationResult::Node { node } = result else {
        panic!("expected host slot node");
    };
    let bounds = node.bounds.expect("host slot has committed bounds");
    visual.simulate_click(
        point(
            px((bounds.x + bounds.width / 2.0) as f32),
            px((bounds.y + bounds.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();

    assert_eq!(native_clicks.get(), 1);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(texts.contains(&"script:0".to_owned()), "{texts:?}");
}

#[gpui::test]
fn host_slot_does_not_consume_text_input_before_ime_delivery(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let resident_entry = ModuleId::parse("resident").unwrap();
    let input_module = ModuleId::parse("components/input").unwrap();
    let resident = EmbeddedScriptView::new(
        resident_entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                resident_entry,
                r#"
                    import "components/input" as input;
                    fn state_schema() { #{ fields: #{ value: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "" } } } } }
                    fn changed(ctx, value) { ctx.set_state("value", value); }
                    fn view(ctx) {
                        column([
                            input::Input(#{ key: "resident-input", label: "Resident input",
                                value: ctx.get_state("value"), autofocus: true,
                                on_change: Fn("changed") }),
                            text(`resident:${ctx.get_state("value")}`)
                        ])
                    }
                "#
                .to_owned(),
            ),
            (
                input_module,
                include_str!("../../../../../registry/components/input.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let shell_entry = ModuleId::parse("shell").unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let shell_host = ScriptViewHost::new("ime-shell-host", cx).unwrap();
        let resident_host = ScriptViewHost::new("ime-resident-host", cx).unwrap();
        let resident = resident
            .mount(
                ScriptViewConfig::new("ime-resident-view"),
                resident_host,
                window,
                cx,
            )
            .unwrap();
        let slots = HostSlotRegistry::new()
            .with_script_view("content", resident.clone())
            .unwrap();
        let shell = EmbeddedScriptView::new(
            shell_entry.clone(),
            EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
                shell_entry,
                r#"fn view(ctx) {
                    gpui_rhai::HostSlot(#{ key: "slot", name: "content" })
                        .with_style(style().width(px(320)).height(px(100)))
                }"#
                .to_owned(),
            )])),
            include_str!("../../../../../registry/themes/default_dark.rhai"),
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .extension(slots)
        .prepare()
        .unwrap()
        .mount(
            ScriptViewConfig::new("ime-shell-view"),
            shell_host.clone(),
            window,
            cx,
        )
        .unwrap();
        *captured_for_window.borrow_mut() = Some(resident);
        SingleEmbeddedHost {
            host: shell_host,
            view: shell,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    cx.simulate_input(*window, "中文");
    let resident = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    wait_for_view_text(
        &mut visual,
        &resident,
        "resident:中文",
        "HostSlot must not mark printable keys handled before IME insertion",
    );
}

#[gpui::test]
fn host_slot_preserves_an_independent_script_view_host(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let resident_entry = ModuleId::parse("resident").unwrap();
    let resident = EmbeddedScriptView::new(
        resident_entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            resident_entry,
            r#"fn view(ctx) {
                text("Resident").accessibility_role("status").accessibility_label("Resident")
            }"#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let shell_entry = ModuleId::parse("shell").unwrap();
    let shell_source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        shell_entry.clone(),
        r#"fn view(ctx) {
            gpui_rhai::HostSlot(#{ key: "slot", name: "content" })
                .with_style(style().width(px(300)).height(px(160)))
        }"#
        .to_owned(),
    )]));
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let shell_host = ScriptViewHost::new("shell-host", cx).unwrap();
        let resident_host = ScriptViewHost::new("resident-host", cx).unwrap();
        let resident = resident
            .mount(
                ScriptViewConfig::new("resident-view"),
                resident_host,
                window,
                cx,
            )
            .unwrap();
        let slots = HostSlotRegistry::new()
            .with_script_view("content", resident.clone())
            .unwrap();
        let shell = EmbeddedScriptView::new(
            shell_entry,
            shell_source,
            include_str!("../../../../../registry/themes/default_dark.rhai"),
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .extension(slots)
        .prepare()
        .unwrap()
        .mount(
            ScriptViewConfig::new("shell-view"),
            shell_host.clone(),
            window,
            cx,
        )
        .unwrap();
        *captured_for_window.borrow_mut() = Some(resident);
        SingleEmbeddedHost {
            host: shell_host,
            view: shell,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let resident = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..4 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        visual.run_until_parked();
    }
    assert_eq!(
        visual.update(|_, cx| resident.last_error(cx).unwrap()),
        None
    );
    assert_eq!(resident.state(), gpui_rhai::ScriptViewState::Active);
    let snapshot = visual.update(|_, cx| resident.accessibility_snapshot(cx).unwrap());
    let node = snapshot
        .find_by_role_and_name("status", "Resident")
        .next()
        .expect("resident semantics remain owned by the resident view");
    assert!(node.geometry.is_some(), "resident view was not presented");
}
