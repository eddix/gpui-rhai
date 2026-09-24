// Reuse the immutable previous 12 probes and public fixtures.
include!(concat!(
    env!("GPUI_RHAI_REVIEW_REPO"),
    "/docs/audits/2026-09-23-charts-e09b5b2f/release-review-145320d1/native-probes.rs"
));

fn click_control(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let point = v.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let b = tree
            .find_by_role_and_name("button", name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        point(
            px((b.x + b.width / 2.0) as f32),
            px((b.y + b.height / 2.0) as f32),
        )
    });
    v.simulate_click(point, gpui::Modifiers::default());
    v.run_until_parked();
}
fn wheel_named(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    delta: f32,
    phase: gpui::TouchPhase,
) {
    let pos = figure_point(v, view, name, ChartPoint { x: 150., y: 140. });
    v.simulate_event(gpui::ScrollWheelEvent {
        position: pos,
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(delta))),
        touch_phase: phase,
        ..Default::default()
    });
}
fn xwindow(windows: &AxisWindows, name: &str) -> (f64, f64) {
    windows.lock().unwrap()[name].0
}
fn window_eq(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

struct CommitFailurePrimitive(Rc<std::cell::Cell<bool>>);
impl PrimitiveHandler for CommitFailurePrimitive {
    fn commit_resume(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        panic!("injected compensation commit failure");
    }
    fn unmount(&mut self, _: &PrimitiveInstanceId) {
        self.0.set(true);
    }
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        _: &PrimitiveTheme,
        _: &mut Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, String> {
        Ok(gpui::div().into_any_element())
    }
}
struct CommitFailureExtension {
    motion: MotionExtension,
    unmounted: Rc<std::cell::Cell<bool>>,
}
impl ScriptViewExtension for CommitFailureExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        self.motion.configure_engine(e)?;
        e.register_primitive(
            PrimitiveDescriptor {
                id: PrimitiveId::parse("zz_lifecycle.commit_bomb").unwrap(),
                export: "CommitBomb".into(),
                props: BTreeMap::new(),
                events: BTreeMap::new(),
                state: ComponentStateSchema::default(),
                lifecycle: true,
                effect: None,
            },
            CommitFailurePrimitive(self.unmounted.clone()),
        )
        .map_err(|e| e.to_string())
    }
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        self.motion.configure_runtime(r)
    }
}
#[gpui::test]
fn compensation_commit_failure_disposes_chart_resources(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let unmounted = Rc::new(std::cell::Cell::new(false));
    let src = r#"import "charts/chart" as chart;fn suspend(ctx){throw "injected script suspend failure";}fn view(ctx){column([chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300))),zz_lifecycle::CommitBomb(#{key:"bomb"})])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "compensation-commit-failure",
        CommitFailureExtension {
            motion: MotionExtension {
                data: data.clone(),
                positions: positions.clone(),
            },
            unmounted: unmounted.clone(),
        },
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let layouts = positions.lock().unwrap().len();
    let result = v.update(|window, cx| view.suspend(window, cx));
    assert!(result.is_err());
    assert_eq!(view.state(), ScriptViewState::Disposed);
    assert!(unmounted.get());
    data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut v);
    assert_eq!(positions.lock().unwrap().len(), layouts);
    assert!(matches!(
        v.update(|_, cx| view.resume(cx)),
        Err(ScriptViewError::DisposedView(_))
    ));
    println!(
        "COMPENSATION_COMMIT_FAILURE result={result:?} state={:?} unmounted={} layouts_before={layouts} layouts_after={}",
        view.state(),
        unmounted.get(),
        positions.lock().unwrap().len()
    );
}

#[gpui::test]
fn resize_keeps_an_active_unlinked_preview(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let src = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{w:#{schema:#{type:"float"},"default":#{type:"float",value:280.0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn wider(ctx,p){ctx.set_state("w",420.0);}fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}
fn view(ctx){column([text("Resize").accessibility_role("button").accessibility_label("Resize").on_click(Fn("wider")).with_style(style().width(px(140)).height(px(32))),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("rev"),spec:#{title:"c",legend:#{visible:false},axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:"c",kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(ctx.get_state("w"))).height(px(240)))])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "resize-live-preview",
        AxisProbeExtension(windows.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let initial = xwindow(&windows, "c");
    wheel_named(&mut v, &view, "c", -200., gpui::TouchPhase::Started);
    pump(cx, &mut v);
    let preview = xwindow(&windows, "c");
    assert!(preview.0 > 10. && preview.1 < 90.);
    click_control(&mut v, &view, "Resize");
    pump(cx, &mut v);
    let resized = xwindow(&windows, "c");
    wheel_named(&mut v, &view, "c", 0., gpui::TouchPhase::Ended);
    pump(cx, &mut v);
    let committed = xwindow(&windows, "c");
    println!(
        "RESIZE_PREVIEW initial={initial:?} preview={preview:?} resized={resized:?} committed={committed:?}"
    );
    assert!(
        window_eq(preview, resized),
        "ordinary resize discarded an in-progress viewport preview"
    );
    assert!(
        window_eq(preview, committed),
        "Ended committed a viewport different from the preview before resize"
    );
}

#[gpui::test]
fn changing_link_group_does_not_alias_same_numeric_version(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let src = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{group:#{schema:#{type:"string"},"default":#{type:"string",value:"group_a"}},za:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},zb:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},ra:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rb:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn za(ctx,p){ctx.set_state("za",p.zoom);ctx.set_state("ra",p.viewport_revision);}fn zb(ctx,p){ctx.set_state("zb",p.zoom);ctx.set_state("rb",p.viewport_revision);}fn change_group(ctx,p){ctx.set_state("group","group_b");}
fn one(ctx,k,g,z,r,cb){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:z,viewport_revision:r,spec:#{title:k,legend:#{visible:false},link_group:g,link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:cb}).with_style(style().width(px(280)).height(px(240)))}
fn view(ctx){column([text("Switch").accessibility_role("button").accessibility_label("Switch").on_click(Fn("change_group")).with_style(style().width(px(140)).height(px(32))),row([one(ctx,"a","group_a",ctx.get_state("za"),ctx.get_state("ra"),Fn("za")),one(ctx,"b","group_b",ctx.get_state("zb"),ctx.get_state("rb"),Fn("zb")),one(ctx,"target",ctx.get_state("group"),1.0,0,())])])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "switch-link-group",
        AxisProbeExtension(windows.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    for (name, delta) in [("a", -200.), ("b", -400.)] {
        wheel_named(&mut v, &view, name, delta, gpui::TouchPhase::Started);
        wheel_named(&mut v, &view, name, 0., gpui::TouchPhase::Ended);
        pump(cx, &mut v);
    }
    let a = xwindow(&windows, "a");
    let b = xwindow(&windows, "b");
    let before = xwindow(&windows, "target");
    assert!(
        window_eq(a, before) && !window_eq(a, b),
        "fixture must establish different acknowledged groups"
    );
    click_control(&mut v, &view, "Switch");
    pump(cx, &mut v);
    let after = xwindow(&windows, "target");
    println!("GROUP_VERSION a={a:?} b={b:?} before={before:?} after={after:?}");
    assert!(
        window_eq(after, b),
        "same numeric version in a different group suppressed its new projection"
    );
}

const TWO_LINKED_CARTESIAN: &str = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{zs:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},zt:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rs:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rt:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn zs(ctx,p){ctx.set_state("zs",p.zoom);ctx.set_state("rs",p.viewport_revision);}fn zt(ctx,p){ctx.set_state("zt",p.zoom);ctx.set_state("rt",p.viewport_revision);}
fn one(ctx,k,z,r,cb){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:z,viewport_revision:r,spec:#{title:k,legend:#{visible:false},link_group:"shared",link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:cb}).with_style(style().width(px(280)).height(px(240)))}
fn view(ctx){row([one(ctx,"source",ctx.get_state("zs"),ctx.get_state("rs"),Fn("zs")),one(ctx,"target",ctx.get_state("zt"),ctx.get_state("rt"),Fn("zt"))])}"#;

#[gpui::test]
fn linked_target_can_zoom_in_from_its_displayed_window(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let (w, view) = mount_extension(
        cx,
        TWO_LINKED_CARTESIAN,
        "linked-target-gesture",
        AxisProbeExtension(windows.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    wheel_named(&mut v, &view, "source", -400., gpui::TouchPhase::Started);
    wheel_named(&mut v, &view, "source", 0., gpui::TouchPhase::Ended);
    pump(cx, &mut v);
    let linked = xwindow(&windows, "target");
    assert!(window_eq(linked, xwindow(&windows, "source")) && linked.0 > 30.);
    wheel_named(&mut v, &view, "target", -200., gpui::TouchPhase::Started);
    pump(cx, &mut v);
    let preview = xwindow(&windows, "target");
    wheel_named(&mut v, &view, "target", 0., gpui::TouchPhase::Ended);
    pump(cx, &mut v);
    let committed = xwindow(&windows, "target");
    let source = xwindow(&windows, "source");
    println!(
        "LINKED_TARGET_GESTURE linked={linked:?} preview={preview:?} committed={committed:?} source={source:?}"
    );
    let span = |w: (f64, f64)| w.1 - w.0;
    assert!(
        span(preview) < span(linked) - 1.,
        "target's existing linked axis override swallowed its native preview"
    );
    assert!(
        span(committed) < span(linked) - 1.,
        "zoom-in on the linked target expanded rather than narrowed the logical window"
    );
}

type GeoRects = std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartRect>>>;
struct SizedGeoProbe {
    points: std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>,
    plots: GeoRects,
}
impl HostChartSeries for SizedGeoProbe {
    fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        self.plots
            .lock()
            .unwrap()
            .insert(c.spec.key.clone(), c.bounds);
        GeoMarker(self.points.clone()).layout(c)
    }
}
struct SizedGeoExtension {
    points: std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>,
    plots: GeoRects,
}
impl ScriptViewExtension for SizedGeoExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        GeoExtension(self.points.clone()).configure_engine(e)?;
        e.register_chart_series(
            "sized_geo",
            SizedGeoProbe {
                points: self.points.clone(),
                plots: self.plots.clone(),
            },
        )
        .map_err(|e| e.to_string())
    }
}
#[gpui::test]
fn geo_camera_pan_uses_plot_size_before_and_after_target_resize(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let points: std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>> = Default::default();
    let plots = GeoRects::default();
    let src = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{pan:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}},w:#{schema:#{type:"float"},"default":#{type:"float",value:280.0}},hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn pan(ctx,p){ctx.set_state("pan",60.0);}fn resize(ctx,p){ctx.set_state("w",420.0);}fn hit(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}
fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0}],pan_x:if k=="source"{ctx.get_state("pan")}else{0.0},pan_y:if k=="source"{ctx.get_state("pan")}else{0.0},spec:#{title:k,legend:#{visible:false},link_group:"maps",link_domain:"location",regions:[#{key:"main",kind:"geo_2d",map:"map"}],series:[#{key:k,kind:"custom",renderer:"sized_geo",encode:#{x:"x",y:"y"}}]},on_select:Fn("hit")}).with_style(style().width(px(if k=="source"{400.0}else{ctx.get_state("w")})).height(px(if k=="source"{300.0}else{240.0})))}
fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),text("Pan").accessibility_role("button").accessibility_label("Pan").on_click(Fn("pan")).with_style(style().width(px(140)).height(px(32))),text("Resize").accessibility_role("button").accessibility_label("Resize").on_click(Fn("resize")).with_style(style().width(px(140)).height(px(32))),row([one(ctx,"source"),one(ctx,"target")])])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "geo-sized-camera",
        SizedGeoExtension {
            points: points.clone(),
            plots: plots.clone(),
        },
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    click_control(&mut v, &view, "Pan");
    pump(cx, &mut v);
    for round in 0..2 {
        if round == 1 {
            click_control(&mut v, &view, "Resize");
            pump(cx, &mut v);
        }
        let map = plots.lock().unwrap().clone();
        let raw = points.lock().unwrap()["target"];
        let expected = ChartPoint {
            x: raw.x + 60. * map["target"].width / map["source"].width,
            y: raw.y + 60. * map["target"].height / map["source"].height,
        };
        let pos = figure_point(&mut v, &view, "target", expected);
        v.simulate_click(pos, gpui::Modifiers::default());
        v.run_until_parked();
        println!(
            "GEO_SIZED round={round} source_plot={:?} target_plot={:?} expected={expected:?} hits={}",
            map["source"],
            map["target"],
            status(&mut v, &view)
        );
        assert_eq!(
            status(&mut v, &view),
            (round + 1).to_string(),
            "Geo camera must denormalize using the actual target plot"
        );
    }
}

#[gpui::test]
fn source_unmount_releases_its_link_projection(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let src=TWO_LINKED_CARTESIAN.replace("fields:#{zs:","fields:#{show:#{schema:#{type:\"bool\"},\"default\":#{type:\"bool\",value:true}},zs:")
        .replace("fn view(ctx){row([one(ctx,\"source\",ctx.get_state(\"zs\"),ctx.get_state(\"rs\"),Fn(\"zs\")),one(ctx,\"target\",ctx.get_state(\"zt\"),ctx.get_state(\"rt\"),Fn(\"zt\"))])}",
        r#"fn remove_source(ctx,p){ctx.set_state("show",false);}fn view(ctx){let charts=[];if ctx.get_state("show"){charts.push(one(ctx,"source",ctx.get_state("zs"),ctx.get_state("rs"),Fn("zs")));}charts.push(one(ctx,"target",ctx.get_state("zt"),ctx.get_state("rt"),Fn("zt")));column([text("Remove").accessibility_role("button").accessibility_label("Remove").on_click(Fn("remove_source")).with_style(style().width(px(140)).height(px(32))),row(charts)])}"#);
    let (w, view) = mount_extension(
        cx,
        &src,
        "unlink-source",
        AxisProbeExtension(windows.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    wheel_named(&mut v, &view, "source", -400., gpui::TouchPhase::Started);
    wheel_named(&mut v, &view, "source", 0., gpui::TouchPhase::Ended);
    pump(cx, &mut v);
    assert!(xwindow(&windows, "target").0 > 30.);
    click_control(&mut v, &view, "Remove");
    pump(cx, &mut v);
    let after = xwindow(&windows, "target");
    println!("SOURCE_UNMOUNT target={after:?}");
    assert!(
        window_eq(after, (0., 100.)),
        "removed source left a stale link projection"
    );
}
