use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
struct AuditHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for AuditHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            self.view
                .element()
                .unwrap_or_else(|_| gpui::div().into_any_element()),
        )
    }
}
fn builder(source: &str) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), source.to_owned())]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), source.to_string());
    }
    EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(p, s)| {
                (
                    p.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: s.as_bytes().to_vec(),
                    },
                )
            }),
    )
}
fn mount(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    id: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let id = id.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&id, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&id), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        AuditHost { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}
fn settle(v: &mut VisualTestContext) {
    v.run_until_parked();
    v.executor().advance_clock(Duration::from_millis(32));
    v.run_until_parked();
    for _ in 0..3 {
        v.update(|w, cx| w.simulate_next_frame(cx));
        v.run_until_parked();
    }
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
#[gpui::test]
fn audit_grip_must_not_start_through_an_occluding_sibling(cx: &mut TestAppContext) {
    let mut results = vec![];
    for (grip, covered) in [(false, false), (true, false), (true, true)] {
        let handle = if grip {
            r#"handle:box([]).with_style(style().width(px(16)).height(px(40))),"#
        } else {
            ""
        };
        let cover = if covered {
            r#",box([]).test_id("cover").with_style(style().absolute().left(px(0)).top(px(0)).width(px(420)).height(px(240)).background(rgba(0xaaaaaaff)).occlude())"#
        } else {
            ""
        };
        let source = format!(
            r#"import "components/split_pane" as split;
 fn state_schema(){{#{{fields:#{{ratio:#{{schema:#{{type:"number"}},"default":#{{type:"float",value:0.5}}}}}}}}}}
 fn resized(ctx,value){{ctx.set_state("ratio",value);}}
 fn view(ctx){{box([column([text(ctx.get_state("ratio").to_string()).accessibility_role("status"),
 split::SplitPane(#{{key:"split",label:"Split",size:ctx.get_state("ratio"),{handle}min_start:80.0,min_end:80.0,start:text("Start"),end:text("End"),on_resize:Fn("resized")}}).with_style(style().width(px(420)).height(px(180)))]){cover}]).with_style(style().relative().width(px(420)).height(px(240)))}}"#
        );
        let (mut v, view) = mount(
            cx,
            builder(&source).prepare().unwrap(),
            &format!("grip-{grip}-{covered}"),
        );
        let b = bounds(&mut v, &view, "separator", "Split");
        let p = point(
            px((b.x + b.width / 2.0 - 7.0) as f32),
            px((b.y + b.height / 2.0) as f32),
        );
        let q = point(p.x + px(72.0), p.y);
        v.simulate_mouse_move(p, None, Modifiers::none());
        v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
        v.simulate_mouse_move(q, MouseButton::Left, Modifiers::none());
        v.simulate_mouse_up(q, MouseButton::Left, Modifiers::none());
        settle(&mut v);
        let ratio = status(&mut v, &view).parse::<f64>().unwrap();
        println!(
            "GRIP grip={grip} covered={covered} ratio={ratio} error={:?}",
            v.update(|_, cx| view.last_error(cx).unwrap())
        );
        results.push(ratio);
    }
    assert_eq!(results[0], 0.5);
    assert!(results[1] > 0.5);
    assert_eq!(
        results[2], 0.5,
        "occluded grip must not take pointer input: {results:?}"
    );
}
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<(String, InvocationOrigin)>>>);
impl CapabilityHandler for Capture {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        Err("expected call_with".into())
    }
    fn call_with(
        &mut self,
        ctx: &InvocationContext,
        _: &str,
        input: UiValue,
    ) -> Result<UiValue, String> {
        if let UiValue::String(label) = input {
            self.0.borrow_mut().push((label, ctx.origin.clone()));
        }
        Ok(UiValue::Null)
    }
}
struct Echo;
impl AsyncCapabilityHandler for Echo {
    fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
        Ok(TaskWork::new(move || Ok(input)))
    }
}
impl ScriptViewExtension for Capture {
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        let desc = |id: &str, output: ValueSchema| CapabilityDescriptor {
            id: CapabilityId::parse(id).unwrap(),
            version: semver::Version::new(1, 0, 0),
            methods: BTreeMap::from([(
                "run".into(),
                CapabilityMethod {
                    input: ValueSchema::string(),
                    output,
                },
            )]),
        };
        r.capabilities
            .register(desc("audit.record", ValueSchema::Null), self.clone())
            .map_err(|e| e.to_string())?;
        r.capabilities
            .register_async(desc("audit.work", ValueSchema::string()), Echo)
            .map_err(|e| e.to_string())
    }
}
#[gpui::test]
fn audit_task_followup_action_keeps_the_delivery_origin(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let records = Rc::new(RefCell::new(vec![]));
    let manifest = AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("audit.record", "*")
        .unwrap()
        .with_capability("audit.work", "*")
        .unwrap();
    let src = r#"fn init(ctx){ctx.register_action("audit.after",Fn("after"));ctx.register_action("audit.sync",Fn("sync_action"));}
 fn sync_action(ctx,p){ctx.call_capability("audit.record","run","sync-action");}
 fn after(ctx,p){ctx.call_capability("audit.record","run","after-task-action");}
 fn done(ctx,p){ctx.call_capability("audit.record","run","completion");ctx.dispatch_action("audit.after",());}
 fn failed(ctx,p){throw p;}
 fn audit_clicked(ctx,p){ctx.call_capability("audit.record","run","click");ctx.dispatch_action("audit.sync",());ctx.start_task("audit.work","run","go",Fn("done"),Fn("failed"));}
 fn view(ctx){box([text("Go")]).accessibility_role("button").accessibility_label("Go").on_click(Fn("audit_clicked")).with_style(style().width(px(100)).height(px(40)))}"#;
    let (mut v, view) = mount(
        cx,
        builder(src)
            .manifest(manifest)
            .extension(Capture(records.clone()))
            .prepare()
            .unwrap(),
        "origin-after",
    );
    let b = bounds(&mut v, &view, "button", "Go");
    let p = point(
        px((b.x + b.width / 2.0) as f32),
        px((b.y + b.height / 2.0) as f32),
    );
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_up(p, MouseButton::Left, Modifiers::none());
    for _ in 0..60 {
        settle(&mut v);
        if records
            .borrow()
            .iter()
            .any(|(s, _)| s == "after-task-action")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!(
        "ORIGINS {:?} error={:?}",
        records.borrow(),
        v.update(|_, cx| view.last_error(cx).unwrap())
    );
    let records = records.borrow();
    let origin = |label: &str| records.iter().find(|(s, _)| s == label).unwrap().1.clone();
    assert_eq!(origin("click"), origin("sync-action"));
    assert!(matches!(
        origin("completion"),
        InvocationOrigin::TaskCompletion { .. }
    ));
    assert_eq!(origin("after-task-action"), origin("completion"));
}
#[gpui::test]
fn audit_signal_style_margins_keep_stretch_equivalent(cx: &mut TestAppContext) {
    let src = r#"define_component(#{metadata:#{id:"audit/styles","export":"Styles",version:"0.2.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_styles")});
 fn render_styles(ctx,props){let state=signal("state","padded");column([
  box([text("base")]).accessibility_role("group").test_id("base").with_style(style().height(px(30)).margin_x(px(20))),
  box([text("signal")]).accessibility_role("group").test_id("signal").with_style(style().height(px(30))).signal_style(state,#{padded:style().margin_x(px(20))})
 ]).with_style(style().width(px(300)).height(px(100)))}
 fn view(ctx){render_component("audit/styles",#{key:"styles"})}"#;
    let (mut v, view) = mount(cx, builder(src).prepare().unwrap(), "signal-style");
    let (a, b) = v.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let find = |id: &str| {
            tree.nodes()
                .find(|n| n.test_id.as_deref() == Some(id))
                .unwrap()
                .geometry
                .unwrap()
                .visual
        };
        (find("base"), find("signal"))
    });
    println!("SIGNAL MARGIN base={a:?}, signal={b:?}");
    assert_eq!(a.width, 260.0);
    assert_eq!(
        b.width, a.width,
        "same effective margins must produce the same stretch box"
    );
    assert_eq!(a.x, b.x);
}
#[gpui::test]
fn audit_resizable_grip_must_not_start_through_cover(cx: &mut TestAppContext) {
    let mut widths = vec![];
    for (grip, covered) in [(false, false), (true, false), (true, true)] {
        let grips = if grip {
            r#",grips:#{se:box([]).with_style(style().width(px(20)).height(px(20)))}"#
        } else {
            ""
        };
        let cover = if covered {
            r#",box([]).with_style(style().absolute().left(px(0)).top(px(0)).width(px(500)).height(px(450)).background(rgba(0xaaaaaaff)).occlude())"#
        } else {
            ""
        };
        let src = format!(
            r#"import "components/resizable" as resize;
 fn state_schema(){{#{{fields:#{{width:#{{schema:#{{type:"number"}},"default":#{{type:"float",value:200.0}}}}}}}}}}
 fn resized(ctx,p){{ctx.set_state("width",p.width);}}
 fn view(ctx){{box([column([text(ctx.get_state("width").to_string()).accessibility_role("status"),resize::Resizable(#{{key:"card",label:"Card",rect:#{{x:100.0,y:80.0,width:ctx.get_state("width"),height:120.0}},handles:["se"]{grips},min_width:80.0,min_height:60.0,content:text("Card"),on_resize:Fn("resized")}}).with_style(style().width(px(500)).height(px(400)))]){cover}]).with_style(style().relative().width(px(500)).height(px(450)))}}"#
        );
        let (mut v, view) = mount(
            cx,
            builder(&src).prepare().unwrap(),
            &format!("resize-{grip}-{covered}"),
        );
        let b = bounds(&mut v, &view, "separator", "Card: se resize handle");
        let p = point(
            px((b.x + b.width / 2.0 + 9.0) as f32),
            px((b.y + b.height / 2.0) as f32),
        );
        let q = point(p.x + px(40.0), p.y + px(30.0));
        v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
        v.simulate_mouse_move(q, MouseButton::Left, Modifiers::none());
        v.simulate_mouse_up(q, MouseButton::Left, Modifiers::none());
        settle(&mut v);
        let width = status(&mut v, &view).parse::<f64>().unwrap();
        println!("RESIZE grip={grip} covered={covered}: width={width}");
        widths.push(width);
    }
    assert_eq!(widths[0], 200.0);
    assert_eq!(widths[1], 240.0);
    assert_eq!(
        widths[2], 200.0,
        "covered resizable must not take pointer input"
    );
}
#[gpui::test]
fn audit_region_default_clips_overflowing_body_input(cx: &mut TestAppContext) {
    let mut results = vec![];
    for scroll in [true, false] {
        let src = format!(
            r#"import "layouts/region" as region;
 fn state_schema(){{#{{fields:#{{clicks:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
 fn clicked_body(ctx,p){{ctx.set_state("clicks",ctx.get_state("clicks")+1);}}
 fn view(ctx){{column([region::Region(#{{label:"Region",fill:false,inset:false,scroll:{scroll},body:box([text("Long body")]).accessibility_role("button").accessibility_label("Long body").on_click(Fn("clicked_body")).with_style(style().height(px(500)).flex_shrink(false).background(rgba(0xff00ffff)))}}).with_style(style().height(px(200)).flex_shrink(false)),text(ctx.get_state("clicks").to_string()).accessibility_role("status")]).with_style(style().width(px(300)).height(px(600)))}}"#
        );
        let (mut v, view) = mount(
            cx,
            builder(&src).prepare().unwrap(),
            &format!("region-{scroll}"),
        );
        let region = bounds(&mut v, &view, "region", "Region");
        assert!((region.height - 200.0).abs() < 0.5);
        let p = point(
            px((region.x + 50.0) as f32),
            px((region.y + region.height + 100.0) as f32),
        );
        v.simulate_click(p, Modifiers::none());
        settle(&mut v);
        let clicks = status(&mut v, &view);
        println!("REGION scroll={scroll} region={region:?} click100px_below_region={clicks}");
        results.push(clicks);
    }
    assert_eq!(
        results,
        vec!["0", "0"],
        "the non-scrolling body should still clip, as documented"
    );
}
