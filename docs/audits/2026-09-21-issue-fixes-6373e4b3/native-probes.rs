use gpui::{
    Context, ImageSource, InteractiveElement, IntoElement, Modifiers, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, WindowHandle,
    div, point, px,
};
use gpui_rhai::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    time::Duration,
};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
fn repo() -> String {
    std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()
}
fn builder(src: &str) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    let mut scripts = BTreeMap::from([(entry.clone(), src.to_owned())]);
    for dir in ["components", "motion"] {
        for e in std::fs::read_dir(format!("{}/registry/{dir}", repo())).unwrap() {
            let p = e.unwrap().path();
            if p.extension().and_then(|x| x.to_str()) == Some("rhai") {
                scripts.insert(
                    ModuleId::parse(format!(
                        "{dir}/{}",
                        p.file_stem().unwrap().to_str().unwrap()
                    ))
                    .unwrap(),
                    std::fs::read_to_string(p).unwrap(),
                );
            }
        }
    }
    let mut assets = vec![];
    for e in std::fs::read_dir(format!("{}/registry/assets/icons", repo())).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) == Some("svg") {
            assets.push((
                format!("icons/{}", p.file_stem().unwrap().to_str().unwrap()),
                AssetData {
                    mime_type: "image/svg+xml".into(),
                    bytes: std::fs::read(p).unwrap(),
                },
            ));
        }
    }
    EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(scripts),
        std::fs::read_to_string(format!("{}/registry/themes/default_dark.rhai", repo())).unwrap(),
    )
    .asset_sources(assets)
}
fn mount(
    cx: &mut TestAppContext,
    p: PreparedScriptView,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle, ScriptViewHost) {
    let cap = Rc::new(RefCell::new(None));
    let take = cap.clone();
    let name = name.to_owned();
    let w = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = p
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some((view.clone(), host.clone()));
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (v, h) = cap.borrow().as_ref().unwrap().clone();
    (w, v, h)
}
fn pump(cx: &mut TestAppContext, v: &mut VisualTestContext) {
    for _ in 0..8 {
        cx.background_executor
            .advance_clock(Duration::from_millis(20));
        v.run_until_parked();
        cx.refresh().unwrap();
        v.run_until_parked();
    }
}
fn pixel(cx: &mut TestAppContext, body: &str, color: Option<Rgba8>) -> Vec<u8> {
    let r = AssetRegistry::new();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" {body}/></svg>"#
    );
    r.register(
        "app",
        InMemoryAssetProvider::new(BTreeMap::from([(
            "pixel".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: svg.into_bytes(),
            },
        )])),
    )
    .unwrap();
    let h = r.load_image(&AssetId::parse("app/pixel").unwrap()).unwrap();
    let ImageSource::Image(i) = r.image_source_tinted(h.opaque(), color).unwrap() else {
        panic!()
    };
    cx.update(|cx| {
        i.to_image_data(cx.svg_renderer())
            .unwrap()
            .as_bytes(0)
            .unwrap()
            .to_vec()
    })
}
#[gpui::test]
fn svg_fixed_colors_preserve_bgra(cx: &mut TestAppContext) {
    let current = pixel(
        cx,
        "fill=\"currentColor\"",
        Some(Rgba8::from_rgb_hex(0x1234ab)),
    );
    let fixed = pixel(cx, "fill=\"#ff0000\"", Some(Rgba8::from_rgb_hex(0x1234ab)));
    println!(
        "SVG currentColor BGRA={current:?}; fixed red actual={fixed:?}, expected=[0,0,255,255]"
    );
    assert_eq!(current, vec![171, 52, 18, 255]);
    assert_eq!(fixed, vec![0, 0, 255, 255]);
}
#[gpui::test]
fn svg_semantic_color_preserves_alpha(cx: &mut TestAppContext) {
    let transparent = pixel(
        cx,
        "fill=\"currentColor\"",
        Some(Rgba8::from_rgba_hex(0x1234ab00)),
    );
    println!("SVG alpha=0 actual={transparent:?}");
    assert_eq!(transparent[3], 0);
}
#[gpui::test]
fn host_slot_capture_isolation(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let hits = Rc::new(Cell::new(0));
    let h = hits.clone();
    let slots = HostSlotRegistry::new()
        .with_slot("content", move |_, _| {
            let h = h.clone();
            Ok(div()
                .id("native")
                .size_full()
                .on_click(move |_, _, _| h.set(h.get() + 1))
                .into_any_element())
        })
        .unwrap();
    let src = r#"
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn hit(ctx,p){ctx.set_state("n",ctx.get_state("n")+1);}
 fn view(ctx){column([text(`capture:${ctx.get_state("n")}`).accessibility_role("status").accessibility_label(`capture:${ctx.get_state("n")}`),gpui_rhai::HostSlot(#{key:"slot",name:"content"}).accessibility_role("group").accessibility_label("Slot").with_style(style().width(px(220)).height(px(120))) ]).with_style(style().width(px(300)).height(px(240))).on_capture("pointer_down",Fn("hit"))}
 "#;
    let (w, view, _) = mount(
        cx,
        builder(src).extension(slots).prepare().unwrap(),
        "capture",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("group", "Slot")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    v.simulate_click(
        point(px((b.x + 50.) as f32), px((b.y + 50.) as f32)),
        Modifiers::default(),
    );
    v.run_until_parked();
    let unchanged = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "capture:0")
            .next()
            .is_some()
    });
    println!(
        "HostSlot native clicks={} ancestor capture unchanged={unchanged}",
        hits.get()
    );
    assert_eq!(hits.get(), 1);
    assert!(unchanged, "Rhai capture handler observed opaque host input");
    v.simulate_click(point(px(280.0), px(220.0)), Modifiers::default());
    v.run_until_parked();
    assert!(
        v.update(|_, cx| view
            .accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "capture:1")
            .next()
            .is_some()),
        "outside-slot control must exercise the parent capture handler"
    );
}
#[gpui::test]
fn host_slot_fills_flex_height(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let slots = HostSlotRegistry::new()
        .with_slot("content", |_, _| {
            Ok(div()
                .size_full()
                .debug_selector(|| "host-fill-content".to_owned())
                .into_any_element())
        })
        .unwrap();
    let src = r#"fn view(ctx){column([text("Header").with_style(style().height(px(30)).flex_shrink(false)),gpui_rhai::HostSlot(#{key:"slot",name:"content"}).accessibility_role("group").accessibility_label("Slot").with_style(style().flex_grow().min_height(px(0))) ]).with_style(style().width(px(300)).height(px(240)))}"#;
    let (w, view, _) = mount(cx, builder(src).extension(slots).prepare().unwrap(), "fill");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let b = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("group", "Slot")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let native = v.debug_bounds("host-fill-content").unwrap();
    println!("HostSlot flex slot={b:?} native={native:?}");
    assert!(f64::from(native.size.height) > 200.0);
}
#[gpui::test]
fn table_fill_height_realizes_rows(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut results = vec![];
    for fill in [false, true] {
        let src = format!(
            r#"import "components/table" as table; fn view(ctx){{let rows=[];for i in 0..15{{rows.push(#{{id:i.to_string(),name:`Row ${{i}}`}});}}column([text("Header").with_style(style().height(px(30)).flex_shrink(false)),table::Table(#{{key:"table",label:"Rows",row_key:"id",rows:rows,columns:[#{{key:"name",title:"Name",width:#{{kind:"fixed",value:200}}}}],{}}})]).with_style(style().width(px(600)).height(px(500)))}}"#,
            if fill {
                "fill_height:true"
            } else {
                "height:430"
            }
        );
        let (w, view, _) = mount(
            cx,
            builder(&src).prepare().unwrap(),
            if fill { "table-fill" } else { "table-fixed" },
        );
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let (b, rows) = v.update(|_, cx| {
            let s = view.accessibility_snapshot(cx).unwrap();
            let b = s
                .find_by_role_and_name("table", "Rows")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual;
            let rows = s.nodes().filter(|n| n.role == "row").count();
            (b, rows)
        });
        let native = v.debug_bounds("virtual-list:table-body");
        let perf = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        println!(
            "Table parent=fixed fill={fill} outer={b:?} rows={rows} native={native:?} virtual={:?} error={:?}",
            perf.virtual_collections,
            v.update(|_, cx| view.last_error(cx).unwrap())
        );
        results.push(rows);
    }
    assert!(results[0] > 1);
    assert!(results[1] > 1, "fill_height table has no presented rows");
}
#[gpui::test]
fn motion_group_survives_incremental_component_updates(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut ok = vec![];
    for mode in ["none", "explicit", "inherited"] {
        let declaration = match mode {
            "explicit" => ".shared_layout(\"g\",\"item\").layout_motion(100,\"linear\")",
            "inherited" => ".shared_layout(\"item\").layout_motion(100,\"linear\")",
            _ => "",
        };
        let src = format!(
            r#"
 define_component(#{{metadata:#{{id:"components/group_probe","export":"GroupProbe",version:"0.1.0",runtime_api:#{{min_inclusive:2,max_exclusive:3}},dependencies:[],capabilities:#{{}}}},schema:#{{props:#{{key:#{{schema:#{{type:"string"}},required:true,sensitive:false}}}},state:#{{fields:#{{n:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}},events:#{{}},slots:#{{}},parts:["root"]}},render:Fn("render_probe")}});
 fn increment(ctx,p){{ctx.set_state("n",ctx.get_state("n")+1);}}
 fn render_probe(ctx,props){{text(`count:${{ctx.get_state("n")}}`).with_key("child").test_id("child").accessibility_role("button").accessibility_label(`count:${{ctx.get_state("n")}}`).with_style(style().width(px(100)).height(px(30))).on_click(Fn("increment")){} }}
 fn view(ctx){{motion_group("g",[render_component("components/group_probe",#{{key:"probe"}})])}}
 "#,
            declaration
        );
        let (w, view, _) = mount(
            cx,
            builder(&src).prepare().unwrap(),
            &format!("group-{mode}"),
        );
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let result = v.update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId { id: "child".into() },
                    event: "click".into(),
                    payload: None,
                },
                window,
                cx,
            )
        });
        let has_one = v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", "count:1")
                .next()
                .is_some()
        });
        println!(
            "motion_group mode={mode} click={result:?} count1={has_one} last_error={:?}",
            v.update(|_, cx| view.last_error(cx).unwrap())
        );
        ok.push(result.is_ok());
    }
    assert!(
        ok.iter().all(|x| *x),
        "inherited group identity was lost by incremental rerender"
    );
}
#[gpui::test]
fn enter_motion_on_owned_node_prop_keeps_callbacks_live(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for property in ["opacity", "translate_y"] {
        let source = format!(
            r#"
 define_component(#{{metadata:#{{id:"components/counter_probe","export":"CounterProbe",version:"0.1.0",runtime_api:#{{min_inclusive:2,max_exclusive:3}},dependencies:[],capabilities:#{{}}}},schema:#{{props:#{{key:#{{schema:#{{type:"string"}},required:true,sensitive:false}}}},state:#{{fields:#{{n:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}},events:#{{}},slots:#{{}},parts:[]}},render:Fn("counter_render")}});
 define_component(#{{metadata:#{{id:"components/receiver_probe","export":"ReceiverProbe",version:"0.1.0",runtime_api:#{{min_inclusive:2,max_exclusive:3}},dependencies:[],capabilities:#{{}}}},schema:#{{props:#{{key:#{{schema:#{{type:"string"}},required:true,sensitive:false}},content:#{{schema:#{{type:"node"}},required:true,sensitive:false}}}},state:#{{fields:#{{}}}},events:#{{}},slots:#{{}},parts:[]}},render:Fn("receiver_render")}});
 fn bump(ctx,p){{ctx.set_state("n",ctx.get_state("n")+1);}}
 fn counter_render(ctx,props){{text(`value:${{ctx.get_state("n")}}`).with_key("counter").test_id("counter").accessibility_role("button").accessibility_label(`value:${{ctx.get_state("n")}}`).on_click(Fn("bump"))}}
 fn receiver_render(ctx,props){{column([props.content]).with_key("wrapper").enter_motion(motion_transition("{}",0.0,1.0,#{{duration_ms:100}}))}}
 fn view(ctx){{let content=render_component("components/counter_probe",#{{key:"counter"}});render_component("components/receiver_probe",#{{key:"receiver",content:content}})}}
 "#,
            property
        );
        let (w, view, _) = mount(
            cx,
            builder(&source).prepare().unwrap(),
            &format!("enter-{property}"),
        );
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let r = v.update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId {
                        id: "counter".into(),
                    },
                    event: "click".into(),
                    payload: None,
                },
                window,
                cx,
            )
        });
        let one = v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", "value:1")
                .next()
                .is_some()
        });
        println!("node prop enter_motion property={property} dispatch={r:?} value1={one}");
        assert!(r.is_ok());
        assert!(one);
    }
}

#[gpui::test]
fn table_relative_parent_fill_height_realizes_rows(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut results = vec![];
    for fill in [false, true] {
        let src = format!(
            r#"import "components/table" as table; fn view(ctx){{let rows=[];for i in 0..15{{rows.push(#{{id:i.to_string(),name:`Row ${{i}}`}});}}column([text("Header").with_style(style().height(px(30)).flex_shrink(false)),table::Table(#{{key:"table",label:"Rows",row_key:"id",rows:rows,columns:[#{{key:"name",title:"Name",width:#{{kind:"fixed",value:200}}}}],{}}})]).with_style(style().width(px(600)).height(relative(1.0)))}}"#,
            if fill {
                "fill_height:true"
            } else {
                "height:430"
            }
        );
        let (w, view, _) = mount(
            cx,
            builder(&src).prepare().unwrap(),
            if fill { "table-fill" } else { "table-fixed" },
        );
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let (b, rows) = v.update(|_, cx| {
            let s = view.accessibility_snapshot(cx).unwrap();
            let b = s
                .find_by_role_and_name("table", "Rows")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual;
            let rows = s.nodes().filter(|n| n.role == "row").count();
            (b, rows)
        });
        let native = v.debug_bounds("virtual-list:table-body");
        let perf = v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        println!(
            "Table parent=relative fill={fill} outer={b:?} rows={rows} native={native:?} virtual={:?} error={:?}",
            perf.virtual_collections,
            v.update(|_, cx| view.last_error(cx).unwrap())
        );
        results.push(rows);
    }
    assert!(results[0] > 1);
    assert!(results[1] > 1, "fill_height table has no presented rows");
}
#[gpui::test]
fn host_slot_preserves_independent_script_view_host(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut outcomes = vec![];
    for case in 0..3 {
        let separate = case != 0;
        let resident=builder(r#"fn view(ctx){text("Resident").accessibility_role("button").accessibility_label("Resident")}"#).prepare().unwrap();
        let shell = builder(
            r#"fn view(ctx){gpui_rhai::HostSlot(#{key:"slot",name:"content"}).with_style(style().width(px(300)).height(px(160)))}"#,
        );
        let captured = Rc::new(RefCell::new(None));
        let cap = captured.clone();
        let w = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new(
                if separate {
                    "outer-distinct"
                } else {
                    "outer-shared"
                },
                cx,
            )
            .unwrap();
            let inner_host = if separate {
                ScriptViewHost::new("resident-host", cx).unwrap()
            } else {
                host.clone()
            };
            let resident = resident
                .mount(
                    ScriptViewConfig::new("resident"),
                    inner_host.clone(),
                    window,
                    cx,
                )
                .unwrap();
            let slots = if case == 2 {
                let child = resident.clone();
                HostSlotRegistry::new()
                    .with_slot("content", move |_, _| {
                        child
                            .flex_item()
                            .map(|e| inner_host.container(e).into_any_element())
                            .map_err(|e| e.to_string())
                    })
                    .unwrap()
            } else {
                HostSlotRegistry::new()
                    .with_script_view("content", resident.clone())
                    .unwrap()
            };
            let shell = shell.extension(slots).prepare().unwrap();
            let view = shell
                .mount(ScriptViewConfig::new("shell"), host.clone(), window, cx)
                .unwrap();
            *cap.borrow_mut() = Some(resident);
            Host { host, view }
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        let resident = captured.borrow().as_ref().unwrap().clone();
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let error = v.update(|_, cx| resident.last_error(cx).unwrap());
        println!(
            "HostSlot with_script_view separate_host={separate} explicit_container={} resident_error={error:?}",
            case == 2
        );
        outcomes.push(error.is_none());
    }
    assert!(
        outcomes.iter().all(|x| *x),
        "slotted view lost its independent ScriptViewHost frame boundary"
    );
}
