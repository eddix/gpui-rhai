//! Real mounted Markdown interaction, not only component preparation.

use gpui::{
    Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton, ParentElement as _,
    Render, ScrollDelta, ScrollWheelEvent, Styled as _, TestAppContext, VisualTestContext, Window,
    point, px,
};
use gpui_rhai::NativeTextDocument;

use crate::support::{
    ActiveHost, Mount, ReplaceOnce as _, Step, bounds, last_error, palette_texts, settle,
};

const COMPONENT: &str = include_str!("../../../../../registry/components/markdown_viewer.rhai");

struct CountingMarkdownImageProvider(std::rc::Rc<std::cell::Cell<usize>>);

impl gpui_rhai::AssetProvider for CountingMarkdownImageProvider {
    fn load(&self, name: &str) -> Result<gpui_rhai::AssetData, String> {
        self.0.set(self.0.get() + 1);
        if name != "cover" {
            return Err("only the Host cover fixture is available".to_owned());
        }
        Ok(gpui_rhai::AssetData {
            mime_type: "image/svg+xml".to_owned(),
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="blue"/></svg>"#.to_vec(),
        })
    }
}

struct MarkdownImageAssets {
    calls: std::rc::Rc<std::cell::Cell<usize>>,
    preload: bool,
}

impl gpui_rhai::ScriptViewExtension for MarkdownImageAssets {
    fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
        runtime
            .assets
            .register(
                "markdown",
                CountingMarkdownImageProvider(self.calls.clone()),
            )
            .map_err(|error| error.to_string())?;
        if self.preload {
            runtime
                .assets
                .preload_images([gpui_rhai::AssetId::parse("markdown/cover").unwrap()])
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

struct MarkdownKeyAncestor {
    host: gpui_rhai::ScriptViewHost,
    view: gpui_rhai::ScriptViewHandle,
    keys: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Render for MarkdownKeyAncestor {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let keys = self.keys.clone();
        self.host.container(
            gpui::div()
                .size_full()
                .on_key_down(move |event, _, cx| {
                    if matches!(
                        event.keystroke.key.as_str(),
                        "up" | "down" | "pageup" | "pagedown" | "home" | "end"
                    ) {
                        keys.borrow_mut().push(event.keystroke.key.clone());
                        cx.stop_propagation();
                    }
                })
                .child(self.view.element().unwrap()),
        )
    }
}

const NESTED_MARKDOWN_KEYS: &str = r#"
import "components/markdown_viewer" as markdown;
fn article() {
    let value = "";
    // Backtick templates are verbatim in Rhai: append real newline escapes
    // through a normal quoted string, not inside the template.
    for index in 0..80 { value += `Paragraph ${index} keeps its own reader position.` + "\n\n"; }
    value
}
fn view(ctx) {
    column([
        markdown::MarkdownViewer(#{ key: "nested", label: "Nested reply",
            source: "Short reply", height: "content" }),
    ]).with_style(style().width(px(480)).height(px(260)))
}
"#;

#[gpui::test]
fn content_height_markdown_bubbles_unconsumed_scroll_keys_to_its_ancestor(cx: &mut TestAppContext) {
    let keys = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let captured = keys.clone();
    let (window, view) = Mount::new(NESTED_MARKDOWN_KEYS)
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-key-bubble")
        .mount_root(cx, move |host, view| MarkdownKeyAncestor {
            host,
            view,
            keys: captured,
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let rect = bounds(&mut visual, &view, "document", "Nested reply");
    let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    for key in ["up", "down", "pageup", "pagedown", "home", "end"] {
        keys.borrow_mut().clear();
        visual.simulate_keystrokes(key);
        settle(&mut visual, Step::Frame, 1);
        assert_eq!(
            *keys.borrow(),
            [key],
            "non-scrollable Markdown swallowed {key}"
        );
    }
}

#[gpui::test]
fn scrollable_markdown_consumes_only_keys_that_move_its_native_reader(cx: &mut TestAppContext) {
    let keys = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let captured = keys.clone();
    let source = NESTED_MARKDOWN_KEYS.replace_once(
        "source: \"Short reply\", height: \"content\"",
        "source: article(), height: \"content\", max_height: 96",
    );
    let (window, view) = Mount::new(source)
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-key-consumption")
        .mount_root(cx, move |host, view| MarkdownKeyAncestor {
            host,
            view,
            keys: captured,
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let rect = bounds(&mut visual, &view, "document", "Nested reply");
    let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("alt-cmd-c");
    let raw = visual
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .expect("prepared source for the native keyboard fixture");
    assert_eq!(
        raw.matches("\n\n").count(),
        80,
        "fixture must contain 80 real Markdown paragraphs"
    );
    assert!(
        !raw.contains("\\n"),
        "fixture must not contain verbatim newline escapes"
    );
    for key in ["up", "pageup", "home"] {
        keys.borrow_mut().clear();
        visual.simulate_keystrokes(key);
        settle(&mut visual, Step::Frame, 1);
        assert_eq!(
            *keys.borrow(),
            [key],
            "top edge must leave {key} to the ancestor"
        );
    }
    keys.borrow_mut().clear();
    for key in ["down", "pagedown", "end", "up", "pageup", "home"] {
        visual.simulate_keystrokes(key);
        settle(&mut visual, Step::Frame, 2);
        assert!(
            keys.borrow().is_empty(),
            "internal scrolling did not consume {key}"
        );
    }
    visual.simulate_keystrokes("end");
    settle(&mut visual, Step::Frame, 3);
    assert!(keys.borrow().is_empty());
    for key in ["down", "pagedown", "end"] {
        keys.borrow_mut().clear();
        visual.simulate_keystrokes(key);
        settle(&mut visual, Step::Frame, 1);
        assert_eq!(
            *keys.borrow(),
            [key],
            "bottom edge must leave {key} to the ancestor"
        );
    }
}

fn copy_visible_markdown_line(
    visual: &mut VisualTestContext,
    start: gpui::Point<gpui::Pixels>,
) -> String {
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()))
    });
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    let end = point(start.x + px(180.0), start.y);
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("cmd-c");
    let copied = visual
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .expect("visible native text selection");
    assert!(
        copied.contains("mark"),
        "selection must come from visible paragraph text, got {copied:?}"
    );
    copied
}

#[gpui::test]
fn one_wrapped_markdown_item_keeps_end_and_up_positions_inside_its_viewport(
    cx: &mut TestAppContext,
) {
    let paragraph = (0..300)
        .map(|index| format!("mark{index:03} "))
        .collect::<String>();
    assert!(paragraph.len() < 4096 && !paragraph.contains('\n'));
    let source = NESTED_MARKDOWN_KEYS.replace_once(
        "source: \"Short reply\", height: \"content\"",
        &format!("source: {paragraph:?}, height: \"content\", max_height: 96, part_styles: #{{ root: style().padding(px(16)).border(px(2)) }}"),
    );
    let keys = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let captured = keys.clone();
    let (window, view) = Mount::new(source)
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-one-tall-item")
        .mount_root(cx, move |host, view| MarkdownKeyAncestor {
            host,
            view,
            keys: captured,
        });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(last_error(&mut visual, &view), None);
    let rect = bounds(&mut visual, &view, "document", "Nested reply");
    // The scripted root has 16px padding plus a 2px border. The row must wrap
    // inside that inset content width, while max_height bounds its native list.
    let position = point(px((rect.x + 22.0) as f32), px((rect.y + 28.0) as f32));
    let initial = copy_visible_markdown_line(&mut visual, position);
    assert!(initial.contains("000"), "initial top line: {initial:?}");
    visual.simulate_keystrokes("alt-cmd-c");
    let raw = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(
        raw.as_deref(),
        Some(paragraph.as_str()),
        "fixture is exactly one real paragraph below the native segment boundary"
    );

    visual.simulate_keystrokes("end");
    settle(&mut visual, Step::Frame, 3);
    assert!(
        keys.borrow().is_empty(),
        "End must scroll the tall native item"
    );
    let at_end = copy_visible_markdown_line(&mut visual, position);
    assert_ne!(
        at_end, initial,
        "End must remain scrolled after layout/remeasure, not merely consume a key"
    );

    visual.simulate_keystrokes("up");
    settle(&mut visual, Step::Frame, 3);
    assert!(
        keys.borrow().is_empty(),
        "Up must move within the same tall item after End"
    );
    let after_up = copy_visible_markdown_line(&mut visual, position);
    assert_ne!(
        after_up, at_end,
        "Up must change the visible native text after layout"
    );
    visual.simulate_keystrokes("home");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(copy_visible_markdown_line(&mut visual, position), initial);
    visual.simulate_keystrokes("up");
    assert_eq!(
        *keys.borrow(),
        ["up"],
        "at the true top edge, Up belongs to the ancestor"
    );
}

fn markdown_nested_scroll_fixture(source: &str) -> String {
    format!(
        r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) {{
    box([column([
        text("Outer marker").with_style(style().height(px(20))),
        markdown::MarkdownViewer(#{{key: "scroll", label: "Scroll reply", source: {source:?}}})
            .with_style(style().width(px(420)).height(px(180))),
        box([]).with_style(style().height(px(400))),
    ]).with_style(style().width(px(420)))])
        .with_key("outer")
        .with_style(style().width(px(420)).height(px(240)).overflow_scroll())
}}
"#
    )
}

fn markdown_wheel(
    visual: &mut VisualTestContext,
    position: gpui::Point<gpui::Pixels>,
    x: f32,
    y: f32,
) {
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(x), px(y))),
        ..Default::default()
    });
    settle(visual, Step::Frame, 2);
}

fn assert_scroll_delta(actual: f64, expected: f64, message: &str) {
    assert!(
        (actual - expected).abs() < 0.6,
        "{message}: got {actual}, expected {expected}"
    );
}

#[gpui::test]
fn vertical_wheel_over_a_wide_markdown_table_never_becomes_horizontal_scroll(
    cx: &mut TestAppContext,
) {
    // A legal header-only table has horizontal overflow but no internal Y
    // range. Its surrounding document can consume the vertical wheel instead.
    let source = format!(
        "| Left | Middle | Right |\n| --- | --- | --- |\n\n{}",
        "Following paragraph.\n\n".repeat(30)
    );
    let (mut visual, view) = Mount::new(markdown_nested_scroll_fixture(&source))
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-table-wheel-axis")
        .mount(cx);
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(last_error(&mut visual, &view), None);
    let header = visual.debug_bounds("markdown-cell:0:0:0").unwrap();
    let outer = bounds(&mut visual, &view, "text", "Outer marker");
    let doc_before = bounds(&mut visual, &view, "document", "Scroll reply");
    markdown_wheel(
        &mut visual,
        point(header.left() + px(100.0), header.top() + px(12.0)),
        0.0,
        -24.0,
    );
    let after_y = visual.debug_bounds("markdown-cell:0:0:0").unwrap();
    let outer_after = bounds(&mut visual, &view, "text", "Outer marker");
    let doc_after = bounds(&mut visual, &view, "document", "Scroll reply");
    assert_scroll_delta(
        f64::from(after_y.left() - header.left()),
        0.0,
        "a vertical body wheel must not shift table columns",
    );
    assert_scroll_delta(
        f64::from(after_y.top() - header.top()),
        -24.0,
        &format!(
            "the Markdown reader consumes the supported vertical axis exactly once; header {header:?} -> {after_y:?}; document {doc_before:?} -> {doc_after:?}; SDK ancestor {outer:?} -> {outer_after:?}"
        ),
    );
    assert_scroll_delta(
        bounds(&mut visual, &view, "text", "Outer marker").y - outer.y,
        0.0,
        "the background must not scroll while this document can consume Y",
    );
    let doc = bounds(&mut visual, &view, "document", "Scroll reply");
    markdown_wheel(
        &mut visual,
        point(after_y.left() + px(100.0), px((doc.y + 3.0) as f32)),
        -40.0,
        0.0,
    );
    let after_x = visual.debug_bounds("markdown-cell:0:0:0").unwrap();
    assert_scroll_delta(
        f64::from(after_x.left() - after_y.left()),
        -40.0,
        "explicit horizontal wheel still scrolls the wide table",
    );
}

fn markdown_cell_track(visual: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    let cell = visual.debug_bounds("markdown-cell-viewport:0:1:0").unwrap();
    point(cell.right() - px(6.0), cell.top().max(px(0.0)) + px(40.0))
}

#[gpui::test]
fn a_wide_markdown_table_keeps_its_vertical_track_inside_the_visible_width(
    cx: &mut TestAppContext,
) {
    let mut source = "| Left | Middle | Right |\n| --- | --- | --- |\n".to_owned();
    for index in 0..40 {
        source.push_str(&format!(
            "| row {index} | middle {index} | right {index} |\n"
        ));
    }
    for rtl in [false, true] {
        for content in [true, false] {
            check_wide_markdown_table_track(cx, &source, rtl, content);
        }
    }
}

fn check_wide_markdown_table_track(
    cx: &mut TestAppContext,
    source: &str,
    rtl: bool,
    content: bool,
) {
    // Content height has no document bar. Fill height has both bars and must
    // reserve one logical track inward so the parent cannot cover the table.
    let locale = if rtl { "ar" } else { "en" };
    let height = if content { "content" } else { "fill" };
    let fixed_height = if content { "" } else { ".height(px(180))" };
    let main = format!(
        r#"
import "components/markdown_viewer" as markdown;
fn init(ctx) {{ ctx.set_locale("{locale}"); }}
fn view(ctx) {{ markdown::MarkdownViewer(#{{key: "wide", label: "Wide table",
    source: {source:?}, height: "{height}"}}).with_style(style().width(px(420)){fixed_height}) }}
"#
    );
    let (mut visual, view) = Mount::new(main)
        .module("components/markdown_viewer", COMPONENT)
        .locale(
            "en.rhai",
            include_str!("../../../../../registry/locales/en.rhai"),
        )
        .locale(
            "ar.rhai",
            include_str!("../../../../../registry/locales/ar.rhai"),
        )
        .id(&format!("markdown-visible-y-track-{locale}-{height}"))
        .mount(cx);
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(last_error(&mut visual, &view), None);
    let table = visual.debug_bounds("markdown-table:0").unwrap();
    let header = visual.debug_bounds("markdown-cell:0:0:0").unwrap();
    assert!(
        table.size.width < px(500.0),
        "logical 600px table is clipped: {table:?}"
    );
    let inset = if content {
        px(0.0)
    } else {
        gpui_rhai::SCROLLBAR_TRACK_WIDTH
    };
    let x = if rtl {
        table.left() + inset + px(6.0)
    } else {
        table.right() - inset - px(6.0)
    };
    let track = point(x, table.top() + px(if content { 240.0 } else { 140.0 }));
    visual.simulate_mouse_move(track, None, Modifiers::default());
    settle(&mut visual, Step::Frame, 1);
    visual.simulate_mouse_down(track, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(track, MouseButton::Left, Modifiers::default());
    settle(&mut visual, Step::Frame, 3);
    let after = visual.debug_bounds("markdown-table:0").unwrap();
    assert_eq!(after, table, "the table viewport itself must not move");
    assert!(
        visual.debug_bounds("markdown-cell:0:0:0").is_none(),
        "clicking the visible Y track must scroll past the header, not miss at the offscreen 600px logical edge"
    );
    let later_cell = [
        "markdown-cell:0:10:0",
        "markdown-cell:0:20:0",
        "markdown-cell:0:30:0",
    ]
    .into_iter()
    .find_map(|selector| visual.debug_bounds(selector))
    .expect("the track click realizes later logical rows inside the same viewport");
    assert_scroll_delta(
        f64::from(later_cell.left() - header.left()),
        0.0,
        "the visible vertical track does not alter horizontal scroll",
    );
}

#[gpui::test]
fn markdown_cell_bar_routes_only_residual_to_table_then_document_then_sdk_parent(
    cx: &mut TestAppContext,
) {
    let mut source = format!(
        "| Left | Right |\n| --- | --- |\n| {} | side |\n",
        "cellword ".repeat(700)
    );
    for index in 0..24 {
        source.push_str(&format!("| row {index} | right {index} |\n"));
    }
    source.push_str(&format!("\n{}", "Following paragraph.\n\n".repeat(30)));
    let (mut visual, view) = Mount::new(markdown_nested_scroll_fixture(&source))
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-cell-wheel-chain")
        .mount(cx);
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(last_error(&mut visual, &view), None);

    // Put the SDK parent away from its top so an incorrectly ordered ancestor
    // would visibly steal the upward wheel. This point is below the document.
    markdown_wheel(&mut visual, point(px(210.0), px(220.0)), 0.0, -30.0);
    let outer = bounds(&mut visual, &view, "text", "Outer marker");
    assert_scroll_delta(outer.y, -30.0, "fixture scrolls only the SDK parent");
    let table = visual.debug_bounds("markdown-table:0").unwrap();
    let cell = visual.debug_bounds("markdown-cell-viewport:0:1:0").unwrap();
    // Table width is 2 * 200px. Its own bar is distinct from both the 420px
    // document bar and the first cell's bar.
    let table_track = point(table.left() + px(394.0), px(70.0));
    markdown_wheel(&mut visual, table_track, 0.0, -24.0);
    let cell_after_table = visual.debug_bounds("markdown-cell-viewport:0:1:0").unwrap();
    assert_scroll_delta(
        f64::from(cell_after_table.top() - cell.top()),
        -24.0,
        "the table bar first moves the native table rows",
    );

    let track = markdown_cell_track(&mut visual);
    markdown_wheel(&mut visual, track, 0.0, 12.0);
    let cell_after_residual = visual.debug_bounds("markdown-cell-viewport:0:1:0").unwrap();
    assert_scroll_delta(
        f64::from(cell_after_residual.top() - cell_after_table.top()),
        12.0,
        "a top-edge cell passes upward residual to its nearest table",
    );
    assert_scroll_delta(
        bounds(&mut visual, &view, "text", "Outer marker").y - outer.y,
        0.0,
        "the table consumes before the scrollable SDK background",
    );
    markdown_wheel(&mut visual, table_track, 0.0, 12.0);

    let doc = bounds(&mut visual, &view, "document", "Scroll reply");
    let doc_track = point(px((doc.x + doc.width - 6.0) as f32), px(80.0));
    let table_before_doc = visual.debug_bounds("markdown-table:0").unwrap();
    markdown_wheel(&mut visual, doc_track, 0.0, -24.0);
    let table_after_doc = visual.debug_bounds("markdown-table:0").unwrap();
    assert_scroll_delta(
        f64::from(table_after_doc.top() - table_before_doc.top()),
        -24.0,
        "the document bar establishes its own native scroll offset",
    );
    let track = markdown_cell_track(&mut visual);
    markdown_wheel(&mut visual, track, 0.0, 12.0);
    let table_after_cell = visual.debug_bounds("markdown-table:0").unwrap();
    assert_scroll_delta(
        f64::from(table_after_cell.top() - table_after_doc.top()),
        12.0,
        "a top-edge cell and table pass residual to this Markdown reader",
    );
    assert_scroll_delta(
        bounds(&mut visual, &view, "text", "Outer marker").y - outer.y,
        0.0,
        "the document consumes before the SDK background",
    );
    let track = markdown_cell_track(&mut visual);
    markdown_wheel(&mut visual, track, 0.0, 20.0);
    assert_scroll_delta(
        bounds(&mut visual, &view, "text", "Outer marker").y - outer.y,
        8.0,
        "only the final 8px left after the reader's 12px reaches the SDK parent once",
    );
}

#[gpui::test]
fn markdown_image_rendering_only_resolves_prepared_host_assets(cx: &mut TestAppContext) {
    let mut fallback_height = 0.0;
    for preload in [false, true] {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let captured = calls.clone();
        let (mut visual, view) = Mount::new(
            r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) {
    markdown::MarkdownViewer(#{ key: "image", label: "Asset reply", height: "content",
        source: "![Fixture alt](https://example.invalid/cover.png)",
        image_assets: #{ "https://example.invalid/cover.png": "markdown/cover" },
    }).with_style(style().width(px(420)))
}
"#,
        )
        .module("components/markdown_viewer", COMPONENT)
        .with(move |view| {
            view.extension(MarkdownImageAssets {
                calls: captured,
                preload,
            })
        })
        .id(if preload {
            "markdown-image-prepared"
        } else {
            "markdown-image-pending"
        })
        .mount(cx);
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(last_error(&mut visual, &view), None);
        assert_eq!(
            calls.get(),
            usize::from(preload),
            "native Markdown rendering must not invoke the provider; only Host setup can preload"
        );
        if preload {
            for _ in 0..12 {
                if bounds(&mut visual, &view, "document", "Asset reply").height
                    > fallback_height + 48.0
                {
                    break;
                }
                settle(&mut visual, Step::Frame, 1);
            }
        }
        let rect = bounds(&mut visual, &view, "document", "Asset reply");
        if preload {
            assert!(
                rect.height > fallback_height + 48.0,
                "the prepared 64px Host image must participate in native layout: {rect:?}, alt-only height {fallback_height}"
            );
        } else {
            fallback_height = rect.height;
            assert!(
                fallback_height > 0.0 && fallback_height < 48.0,
                "alt-only layout: {rect:?}"
            );
        }
        let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
        visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        visual.simulate_keystrokes("cmd-a cmd-c");
        let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
        assert_eq!(copied.as_deref(), Some("Fixture alt"));
        assert_eq!(
            calls.get(),
            usize::from(preload),
            "selection and repaint stay cache-only"
        );
    }
}

#[gpui::test]
fn markdown_selection_spans_blocks_and_plain_source_copy_are_distinct(cx: &mut TestAppContext) {
    let source = "**Alpha 世界**\n\nSecond paragraph\n\n```rhai\nlet value = 42;\n```\n";
    let main = format!(
        r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) {{ markdown::MarkdownViewer(#{{ key: "message", label: "Reply", source: {source:?} }}) }}
"#
    );
    let (mut visual, view) = Mount::new(main)
        .module("components/markdown_viewer", COMPONENT)
        .id("markdown-copy")
        .mount(cx);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let rect = bounds(&mut visual, &view, "document", "Reply");
    assert!(rect.width > 200.0 && rect.height > 100.0, "{rect:?}");
    let start = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
    let end = point(px((rect.x + 125.0) as f32), px((rect.y + 44.0) as f32));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("cmd-c");
    settle(&mut visual, Step::Frame, 1);
    let selected = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert!(
        selected.as_deref().is_some_and(|text| text.contains("世界")
            && text.contains('\n')
            && text.contains("Second")),
        "cross-block pointer selection: {selected:?}"
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_keystrokes("cmd-c");
    let plain = visual
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .unwrap();
    assert!(
        plain.contains("Alpha 世界\n\nSecond paragraph"),
        "{plain:?}"
    );
    assert!(!plain.contains("**") && !plain.contains("```"));
    visual.simulate_keystrokes("alt-cmd-c");
    let raw = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(raw.as_deref(), Some(source));
}

#[gpui::test]
fn mounted_documents_register_stream_release_and_reject_disposed_owners(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) {
    let doc = ();
    try { doc = ctx.get_native_text_document("reply"); } catch (error) { () }
    column([
        text(if !is_native_text_document(doc) { "pending" } else { `revision:${doc.revision}` }),
        if !is_native_text_document(doc) { text("Waiting for a Host document") } else {
            markdown::MarkdownViewer(#{ key: "reply", label: "Streamed reply", source: doc })
        }
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)))
}
"#,
    )
    .module("components/markdown_viewer", COMPONENT)
    .id("markdown-register")
    .mount_root(cx, |host, view| ActiveHost { host, view });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 2);
    assert!(palette_texts(&mut visual, &view).contains(&"pending".to_owned()));
    visual
        .update(|_, cx| {
            view.register_native_text_document(
                "reply",
                NativeTextDocument::new("reply/1", 1, "[earlier][ref]\n\nStreaming").unwrap(),
                cx,
            )
        })
        .unwrap();
    settle(&mut visual, Step::Tick, 4);
    let rendered = palette_texts(&mut visual, &view);
    assert!(
        rendered.contains(&"revision:1".to_owned()),
        "texts={rendered:?}; error={:?}",
        last_error(&mut visual, &view)
    );
    visual.update(|_, cx| {
        assert!(
            view.register_native_text_document(
                "reply",
                NativeTextDocument::new("reply/1", 2, "duplicate").unwrap(),
                cx
            )
            .is_err()
        );
        for revision in 2..=12 {
            view.replace_native_text_document(
                "reply",
                NativeTextDocument::new(
                    "reply/1",
                    revision,
                    format!("[earlier][ref]\n\nStreaming {revision}\n\n[ref]: https://example.com"),
                )
                .unwrap(),
                cx,
            )
            .unwrap();
        }
        assert!(
            view.replace_native_text_document(
                "reply",
                NativeTextDocument::new("reply/1", 11, "stale").unwrap(),
                cx
            )
            .is_err()
        );
    });
    settle(&mut visual, Step::Tick, 4);
    assert!(palette_texts(&mut visual, &view).contains(&"revision:12".to_owned()));
    assert_eq!(last_error(&mut visual, &view), None);
    visual
        .update(|_, cx| view.release_native_text_document("reply", cx))
        .unwrap();
    settle(&mut visual, Step::Tick, 2);
    assert!(palette_texts(&mut visual, &view).contains(&"pending".to_owned()));
    visual.update(|_, cx| {
        view.dispose(cx).unwrap();
        assert!(
            view.register_native_text_document(
                "reply",
                NativeTextDocument::new("reply/2", 1, "late").unwrap(),
                cx
            )
            .is_err()
        );
        assert!(
            view.replace_native_text_document(
                "reply",
                NativeTextDocument::new("reply/2", 2, "late").unwrap(),
                cx
            )
            .is_err()
        );
        assert!(view.release_native_text_document("reply", cx).is_err());
    });
}

#[gpui::test]
fn streamed_content_height_remeasures_its_owning_virtual_row(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
import "components/markdown_viewer" as markdown;
fn message(ctx, payload) {
    if payload.key == "reply" {
        let source = "";
        try { source = ctx.get_native_text_document("stream"); } catch (error) { () }
        column([
            text(if is_native_text_document(source) {
                `source-revision:${source.revision}`
            } else { "source-revision:missing" }),
            markdown::MarkdownViewer(#{ key: "reply", label: "Growing reply",
                source: source, height: "content" }),
        ])
    } else {
        text(payload.key).with_style(style().height(px(24)))
    }
}
fn view(ctx) {
    virtual_collection(#{ key: "messages", label: "Messages", height: 300,
        estimated_height: 24, overdraw_pixels: 120,
        data: [#{key: "before"}, #{key: "reply"}, #{key: "after"}]
    }, Fn("message")).with_style(style().width(px(420)))
}
"#,
    )
    .module("components/markdown_viewer", COMPONENT)
    .id("markdown-streamed-intrinsic-height")
    .mount(cx);
    visual
        .update(|_, cx| {
            view.register_native_text_document(
                "stream",
                NativeTextDocument::new("stream/reply", 0, "").unwrap(),
                cx,
            )
        })
        .unwrap();
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        palette_texts(&mut visual, &view).contains(&"source-revision:0".to_owned()),
        "the virtual row must first read the Host's empty revision zero"
    );
    let empty = bounds(&mut visual, &view, "document", "Growing reply");
    let after_empty = bounds(&mut visual, &view, "text", "after");
    assert!(empty.height < 1.0, "revision zero is empty: {empty:?}");

    visual
        .update(|_, cx| {
            view.replace_native_text_document(
                "stream",
                NativeTextDocument::new(
                    "stream/reply",
                    1,
                    "## Native heading\n\nFirst streamed paragraph.\n\nSecond streamed paragraph.",
                )
                .unwrap(),
                cx,
            )
        })
        .unwrap();
    // First distinguish a missed Host-update wake from native preparation or
    // measured-height invalidation. This marker belongs to the script row and
    // is independent of the Markdown entity's background work and geometry.
    for _ in 0..12 {
        settle(&mut visual, Step::Frame, 1);
        if palette_texts(&mut visual, &view).contains(&"source-revision:1".to_owned()) {
            break;
        }
    }
    assert!(
        palette_texts(&mut visual, &view).contains(&"source-revision:1".to_owned()),
        "Host-update wake must deliver revision one to the virtual row before checking native geometry"
    );
    // Let async preparation commit and native layout run. There is no script
    // event, data-row edit, scroll, focus or root refresh to hide a missing
    // descendant-intrinsic-size invalidation.
    for _ in 0..12 {
        settle(&mut visual, Step::Frame, 1);
        if bounds(&mut visual, &view, "document", "Growing reply").height > 60.0 {
            break;
        }
    }
    assert_eq!(last_error(&mut visual, &view), None);
    let grown = bounds(&mut visual, &view, "document", "Growing reply");
    let after_grown = bounds(&mut visual, &view, "text", "after");
    assert!(
        grown.height > 60.0,
        "async content must gain height: {grown:?}"
    );
    assert!(
        after_grown.y >= grown.y + grown.height - 0.5,
        "outer measured row must accommodate its new content: {grown:?}, {after_grown:?}"
    );
    assert!(
        after_grown.y > after_empty.y + 60.0,
        "the next logical row must move after async preparation: {after_empty:?} -> {after_grown:?}"
    );
}

#[gpui::test]
fn markdown_copy_is_isolated_to_one_viewer(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) { row([
    markdown::MarkdownViewer(#{ key: "a", label: "First", source: "**One**\n\nFirst only" }),
    markdown::MarkdownViewer(#{ key: "b", label: "Second", source: "**Two**\n\nSecond only" }),
]).with_style(style().width(relative(1.0)).height(relative(1.0))) }
"#,
    )
    .module("components/markdown_viewer", COMPONENT)
    .id("markdown-isolation")
    .mount(cx);
    settle(&mut visual, Step::Frame, 4);
    for (label, expected, excluded) in [("First", "One", "Two"), ("Second", "Two", "One")] {
        let rect = bounds(&mut visual, &view, "document", label);
        let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
        visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        visual.simulate_keystrokes("cmd-a");
        visual.simulate_keystrokes("cmd-c");
        let copied = visual
            .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .unwrap();
        assert!(
            copied.contains(expected) && !copied.contains(excluded),
            "{label}: {copied:?}"
        );
    }
}

#[gpui::test]
fn markdown_links_and_code_copy_are_keyboard_accessible_without_opening_urls(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(r###"
import "components/markdown_viewer" as markdown;
fn state_schema() { #{ fields: #{ link: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "none" } } } } }
fn link(ctx, payload) { ctx.set_state("link", `${payload.href}|${payload.text}`); }
fn view(ctx) { column([
    text(`last:${ctx.get_state("link")}`),
    markdown::MarkdownViewer(#{ key: "reply", label: "Linked reply", source: "## Heading\n\n[**Open** example](https://example.com/a_(b))\n\n```rhai\nlet value = 42;\n```\n", on_link: Fn("link") }),
]).with_style(style().width(relative(1.0)).height(relative(1.0))) }
"###).module("components/markdown_viewer", COMPONENT).id("markdown-keyboard").mount(cx);
    settle(&mut visual, Step::Frame, 4);
    let rect = bounds(&mut visual, &view, "document", "Linked reply");
    let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("tab");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        palette_texts(&mut visual, &view)
            .contains(&"last:https://example.com/a_(b)|Open example".to_owned())
    );
    visual.simulate_keystrokes("tab");
    visual.simulate_keystrokes("enter");
    let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("let value = 42;\n"));
    // The headless platform does not activate assistive technology, so native
    // AccessKit nodes are gated. The keyboard path above exercises their real
    // focus handles and actions; the runtime document semantic root remains
    // available through the production snapshot used by `bounds`.
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn markdown_large_host_document_exceeds_rhai_string_limit_without_entering_rhai_values(
    cx: &mut TestAppContext,
) {
    #[derive(Clone)]
    struct LargeDocument;
    impl gpui_rhai::ScriptViewExtension for LargeDocument {
        fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
            let text = format!("**{}**\n\nEnd of native document", "世界".repeat(180_000));
            assert!(text.len() > 1024 * 1024);
            runtime
                .native_documents
                .register("large", NativeTextDocument::new("large", 1, text).unwrap())
                .map_err(|error| error.to_string())
        }
    }
    let (mut visual, view) = Mount::new(r#"
import "components/markdown_viewer" as markdown;
fn view(ctx) { markdown::MarkdownViewer(#{ key: "large", label: "Large native Markdown", source: ctx.get_native_text_document("large") }) }
"#).module("components/markdown_viewer", COMPONENT).with(|view| view.extension(LargeDocument)).id("markdown-large-native").mount(cx);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let rect = bounds(&mut visual, &view, "document", "Large native Markdown");
    let position = point(px((rect.x + 2.0) as f32), px((rect.y + 10.0) as f32));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_keystrokes("cmd-c");
    let plain = visual
        .update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
        .unwrap();
    assert!(plain.len() > 1024 * 1024);
    assert!(plain.starts_with("世界") && plain.ends_with("End of native document"));
    assert!(!plain.contains("**"));
}
