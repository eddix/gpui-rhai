//! Table context requests (#89), for array rows and a NativeCollection alike: a
//! right press on a cell selects its row (unless the selection holds it) and
//! asks for a menu at the pointer; Shift+F10 asks for one at the current row, or for
//! the table without a selection mode.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;

const ROWS: [(&str, &str, &str); 3] = [
    ("r0", "Alpha", "A"),
    ("r1", "Bravo", "A"),
    ("r2", "Charlie", "B"),
];

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

#[derive(Clone)]
struct Rows;

impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let rows = ROWS.iter().map(|(id, label, group)| {
            BTreeMap::from([
                ("id".into(), UiValue::String((*id).into())),
                ("label".into(), UiValue::String((*label).into())),
                ("group".into(), UiValue::String((*group).into())),
            ])
        });
        runtime
            .native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|error| error.to_string())
    }
}

fn script(native: bool, mode: &str, selected: &str) -> String {
    let rows = if native {
        r#"ctx.get_native_collection("rows")"#.to_owned()
    } else {
        let items = ROWS
            .iter()
            .map(|(id, label, group)| format!(r#"#{{id:"{id}",label:"{label}",group:"{group}"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    };
    format!(
        r#"import "components/table" as table;
fn state_schema(){{#{{fields:#{{
    selected:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"{selected}"}}}},
    request:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn changed(ctx,keys){{ctx.set_state("selected",if keys.len>0{{keys[0]}}else{{""}});}}
fn asked(ctx,request){{ctx.set_state("request",
    `${{request.key}}|${{request.column}}|${{request.source}}|${{request.anchor.x}},${{request.anchor.y}},${{request.anchor.width}},${{request.anchor.height}}`);}}
fn view(ctx){{column([
    table::Table(#{{key:"rows",label:"Rows",row_key:"id",height:200,rows:{rows},
        columns:[#{{key:"label",title:"Label",width:#{{kind:"fixed",value:160}}}},
            #{{key:"group",title:"Group",width:#{{kind:"fixed",value:80}}}}],
        selection_mode:"{mode}",selected_keys:if ctx.get_state("selected")=="" {{[]}} else {{[ctx.get_state("selected")]}},
        on_selection_change:Fn("changed"),on_context_request:Fn("asked")}}),
    text(`${{ctx.get_state("selected")}}#${{ctx.get_state("request")}}`).test_id("state"),
]).with_style(style().width(px(400)))}}"#
    )
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .extension(Rows)
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("context", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("context"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    (visual, view)
}

fn state(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.test_id.as_deref() == Some("state"))
            .unwrap()
            .name
            .clone()
    })
}

/// The bounds of a row by its data label (rows are named `Rows row <n>`).
fn row_bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    label: &str,
) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| {
                let position = ROWS.iter().position(|(_, row, _)| *row == label).unwrap();
                node.role == "row" && node.name == format!("Rows row {}", position + 1)
            })
            .and_then(|node| node.geometry)
            .map(|geometry| geometry.visual)
            .unwrap_or_else(|| {
                let names = view
                    .accessibility_snapshot(cx)
                    .unwrap()
                    .nodes()
                    .map(|node| format!("{}:{}", node.role, node.name))
                    .collect::<Vec<_>>();
                panic!("no row {label}: {names:?}")
            })
    })
}

fn settle(visual: &mut VisualTestContext) {
    visual.run_until_parked();
    visual.executor().advance_clock(Duration::from_millis(32));
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
}

fn right_press(visual: &mut VisualTestContext, x: f64, y: f64) {
    #[allow(clippy::cast_possible_truncation)]
    let position = point(px(x as f32), px(y as f32));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Right, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Right, Modifiers::none());
    settle(visual);
}

#[gpui::test]
fn a_right_press_selects_the_row_and_asks_at_the_pointer(cx: &mut TestAppContext) {
    for native in [false, true] {
        let (mut visual, view) = mount(cx, script(native, "single", ""));
        let bravo = row_bounds(&mut visual, &view, "Bravo");
        // The group cell starts 160px into the row.
        let (x, y) = (bravo.x + 200.0, bravo.y + bravo.height / 2.0);
        right_press(&mut visual, x, y);
        assert_eq!(
            state(&mut visual, &view),
            format!("r1#r1|group|pointer|{x:?},{y:?},0.0,0.0"),
            "native {native}"
        );
        // A right press on another row replaces the selection.
        let charlie = row_bounds(&mut visual, &view, "Charlie");
        right_press(&mut visual, charlie.x + 20.0, charlie.y + 10.0);
        assert!(state(&mut visual, &view).starts_with("r2#r2|label|pointer"));
    }
}

#[gpui::test]
fn a_right_press_keeps_a_selection_that_holds_the_row(cx: &mut TestAppContext) {
    for native in [false, true] {
        let (mut visual, view) = mount(cx, script(native, "multiple", "r0"));
        let alpha = row_bounds(&mut visual, &view, "Alpha");
        right_press(&mut visual, alpha.x + 20.0, alpha.y + 10.0);
        assert!(
            state(&mut visual, &view).starts_with("r0#r0|label|pointer"),
            "native {native}: {}",
            state(&mut visual, &view)
        );
    }
    // Without a selection mode nothing is selected; the request still comes.
    let (mut visual, view) = mount(cx, script(false, "none", ""));
    let bravo = row_bounds(&mut visual, &view, "Bravo");
    right_press(&mut visual, bravo.x + 20.0, bravo.y + 10.0);
    assert!(
        state(&mut visual, &view).starts_with("#r1|label|pointer"),
        "{}",
        state(&mut visual, &view)
    );
}

#[gpui::test]
fn shift_f10_asks_at_the_current_row(cx: &mut TestAppContext) {
    for native in [false, true] {
        let (mut visual, view) = mount(cx, script(native, "single", "r1"));
        visual.update(|window, cx| window.focus_next(cx));
        settle(&mut visual);
        visual.simulate_keystrokes("shift-f10");
        settle(&mut visual);
        let bravo = row_bounds(&mut visual, &view, "Bravo");
        assert_eq!(
            state(&mut visual, &view),
            format!(
                "r1#r1||keyboard|{:?},{:?},{:?},{:?}",
                bravo.x, bravo.y, bravo.width, bravo.height
            ),
            "native {native}"
        );
    }
}

#[gpui::test]
fn without_a_selection_mode_shift_f10_asks_for_the_table(cx: &mut TestAppContext) {
    for (native, key) in [(false, "shift-f10"), (true, "shift-f10"), (false, "menu")] {
        let (mut visual, view) = mount(cx, script(native, "none", ""));
        visual.update(|window, cx| window.focus_next(cx));
        settle(&mut visual);
        visual.simulate_keystrokes(key);
        settle(&mut visual);
        let table = visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("table", "Rows")
                .next()
                .and_then(|node| node.geometry)
                .unwrap()
                .visual
        });
        // Nothing is selected and the request is for the table, at its bounds.
        assert_eq!(
            state(&mut visual, &view),
            format!(
                "#||keyboard|{:?},{:?},{:?},{:?}",
                table.x, table.y, table.width, table.height
            ),
            "native {native}, {key}"
        );
    }
}
