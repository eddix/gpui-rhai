//! A Table or List that holds focus with no current row to show where the keys act
//! (no selection mode with a context request, or nothing selected) draws the 2px
//! `focus_ring` frame over its edge; a current row shows its cursor instead.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, rgba,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
};

const FOCUS: u32 = 0x00ff_00ff;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, body: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(
        entry.clone(),
        format!(
            "import \"components/table\" as table;\nimport \"components/list\" as list;\n\
             fn noop(ctx, payload) {{}}\nfn view(ctx) {{ column([{body}]).with_style(style().width(px(480)).padding(px(16))) }}\n"
        ),
    );
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS))]),
        ..Default::default()
    };
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .theme_token_overrides(overrides)
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
        let host = ScriptViewHost::new("frames", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("frames"), host.clone(), window, cx)
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

/// Whether a frame spans the whole collection (448 wide, taller than one row).
fn frames_the_collection(frames: &[(f32, f32)]) -> bool {
    frames
        .iter()
        .any(|(width, height)| (width - 448.0).abs() < 1.0 && *height > 64.0)
}

/// Logical sizes of the quads whose border is drawn in the focus color.
fn framed_sizes(visual: &mut VisualTestContext) -> Vec<(f32, f32)> {
    let color: gpui::Hsla = rgba(FOCUS).into();
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_color == color && quad.border_widths.top.0 > 0.0)
            .map(|quad| {
                (
                    quad.bounds.size.width.0 / scale,
                    quad.bounds.size.height.0 / scale,
                )
            })
            .collect()
    })
}

/// Focus the first tab stop and return the sizes of focus-colored frames.
fn focused_frames(cx: &mut TestAppContext, body: &str) -> Vec<(f32, f32)> {
    let (mut visual, _view) = mount(cx, body);
    assert!(
        framed_sizes(&mut visual).is_empty(),
        "no frame before focus"
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    framed_sizes(&mut visual)
}

const COLUMNS: &str = r#"[#{ key: "h", title: "Host", width: #{ kind: "flex", value: 1.0 } }]"#;
const ROWS: &str = r#"[#{ id: "a", h: "a" }, #{ id: "b", h: "b" }]"#;

#[gpui::test]
fn a_focused_table_without_a_current_row_is_framed(cx: &mut TestAppContext) {
    for (case, extra) in [
        (
            "none with a context request",
            r#"on_context_request: Fn("noop")"#,
        ),
        (
            "single with nothing selected",
            r#"selection_mode: "single", selected_keys: [], on_selection_change: Fn("noop")"#,
        ),
    ] {
        let frames = focused_frames(
            cx,
            &format!(
                r#"table::Table(#{{ key: "t", label: "Hosts", row_key: "id", height: 120,
                    columns: {COLUMNS}, rows: {ROWS}, {extra} }})"#
            ),
        );
        assert!(
            frames_the_collection(&frames),
            "{case}: the table's edge is framed: {frames:?}"
        );
    }
}

#[gpui::test]
fn a_focused_table_with_a_current_row_shows_its_cursor_not_a_frame(cx: &mut TestAppContext) {
    let frames = focused_frames(
        cx,
        &format!(
            r#"table::Table(#{{ key: "t", label: "Hosts", row_key: "id", height: 120,
                columns: {COLUMNS}, rows: {ROWS}, selection_mode: "single", selected_keys: ["a"],
                on_selection_change: Fn("noop") }})"#
        ),
    );
    assert!(
        !frames_the_collection(&frames),
        "no frame around the whole table: {frames:?}"
    );
}

#[gpui::test]
fn a_focused_list_without_a_selection_mode_is_framed(cx: &mut TestAppContext) {
    let frames = focused_frames(
        cx,
        r#"list::List(#{ key: "l", label: "Jobs", height: 120,
            items: [#{ key: "a", title: "Alpha" }, #{ key: "b", title: "Beta" }],
            on_context_request: Fn("noop") })"#,
    );
    assert!(
        frames_the_collection(&frames),
        "the list's edge is framed: {frames:?}"
    );
}
