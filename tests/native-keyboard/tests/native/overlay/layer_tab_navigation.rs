//! Tab / Shift+Tab inside `layer(...)` content when the view is embedded in a
//! ScriptViewHost. In host-managed mode the layer content is registered with the
//! window overlay coordinator and painted by the Host's shared layer portal,
//! outside the view root whose key handler turns Tab into `focus_next` and whose
//! action handler runs the view's key-binding actions. The view hands its
//! keyboard handling to the layer content it declares.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME};

const BUTTONS: &str = r#"row([
    button::Button(#{ key: "first", text: "First", on_click: Fn("noop") }),
    button::Button(#{ key: "second", text: "Second", on_click: Fn("noop") }),
]).with_style(style().gap(px(8)))"#;

struct Host {
    host: ScriptViewHost,
    views: Vec<ScriptViewHandle>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            div()
                .size_full()
                .flex()
                .flex_col()
                .children(self.views.iter().map(|view| view.element().unwrap())),
        )
    }
}

fn prepared(body: &str) -> PreparedScriptView {
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/button" as button;
fn state_schema() {{ #{{ fields: #{{ hits: #{{ schema: #{{ type: "integer" }},
    "default": #{{ type: "integer", value: 0 }} }},
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }} }} }} }}
fn noop(ctx, payload) {{ () }}
fn hit(ctx, payload) {{ ctx.set_state("hits", ctx.get_state("hits") + 1); }}
fn set_open(ctx, open) {{ ctx.set_state("open", open); }}
fn init(ctx) {{ ctx.register_action("probe.hit", Fn("hit")); }}
fn view(ctx) {{ column([
    text(`hits=${{ctx.get_state("hits")}}`).accessibility_role("group").test_id("hits"),
    {body}]).with_style(style().width(px(800)).height(px(300))) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .motion_preference(MotionPreference::None)
        .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
            (
                path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                },
            )
        }))
        .prepare()
        .unwrap()
}

/// How the first view is mounted; later views always use `mount`.
#[derive(Clone, Copy)]
enum FirstView {
    Mount,
    /// The window's command owner (`mount_window`).
    MountWindow,
}

/// Mount one view per body, stacked, under one Host that binds `ctrl-k` to
/// the views' `probe.hit` action.
fn mount_views(
    cx: &mut TestAppContext,
    bodies: &[&str],
    first: FirstView,
) -> (VisualTestContext, Vec<ScriptViewHandle>) {
    cx.update(gpui_rhai::install);
    let prepared = bodies.iter().map(|body| prepared(body)).collect::<Vec<_>>();
    let saved = Rc::new(RefCell::new(Vec::new()));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("layers", cx).unwrap();
        host.bind_keys(
            [KeyBindingSpec::new("ctrl-k", ActionId::parse("probe.hit").unwrap(), None).unwrap()],
            cx,
        )
        .unwrap();
        let views = prepared
            .into_iter()
            .enumerate()
            .map(|(index, prepared)| {
                let config = ScriptViewConfig::new(format!("layers{index}"));
                match (index, first) {
                    (0, FirstView::MountWindow) => {
                        prepared.mount_window(config, host.clone(), window, cx)
                    }
                    _ => prepared.mount(config, host.clone(), window, cx),
                }
                .unwrap()
            })
            .collect::<Vec<_>>();
        save.borrow_mut().extend(views.iter().cloned());
        Host { host, views }
    });
    cx.run_until_parked();
    let views = saved.borrow().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    for view in &views {
        assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    }
    (visual, views)
}

fn mount(cx: &mut TestAppContext, body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (visual, mut views) = mount_views(cx, &[body], FirstView::Mount);
    (visual, views.remove(0))
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn click_button(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let bounds = tree
        .find_by_role_and_name("button", name)
        .next()
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no button {name}"))
        .visual;
    #[allow(clippy::cast_possible_truncation)]
    let position = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(visual);
}

fn focused(visual: &mut VisualTestContext) -> Option<FocusHandle> {
    visual.update(|window, cx| window.focused(cx))
}

/// Click each button to learn its focus handle, focus the first, then press Tab
/// and Shift+Tab. Returns (first, second, after Tab, after Shift+Tab).
fn tab_between_buttons(
    cx: &mut TestAppContext,
    body: &str,
) -> (
    FocusHandle,
    FocusHandle,
    Option<FocusHandle>,
    Option<FocusHandle>,
) {
    let (visual, view) = mount(cx, body);
    tab_between_buttons_in(visual, &view)
}

fn tab_between_buttons_in(
    mut visual: VisualTestContext,
    view: &ScriptViewHandle,
) -> (
    FocusHandle,
    FocusHandle,
    Option<FocusHandle>,
    Option<FocusHandle>,
) {
    click_button(&mut visual, view, "Second");
    let second = focused(&mut visual).expect("clicking Second focuses it");
    click_button(&mut visual, view, "First");
    let first = focused(&mut visual).expect("clicking First focuses it");
    assert_ne!(first, second, "each button has its own focus handle");

    visual.simulate_keystrokes("tab");
    settle(&mut visual);
    let after_tab = focused(&mut visual);

    // Shift+Tab from Second (focused by a click, whatever Tab did).
    click_button(&mut visual, view, "Second");
    visual.simulate_keystrokes("shift-tab");
    settle(&mut visual);
    let after_shift_tab = focused(&mut visual);
    (first, second, after_tab, after_shift_tab)
}

/// Control: the same two buttons in the view's own tree.
#[gpui::test]
fn tab_moves_between_buttons_in_the_view(cx: &mut TestAppContext) {
    let (first, second, after_tab, after_shift_tab) = tab_between_buttons(cx, BUTTONS);
    assert_eq!(after_tab, Some(second), "Tab from First focuses Second");
    assert_eq!(
        after_shift_tab,
        Some(first),
        "Shift+Tab from Second focuses First"
    );
}

/// The same buttons inside a Layer: Tab should move focus the same way.
#[gpui::test]
fn tab_moves_between_buttons_in_a_layer(cx: &mut TestAppContext) {
    let body =
        format!(r#"layer({BUTTONS}, #{{ id: "floated", placement: "top_left", inset: 40.0 }})"#);
    let (first, second, after_tab, after_shift_tab) = tab_between_buttons(cx, &body);
    println!(
        "first {:?} second {:?} after tab {:?} after shift-tab {:?}",
        first, second, after_tab, after_shift_tab
    );
    assert_eq!(
        after_tab,
        Some(second),
        "Tab from First (in a layer) focuses Second"
    );
    assert_eq!(
        after_shift_tab,
        Some(first),
        "Shift+Tab from Second (in a layer) focuses First"
    );
}

fn hits(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| node.test_id.as_deref() == Some("hits"))
        .map(|node| node.name.clone())
        .expect("hits node")
}

/// Focus First by a click and press the Host-bound key of a view action.
fn key_binding_from_first(cx: &mut TestAppContext, body: &str) -> String {
    let (mut visual, view) = mount(cx, body);
    click_button(&mut visual, &view, "First");
    assert!(focused(&mut visual).is_some(), "clicking First focuses it");
    visual.simulate_keystrokes("ctrl-k");
    settle(&mut visual);
    hits(&mut visual, &view)
}

/// Control: a view action bound with `bind_keys` runs while focus is in the view.
#[gpui::test]
fn a_view_key_binding_runs_with_focus_in_the_view(cx: &mut TestAppContext) {
    assert_eq!(key_binding_from_first(cx, BUTTONS), "hits=1");
}

/// The same with focus on a button inside a Layer.
#[gpui::test]
fn a_view_key_binding_runs_with_focus_in_a_layer(cx: &mut TestAppContext) {
    let body =
        format!(r#"layer({BUTTONS}, #{{ id: "floated", placement: "top_left", inset: 40.0 }})"#);
    assert_eq!(
        key_binding_from_first(cx, &body),
        "hits=1",
        "ctrl-k with focus on First (in a layer) runs the view action"
    );
}

fn layered(body: &str) -> String {
    format!(r#"layer({body}, #{{ id: "floated", placement: "top_left", inset: 40.0 }})"#)
}

/// A view that is the window's command owner (`mount_window`) hands its
/// keyboard handling to its layers the same way.
#[gpui::test]
fn tab_moves_between_buttons_in_a_layer_of_a_window_owner_view(cx: &mut TestAppContext) {
    let (visual, views) = mount_views(cx, &[&layered(BUTTONS)], FirstView::MountWindow);
    let (first, second, after_tab, after_shift_tab) = tab_between_buttons_in(visual, &views[0]);
    assert_eq!(
        after_tab,
        Some(second),
        "Tab from First (in a layer) focuses Second"
    );
    assert_eq!(
        after_shift_tab,
        Some(first),
        "Shift+Tab from Second (in a layer) focuses First"
    );
}

/// With two views, a bound key pressed in a layer runs the action of the view
/// that declared the layer, not of its sibling.
#[gpui::test]
fn a_view_key_binding_in_a_layer_runs_in_the_view_that_declared_it(cx: &mut TestAppContext) {
    let (mut visual, views) = mount_views(
        cx,
        &[&layered(BUTTONS), &layered(BUTTONS)],
        FirstView::Mount,
    );
    click_button(&mut visual, &views[1], "First");
    assert!(focused(&mut visual).is_some(), "clicking First focuses it");
    visual.simulate_keystrokes("ctrl-k");
    settle(&mut visual);
    assert_eq!(hits(&mut visual, &views[0]), "hits=0");
    assert_eq!(hits(&mut visual, &views[1]), "hits=1");
}

fn popover_open(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| node.test_id.as_deref() == Some("open"))
        .map(|node| node.name.clone())
        .expect("open node")
}

/// Escape still reaches the Host's overlays with focus in a layer.
#[gpui::test]
fn escape_closes_an_open_overlay_with_focus_in_a_layer(cx: &mut TestAppContext) {
    let body = format!(
        r#"text(`open=${{ctx.get_state("open")}}`).accessibility_role("group").test_id("open"),
        overlay(text("anchor"), text("popover body"), #{{ id: "pop", kind: "popover",
            open: ctx.get_state("open"), dismiss_on_outside: false }}).on_open_change(Fn("set_open")),
        {}"#,
        layered(BUTTONS)
    );
    let (mut visual, view) = mount(cx, &body);
    click_button(&mut visual, &view, "First");
    assert!(focused(&mut visual).is_some(), "clicking First focuses it");
    assert_eq!(popover_open(&mut visual, &view), "open=true");
    visual.simulate_keystrokes("escape");
    settle(&mut visual);
    assert_eq!(popover_open(&mut visual, &view), "open=false");
}

/// Copy (`cmd-c`, bound in the view's key context) copies text selected in a
/// layer while focus is on a control in that layer.
#[gpui::test]
fn copy_runs_with_focus_in_a_layer(cx: &mut TestAppContext) {
    let content =
        format!(r#"column([{BUTTONS}, text("Copy this").selectable(true).test_id("copyable")])"#);
    let (mut visual, view) = mount(cx, &layered(&content));
    click_button(&mut visual, &view, "First");
    let first = focused(&mut visual).expect("clicking First focuses it");
    let bounds = {
        let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
        tree.nodes()
            .find(|node| node.test_id.as_deref() == Some("copyable"))
            .and_then(|node| node.geometry)
            .expect("selectable text has bounds")
            .visual
    };
    visual.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()));
    #[allow(clippy::cast_possible_truncation)]
    let (start, end) = (
        point(px((bounds.x + 1.0) as f32), px((bounds.y + 2.0) as f32)),
        point(
            px((bounds.x + bounds.width - 1.0) as f32),
            px((bounds.y + bounds.height - 1.0) as f32),
        ),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    // Selecting moves focus out of the layer; Tab brings it back to First
    // and keeps the selection.
    for _ in 0..3 {
        if focused(&mut visual).as_ref() == Some(&first) {
            break;
        }
        visual.simulate_keystrokes("tab");
        settle(&mut visual);
    }
    assert_eq!(focused(&mut visual), Some(first), "Tab reaches First");
    visual.simulate_keystrokes("cmd-c");
    settle(&mut visual);
    assert_eq!(
        visual
            .read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some("Copy this")
    );
}
