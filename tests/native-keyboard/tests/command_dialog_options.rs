//! CommandDialog can replace a hand-built palette (#120): the title can be hidden while the
//! label still names the dialog, `on_escape` lets the caller see Escape before the dialog
//! closes (a palette with levels goes back one level), and `width`,
//! `command_part_styles` and `dialog_part_styles` reach the inner Dialog and Command.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// A palette open from the start; `extra` adds CommandDialog props.
fn mount(cx: &mut TestAppContext, extra: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/command_dialog" as command_dialog;
fn state_schema() {{ #{{ fields: #{{
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }},
    escapes: #{{ schema: #{{ type: "integer" }}, "default": #{{ type: "integer", value: 0 }} }} }} }} }}
fn opened(ctx, open) {{ ctx.set_state("open", open); }}
fn escaped(ctx, payload) {{ ctx.set_state("escapes", ctx.get_state("escapes") + 1); }}
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([
    text(`${{ctx.get_state("open")}}/${{ctx.get_state("escapes")}}`).accessibility_role("status"),
    command_dialog::CommandDialog(#{{ key: "palette", open: ctx.get_state("open"), query: "",
        active_value: "", label: "Commands", on_open_change: Fn("opened"),
        on_query_change: Fn("noop"), on_active_change: Fn("noop"), on_action: Fn("noop"),
        items: [#{{ value: "theme", label: "Switch theme" }}, #{{ value: "open", label: "Open with" }}]
        {extra} }}),
]).with_style(style().width(px(900)).height(px(700))) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
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
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("palette", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("palette"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    })
}

fn nodes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<(String, String)> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .map(|node| (node.role.clone(), node.name.clone()))
            .collect()
    })
}

fn press_escape(visual: &mut VisualTestContext) {
    visual.simulate_keystrokes("escape");
    settle(visual);
}

#[gpui::test]
fn a_hidden_title_still_names_the_dialog(cx: &mut TestAppContext) {
    for (visible, extra) in [(true, ""), (false, ", title_visible: false")] {
        let (mut visual, view) = mount(cx, extra);
        let found = nodes(&mut visual, &view);
        assert!(
            found
                .iter()
                .any(|(role, name)| role == "dialog" && name == "Commands"),
            "title_visible {visible}: {found:?}"
        );
        let title_text = found
            .iter()
            .any(|(role, name)| role == "text" && name == "Commands");
        assert_eq!(title_text, visible, "title_visible {visible}: {found:?}");
    }
}

#[gpui::test]
fn on_escape_sees_escape_before_the_dialog_closes(cx: &mut TestAppContext) {
    // The control: without on_escape, Escape closes the dialog.
    let (mut visual, view) = mount(cx, "");
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "false/0");
    // With on_escape the caller gets the key and the dialog stays open.
    let (mut visual, view) = mount(cx, r#", on_escape: Fn("escaped")"#);
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "true/1");
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "true/2");
}

/// The panel's layout bounds and the green `search` part's left edge inside it.
fn search_inset(cx: &mut TestAppContext, extra: &str) -> (GeometryBounds, f64) {
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#", command_part_styles: #{{ search: style().background(rgba(0x00ff00ff)) }} {extra}"#
        ),
    );
    let dialog = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("dialog", "Commands")
            .next()
            .and_then(|node| node.geometry)
            .unwrap()
            .layout
    });
    let green: gpui::Hsla = gpui::rgba(0x00ff_00ff).into();
    let search_x = visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .find(|quad| quad.background == green.into())
            .map(|quad| f64::from(quad.bounds.origin.x.0 / scale))
            .expect("the search part takes the caller's background")
    });
    (dialog, search_x - dialog.x)
}

#[gpui::test]
fn width_and_part_styles_reach_the_inner_parts(cx: &mut TestAppContext) {
    let (default, flush) = search_inset(cx, "");
    // The documented 420px, no longer 90% of itself.
    assert!(
        (default.width - 420.0).abs() < 0.5,
        "default width {}",
        default.width
    );
    // The command fills the panel: its search starts inside the 1px hairline.
    assert!((flush - 1.0).abs() < 0.5, "search inset {flush}");
    let (wide, padded) = search_inset(
        cx,
        r#", width: px(560), dialog_part_styles: #{ panel: style().padding(px(16)) }"#,
    );
    assert!(
        (wide.width - 560.0).abs() < 0.5,
        "panel width {}",
        wide.width
    );
    // A caller's panel padding still wins over the flush default.
    assert!(
        (padded - flush - 16.0).abs() < 0.5,
        "search inset {padded} with a 16px panel padding, {flush} without"
    );
}
