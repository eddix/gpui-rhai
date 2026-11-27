//! A Table or List that holds focus with no current row to show where the keys act
//! (no selection mode with a context request, or nothing selected) draws the 2px
//! `focus_ring` frame over its edge; a current row shows its cursor instead.

use std::collections::BTreeMap;

use gpui::{TestAppContext, VisualTestContext, rgba};
use gpui_rhai::*;

use crate::support::{Mount, Step, painted_quads};

const FOCUS: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        "import \"components/table\" as table;\nimport \"components/list\" as list;\n\
         fn noop(ctx, payload) {{}}\nfn view(ctx) {{ column([{body}]).with_style(style().width(px(480)).padding(px(16))) }}\n"
    );
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS))]),
        ..Default::default()
    };
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .with(move |view| view.theme_token_overrides(overrides))
        .id("frames")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Refresh, 3);
}

/// A whole-collection cue may lie on its client edge or decorative outer edge.
/// Both must be absent with a current row; a 1px preset frame is not row chrome.
fn frames_the_collection(frames: &[(f32, f32)], client_width: f32) -> bool {
    frames.iter().any(|(width, height)| {
        ((width - client_width).abs() < 0.5 || (width - 448.0).abs() < 0.5) && *height > 64.0
    })
}

/// Logical sizes of the quads whose border is drawn in the focus color.
fn framed_sizes(visual: &mut VisualTestContext) -> Vec<(f32, f32)> {
    let color: gpui::Hsla = rgba(FOCUS).into();
    painted_quads(visual)
        .iter()
        .filter(|quad| quad.border_color == color && quad.border_widths[0] > 0.0)
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

/// Focus the first tab stop and return the sizes of focus-colored frames.
fn focused_frames(cx: &mut TestAppContext, body: &str) -> (Vec<(f32, f32)>, f32) {
    let (mut visual, view) = mount(cx, body);
    assert!(
        framed_sizes(&mut visual).is_empty(),
        "no frame before focus"
    );
    // Tab from the view, a navigation key, so the default focus indicators show.
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab");
    settle(&mut visual);
    let client = visual
        .debug_bounds("virtual-list:t-body")
        .or_else(|| visual.debug_bounds("virtual-list:l-rows"))
        .expect("collection client viewport");
    (framed_sizes(&mut visual), f32::from(client.size.width))
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
        let (frames, client) = focused_frames(
            cx,
            &format!(
                r#"table::Table(#{{ key: "t", label: "Hosts", row_key: "id", height: 120,
                    columns: {COLUMNS}, rows: {ROWS}, {extra} }})"#
            ),
        );
        assert!(
            frames_the_collection(&frames, client),
            "{case}: the table's edge is framed: {frames:?}"
        );
    }
}

#[gpui::test]
fn a_focused_table_with_a_current_row_shows_its_cursor_not_a_frame(cx: &mut TestAppContext) {
    let (frames, client) = focused_frames(
        cx,
        &format!(
            r#"table::Table(#{{ key: "t", label: "Hosts", row_key: "id", height: 120,
                columns: {COLUMNS}, rows: {ROWS}, selection_mode: "single", selected_keys: ["a"],
                on_selection_change: Fn("noop") }})"#
        ),
    );
    assert!(
        !frames_the_collection(&frames, client),
        "no frame around the whole table: {frames:?}"
    );
}

#[gpui::test]
fn a_focused_list_without_a_selection_mode_is_framed(cx: &mut TestAppContext) {
    let (frames, client) = focused_frames(
        cx,
        r#"list::List(#{ key: "l", label: "Jobs", height: 120,
            items: [#{ key: "a", title: "Alpha" }, #{ key: "b", title: "Beta" }],
            on_context_request: Fn("noop") })"#,
    );
    assert!(
        frames_the_collection(&frames, client),
        "the list's edge is framed: {frames:?}"
    );
}
