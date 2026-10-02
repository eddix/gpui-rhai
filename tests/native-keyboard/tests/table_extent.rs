//! Real viewport/extent, shared-width and input regressions for Table.
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
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .native_collections
            .register(
                "rows",
                NativeCollection::new(
                    "id",
                    (0..1000).map(|index| {
                        BTreeMap::from([
                            ("id".into(), UiValue::String(format!("row-{index}"))),
                            ("a".into(), UiValue::String("Alpha".into())),
                            ("b".into(), UiValue::String("Beta".into())),
                            ("c".into(), UiValue::String("Gamma".into())),
                        ])
                    }),
                )
                .unwrap(),
            )
            .map_err(|error| error.to_string())
    }
}
fn script(rtl: bool, native: bool, widths: &str, controlled: bool) -> String {
    let rows = if native {
        "ctx.get_native_collection(\"rows\")"
    } else {
        "[#{id:\"row-0\",a:\"Alpha\",b:\"Beta\",c:\"Gamma\"},#{id:\"row-1\",a:\"Alpha\",b:\"Beta\",c:\"Gamma\"}]"
    };
    format!(
        r#"
import "components/table" as table;
fn state_schema() {{ #{{fields:#{{
    mode:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}},
    viewport:#{{schema:#{{type:"number"}},"default":#{{type:"float",value:300.0}}}},
    changed:#{{schema:#{{type:"number"}},"default":#{{type:"float",value:0.0}}}},
    sorted:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}}
}}}} }}
fn init(ctx) {{ ctx.set_locale("{}"); }}
fn mode(ctx, value) {{ ctx.set_state("mode",value); }}
fn viewport(ctx, value) {{ ctx.set_state("viewport",value); }}
fn sorted(ctx, value) {{ ctx.set_state("sorted",value.key); }}
fn resized(ctx, value) {{ ctx.set_state("changed",value.width.value); }}
fn view(ctx) {{
    let mode = ctx.get_state("mode");
    let columns = {widths};
    let changed=ctx.get_state("changed");
    if changed > 0.0 {{columns[columns.len-1].width=#{{kind:"fixed",value:changed}};}}
    let props = #{{key:"table",label:"Extent Table",row_key:"id",rows:if mode==2 {{[]}} else {{{rows}}},
        columns:columns,height:180.0,resizable_columns:true,loading:mode==1,
        query:if mode==3 {{"missing"}} else {{""}},search_fields:["a","b","c"],on_sort_change:Fn("sorted")}};
    {} 
    column([
        row([text("Data").accessibility_role("button").accessibility_label("Mode0").on_click_value(Fn("mode"),0),
            text("Loading").accessibility_role("button").accessibility_label("Mode1").on_click_value(Fn("mode"),1),
            text("Empty").accessibility_role("button").accessibility_label("Mode2").on_click_value(Fn("mode"),2),
            text("Filtered").accessibility_role("button").accessibility_label("Mode3").on_click_value(Fn("mode"),3),
            text("Wide").accessibility_role("button").accessibility_label("Viewport").on_click_value(Fn("viewport"),500.0)]).with_style(style().height(px(24))),
        text(ctx.get_state("sorted")).accessibility_role("status"),
        table::Table(props)
    ]).with_style(style().width(px(ctx.get_state("viewport"))))
}}
"#,
        if rtl { "ar" } else { "en" },
        if controlled {
            "props.on_column_resize=Fn(\"resized\");"
        } else {
            ""
        }
    )
}
const FIXED: &str = r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:120.0},sortable:true},#{key:"b",title:"B",width:#{kind:"fixed",value:140.0},sortable:true},#{key:"c",title:"C",width:#{kind:"fixed",value:160.0},sortable:true}]"#;
fn mount(
    cx: &mut TestAppContext,
    source: String,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    mount_with_overrides(cx, source, name, ThemeTokenOverrides::default())
}

fn mount_with_overrides(
    cx: &mut TestAppContext,
    source: String,
    name: &str,
    overrides: ThemeTokenOverrides,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    mount_with_styles(cx, source, name, overrides, "fn component_styles() { #{} }")
}

fn mount_with_styles(
    cx: &mut TestAppContext,
    source: String,
    name: &str,
    overrides: ThemeTokenOverrides,
    styles: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, source),
            (
                ModuleId::parse("components/table").unwrap(),
                include_str!("../../../registry/components/table.rhai").into(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                include_str!("../../../registry/components/badge.rhai").into(),
            ),
        ])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .locale_sources([
        (
            "en.rhai".into(),
            include_str!("../../../registry/locales/en.rhai").into(),
        ),
        (
            "ar.rhai".into(),
            include_str!("../../../registry/locales/ar.rhai").into(),
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
    .theme_token_overrides(overrides)
    .component_styles(styles)
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
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        snapshot
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| {
                panic!(
                    "missing {role}/{name}; nodes={:?}",
                    snapshot
                        .nodes()
                        .map(|node| (&node.role, &node.name))
                        .collect::<Vec<_>>()
                )
            })
            .geometry
            .unwrap()
            .visual
    })
}
fn command(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
    event: &str,
) {
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::RoleName {
                    role: role.into(),
                    name: name.into(),
                },
                event: event.into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    v.run_until_parked();
    v.update(|window, _| window.refresh());
    v.run_until_parked();
}
fn wheel(v: &mut VisualTestContext, p: gpui::Point<gpui::Pixels>, x: f32, y: f32) {
    v.simulate_mouse_move(p, None, Modifiers::none());
    v.simulate_event(gpui::ScrollWheelEvent {
        position: p,
        delta: gpui::ScrollDelta::Pixels(point(px(x), px(y))),
        touch_phase: gpui::TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    v.run_until_parked();
    v.update(|window, _| window.refresh());
    v.run_until_parked();
}
fn settle(v: &mut VisualTestContext) {
    v.executor()
        .advance_clock(std::time::Duration::from_millis(32));
    v.run_until_parked();
    v.update(|window, _| window.refresh());
    v.run_until_parked();
}
fn assert_width(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    title: &str,
    text: &str,
    width: f64,
) {
    let header = bounds(v, view, "columnheader", title);
    let body = bounds(v, view, "gridcell", text);
    assert!(
        (header.width - width).abs() < 0.6,
        "{title} header: {header:?}, expected {width}"
    );
    assert!(
        (header.width - body.width).abs() < 0.6,
        "header/body widths diverged: {header:?} / {body:?}"
    );
    assert!(
        (header.x - body.x).abs() < 0.6,
        "header/body offset diverged: {header:?} / {body:?}"
    );
}

#[gpui::test]
fn fixed_wide_ltr_rtl_array_native_share_scroll_and_all_states(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        for native in [false, true] {
            let (window, view) = mount(
                cx,
                script(rtl, native, FIXED, true),
                &format!("extent-{rtl}-{native}"),
            );
            let mut v = VisualTestContext::from_window(*window, cx);
            settle(&mut v);
            let table = bounds(&mut v, &view, "table", "Extent Table");
            let a = bounds(&mut v, &view, "columnheader", "A");
            let p = point(px((table.x + 150.0) as f32), px((a.y + 15.0) as f32));
            v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
            wheel(&mut v, p, if rtl { 80.0 } else { -80.0 }, 0.0);
            let scrolled = bounds(&mut v, &view, "columnheader", "A");
            assert!(
                (scrolled.x - a.x - if rtl { 80.0 } else { -80.0 }).abs() < 0.6,
                "actual extent must permit horizontal scrolling"
            );
            assert_width(&mut v, &view, "A", "Alpha", 120.0);
            assert_width(&mut v, &view, "C", "Gamma", 160.0);
            let snapshot = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
            assert_eq!(
                snapshot
                    .timings
                    .iter()
                    .map(|timing| timing.operations)
                    .sum::<u64>(),
                0,
                "built-in horizontal scrolling must not execute Rhai"
            );
            assert!(
                snapshot
                    .virtual_collections
                    .iter()
                    .all(|collection| collection.realized_count < 40)
            );
            for mode in [1, 2, 3, 0] {
                command(&mut v, &view, "button", &format!("Mode{mode}"), "click");
                let current = bounds(&mut v, &view, "table", "Extent Table");
                let a = bounds(&mut v, &view, "columnheader", "A");
                assert!((current.height - table.height).abs() < 0.6);
                assert!(
                    (a.x - scrolled.x).abs() < 0.6,
                    "state transitions retain the shared offset: rtl={rtl}, native={native}, mode={mode}, current={a:?}, expected={scrolled:?}"
                );
                wheel(&mut v, p, if rtl { 500.0 } else { -500.0 }, 0.0);
                let last = bounds(&mut v, &view, "columnheader", "C");
                let click = point(
                    px((last.x + last.width / 2.0) as f32),
                    px((last.y + 15.0) as f32),
                );
                assert!(
                    f64::from(click.x) > current.x
                        && f64::from(click.x) < current.x + current.width,
                    "hidden last column must be reachable"
                );
                v.simulate_mouse_down(click, MouseButton::Left, Modifiers::none());
                v.simulate_mouse_up(click, MouseButton::Left, Modifiers::none());
                v.run_until_parked();
                assert_eq!(
                    v.update(|_, cx| view
                        .accessibility_snapshot(cx)
                        .unwrap()
                        .nodes()
                        .find(|node| node.role == "status")
                        .unwrap()
                        .name
                        .clone()),
                    "c"
                );
                wheel(&mut v, p, if rtl { -500.0 } else { 500.0 }, 0.0);
                wheel(&mut v, p, if rtl { 80.0 } else { -80.0 }, 0.0);
            }
        }
    }
}

#[gpui::test]
fn percentage_and_weighted_flex_use_viewport_not_extent(cx: &mut TestAppContext) {
    let columns = r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:120.0}},#{key:"b",title:"B",width:#{kind:"percent",value:50.0}},#{key:"c",title:"C",width:#{kind:"flex",value:1.0}}]"#;
    for rtl in [false, true] {
        let (window, view) = mount(
            cx,
            script(rtl, true, columns, true),
            &format!("extent-mixed-{rtl}"),
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        settle(&mut v);
        assert_width(&mut v, &view, "A", "Alpha", 120.0);
        assert_width(&mut v, &view, "B", "Beta", 149.0);
        assert_width(&mut v, &view, "C", "Gamma", 29.0);
        command(&mut v, &view, "button", "Viewport", "click");
        settle(&mut v);
        assert_width(&mut v, &view, "B", "Beta", 249.0);
        assert_width(&mut v, &view, "C", "Gamma", 129.0);
    }
}

#[gpui::test]
fn percentage_overflow_does_not_feedback_into_its_basis(cx: &mut TestAppContext) {
    let columns = r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:350.0}},#{key:"b",title:"B",width:#{kind:"percent",value:50.0}},#{key:"c",title:"C",width:#{kind:"flex",value:1.0}}]"#;
    let (window, view) = mount(
        cx,
        script(false, true, columns, true),
        "extent-percent-overflow",
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    settle(&mut v);
    assert_width(&mut v, &view, "B", "Beta", 149.0);
    command(&mut v, &view, "button", "Viewport", "click");
    settle(&mut v);
    assert_width(&mut v, &view, "B", "Beta", 249.0);
}

#[gpui::test]
fn horizontal_body_input_and_vertical_input_are_independent(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, script(false, true, FIXED, true), "extent-axes");
    let mut v = VisualTestContext::from_window(*window, cx);
    settle(&mut v);
    let table = bounds(&mut v, &view, "table", "Extent Table");
    let before = bounds(&mut v, &view, "gridcell", "Alpha");
    let p = point(px((table.x + 150.0) as f32), px((table.y + 80.0) as f32));
    wheel(&mut v, p, -80.0, 0.0);
    let after = bounds(&mut v, &view, "gridcell", "Alpha");
    assert!((after.x - before.x + 80.0).abs() < 0.6);
    assert!((after.y - before.y).abs() < 0.6);
    let header = bounds(&mut v, &view, "columnheader", "A");
    wheel(&mut v, p, 0.0, -120.0);
    settle(&mut v);
    let current = bounds(&mut v, &view, "columnheader", "A");
    assert!((current.x - header.x).abs() < 0.6);
    let snapshot = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    assert!(
        snapshot
            .virtual_collections
            .iter()
            .all(|collection| collection.realized_count < 40)
    );
}

#[gpui::test]
fn accepted_final_column_width_recomputes_extent_and_clamps_offset(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        let (window, view) = mount(
            cx,
            script(rtl, true, FIXED, true),
            &format!("extent-accept-{rtl}"),
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        settle(&mut v);
        let table = bounds(&mut v, &view, "table", "Extent Table");
        let a = bounds(&mut v, &view, "columnheader", "A");
        let p = point(px((table.x + 150.0) as f32), px((a.y + 15.0) as f32));
        wheel(&mut v, p, if rtl { 500.0 } else { -500.0 }, 0.0);
        command(&mut v, &view, "separator", "Resize C column", "key:right");
        settle(&mut v);
        assert_width(&mut v, &view, "C", "Gamma", 168.0);
        wheel(&mut v, p, if rtl { 500.0 } else { -500.0 }, 0.0);
        let c = bounds(&mut v, &view, "columnheader", "C");
        if rtl {
            assert!((c.x - table.x - 1.0).abs() < 0.6);
        } else {
            assert!((c.x + c.width - table.x - table.width + 1.0).abs() < 0.6);
        }
    }
}

#[gpui::test]
fn theme_insets_are_in_the_shared_column_plan_and_real_extent(cx: &mut TestAppContext) {
    let columns = r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:350.0},sortable:true},#{key:"b",title:"B",width:#{kind:"percent",value:50.0},sortable:true},#{key:"c",title:"C",width:#{kind:"flex",value:1.0},sortable:true}]"#;
    for rtl in [false, true] {
        for (spacing_index, spacing) in [Length::Pixels(24.0), Length::Rems(1.5)]
            .into_iter()
            .enumerate()
        {
            let overrides = ThemeTokenOverrides {
                spacing: BTreeMap::from([("sm".into(), spacing)]),
                typography: ThemeTypographyOverrides {
                    roles: BTreeMap::from([
                        (
                            "body".into(),
                            TypographyToken {
                                size: Length::Pixels(20.0),
                                line_height: Length::Pixels(26.0),
                                weight: 400,
                            },
                        ),
                        (
                            "body_small".into(),
                            TypographyToken {
                                size: Length::Pixels(20.0),
                                line_height: Length::Pixels(26.0),
                                weight: 700,
                            },
                        ),
                    ]),
                    ..Default::default()
                },
                ..Default::default()
            };
            let (window, view) = mount_with_overrides(
                cx,
                script(rtl, true, columns, true),
                &format!("extent-insets-{rtl}-{spacing_index}"),
                overrides,
            );
            let mut v = VisualTestContext::from_window(*window, cx);
            settle(&mut v);
            let minimum = v.update(|window, _| match spacing {
                Length::Pixels(value) => 2.0 * value,
                Length::Rems(value) => 2.0 * value * f64::from(window.rem_size()),
                _ => unreachable!(),
            });
            assert_width(&mut v, &view, "A", "Alpha", 350.0);
            assert_width(&mut v, &view, "B", "Beta", 149.0);
            assert_width(&mut v, &view, "C", "Gamma", minimum);
            let table = bounds(&mut v, &view, "table", "Extent Table");
            let c = bounds(&mut v, &view, "columnheader", "C");
            wheel(
                &mut v,
                point(px((table.x + 150.0) as f32), px((c.y + 15.0) as f32)),
                if rtl { 1000.0 } else { -1000.0 },
                0.0,
            );
            let c = bounds(&mut v, &view, "columnheader", "C");
            if rtl {
                assert!((c.x - table.x - 1.0).abs() < 0.6);
            } else {
                assert!((c.x + c.width - table.x - table.width + 1.0).abs() < 0.6);
            }
            command(&mut v, &view, "button", "Viewport", "click");
            settle(&mut v);
            assert_width(&mut v, &view, "B", "Beta", 249.0);
            assert_width(&mut v, &view, "C", "Gamma", minimum);
        }
    }
}

#[gpui::test]
fn native_drag_override_repaints_widths_and_extent_without_rhai(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        let (window, view) = mount(
            cx,
            script(rtl, true, FIXED, false),
            &format!("extent-native-preview-{rtl}"),
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        settle(&mut v);
        let table = bounds(&mut v, &view, "table", "Extent Table");
        let c = bounds(&mut v, &view, "columnheader", "C");
        let scroll_point = point(px((table.x + 150.0) as f32), px((c.y + 15.0) as f32));
        wheel(
            &mut v,
            scroll_point,
            if rtl { 1000.0 } else { -1000.0 },
            0.0,
        );
        let handle = bounds(&mut v, &view, "separator", "Resize C column");
        let start = point(
            px((handle.x + handle.width / 2.0) as f32),
            px((handle.y + 15.0) as f32),
        );
        let moved = point(start.x + px(if rtl { -40.0 } else { 40.0 }), start.y);
        v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        v.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        v.simulate_mouse_move(moved, MouseButton::Left, Modifiers::none());
        v.run_until_parked();
        assert_width(&mut v, &view, "C", "Gamma", 200.0);
        let snapshot = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        assert_eq!(
            snapshot
                .timings
                .iter()
                .map(|timing| timing.operations)
                .sum::<u64>(),
            0,
            "native pointer preview must not execute Rhai"
        );
        v.simulate_mouse_up(moved, MouseButton::Left, Modifiers::none());
        v.run_until_parked();
        wheel(
            &mut v,
            scroll_point,
            if rtl { 1000.0 } else { -1000.0 },
            0.0,
        );
        assert_width(&mut v, &view, "C", "Gamma", 200.0);
        let last = bounds(&mut v, &view, "columnheader", "C");
        if rtl {
            assert!((last.x - table.x - 1.0).abs() < 0.6);
        } else {
            assert!((last.x + last.width - table.x - table.width + 1.0).abs() < 0.6);
        }
    }
}

#[gpui::test]
fn single_column_under_equal_overflow_and_autofit_share_the_viewport(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        for width in [200.0, 298.0, 400.0] {
            let columns = format!(
                r#"[#{{key:"c",title:"C",width:#{{kind:"fixed",value:{width}}},sortable:true}}]"#
            );
            let source = script(rtl, true, &columns, false)
                .replace("search_fields:[\"a\",\"b\",\"c\"]", "search_fields:[\"c\"]");
            let (window, view) = mount(cx, source, &format!("extent-single-{rtl}-{width}"));
            let mut v = VisualTestContext::from_window(*window, cx);
            settle(&mut v);
            let table = bounds(&mut v, &view, "table", "Extent Table");
            let before = bounds(&mut v, &view, "columnheader", "C");
            let p = point(px((table.x + 150.0) as f32), px((before.y + 15.0) as f32));
            wheel(&mut v, p, if rtl { 1000.0 } else { -1000.0 }, 0.0);
            assert_width(&mut v, &view, "C", "Gamma", width);
            let after = bounds(&mut v, &view, "columnheader", "C");
            let expected = (width - 298.0_f64).max(0.0);
            assert!((after.x - before.x - if rtl { expected } else { -expected }).abs() < 0.6);
            let handle = bounds(&mut v, &view, "separator", "Resize C column");
            let h = point(
                px((handle.x + handle.width / 2.0) as f32),
                px((handle.y + 15.0) as f32),
            );
            v.simulate_mouse_move(h, None, Modifiers::none());
            v.simulate_event(gpui::MouseDownEvent {
                button: MouseButton::Left,
                position: h,
                modifiers: Modifiers::none(),
                click_count: 2,
                first_mouse: false,
            });
            v.simulate_mouse_up(h, MouseButton::Left, Modifiers::none());
            v.run_until_parked();
            let fitted = bounds(&mut v, &view, "columnheader", "C");
            assert!(fitted.width >= 48.0 && fitted.width < 200.0);
            assert_width(&mut v, &view, "C", "Gamma", fitted.width);
            if rtl {
                assert!((fitted.x + fitted.width - table.x - table.width + 1.0).abs() < 0.6);
            } else {
                assert!((fitted.x - table.x - 1.0).abs() < 0.6);
            }
        }
    }
}

#[gpui::test]
fn component_min_max_and_native_override_use_one_width_authority(cx: &mut TestAppContext) {
    let columns = r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:20.0}},#{key:"b",title:"B",width:#{kind:"flex",value:1.0}},#{key:"c",title:"C",width:#{kind:"flex",value:3.0}}]"#;
    let styles = r#"fn component_styles(){#{"components/table":#{header_cell:style().min_width(px(90)).max_width(px(150)),row:style().min_width(px(90))}}}"#;
    for rtl in [false, true] {
        let (window, view) = mount_with_styles(
            cx,
            script(rtl, true, columns, false),
            &format!("extent-style-limits-{rtl}"),
            ThemeTokenOverrides::default(),
            styles,
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        settle(&mut v);
        assert_width(&mut v, &view, "A", "Alpha", 90.0);
        assert_width(&mut v, &view, "B", "Beta", 90.0);
        assert_width(&mut v, &view, "C", "Gamma", 118.0);
        for _ in 0..6 {
            command(&mut v, &view, "separator", "Resize C column", "key:right");
        }
        assert_width(&mut v, &view, "C", "Gamma", 150.0);
        let table = bounds(&mut v, &view, "table", "Extent Table");
        let c = bounds(&mut v, &view, "columnheader", "C");
        wheel(
            &mut v,
            point(px((table.x + 150.0) as f32), px((c.y + 15.0) as f32)),
            if rtl { 1000.0 } else { -1000.0 },
            0.0,
        );
        let c = bounds(&mut v, &view, "columnheader", "C");
        if rtl {
            assert!((c.x - table.x - 1.0).abs() < 0.6);
        } else {
            assert!((c.x + c.width - table.x - table.width + 1.0).abs() < 0.6);
        }
    }
}
