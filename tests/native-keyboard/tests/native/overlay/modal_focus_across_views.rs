//! Two modal Dialogs open at once in two ScriptViews under one ScriptViewHost
//! (or in one view). Only the topmost open modal takes focus back. When each
//! ScriptOverlayElement kept the "a modal holds focus" invariant on its own,
//! taking focus back and asking for another frame whenever its panel did not
//! contain focus, neither of two open modals could hold focus at the end of
//! its own layout and the window redrew without input.
//!
//! The Host stops rendering the views after `LIMIT` renders so a runaway loop
//! ends (and fails the assertion) instead of hanging the test.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};
use gpui_rhai::*;

use crate::support::{Mount, Step, last_error, nodes};

const LIMIT: usize = 200;

fn dialog(key: &str, open: bool) -> String {
    format!(
        r#"dialog::Dialog(#{{ key: "{key}", open: {open}, title: "{key}",
            content: text("{key} dialog"), on_open_change: Fn("noop"),
            actions: [button::Button(#{{ key: "ok-{key}", text: "OK {key}", on_click: Fn("noop") }})] }})"#
    )
}

fn prepared(body: &str) -> PreparedScriptView {
    let main = format!(
        r#"import "components/button" as button;
import "components/dialog" as dialog;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([text("view"), {body}]).with_style(style().width(px(400)).height(px(600))) }}"#
    );
    Mount::components(main)
        .motion(MotionPreference::None)
        .prepare()
        .unwrap()
}

struct Host {
    host: ScriptViewHost,
    views: Vec<ScriptViewHandle>,
    renders: Rc<Cell<usize>>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let renders = self.renders.get() + 1;
        self.renders.set(renders);
        if renders > LIMIT {
            // Break a runaway redraw loop so the test can report it.
            return self.host.container(div());
        }
        self.host.container(
            div()
                .size_full()
                .flex()
                .children(self.views.iter().map(|view| {
                    div()
                        .w(px(400.0))
                        .h(px(600.0))
                        .child(view.element().unwrap())
                })),
        )
    }
}

/// Mount one view per body under one Host; returns the number of Host renders
/// it took for the window to go idle.
fn renders_until_idle(cx: &mut TestAppContext, bodies: &[&str]) -> usize {
    cx.update(gpui_rhai::install);
    let prepared = bodies.iter().map(|body| prepared(body)).collect::<Vec<_>>();
    let renders = Rc::new(Cell::new(0));
    let counter = renders.clone();
    let saved = Rc::new(RefCell::new(Vec::new()));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("modals", cx).unwrap();
        let views = prepared
            .into_iter()
            .enumerate()
            .map(|(index, prepared)| {
                prepared
                    .mount(
                        ScriptViewConfig::new(format!("view{index}")),
                        host.clone(),
                        window,
                        cx,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        save.borrow_mut().extend(views.iter().cloned());
        Host {
            host,
            views,
            renders: counter,
        }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    crate::support::settle(&mut visual, Step::Frame, 2);
    for view in saved.borrow().iter() {
        assert_eq!(last_error(&mut visual, view), None);
    }
    let settled = renders.get();
    // Idle: a few more turns of the executor must not render again.
    crate::support::settle(&mut visual, Step::Frame, 5);
    let after = renders.get();
    println!("renders: {settled} to settle, {after} after five more idle turns");
    after
}

/// Control: one open modal (in the second view) settles in a few frames.
#[gpui::test]
fn one_open_modal_settles(cx: &mut TestAppContext) {
    let renders = renders_until_idle(cx, &[&dialog("left", false), &dialog("right", true)]);
    assert!(renders < 20, "one open modal: {renders} renders");
}

/// Two open modals, one per view, under one Host.
#[gpui::test]
fn two_open_modals_in_two_views_settle(cx: &mut TestAppContext) {
    let renders = renders_until_idle(cx, &[&dialog("left", true), &dialog("right", true)]);
    assert!(
        renders < 20,
        "two open modals in two views keep the window redrawing: {renders} renders without input \
         (the Host stopped rendering them at {LIMIT})"
    );
}

/// The same with both modals in one view, for scope.
#[gpui::test]
fn two_open_modals_in_one_view_settle(cx: &mut TestAppContext) {
    let both = format!("{}, {}", dialog("left", true), dialog("right", true));
    let renders = renders_until_idle(cx, &[&both]);
    assert!(
        renders < 20,
        "two open modals in one view keep the window redrawing: {renders} renders without input \
         (the Host stopped rendering them at {LIMIT})"
    );
}

// Which modal holds focus: the topmost (the last in the Host's overlay stack,
// the one Escape closes first), and the next one when it closes.

fn controlled_dialog_view(name: &str) -> PreparedScriptView {
    let main = format!(
        r#"import "components/button" as button;
import "components/dialog" as dialog;
fn state_schema() {{ #{{ fields: #{{
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }},
    hits: #{{ schema: #{{ type: "integer" }}, "default": #{{ type: "integer", value: 0 }} }} }} }} }}
fn set_open(ctx, open) {{ ctx.set_state("open", open); }}
fn hit(ctx, payload) {{ ctx.set_state("hits", ctx.get_state("hits") + 1); }}
fn view(ctx) {{ column([
    text(`{name} open=${{ctx.get_state("open")}} hits=${{ctx.get_state("hits")}}`),
    dialog::Dialog(#{{ key: "{name}", open: ctx.get_state("open"), title: "{name}",
        content: text("{name} dialog"), on_open_change: Fn("set_open"),
        actions: [button::Button(#{{ key: "ok-{name}", text: "OK {name}", on_click: Fn("hit") }})] }})
]).with_style(style().width(px(400)).height(px(600))) }}"#
    );
    Mount::components(main)
        .motion(MotionPreference::None)
        .prepare()
        .unwrap()
}

/// A Host that can take focus itself, outside its container.
struct FocusingHost {
    host: ScriptViewHost,
    views: Vec<ScriptViewHandle>,
    external_focus: gpui::FocusHandle,
}

impl Render for FocusingHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().track_focus(&self.external_focus).child(
            self.host.container(
                div()
                    .size_full()
                    .flex()
                    .children(self.views.iter().map(|view| {
                        div()
                            .w(px(400.0))
                            .h(px(600.0))
                            .child(view.element().unwrap())
                    })),
            ),
        )
    }
}

fn mount_controlled_dialogs(
    cx: &mut TestAppContext,
    names: &[&str],
) -> (
    VisualTestContext,
    ScriptViewHost,
    Vec<ScriptViewHandle>,
    gpui::FocusHandle,
) {
    cx.update(gpui_rhai::install);
    let prepared = names
        .iter()
        .map(|name| controlled_dialog_view(name))
        .collect::<Vec<_>>();
    let names = names
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("dialogs", cx).unwrap();
        let views = prepared
            .into_iter()
            .zip(names)
            .map(|(prepared, name)| {
                prepared
                    .mount(ScriptViewConfig::new(name), host.clone(), window, cx)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let external_focus = cx.focus_handle();
        *save.borrow_mut() = Some((host.clone(), views.clone(), external_focus.clone()));
        FocusingHost {
            host,
            views,
            external_focus,
        }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    let (host, views, external_focus) = saved.borrow().clone().unwrap();
    for view in &views {
        assert_eq!(last_error(&mut visual, view), None);
    }
    (visual, host, views, external_focus)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Frame, 3);
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    nodes(visual, view)
        .into_iter()
        .map(|node| node.name)
        .find(|name| name.contains(" open="))
        .expect("status text")
}

/// Tab from the focused modal panel to its OK button and press it: the view
/// whose `hits` goes up held focus.
fn press_ok_in_focused_modal(visual: &mut VisualTestContext) {
    visual.simulate_keystrokes("tab");
    settle(visual);
    visual.simulate_keystrokes("enter");
    settle(visual);
}

#[gpui::test]
fn the_topmost_open_modal_holds_focus_and_the_next_takes_it_when_it_closes(
    cx: &mut TestAppContext,
) {
    let (mut visual, host, views, _) = mount_controlled_dialogs(cx, &["left", "right"]);
    let stack = host
        .open_overlays()
        .into_iter()
        .map(|overlay| overlay.view_id)
        .collect::<Vec<_>>();
    assert_eq!(stack, ["left", "right"], "the right dialog is the topmost");

    press_ok_in_focused_modal(&mut visual);
    assert_eq!(status(&mut visual, &views[0]), "left open=true hits=0");
    assert_eq!(status(&mut visual, &views[1]), "right open=true hits=1");

    // Escape closes the topmost; the left dialog takes focus.
    visual.simulate_keystrokes("escape");
    settle(&mut visual);
    assert_eq!(status(&mut visual, &views[1]), "right open=false hits=1");
    press_ok_in_focused_modal(&mut visual);
    assert_eq!(status(&mut visual, &views[0]), "left open=true hits=1");
    assert_eq!(status(&mut visual, &views[1]), "right open=false hits=1");
}

#[gpui::test]
fn the_topmost_of_two_open_modals_takes_focus_back_from_the_host(cx: &mut TestAppContext) {
    let (mut visual, _host, views, external_focus) =
        mount_controlled_dialogs(cx, &["left", "right"]);
    visual.update(|window, cx| external_focus.focus(window, cx));
    settle(&mut visual);
    assert!(
        !visual.update(|window, _| external_focus.is_focused(window)),
        "an open modal takes focus back from the Host"
    );
    press_ok_in_focused_modal(&mut visual);
    assert_eq!(status(&mut visual, &views[0]), "left open=true hits=0");
    assert_eq!(status(&mut visual, &views[1]), "right open=true hits=1");
}
