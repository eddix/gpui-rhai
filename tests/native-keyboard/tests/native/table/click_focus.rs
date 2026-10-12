//! A click on a Table or List row leaves the keyboard focus where the keys act: the
//! collection itself, so the selected row shows its 2px `focus_ring` cursor frame, the
//! arrows keep moving the selection and the frame follows it. A row is a pointer target
//! only; were it to keep the focus the click gave it, the keys would still bubble to the
//! collection, but the cursor's `group_focus` owner (the collection) would not be focused
//! and no frame would show.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px, rgba,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
};

use crate::support::ReplaceOnce;

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

/// `body` is the collection, with `selected` (the stored keys) and `changed` in scope.
fn script(body: &str) -> String {
    format!(
        r#"import "components/table" as table;
import "components/list" as list;
fn state_schema(){{#{{fields:#{{
    selected:#{{schema:#{{type:"array",items:#{{type:"string"}}}},"default":#{{type:"array",value:[]}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn changed(ctx,keys){{ctx.set_state("selected",keys);
    ctx.set_state("log",`${{ctx.get_state("log")}} sel:${{keys}}`);}}
fn asked(ctx,request){{ctx.set_state("log",`${{ctx.get_state("log")}} ctx:${{request.key}}:${{request.source}}`);}}
fn view(ctx){{let selected=ctx.get_state("selected");column([{body},
    text(ctx.get_state("log")).test_id("log")])
    .with_style(style().width(px(480)).padding(px(16)))}}"#
    )
}

const TABLE: &str = r#"table::Table(#{key:"t",label:"Hosts",row_key:"id",height:160,
    columns:[#{key:"h",title:"Host",width:#{kind:"flex",value:1.0}}],
    rows:[#{id:"a",h:"Alpha"},#{id:"b",h:"Bravo"},#{id:"c",h:"Charlie"}],
    selection_mode:"single",selected_keys:selected,on_selection_change:Fn("changed"),
    on_context_request:Fn("asked")})"#;

const LIST: &str = r#"list::List(#{key:"l",label:"Hosts",height:160.0,
    items:[#{key:"a",title:"Alpha"},#{key:"b",title:"Bravo"},#{key:"c",title:"Charlie"}],
    selection_mode:"single",selected_keys:selected,on_selection_change:Fn("changed"),
    on_context_request:Fn("asked")})"#;

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), source)]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
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
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("click-focus", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("click-focus"),
                host.clone(),
                window,
                cx,
            )
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
    for _ in 0..2 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        assert_eq!(view.last_error(cx).unwrap(), None);
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_test_id("log")
            .next()
            .unwrap()
            .name
            .trim()
            .to_owned()
    })
}

/// The bounds of the row whose accessible name starts with `title`.
fn row(visual: &mut VisualTestContext, view: &ScriptViewHandle, title: &str) -> GeometryBounds {
    visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        tree.nodes()
            .find(|node| {
                (node.role == "row" || node.role == "option")
                    && (node.name.starts_with(title)
                        || ["Alpha", "Bravo", "Charlie"]
                            .iter()
                            .position(|name| *name == title)
                            .is_some_and(|index| node.name == format!("Hosts row {}", index + 1)))
            })
            .and_then(|node| node.geometry)
            .map(|geometry| geometry.visual)
            .unwrap_or_else(|| panic!("no row {title}"))
    })
}

/// (top, width, height) of the quads whose border is drawn in the focus color.
fn frames(visual: &mut VisualTestContext) -> Vec<(f32, f32, f32)> {
    let color: gpui::Hsla = rgba(FOCUS).into();
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_color == color && quad.border_widths.top.0 > 0.0)
            .map(|quad| {
                (
                    quad.bounds.origin.y.0 / scale,
                    quad.bounds.size.width.0 / scale,
                    quad.bounds.size.height.0 / scale,
                )
            })
            .collect()
    })
}

/// Whether a focus frame covers the row `bounds` (a Table row's frame stops above its 1px
/// separator).
fn frames_row(found: &[(f32, f32, f32)], bounds: GeometryBounds) -> bool {
    #[allow(clippy::cast_possible_truncation)]
    let (top, width, height) = (bounds.y as f32, bounds.width as f32, bounds.height as f32);
    found.iter().any(|(y, w, h)| {
        (y - top).abs() < 0.5
            && (w - width).abs() < 0.5
            && (height - h) > -0.5
            && (height - h) < 1.5
    })
}

fn press(visual: &mut VisualTestContext, bounds: GeometryBounds, button: MouseButton) {
    #[allow(clippy::cast_possible_truncation)]
    let position = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, button, Modifiers::none());
    visual.simulate_mouse_up(position, button, Modifiers::none());
    settle(visual);
}

fn a_click_leaves_the_cursor_frame_on_the_selected_row(cx: &mut TestAppContext, body: &str) {
    let context = body.contains("on_context_request");
    let (mut visual, view) = mount(cx, script(body));
    assert!(frames(&mut visual).is_empty(), "no frame before focus");
    // Tab reaches the collection, its one tab stop.
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    let collection = visual
        .update(|window, cx| window.focused(cx))
        .expect("Tab focuses the collection");
    let bravo = row(&mut visual, &view, "Bravo");
    press(&mut visual, bravo, MouseButton::Left);
    assert_eq!(log(&mut visual, &view), r#"sel:["b"]"#);
    let focused = visual.update(|window, cx| window.focused(cx));
    let found = frames(&mut visual);
    println!(
        "after the click: focus still on the collection {}, row {bravo:?}, frames {found:?}",
        focused.as_ref() == Some(&collection)
    );
    assert!(
        focused.as_ref() == Some(&collection),
        "the click leaves the focus on the collection, not on the row it pressed"
    );
    assert!(
        frames_row(&found, bravo),
        "the clicked, selected row shows the cursor frame: row {bravo:?}, frames {found:?}"
    );
    // The keys act on the same collection and the frame follows the selection.
    visual.simulate_keystrokes("down");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"sel:["b"] sel:["c"]"#);
    let charlie = row(&mut visual, &view, "Charlie");
    let found = frames(&mut visual);
    assert!(
        frames_row(&found, charlie),
        "the frame follows Down: row {charlie:?}, frames {found:?}"
    );
    if !context {
        return;
    }
    // The keyboard context request still comes from the focused collection.
    visual.simulate_keystrokes("shift-f10");
    settle(&mut visual);
    assert_eq!(
        log(&mut visual, &view),
        r#"sel:["b"] sel:["c"] ctx:c:keyboard"#
    );
    // A right press asks for a menu for its row and selects it.
    let alpha = row(&mut visual, &view, "Alpha");
    press(&mut visual, alpha, MouseButton::Right);
    assert_eq!(
        log(&mut visual, &view),
        r#"sel:["b"] sel:["c"] ctx:c:keyboard sel:["a"] ctx:a:pointer"#
    );
}

#[gpui::test]
fn a_click_on_a_table_row_leaves_the_cursor_frame_on_it(cx: &mut TestAppContext) {
    a_click_leaves_the_cursor_frame_on_the_selected_row(cx, TABLE);
}

#[gpui::test]
fn a_click_on_a_list_row_leaves_the_cursor_frame_on_it(cx: &mut TestAppContext) {
    a_click_leaves_the_cursor_frame_on_the_selected_row(cx, LIST);
}

/// The same without `on_context_request`: the selection mode alone makes the collection
/// take keys.
fn without_context_requests(body: &str) -> String {
    let body = body.replace_once(",\n    on_context_request:Fn(\"asked\")", "");
    assert!(!body.contains("on_context_request"), "{body}");
    body
}

#[gpui::test]
fn a_click_on_a_row_of_a_table_without_context_requests_keeps_the_focus(cx: &mut TestAppContext) {
    a_click_leaves_the_cursor_frame_on_the_selected_row(cx, &without_context_requests(TABLE));
}

#[gpui::test]
fn a_click_on_a_row_of_a_list_without_context_requests_keeps_the_focus(cx: &mut TestAppContext) {
    a_click_leaves_the_cursor_frame_on_the_selected_row(cx, &without_context_requests(LIST));
}
