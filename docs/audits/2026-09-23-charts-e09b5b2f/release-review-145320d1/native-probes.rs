// Reuse the unchanged previous audit controls and public native fixture helpers.
include!(concat!(
    env!("GPUI_RHAI_REVIEW_REPO"),
    "/docs/audits/2026-09-23-charts-e09b5b2f/core-review-a31da5a8/native-probes.rs"
));

type AxisWindows = std::sync::Arc<std::sync::Mutex<BTreeMap<String, ((f64, f64), (f64, f64))>>>;
struct AxisProbe(AxisWindows);
impl HostChartSeries for AxisProbe {
    fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let x = c.x_scale.unwrap();
        let y = c.y_scale.unwrap();
        self.0.lock().unwrap().insert(
            c.spec.key.clone(),
            (
                (
                    x.invert(c.bounds.x).unwrap(),
                    x.invert(c.bounds.x + c.bounds.width).unwrap(),
                ),
                (
                    y.invert(c.bounds.y + c.bounds.height).unwrap(),
                    y.invert(c.bounds.y).unwrap(),
                ),
            ),
        );
        Ok(Vec::new())
    }
}
struct AxisProbeExtension(AxisWindows);
impl ScriptViewExtension for AxisProbeExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_series("axis_probe", AxisProbe(self.0.clone()))
            .map_err(|e| e.to_string())
    }
}

#[gpui::test]
fn linked_cartesian_windows_align_both_axes_with_different_full_domains(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let src = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}
fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:if k=="source"{ctx.get_state("z")}else{1.0},viewport_revision:if k=="source"{ctx.get_state("rev")}else{0},spec:#{title:k,legend:#{visible:false},link_group:"xy",link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:if k=="source"{100}else{200}},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:if k=="source"{Fn("zoomed")}else{()}}).with_style(style().width(px(280)).height(px(240)))}
fn view(ctx){column([text("probe"),row([one(ctx,"source"),one(ctx,"target")])])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "cartesian-xy-domains",
        AxisProbeExtension(windows.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let initial = windows.lock().unwrap().clone();
    assert!(
        !initial.is_empty(),
        "fixture did not layout: {:?}",
        v.update(|_, cx| view
            .accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .map(|n| (
                n.role.clone(),
                n.name.clone(),
                n.description.clone(),
                n.geometry
            ))
            .collect::<Vec<_>>())
    );
    assert_eq!(initial["source"], ((0.0, 100.0), (0.0, 100.0)));
    assert_eq!(initial["target"], ((0.0, 200.0), (0.0, 100.0)));
    let wheel = figure_point(&mut v, &view, "source", ChartPoint { x: 150., y: 140. });
    v.simulate_event(gpui::ScrollWheelEvent {
        position: wheel,
        delta: gpui::ScrollDelta::Lines(point(0., -25.)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut v);
    let result = windows.lock().unwrap().clone();
    let source = result["source"];
    let target = result["target"];
    println!("LINKED_XY source={source:?} target={target:?}");
    assert!(source.0.0 > 20.0, "fixture must commit a real source zoom");
    assert!(
        (source.0.0 - target.0.0).abs() < 1e-6 && (source.0.1 - target.0.1).abs() < 1e-6,
        "X windows diverged"
    );
    assert!(
        (source.1.0 - target.1.0).abs() < 1e-6 && (source.1.1 - target.1.1).abs() < 1e-6,
        "Y window was compressed by the zoom derived only from X"
    );
}

struct SuspendFailureChartExtension {
    motion: MotionExtension,
    hooks: HookExtension,
}
impl ScriptViewExtension for SuspendFailureChartExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        self.motion.configure_engine(e)?;
        self.hooks.configure_engine(e)
    }
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        self.motion.configure_runtime(r)
    }
}

#[gpui::test]
fn failed_suspend_restores_chart_streaming_in_active_view(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    states[1].fail_suspend.set(true);
    let src = r#"import "charts/chart" as chart;fn view(ctx){column([chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300))),audit::P0(#{key:"a"}),audit::P1(#{key:"b"}),audit::P2(#{key:"d"})])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "failed-suspend-stream",
        SuspendFailureChartExtension {
            motion: MotionExtension {
                data: data.clone(),
                positions: positions.clone(),
            },
            hooks: HookExtension(states.clone()),
        },
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let old = presented_revision(&mut v, &view);
    let failed = v.update(|w, cx| view.suspend(w, cx));
    assert!(failed.is_err());
    assert_eq!(view.state(), ScriptViewState::Active);
    assert!(states.iter().all(|s| s.active.get()));
    let wanted = data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut v);
    let actual = presented_revision(&mut v, &view);
    println!(
        "SUSPEND_ROLLBACK_STREAM result={failed:?} view={:?} before={old} wanted={wanted} presented={actual} layouts={}",
        view.state(),
        positions.lock().unwrap().len()
    );
    assert_eq!(
        actual, wanted,
        "failed suspension reports Active but leaves Chart unable to install stream updates"
    );
}

#[gpui::test]
fn acknowledged_geo_link_survives_suspend_resume(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let positions = std::sync::Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let src = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},source_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},target_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}fn source_clicked(ctx,p){ctx.set_state("source_hits",ctx.get_state("source_hits")+1);}fn target_clicked(ctx,p){ctx.set_state("target_hits",ctx.get_state("target_hits")+1);}
 fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0}],zoom:if k=="source"{ctx.get_state("z")}else{1.0},viewport_revision:if k=="source"{ctx.get_state("rev")}else{0},spec:#{title:k,legend:#{visible:false},link_group:"maps",link_domain:"location",regions:[#{key:"main",kind:"geo_2d",map:"map"}],series:[#{key:k,kind:"custom",renderer:"geo_marker",encode:#{x:"x",y:"y"}}]},on_zoom_change:if k=="source"{Fn("zoomed")}else{()},on_select:if k=="source"{Fn("source_clicked")}else{Fn("target_clicked")}}).with_style(style().width(px(280)).height(px(240)))}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("source_hits")}|${ctx.get_state("target_hits")}|${ctx.get_state("z")}`),row([one(ctx,"source"),one(ctx,"target")])])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "geo-link",
        GeoExtension(positions.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let ps = positions.lock().unwrap()["source"];
    let pt = positions.lock().unwrap()["target"];
    let source = figure_point(&mut v, &view, "source", ps);
    let target = figure_point(&mut v, &view, "target", pt);
    v.simulate_click(source, gpui::Modifiers::default());
    v.simulate_click(target, gpui::Modifiers::default());
    v.run_until_parked();
    assert!(status(&mut v, &view).starts_with("1|1|"));
    let wheel = figure_point(&mut v, &view, "source", ChartPoint { x: 150., y: 140. });
    v.simulate_event(gpui::ScrollWheelEvent {
        position: wheel,
        delta: gpui::ScrollDelta::Lines(point(0., -25.)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut v);
    v.simulate_click(source, gpui::Modifiers::default());
    v.simulate_click(target, gpui::Modifiers::default());
    v.run_until_parked();
    let result = status(&mut v, &view);
    println!("GEO_LINK source_hits|target_hits|zoom={result}");
    let zoom = result.split('|').nth(2).unwrap().parse::<f64>().unwrap();
    assert!(zoom > 2.0, "fixture must commit a real source zoom");
    assert!(
        result.starts_with("1|1|"),
        "source viewport changed but linked target remained at its old geometry"
    );
    v.update(|w, cx| view.suspend(w, cx).unwrap());
    v.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut v);
    v.simulate_click(source, gpui::Modifiers::default());
    v.simulate_click(target, gpui::Modifiers::default());
    v.run_until_parked();
    let resumed = status(&mut v, &view);
    println!("GEO_LINK_RESUME before={result} after={resumed}");
    assert!(
        resumed.starts_with("1|1|"),
        "a committed linked camera was reset only on the target during suspension"
    );
}

#[gpui::test]
fn failed_script_suspend_restores_chart_streaming_in_active_view(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let src = r#"import "charts/chart" as chart;fn suspend(ctx){throw "injected script suspend failure";}fn view(ctx){column([chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "failed-script-suspend-stream",
        MotionExtension {
            data: data.clone(),
            positions: positions.clone(),
        },
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let old = presented_revision(&mut v, &view);
    let failed = v.update(|w, cx| view.suspend(w, cx));
    assert!(failed.is_err());
    assert_eq!(view.state(), ScriptViewState::Active);
    let wanted = data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut v);
    let actual = presented_revision(&mut v, &view);
    println!(
        "SCRIPT_SUSPEND_ROLLBACK_STREAM result={failed:?} view={:?} before={old} wanted={wanted} presented={actual} layouts={}",
        view.state(),
        positions.lock().unwrap().len()
    );
    assert_eq!(
        actual, wanted,
        "failed suspension reports Active but leaves Chart unable to install stream updates"
    );
}
