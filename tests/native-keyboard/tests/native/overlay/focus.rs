//! Overlay focus: nested overlays render inside their parent's deferred subtree, initial
//! focus lands on every open, the default panel focus keeps the keyboard on the panel, and a
//! modal Dialog takes focus back from an embedding Host.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{
    Host, KeyboardHost, Mount, ReplaceOnce, click_palette_trigger, palette_texts,
};
use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div,
};
use gpui_rhai::{
    EventPropagation, NodeEventDispatcher, OverlayDismissPolicy, OverlayId, OverlayKind,
    OverlayNodeSpec, OverlayPlacement, PrimitiveRegistry, ScriptViewConfig, ScriptViewHandle,
    ScriptViewHost, UiNode,
};

#[gpui::test]
fn nested_overlay_renders_inside_parent_deferred_subtree(cx: &mut TestAppContext) {
    let child = UiNode::overlay(
        UiNode::text("Nested popover"),
        UiNode::text("Nested content"),
        OverlayNodeSpec {
            initial_focus: gpui_rhai::OverlayInitialFocus::Panel,
            id: OverlayId::new("child-popover"),
            owner: None,
            parent: Some(OverlayId::new("parent-dialog")),
            kind: OverlayKind::Popover,
            placement: OverlayPlacement::Right,
            align: gpui_rhai::OverlayAlign::Center,
            anchor: None,
            open: true,
            gap: 4.0,
            modal: false,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
            activate_on_trigger: true,
            width_policy: gpui_rhai::OverlayWidthPolicy::Content,
        },
    )
    .with_key("child-popover");
    let root = UiNode::overlay(
        UiNode::text("Open dialog"),
        child,
        OverlayNodeSpec {
            initial_focus: gpui_rhai::OverlayInitialFocus::Panel,
            id: OverlayId::new("parent-dialog"),
            owner: None,
            parent: None,
            kind: OverlayKind::Dialog,
            placement: OverlayPlacement::Bottom,
            align: gpui_rhai::OverlayAlign::Center,
            anchor: None,
            open: true,
            gap: 0.0,
            modal: true,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
            activate_on_trigger: true,
            width_policy: gpui_rhai::OverlayWidthPolicy::Content,
        },
    )
    .with_key("parent-dialog");
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher: NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled),
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });

    cx.run_until_parked();
    assert!(cx.windows().contains(&*window));
}

struct ExternalFocusEmbeddedHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
    external_focus: FocusHandle,
}

impl Render for ExternalFocusEmbeddedHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.external_focus)
            .child(self.host.container(self.view.element().unwrap()))
    }
}

// PR #11 review: overlays keep their semantic subtree (and thus primitive
// identity) across close/reopen, so mount-time `autofocus` fires only on the
// first open. The overlay-level `initial_focus: "first"` contract must land
// focus in the content input on EVERY closed -> open cycle.
const CONTROLLED_PALETTE_SCRIPT: &str = r#"
import "components/input" as input;

fn state_schema() {
    #{ fields: #{
        open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        value: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    } }
}

fn set_open(ctx, open) { ctx.set_state("open", open) }
fn value_changed(ctx, value) { ctx.set_state("value", value) }

fn view(ctx) {
    column([
        text(`typed:${ctx.get_state("value")}`),
        overlay(
            text("open palette")
                .with_style(style().height(px(28)))
                .accessibility_role("button")
                .accessibility_label("open palette"),
            column([
                input::Input(#{ key: "filter", label: "Filter", value: ctx.get_state("value"),
                    placeholder: "filter", on_change: Fn("value_changed") })
            ]).with_style(style().width(px(320)).padding(px(8))),
            #{ id: "palette", kind: "dialog", placement: "center",
               open: ctx.get_state("open"), modal: true__INITIAL_FOCUS__ }
        ).with_key("palette").on_open_change(Fn("set_open"))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).gap(px(4)))
}
"#;

fn mount_controlled_palette(
    cx: &mut TestAppContext,
    initial_focus: Option<&str>,
) -> (gpui::WindowHandle<Host>, ScriptViewHost, ScriptViewHandle) {
    let script = CONTROLLED_PALETTE_SCRIPT.replace_once(
        "__INITIAL_FOCUS__",
        &initial_focus
            .map(|policy| format!(", initial_focus: \"{policy}\""))
            .unwrap_or_default(),
    );
    let (window, view) = Mount::new(script)
        .registry("components/input")
        .bundled_assets()
        .ids("palette-window", "palette-view")
        .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let host = window.read_with(cx, |root, _| root.host.clone()).unwrap();
    (window, host, view)
}

#[gpui::test]
fn overlay_initial_focus_first_reaches_input_on_every_open(cx: &mut TestAppContext) {
    let (window, _host, view) = mount_controlled_palette(cx, Some("first"));
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    // Cycle 1: open -> type without any click or tab.
    click_palette_trigger(&mut visual, &view);
    visual.simulate_input("hi");
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    assert!(
        texts.contains(&"typed:hi".to_owned()),
        "first open must focus the input: {texts:?}"
    );

    // Close via Escape: the overlay dismisses and reports open_change(false),
    // the controlled state closes it, and the subtree identity is retained.
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();

    // Cycle 2: reopen -> type again. This is the exact boundary from the
    // review: entity reuse used to leave focus on the overlay panel.
    click_palette_trigger(&mut visual, &view);
    visual.simulate_input("x");
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    assert!(
        texts.contains(&"typed:hix".to_owned()),
        "reopen must focus the input again: {texts:?}"
    );
}

#[gpui::test]
fn overlay_default_panel_focus_keeps_keyboard_on_panel(cx: &mut TestAppContext) {
    // Control experiment: without `initial_focus: "first"` the panel takes
    // focus and typing never reaches the input. If this ever starts passing
    // text through, the default has changed and the test above proves nothing.
    let (window, _host, view) = mount_controlled_palette(cx, None);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    click_palette_trigger(&mut visual, &view);
    visual.simulate_input("hi");
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    assert!(
        texts.contains(&"typed:".to_owned()),
        "panel default must not route typing into the input: {texts:?}"
    );
}

#[gpui::test]
fn modal_dialog_reclaims_focus_stolen_by_an_embedding_host(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = Mount::new(
        r#"
                    import "components/dialog" as dialog;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: true } }
                    } } }
                    fn changed(ctx, open) { ctx.set_state("open", open); }
                    fn view(ctx) {
                        column([
                            text(`PROBE open=${ctx.get_state("open")}`),
                            dialog::Dialog(#{
                                key: "probe-dialog",
                                open: ctx.get_state("open"),
                                title: "Probe dialog",
                                content: text("Press Escape"),
                                on_open_change: Fn("changed")
                            })
                        ])
                    }
        "#,
    )
    .registry("components/dialog")
    .bundled_assets()
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("dialog-focus-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("dialog-focus-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        let external_focus = cx.focus_handle();
        *captured_for_window.borrow_mut() = Some((view.clone(), external_focus.clone()));
        ExternalFocusEmbeddedHost {
            host,
            view,
            external_focus,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (view, external_focus) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| external_focus.focus(window, cx));
    // The Host focus request dirties the window. During that frame, the modal
    // invariant sees that focus is outside its panel and reclaims it.
    visual.run_until_parked();
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();

    let texts = palette_texts(&mut visual, &view);
    assert!(
        texts.contains(&"PROBE open=false".to_owned()),
        "Escape must dismiss after an embedding host moves focus: {texts:?}"
    );
}
