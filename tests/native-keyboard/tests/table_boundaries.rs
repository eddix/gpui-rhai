// Real geometry and input regressions from the independent PR101 review.
use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
struct Rows;
impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        let rows = (0..1000).map(|i| {
            BTreeMap::from([
                ("id".into(), UiValue::String(format!("row-{i}"))),
                ("a".into(), UiValue::String("Alpha body".into())),
                ("b".into(), UiValue::String("Beta body".into())),
                (
                    "c".into(),
                    UiValue::String("Wide status value with many characters".into()),
                ),
            ])
        });
        r.native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|e| e.to_string())
    }
}
fn script(rtl: bool, native: bool, single: bool) -> String {
    let rows = if native {
        "ctx.get_native_collection(\"rows\")"
    } else {
        "[#{id:\"r\",a:\"Alpha body\",b:\"Beta body\",c:\"Wide status value with many characters\"}]"
    };
    let cols = if single {
        r#"[#{key:"c",title:"C",width:#{kind:"fixed",value:w.c},sortable:true,max_width:400}]"#
    } else {
        r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:w.a},sortable:true,max_width:400},#{key:"b",title:"B",width:#{kind:"fixed",value:w.b},sortable:true,max_width:400},#{key:"c",title:"C",width:#{kind:"fixed",value:w.c},sortable:true,max_width:400}]"#
    };
    format!(
        r#"
import "components/table" as table;
fn state_schema(){{#{{fields:#{{widths:#{{schema:#{{type:"map",values:#{{type:"number"}}}},"default":#{{type:"map",value:#{{a:#{{type:"float",value:120.0}},b:#{{type:"float",value:140.0}},c:#{{type:"float",value:160.0}}}}}}}},event:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}},sorted:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}}}}}}}}
fn init(ctx){{ctx.set_locale("{}");}}
fn resized(ctx,p){{let w=ctx.get_state("widths");w[p.key]=p.width.value;ctx.set_state("widths",w);ctx.set_state("event",`${{p.key}}:${{p.width.value}}`);}}
fn sorted(ctx,p){{ctx.set_state("sorted",p.key);}}
fn view(ctx){{let w=ctx.get_state("widths");column([text(`${{ctx.get_state("event")}}|${{ctx.get_state("sorted")}}`).accessibility_role("status"),table::Table(#{{key:"table",label:"Table",row_key:"id",rows:{rows},columns:{cols},height:180.0,resizable_columns:true,on_column_resize:Fn("resized"),on_sort_change:Fn("sorted")}})]).with_style(style().width(px(600)))}}
"#,
        if rtl { "ar" } else { "en" }
    )
}
fn mount(
    cx: &mut TestAppContext,
    script: String,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script),
            (
                ModuleId::parse("components/table").unwrap(),
                include_str!("../../../registry/components/table.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                include_str!("../../../registry/components/badge.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .locale_sources([
        (
            "en.rhai".into(),
            include_str!("../../../registry/locales/en.rhai").to_owned(),
        ),
        (
            "ar.rhai".into(),
            include_str!("../../../registry/locales/ar.rhai").to_owned(),
        ),
    ])
    .asset_sources([
        (
            "icons/disclosure_down".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../registry/assets/icons/disclosure_down.svg")
                    .to_vec(),
            },
        ),
        (
            "icons/chevron_right".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../registry/assets/icons/chevron_right.svg").to_vec(),
            },
        ),
        (
            "icons/sort_ascending".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../registry/assets/icons/sort_ascending.svg").to_vec(),
            },
        ),
        (
            "icons/sort_descending".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../registry/assets/icons/sort_descending.svg")
                    .to_vec(),
            },
        ),
    ])
    .extension(Rows)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}
fn bounds(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    })
}
fn status(v: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.role == "status")
            .unwrap()
            .name
            .clone()
    })
}
fn drag(v: &mut VisualTestContext, p: gpui::Point<gpui::Pixels>, dx: f32) {
    let q = point(p.x + px(dx), p.y);
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_move(q, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_up(q, MouseButton::Left, Modifiers::none());
    v.run_until_parked();
}
#[gpui::test]
fn ltr_array_boundary_and_last_column_owner(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(false, false, false), "pr101-ltr-array");
    let mut v = VisualTestContext::from_window(*window, cx);
    for title in ["A", "B"] {
        let h = bounds(&mut v, &view, "columnheader", title);
        let s = bounds(
            &mut v,
            &view,
            "separator",
            &format!("Resize {title} column"),
        );
        assert!((s.x + s.width / 2.0 - h.x - h.width).abs() < 0.5);
    }
    let h = bounds(&mut v, &view, "columnheader", "A");
    drag(
        &mut v,
        point(
            px((h.x + h.width + 2.0) as f32),
            px((h.y + h.height / 2.0) as f32),
        ),
        40.0,
    );
    assert_eq!(status(&mut v, &view), "a:160.0|none");
    let s = bounds(&mut v, &view, "separator", "Resize C column");
    drag(
        &mut v,
        point(
            px((s.x + s.width - 2.0) as f32),
            px((s.y + s.height / 2.0) as f32),
        ),
        40.0,
    );
    println!("LTR final={}", status(&mut v, &view));
    assert_eq!(status(&mut v, &view), "c:200.0|none");
}
#[gpui::test]
fn rtl_handles_follow_the_owned_columns_logical_end(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(true, false, false), "pr101-rtl-geometry");
    let mut v = VisualTestContext::from_window(*window, cx);
    let mut errors = Vec::new();
    for title in ["A", "B"] {
        let h = bounds(&mut v, &view, "columnheader", title);
        let s = bounds(
            &mut v,
            &view,
            "separator",
            &format!("Resize {title} column"),
        );
        let center = s.x + s.width / 2.0;
        println!(
            "RTL {title}: owned logical end={},handle center={center}",
            h.x
        );
        if (center - h.x).abs() > 0.5 {
            errors.push(title);
        }
    }
    let h = bounds(&mut v, &view, "columnheader", "C");
    let s = bounds(&mut v, &view, "separator", "Resize C column");
    println!("RTL last C: header left={},handle left={}", h.x, s.x);
    assert!(
        errors.is_empty() && (s.x - h.x).abs() < 0.5,
        "physical left/right placement moved handles away from their RTL column boundaries"
    );
}
#[gpui::test]
fn rtl_dragging_a_boundary_resizes_a_not_its_neighbor(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(true, true, false), "pr101-rtl-native-drag");
    let mut v = VisualTestContext::from_window(*window, cx);
    let h = bounds(&mut v, &view, "columnheader", "A");
    drag(
        &mut v,
        point(px((h.x - 2.0) as f32), px((h.y + h.height / 2.0) as f32)),
        -40.0,
    );
    let actual = status(&mut v, &view);
    println!("RTL real A boundary drag={actual}");
    assert_eq!(actual, "a:160.0|none");
}

#[gpui::test]
fn rtl_keyboard_arrows_resize_the_focused_logical_column(cx: &mut TestAppContext) {
    let source = script(true, true, false).replace("sortable:true", "sortable:false");
    let (window, view) = mount(cx, source, "rtl-keyboard");
    let mut v = VisualTestContext::from_window(*window, cx);
    fn reference(node: &UiNode) -> Option<ElementRef> {
        if let UiNodeKind::Custom { primitive } = node.kind()
            && primitive.primitive.as_str() == "gpui_rhai.column_resize"
            && primitive.props.data("column_key") == Some(&UiValue::String("a".into()))
        {
            return node.element_ref().cloned();
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                children.iter().find_map(reference)
            }
            _ => None,
        }
    }
    let reference = v
        .update(|_, cx| reference(&view.root(cx).unwrap().unwrap()))
        .unwrap();
    v.update(|window, cx| view.focus_element(&reference, window, cx))
        .unwrap();
    v.simulate_keystrokes("left");
    v.run_until_parked();
    assert_eq!(status(&mut v, &view), "a:128.0|none");
    v.simulate_keystrokes("right");
    v.run_until_parked();
    assert_eq!(status(&mut v, &view), "a:120.0|none");
}
#[gpui::test]
fn native_last_column_autofit_measures_last_column_without_sort(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(false, true, false), "pr101-native-autofit");
    let mut v = VisualTestContext::from_window(*window, cx);
    let h = bounds(&mut v, &view, "separator", "Resize C column");
    let p = point(
        px((h.x + h.width / 2.0) as f32),
        px((h.y + h.height / 2.0) as f32),
    );
    v.simulate_mouse_move(p, None, Modifiers::none());
    v.simulate_event(gpui::MouseDownEvent {
        button: MouseButton::Left,
        position: p,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    v.simulate_mouse_up(p, MouseButton::Left, Modifiers::none());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("native autofit={actual}");
    assert!(actual.starts_with("c:") && actual.ends_with("|none"));
    let h = bounds(&mut v, &view, "columnheader", "C");
    let c = bounds(
        &mut v,
        &view,
        "gridcell",
        "Wide status value with many characters",
    );
    assert!((h.width - c.width).abs() < 0.5);
}
#[gpui::test]
fn one_column_table_is_resizable(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(false, false, true), "pr101-single");
    let mut v = VisualTestContext::from_window(*window, cx);
    let h = bounds(&mut v, &view, "separator", "Resize C column");
    drag(
        &mut v,
        point(
            px((h.x + h.width / 2.0) as f32),
            px((h.y + h.height / 2.0) as f32),
        ),
        40.0,
    );
    assert_eq!(status(&mut v, &view), "c:200.0|none");
}
#[gpui::test]
fn nonresizable_neighbor_preserves_owner_and_last_keyboard_resize(cx: &mut TestAppContext) {
    let source = script(false, true, false).replace(
        "key:\"b\",title:\"B\",width:",
        "key:\"b\",title:\"B\",resizable:false,width:",
    );
    let (window, view) = mount(cx, source, "pr101-mixed-resizable");
    let mut v = VisualTestContext::from_window(*window, cx);
    assert!(v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("separator", "Resize B column")
            .next()
            .is_none()
    }));
    let h = bounds(&mut v, &view, "columnheader", "A");
    drag(
        &mut v,
        point(
            px((h.x + h.width + 2.0) as f32),
            px((h.y + h.height / 2.0) as f32),
        ),
        40.0,
    );
    assert_eq!(status(&mut v, &view), "a:160.0|none");
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::RoleName {
                    role: "separator".into(),
                    name: "Resize C column".into(),
                },
                event: "key:right".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("mixed native last keyboard={actual}");
    assert_eq!(actual, "c:168.0|none");
}
