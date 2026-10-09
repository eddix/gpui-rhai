//! Collapsible's panel height follows density (#127): omitted, the panel fits its content
//! (measured after layout); a Length such as `theme_length("metrics.row") * 3` resolves in
//! the subtree's density. An explicit integer stays as given (the control).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME,
};

struct Host {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        match &self.view {
            Some(view) => self
                .host
                .container(view.element().unwrap())
                .into_any_element(),
            None => gpui::div().into_any_element(),
        }
    }
}

const SCRIPT: &str = r#"
fn line(label) {
    text(label).with_style(style().min_height(theme_length("metrics.row")).items_center())
}
fn content() {
    column([line("Line 1"), line("Line 2")])
        .accessibility_role("group").accessibility_label("content")
}
fn state_schema() { #{ fields: #{ open: #{ schema: #{ type: "bool" },
    "default": #{ type: "bool", value: false } } } } }
fn toggle(ctx, value) { ctx.set_state("open", value); }
fn scene(density, height) {
    let part = collapsible::Collapsible(#{ key: "details", open: true,
        trigger: text("Details"), content: content(), content_height: height });
    column([part, text("after")]).env(#{ density: density })
        .with_style(style().width(px(300)))
}
"#;

fn mount(
    cx: &mut TestAppContext,
    view_body: &str,
) -> Result<(VisualTestContext, ScriptViewHandle), String> {
    let mut modules = BTreeMap::new();
    let tables = [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
    ];
    for (id, source) in tables.iter().flat_map(|t| t.iter()) {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"
import "components/collapsible" as collapsible;
{SCRIPT}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(400)).height(px(400))) }}
"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
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
            .map_err(|error| format!("{error:?}"))?;
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("collapsible", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("collapsible"),
                host.clone(),
                window,
                cx,
            )
            .map_err(|error| format!("{error:?}"));
        *capture.borrow_mut() = Some(view.clone());
        Host {
            host,
            view: view.ok(),
        }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap()?;
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    Ok((visual, view))
}

/// The content's and the next sibling's layout bounds, (x, y, width, height).
fn layout(cx: &mut TestAppContext, density: &str, height: &str) -> ((f64, f64, f64, f64), f64) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = mount(cx, &format!(r#"scene("{density}", {height})"#)).unwrap();
    let (content, after) = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let bounds = |role: &str, name: &str| {
            let g = snapshot
                .find_by_role_and_name(role, name)
                .next()
                .unwrap_or_else(|| panic!("no {role} {name:?}"))
                .geometry
                .unwrap()
                .layout;
            (g.x, g.y, g.width, g.height)
        };
        (bounds("group", "content"), bounds("text", "after").1)
    });
    eprintln!("{density}, content_height {height}: content {content:?}, next sibling y {after}");
    (content, after)
}

/// The space between the content's bottom and the next sibling.
fn blank_below(cx: &mut TestAppContext, density: &str, height: &str) -> f64 {
    let (content, after) = layout(cx, density, height);
    after - (content.1 + content.3)
}

#[gpui::test]
fn an_explicit_integer_height_is_kept(cx: &mut TestAppContext) {
    // Two metrics.row lines (2 x 32) plus the content's bottom inset (12), comfortable.
    let blank = blank_below(cx, "comfortable", "76");
    assert!(blank.abs() < 0.5, "comfortable: {blank}px blank");
    // The same integer in compact keeps 76px: the caller asked for it.
    let blank = blank_below(cx, "compact", "76");
    assert!((blank - 12.0).abs() < 0.5, "compact: {blank}px blank");
}

#[gpui::test]
fn an_omitted_height_fits_the_content_in_both_densities(cx: &mut TestAppContext) {
    for density in ["comfortable", "compact"] {
        let blank = blank_below(cx, density, "()");
        assert!(blank.abs() < 0.5, "{density}: {blank}px blank");
    }
}

#[gpui::test]
fn a_length_height_follows_density(cx: &mut TestAppContext) {
    for (density, row) in [("comfortable", 32.0), ("compact", 28.0)] {
        let (content, after) = layout(cx, density, r#"theme_length("metrics.row") * 3"#);
        let panel = after - content.1;
        assert!(
            (panel - 3.0 * row).abs() < 0.5,
            "{density}: the panel is {panel}px, three rows are {}px",
            3.0 * row
        );
    }
}

#[gpui::test]
fn a_closed_panel_without_a_height_opens_to_its_content(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let body = r#"column([collapsible::Collapsible(#{ key: "details", open: ctx.get_state("open"),
            trigger: text("Details"), content: content(), on_open_change: Fn("toggle") }),
        text("after")]).env(#{ density: "compact" }).with_style(style().width(px(300)))"#;
    let (mut visual, view) = mount(cx, body).unwrap();
    let bounds = |visual: &mut VisualTestContext, role: &str, name: &str| {
        visual.update(|_, cx| {
            let g = view
                .accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name(role, name)
                .next()
                .unwrap_or_else(|| panic!("no {role} {name:?}"))
                .geometry
                .unwrap()
                .layout;
            (g.x, g.y, g.width, g.height)
        })
    };
    let trigger = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "button")
            .and_then(|node| node.geometry)
            .unwrap()
            .layout
    });
    let closed = bounds(&mut visual, "text", "after").1;
    assert!(
        (closed - (trigger.y + trigger.height)).abs() < 0.5,
        "closed: the panel takes no height ({closed})"
    );
    #[allow(clippy::cast_possible_truncation)]
    let center = gpui::point(
        gpui::px((trigger.x + trigger.width / 2.0) as f32),
        gpui::px((trigger.y + trigger.height / 2.0) as f32),
    );
    visual.simulate_click(center, gpui::Modifiers::none());
    for _ in 0..3 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    let content = bounds(&mut visual, "group", "content");
    let after = bounds(&mut visual, "text", "after").1;
    assert!(
        (after - (content.1 + content.3)).abs() < 0.5,
        "open: the panel fits the content measured while closed: content {content:?}, after {after}"
    );
}
