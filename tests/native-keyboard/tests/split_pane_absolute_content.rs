//! Absolutely positioned content inside either SplitPane pane paints at its offset from that
//! pane (#140): horizontal and vertical, with and without a `handle` grip, LTR and RTL, and
//! with the other pane collapsed.
//!
//! Each pane gets a 40x30 probe at (10, 10). `direct` makes the probe the pane's content;
//! `filled` puts it in a clipping box that fills the pane with `flex_grow`, the shape of a
//! canvas or treemap stage whose children are all absolute. The panes are flex columns, so
//! that box is as tall as the pane; in a block pane it would collapse to its in-flow content,
//! which is nothing, and clip the probe away.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, rgba,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE,
    TOKEN_BASE_SOURCE,
};

const START: u32 = 0xff00_00ff;
const END: u32 = 0x00ff_00ff;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main.to_owned());
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([
                ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
                ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
            ])
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
        let host = ScriptViewHost::new("split", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("split"), host.clone(), window, cx)
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
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
    }
}

/// `(x, y, width, height)` in logical pixels.
type Rect = (f32, f32, f32, f32);

/// The visible part (bounds within the content mask) of each quad filled with `color`.
fn visible(visual: &mut VisualTestContext, color: u32) -> Vec<Rect> {
    let color: gpui::Hsla = rgba(color).into();
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.background == color.into())
            .map(|quad| {
                let bounds = quad.bounds.intersect(&quad.content_mask.bounds);
                (
                    bounds.origin.x.0 / scale,
                    bounds.origin.y.0 / scale,
                    bounds.size.width.0.max(0.0) / scale,
                    bounds.size.height.0.max(0.0) / scale,
                )
            })
            .collect()
    })
}

fn bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> Rect {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} named {name}"))
            .geometry
            .unwrap()
            .visual
    });
    #[allow(clippy::cast_possible_truncation)]
    (
        bounds.x as f32,
        bounds.y as f32,
        bounds.width as f32,
        bounds.height as f32,
    )
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Direct,
    Filled,
}

fn probe(shape: Shape, color: u32) -> String {
    match shape {
        Shape::Direct => format!("probe(rgba({color:#x}))"),
        Shape::Filled => format!("filled(rgba({color:#x}))"),
    }
}

/// A 420x240 frame holding the SplitPane, so the group's bounds are the frame's.
fn script(locale: &str, props: &str) -> String {
    format!(
        r#"import "components/split_pane" as split_pane;
fn init(ctx) {{ ctx.set_locale("{locale}"); }}
fn probe(color) {{
    box([]).with_style(style().absolute().left(px(10)).top(px(10)).width(px(40)).height(px(30))
        .background(color))
}}
fn filled(color) {{
    box([probe(color)]).with_style(style().flex_grow().relative().overflow_hidden())
}}
fn view(ctx) {{ column([
    box([split_pane::SplitPane(#{{ key: "layout", label: "Resize panels", size: 0.5,
        min_start: 0.0, min_end: 0.0, {props} }})])
        .with_style(style().width(px(420)).height(px(240)))
        .accessibility_role("group").accessibility_label("Frame"),
]).with_style(style().padding(px(20))) }}"#
    )
}

fn assert_probe(visual: &mut VisualTestContext, color: u32, pane: Rect, case: &str) {
    let quads = visible(visual, color);
    let expected = (pane.0 + 10., pane.1 + 10., 40., 30.);
    let near = |a: Rect, b: Rect| {
        (a.0 - b.0).abs() < 0.5
            && (a.1 - b.1).abs() < 0.5
            && (a.2 - b.2).abs() < 0.5
            && (a.3 - b.3).abs() < 0.5
    };
    assert!(
        quads.iter().any(|quad| near(*quad, expected)),
        "{case}: the probe should show whole at {expected:?} in the pane at {pane:?}; \
         visible quads in its colour: {quads:?}"
    );
}

#[gpui::test]
fn absolute_content_paints_in_both_panes(cx: &mut TestAppContext) {
    for shape in [Shape::Direct, Shape::Filled] {
        for orientation in ["horizontal", "vertical"] {
            for grip in [false, true] {
                for locale in ["en", "ar"] {
                    let case = format!("{shape:?} {orientation} grip={grip} {locale}");
                    let handle = if grip {
                        "handle: box([]).with_style(style().width(px(6)).height(px(6))),"
                    } else {
                        ""
                    };
                    let main = script(
                        locale,
                        &format!(
                            "orientation: \"{orientation}\", {handle} start: {}, end: {}",
                            probe(shape, START),
                            probe(shape, END)
                        ),
                    );
                    let (mut visual, view) = mount(cx, &main);
                    let group = bounds(&mut visual, &view, "group", "Frame");
                    let separator = bounds(&mut visual, &view, "separator", "Resize panels");
                    let (start, end) = if orientation == "vertical" {
                        let after = separator.1 + separator.3;
                        (
                            (group.0, group.1, group.2, separator.1 - group.1),
                            (group.0, after, group.2, group.1 + group.3 - after),
                        )
                    } else {
                        let before = (group.0, group.1, separator.0 - group.0, group.3);
                        let after_x = separator.0 + separator.2;
                        let after = (after_x, group.1, group.0 + group.2 - after_x, group.3);
                        if locale == "ar" {
                            (after, before)
                        } else {
                            (before, after)
                        }
                    };
                    assert_probe(&mut visual, START, start, &format!("{case}, start pane"));
                    assert_probe(&mut visual, END, end, &format!("{case}, end pane"));
                }
            }
        }
    }
}

#[gpui::test]
fn absolute_content_paints_beside_a_collapsed_pane(cx: &mut TestAppContext) {
    for shape in [Shape::Direct, Shape::Filled] {
        for (collapsed, color) in [("start_collapsed", END), ("end_collapsed", START)] {
            let case = format!("{shape:?} {collapsed}");
            let main = script(
                "en",
                &format!(
                    "{collapsed}: true, start: {}, end: {}",
                    probe(shape, START),
                    probe(shape, END)
                ),
            );
            let (mut visual, view) = mount(cx, &main);
            let group = bounds(&mut visual, &view, "group", "Frame");
            assert_probe(&mut visual, color, group, &case);
        }
    }
}
