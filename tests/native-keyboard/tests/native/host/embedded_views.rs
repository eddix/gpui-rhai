//! Embedded script views under a ScriptViewHost: suspend and resume, element bounds and
//! event keys, node props across re-renders, click context bounds, several views and Hosts
//! sharing one window, and an idempotent runtime install.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{
    ReplaceOnce, SingleEmbeddedHost, node_texts, official_icon_assets, wait_for_view_text,
};
use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};
use gpui_rhai::{
    ActionId, EmbeddedScriptSource, EmbeddedScriptView, KeyBindingSpec, ModuleId, ScriptViewConfig,
    ScriptViewHandle, ScriptViewHost, UiNode,
};

struct SuspensibleEmbeddedHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for SuspensibleEmbeddedHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let content = match self.view.state() {
            gpui_rhai::ScriptViewState::Active => self.view.element().unwrap(),
            gpui_rhai::ScriptViewState::Suspended | gpui_rhai::ScriptViewState::Disposed => {
                div().into_any_element()
            }
        };
        self.host.container(content)
    }
}

#[gpui::test]
fn embedded_view_suspend_resume_retains_state_and_rejects_new_elements(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                fn state_schema() { #{ fields: #{ phase: #{ schema: #{ type: "string" },
                    "default": #{ type: "string", value: "active" } } } } }
                fn suspend(ctx) { ctx.set_state("phase", "suspended"); }
                fn resume(ctx, elapsed_ms) { ctx.set_state("phase", "resumed"); }
                fn view(ctx) { text(ctx.get_state("phase")) }
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("suspend-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("suspend-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SuspensibleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    assert!(
        visual
            .update(|window, cx| view.suspend(window, cx))
            .unwrap()
    );
    assert_eq!(view.state(), gpui_rhai::ScriptViewState::Suspended);
    assert!(matches!(
        view.element(),
        Err(gpui_rhai::ScriptViewError::SuspendedView(_))
    ));
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    assert!(
        matches!(root.kind(), gpui_rhai::UiNodeKind::Text { text } if text == "active"),
        "suspension retains the last-good tree instead of rendering hook mutations"
    );

    assert!(visual.update(|_, cx| view.resume(cx)).unwrap());
    assert_eq!(view.state(), gpui_rhai::ScriptViewState::Active);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    assert!(matches!(root.kind(), gpui_rhai::UiNodeKind::Text { text } if text == "resumed"));
}

#[gpui::test]
fn element_bounds_self_heals_after_first_prepaint_and_resolves_event_keys(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                define_component(#{
                    metadata: #{ id: "tests/geometry_probe", "export": "GeometryProbe",
                        version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{} },
                    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                        state: #{ fields: #{ event_measured: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: false } } } },
                        events: #{}, slots: #{}, parts: ["root"] },
                    render: Fn("render_GeometryProbe"),
                });
                fn clicked(ctx, payload) {
                    let bounds = ctx.element_bounds("target");
                    ctx.set_state("event_measured", bounds.visual.width > 0);
                }
                fn render_GeometryProbe(ctx, props) {
                    let target = element_ref("target");
                    let bounds = ctx.element_bounds(target);
                    let label = if ctx.get_state("event_measured") { "event-measured" }
                        else if bounds == () { "pending" } else { "render-measured" };
                    text(label).with_key("target").with_ref(target)
                        .test_id("geometry-target").on_click(Fn("clicked"))
                }
                fn view(ctx) { render_component("tests/geometry_probe", #{ key: "probe" }) }
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("geometry-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("geometry-view"),
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
    wait_for_view_text(
        &mut visual,
        &view,
        "render-measured",
        "first-prepaint geometry dependency did not self-heal",
    );
    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Dispatch {
                    locator: gpui_rhai::AutomationLocator::TestId {
                        id: "geometry-target".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    wait_for_view_text(
        &mut visual,
        &view,
        "event-measured",
        "event callback could not resolve component-local ref key geometry",
    );
}

#[gpui::test]
fn node_prop_component_keeps_latest_ui_when_receiver_rerenders(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let stateful = ModuleId::parse("components/slot_stateful").unwrap();
    let receiver = ModuleId::parse("components/slot_receiver").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/slot_stateful" as stateful;
                    import "components/slot_receiver" as receiver;
                    fn view(ctx) {
                        receiver::SlotReceiver(#{ key: "receiver",
                            content: stateful::SlotStateful(#{ key: "stateful" }) })
                    }
                "#
                .to_owned(),
            ),
            (
                stateful,
                r#"
                    define_component(#{
                        metadata: #{ id: "components/slot_stateful", "export": "SlotStateful",
                            version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                            dependencies: [], capabilities: #{} },
                        schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                            state: #{ fields: #{ value: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "initial-slot" } } } },
                            events: #{}, slots: #{}, parts: ["root"] },
                        render: Fn("render_SlotStateful"),
                    });
                    fn SlotStateful(props) { render_component("components/slot_stateful", props) }
                    fn update_slot(ctx, payload) { ctx.set_state("value", "updated-slot"); }
                    fn render_SlotStateful(ctx, props) {
                        text(ctx.get_state("value")).test_id("slot-content")
                            .on_click(Fn("update_slot"))
                    }
                "#
                .to_owned(),
            ),
            (
                receiver,
                r#"
                    define_component(#{
                        metadata: #{ id: "components/slot_receiver", "export": "SlotReceiver",
                            version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                            dependencies: [], capabilities: #{} },
                        schema: #{ props: #{
                                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                                content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                            }, state: #{ fields: #{ revision: #{ schema: #{ type: "integer" },
                                "default": #{ type: "integer", value: 0 } } } },
                            events: #{}, slots: #{}, parts: ["root", "content"] },
                        render: Fn("render_SlotReceiver"),
                    });
                    fn SlotReceiver(props) { render_component("components/slot_receiver", props) }
                    fn rerender_receiver(ctx, payload) {
                        ctx.set_state("revision", ctx.get_state("revision") + 1);
                    }
                    fn render_SlotReceiver(ctx, props) {
                        column([
                            props.content.with_style(ctx.component_style("content", style().opacity(0.91))),
                            text(`rerender:${ctx.get_state("revision")}`)
                                .test_id("receiver-control").on_click(Fn("rerender_receiver")),
                        ])
                    }
                "#
                .to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("node-prop-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("node-prop-view"),
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
    for test_id in ["slot-content", "receiver-control"] {
        visual
            .update(|window, cx| {
                view.automate(
                    gpui_rhai::AutomationCommand::Dispatch {
                        locator: gpui_rhai::AutomationLocator::TestId {
                            id: test_id.to_owned(),
                        },
                        event: "click".to_owned(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        visual.run_until_parked();
    }
    wait_for_view_text(
        &mut visual,
        &view,
        "updated-slot",
        "receiver replay replaced the latest stateful node-prop UI",
    );
    wait_for_view_text(
        &mut visual,
        &view,
        "rerender:1",
        "receiver callback did not commit its own state",
    );
}

#[gpui::test]
fn click_context_exposes_untracked_event_target_visual_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                fn state_schema() { #{ fields: #{
                    summary: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "waiting" } }
                } } }
                fn capture_bounds(ctx, payload) {
                    let bounds = ctx.event_target_bounds();
                    ctx.set_state("summary",
                        `${bounds.x},${bounds.y},${bounds.width},${bounds.height}`);
                }
                fn view(ctx) {
                    text(ctx.get_state("summary"))
                        .with_key("geometry-target")
                        .test_id("geometry-target")
                        .accessibility_role("button")
                        .accessibility_label("Geometry target")
                        .with_style(style().width(px(180)).height(px(72)).padding(px(8)))
                        .on_click(Fn("capture_bounds"))
                }
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("event-target-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("event-target-view"),
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
    let query = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: gpui_rhai::AutomationLocator::TestId {
                        id: "geometry-target".to_owned(),
                    },
                },
                window,
                cx,
            )
        })
        .unwrap();
    let gpui_rhai::AutomationResult::Node { node } = query else {
        panic!("geometry target query must return one node");
    };
    let expected = node.bounds.expect("target must have committed geometry");
    visual.simulate_click(
        point(
            px((expected.x + expected.width / 2.0) as f32),
            px((expected.y + expected.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();

    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let gpui_rhai::UiNodeKind::Text { text } = root.kind() else {
        panic!("geometry target root must remain text");
    };
    let actual = text
        .split(',')
        .map(|value| value.parse::<f64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(actual.len(), 4);
    for (actual, expected) in
        actual
            .into_iter()
            .zip([expected.x, expected.y, expected.width, expected.height])
    {
        assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
    }
}

const EMBEDDED_VIEW_SCRIPT: &str = r#"
import "components/toast" as toast;
fn state_schema() {
    #{ fields: #{
        count: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } },
        open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: __OPEN__ } },
    } }
}
fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
fn set_open(ctx, open) { ctx.set_state("open", open); }
fn dismissed(ctx, id) { () }
fn view(ctx) {
    column([
        toast::Toast(#{
            key: "toasts",
            items: [#{ id: "shared-toast", title: `Toast ${ctx.view_id()}`,
                duration_ms: 60000 }],
            on_dismiss: Fn("dismissed")
        }),
        text(`View: ${ctx.view_id()}`),
        text(`Responsive: ${ctx.viewport_class()}`),
        text(`Count: ${ctx.get_state("count")}`),
        text("Increment").with_style(
            style().height(px(32)).padding(px(6)).background(theme_color("accent"))
        ).on_click(Fn("increment")),
        overlay(
            text("Shared overlay trigger").with_style(style().height(px(28))),
            row([text("A 280 point overlay")]).with_style(
                style().width(px(280)).height(px(64)).padding(px(8))
                    .background(theme_color("surface_raised"))
            ),
            #{ id: "shared-overlay", kind: "combobox", open: ctx.get_state("open"),
                placement: "bottom", dismiss_on_escape: true, dismiss_on_outside: true }
        ).with_key("shared-overlay").on_open_change(Fn("set_open"))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).gap(px(4)))
}
"#;

fn prepared_embedded_test_view(open: bool) -> gpui_rhai::PreparedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                EMBEDDED_VIEW_SCRIPT.replace_once("__OPEN__", if open { "true" } else { "false" }),
            ),
            (
                ModuleId::parse("components/toast").unwrap(),
                include_str!("../../../../../registry/components/toast.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap()
}

struct EmbeddedIntegrationHost {
    host: ScriptViewHost,
    first: ScriptViewHandle,
    second: ScriptViewHandle,
    third: ScriptViewHandle,
}

impl EmbeddedIntegrationHost {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_second_open(window, cx, false)
    }

    fn new_with_second_open(
        window: &mut Window,
        cx: &mut Context<Self>,
        second_open: bool,
    ) -> Self {
        let host = ScriptViewHost::new("integration-window", cx).unwrap();
        let first = prepared_embedded_test_view(true)
            .mount(ScriptViewConfig::new("first"), host.clone(), window, cx)
            .unwrap();
        let second = prepared_embedded_test_view(second_open)
            .mount(ScriptViewConfig::new("second"), host.clone(), window, cx)
            .unwrap();
        let third = prepared_embedded_test_view(false)
            .mount(ScriptViewConfig::new("third"), host.clone(), window, cx)
            .unwrap();
        Self {
            host,
            first,
            second,
            third,
        }
    }

    fn dispose_and_remount_third(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.third.dispose(cx).unwrap();
        self.third = prepared_embedded_test_view(false)
            .mount(
                ScriptViewConfig::new("third"),
                self.host.clone(),
                window,
                cx,
            )
            .unwrap();
        cx.notify();
    }
}

impl Render for EmbeddedIntegrationHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            div()
                .size_full()
                .flex()
                .gap(px(12.0))
                .items_start()
                .child(
                    div()
                        .w(px(200.0))
                        .h(px(240.0))
                        .child(self.first.element().unwrap()),
                )
                .child(
                    div()
                        .w(px(240.0))
                        .h(px(240.0))
                        .child(self.second.element().unwrap()),
                )
                .child(
                    div()
                        .w(px(280.0))
                        .h(px(240.0))
                        .child(self.third.element().unwrap()),
                ),
        )
    }
}

fn overlay_open(node: &UiNode) -> Option<bool> {
    match node.kind() {
        gpui_rhai::UiNodeKind::Overlay { spec, .. } => Some(spec.open),
        gpui_rhai::UiNodeKind::Box { children } | gpui_rhai::UiNodeKind::Fragment { children } => {
            children.iter().find_map(overlay_open)
        }
        gpui_rhai::UiNodeKind::Layer { content, .. } => overlay_open(content),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            overlay_open(child).or_else(|| overlay_open(fallback))
        }
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => {
            spec.realized.values().find_map(overlay_open)
        }
        _ => None,
    }
}

#[gpui::test]
fn multiple_embedded_views_share_host_mechanics_but_isolate_runtime_state(cx: &mut TestAppContext) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = EmbeddedIntegrationHost::new(window, cx);
        *captured_for_window.borrow_mut() = Some((
            host.host.clone(),
            host.first.clone(),
            host.second.clone(),
            host.third.clone(),
        ));
        host
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (host, first, second, third) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    let first_root = visual.update(|_, cx| first.root(cx).unwrap().unwrap());
    let mut first_text = Vec::new();
    node_texts(&first_root, &mut first_text);
    assert!(first_text.contains(&"View: first".to_owned()));
    assert!(first_text.contains(&"Responsive: compact".to_owned()));

    let first_placement = host
        .overlay_placement("first", "shared-overlay")
        .unwrap()
        .unwrap();
    assert!(first_placement.bounds.width > 200.0);
    let toast_count = [&first, &second, &third]
        .into_iter()
        .map(|view| {
            visual.update(|_, cx| {
                view.accessibility_snapshot(cx)
                    .unwrap()
                    .nodes()
                    .filter(|node| node.role == "status")
                    .count()
            })
        })
        .sum::<usize>();
    assert_eq!(toast_count, 3);

    let increment = visual
        .debug_bounds("window:integration-window/view:third/root/4")
        .expect("third increment debug bounds");
    let increment_center = point(
        increment.origin.x + increment.size.width / 2.0,
        increment.origin.y + increment.size.height / 2.0,
    );
    visual.simulate_click(increment_center, Modifiers::default());
    visual.run_until_parked();
    let third_root = visual.update(|_, cx| third.root(cx).unwrap().unwrap());
    let mut third_text = Vec::new();
    node_texts(&third_root, &mut third_text);
    assert!(
        third_text.contains(&"Count: 1".to_owned()),
        "{third_text:?}"
    );
    assert_eq!(overlay_open(&third_root), Some(false));

    let first_root = visual.update(|_, cx| first.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&first_root), Some(false));

    let second_root = visual.update(|_, cx| second.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&second_root), Some(false));

    window
        .update(&mut visual, |root, window, cx| {
            root.dispose_and_remount_third(window, cx)
        })
        .unwrap();
    assert!(matches!(
        third.element(),
        Err(gpui_rhai::ScriptViewError::DisposedView(id)) if id == "third"
    ));
    visual.run_until_parked();
}

#[gpui::test]
fn duplicate_local_overlay_ids_are_namespaced_per_embedded_view(cx: &mut TestAppContext) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let _window = cx.add_window(move |window, cx| {
        let root = EmbeddedIntegrationHost::new_with_second_open(window, cx, true);
        *captured_for_window.borrow_mut() = Some(root.host.clone());
        root
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let host = captured.borrow().as_ref().unwrap().clone();
    let first = host
        .overlay_placement("first", "shared-overlay")
        .unwrap()
        .unwrap();
    let second = host
        .overlay_placement("second", "shared-overlay")
        .unwrap()
        .unwrap();
    assert!(first.bounds.width > 200.0);
    assert!(second.bounds.width > 200.0);
    assert_ne!(first.bounds.x, second.bounds.x);
}

#[gpui::test]
fn script_runtime_install_is_idempotent_and_key_conflicts_are_explicit(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_rhai::install(cx);
        gpui_rhai::install(cx);
        let host = ScriptViewHost::new("keys-window", cx).unwrap();
        let save =
            KeyBindingSpec::new("cmd-s", ActionId::parse("document.save").unwrap(), None).unwrap();
        host.bind_keys([save.clone()], cx).unwrap();
        host.bind_keys([save], cx).unwrap();
        let conflicting =
            KeyBindingSpec::new("cmd-s", ActionId::parse("document.sync").unwrap(), None).unwrap();
        assert!(matches!(
            host.bind_keys([conflicting], cx),
            Err(gpui_rhai::ScriptViewError::KeyBindingConflict { .. })
        ));
    });
}

struct DualHostRoot {
    left_host: ScriptViewHost,
    left: ScriptViewHandle,
    right_host: ScriptViewHost,
    right: ScriptViewHandle,
}

impl DualHostRoot {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let left_host = ScriptViewHost::new("left-domain", cx).unwrap();
        let left = prepared_embedded_test_view(true)
            .mount(
                ScriptViewConfig::new("left-view"),
                left_host.clone(),
                window,
                cx,
            )
            .unwrap();
        let right_host = ScriptViewHost::new("right-domain", cx).unwrap();
        let right = prepared_embedded_test_view(false)
            .mount(
                ScriptViewConfig::new("right-view"),
                right_host.clone(),
                window,
                cx,
            )
            .unwrap();
        Self {
            left_host,
            left,
            right_host,
            right,
        }
    }
}

impl Render for DualHostRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .gap(px(20.0))
            .child(
                div().w(px(320.0)).h(px(240.0)).child(
                    self.left_host
                        .container(self.left.element().expect("left live")),
                ),
            )
            .child(
                div().w(px(320.0)).h(px(240.0)).child(
                    self.right_host
                        .container(self.right.element().expect("right live")),
                ),
            )
    }
}

#[gpui::test]
fn separate_hosts_in_one_window_keep_overlay_domains_isolated(cx: &mut TestAppContext) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let root = DualHostRoot::new(window, cx);
        *captured_for_window.borrow_mut() = Some((root.left.clone(), root.right.clone()));
        root
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (left, right) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let increment = visual
        .debug_bounds("window:right-domain/view:right-view/root/4")
        .expect("right increment bounds");
    visual.simulate_click(
        point(
            increment.origin.x + increment.size.width / 2.0,
            increment.origin.y + increment.size.height / 2.0,
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();

    let left_root = visual.update(|_, cx| left.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&left_root), Some(true));
    let right_root = visual.update(|_, cx| right.root(cx).unwrap().unwrap());
    let mut right_text = Vec::new();
    node_texts(&right_root, &mut right_text);
    assert!(right_text.contains(&"Count: 1".to_owned()));
}
