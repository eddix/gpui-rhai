//! `ScriptViewHost::open_overlays` and `ScriptViewHost::dismiss_escape`: a Host
//! asks which script overlays are open in its views, topmost last, and lets
//! them take Escape before it handles the key itself.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME};

/// A raw popover the view declares itself (no owner component).
const POPOVER: &str = r#"overlay(text("anchor"), text("popover body"), #{ id: "pop",
    kind: "popover", open: ctx.get_state("open"), dismiss_on_outside: false })
    .on_open_change(Fn("set_open"))"#;

/// A Dialog component, whose overlay is declared inside the component.
const DIALOG: &str = r#"dialog::Dialog(#{ key: "confirm", open: ctx.get_state("open"),
    title: "Confirm", content: text("Sure?"), on_open_change: Fn("set_open") })"#;

fn prepared(body: &str) -> PreparedScriptView {
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/dialog" as dialog;
fn state_schema() {{ #{{ fields: #{{
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }} }} }} }}
fn set_open(ctx, open) {{ ctx.set_state("open", open); }}
fn view(ctx) {{ column([
    text(`open=${{ctx.get_state("open")}}`).accessibility_role("group").test_id("open"),
    {body}]).with_style(style().width(px(400)).height(px(600))) }}"#
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
                .children(self.views.iter().map(|view| {
                    div()
                        .w(px(400.0))
                        .h(px(600.0))
                        .child(view.element().unwrap())
                })),
        )
    }
}

/// Mount `(view id, body)` pairs side by side under one Host.
fn mount(
    cx: &mut TestAppContext,
    views: &[(&str, &str)],
) -> (VisualTestContext, ScriptViewHost, Vec<ScriptViewHandle>) {
    cx.update(gpui_rhai::install);
    let prepared = views
        .iter()
        .map(|(id, body)| ((*id).to_owned(), prepared(body)))
        .collect::<Vec<_>>();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("overlays", cx).unwrap();
        let views = prepared
            .into_iter()
            .map(|(id, prepared)| {
                prepared
                    .mount(ScriptViewConfig::new(id), host.clone(), window, cx)
                    .unwrap()
            })
            .collect::<Vec<_>>();
        *save.borrow_mut() = Some((host.clone(), views.clone()));
        Host { host, views }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    let (host, views) = saved.borrow().clone().unwrap();
    for view in &views {
        assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    }
    (visual, host, views)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn open_state(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| node.test_id.as_deref() == Some("open"))
        .map(|node| node.name.clone())
        .expect("open node")
}

/// `(view, owner, key, kind, modal, dismiss_on_escape)` per open overlay.
type Row = (String, Option<String>, String, OverlayKind, bool, bool);

fn rows(host: &ScriptViewHost) -> Vec<Row> {
    host.open_overlays()
        .into_iter()
        .map(|overlay| {
            (
                overlay.view_id,
                overlay.owner,
                overlay.key,
                overlay.kind,
                overlay.modal,
                overlay.dismiss_on_escape,
            )
        })
        .collect()
}

#[gpui::test]
fn open_overlays_lists_every_view_topmost_last(cx: &mut TestAppContext) {
    let (_visual, host, _views) = mount(cx, &[("left", POPOVER), ("right", DIALOG)]);
    let rows = rows(&host);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(
        rows[0],
        (
            "left".to_owned(),
            None,
            "pop".to_owned(),
            OverlayKind::Popover,
            false,
            true
        )
    );
    let (view, owner, key, kind, modal, dismiss_on_escape) = rows[1].clone();
    assert_eq!(
        (view.as_str(), key.as_str(), kind, modal, dismiss_on_escape),
        ("right", "confirm", OverlayKind::Dialog, true, true)
    );
    let owner = owner.expect("the Dialog component declared it");
    assert!(owner.ends_with("Dialog[confirm]"), "owner {owner}");
    // The instance path names the overlay for the existing placement lookup.
    assert!(
        host.overlay_placement_in("right", &owner, "confirm")
            .is_some()
    );
}

#[gpui::test]
fn dismiss_escape_closes_the_topmost_overlay_and_reports_it(cx: &mut TestAppContext) {
    let (mut visual, host, views) = mount(cx, &[("left", POPOVER), ("right", DIALOG)]);

    assert!(visual.update(|window, cx| host.dismiss_escape(window, cx)));
    settle(&mut visual);
    assert_eq!(open_state(&mut visual, &views[0]), "open=true");
    assert_eq!(open_state(&mut visual, &views[1]), "open=false");
    let keys = rows(&host).into_iter().map(|row| row.2).collect::<Vec<_>>();
    assert_eq!(keys, ["pop"]);

    assert!(visual.update(|window, cx| host.dismiss_escape(window, cx)));
    settle(&mut visual);
    assert_eq!(open_state(&mut visual, &views[0]), "open=false");
    assert!(host.open_overlays().is_empty());

    // Nothing open: the Host handles Escape itself.
    assert!(!visual.update(|window, cx| host.dismiss_escape(window, cx)));
}

#[gpui::test]
fn an_overlay_that_ignores_escape_is_listed_and_left_open(cx: &mut TestAppContext) {
    let sticky = POPOVER.replace("dismiss_on_outside: false", "dismiss_on_escape: false");
    let (mut visual, host, views) = mount(cx, &[("only", &sticky)]);
    assert_eq!(
        rows(&host),
        [(
            "only".to_owned(),
            None,
            "pop".to_owned(),
            OverlayKind::Popover,
            false,
            false
        )]
    );
    assert!(!visual.update(|window, cx| host.dismiss_escape(window, cx)));
    settle(&mut visual);
    assert_eq!(open_state(&mut visual, &views[0]), "open=true");
}
