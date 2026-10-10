//! Bounded actual macOS appearance smoke. Never substitutes TestPlatform.
//! Only this process's NSApplication appearance is overridden, then cleared.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("unsupported: actual native appearance override probe requires macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    actual_macos::run();
}

#[cfg(target_os = "macos")]
mod actual_macos {
    use gpui::{
        App, AppContext, AsyncApp, Bounds, Context, IntoElement, Render, Window, WindowAppearance,
        WindowBounds, WindowHandle, WindowOptions, px, size,
    };
    use gpui_rhai::*;
    use serde_json::{Value, json};
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
        path::PathBuf,
        rc::Rc,
        time::{Duration, Instant},
    };

    const SOURCE: &str = r#"
fn state_schema(){#{fields:#{init_count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},init_mode:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}},marker:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}}}
fn init(ctx){ctx.set_state("init_count",ctx.get_state("init_count")+1);ctx.set_state("init_mode",ctx.theme_variant().mode);ctx.call_capability("app.probe","record",`init:${ctx.window_id()}:${ctx.get_state("init_count")}:${ctx.get_state("init_mode")}`);}
fn increment(ctx,payload){ctx.set_state("marker",ctx.get_state("marker")+1);}
fn open(ctx,payload){ctx.open_window("child","Owned appearance child",400,240,false);}
define_component(#{metadata:#{id:"probe/theme_reader","export":"ThemeReader",version:"0.1.8",runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{"app.probe":"*"}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{starts:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{},slots:#{},parts:[],effects:["theme"]},render:Fn("reader")});
fn start(ctx,deps){let starts=ctx.get_state("starts")+1;ctx.set_state("starts",starts);ctx.call_capability("app.probe","record",`effect:${ctx.window_id()}:${starts}:${deps.mode}`);}
fn stop(ctx,deps){}
fn reader(ctx,props){effect("theme",ctx.theme_variant(),Fn("start"),Fn("stop"));text(`effect:${ctx.get_state("starts")}`).accessibility_role("status")}
fn view(ctx){let info=ctx.theme_variant();column([text(`${ctx.window_id()}:${ctx.get_state("init_count")}:${ctx.get_state("init_mode")}:${ctx.get_state("marker")}:${info.name}:${info.mode}`).accessibility_role("status"),render_component("probe/theme_reader",#{key:"theme-reader"}),text("Increment").test_id("increment").on_click(Fn("increment")),text("Open child").test_id("open").on_click(Fn("open"))]).with_style(style().padding(px(16)).gap(px(8)))}
"#;

    #[derive(Clone)]
    struct Records(Rc<RefCell<Vec<String>>>);
    impl CapabilityHandler for Records {
        fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
            let UiValue::String(value) = input else {
                return Err("record input must be a string".into());
            };
            self.0.borrow_mut().push(value);
            Ok(UiValue::Null)
        }
    }
    impl ScriptViewExtension for Records {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .theme
                .as_mut()
                .ok_or("theme manager missing")?
                .set_app(ThemePreference::System {
                    family: "Default".into(),
                })
                .map_err(|error| error.to_string())?;
            runtime
                .capabilities
                .register(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.probe").unwrap(),
                        version: semver::Version::new(1, 0, 0),
                        methods: BTreeMap::from([(
                            "record".into(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::Null,
                            },
                        )]),
                    },
                    self.clone(),
                )
                .map_err(|error| error.to_string())
        }
    }

    struct Host {
        domain: ScriptViewHost,
        view: ScriptViewHandle,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.domain.container(self.view.element().unwrap())
        }
    }

    struct Options {
        initial: WindowAppearance,
        output: PathBuf,
        source_sha: String,
    }
    fn options() -> Result<Options, String> {
        let mut args = std::env::args().skip(1);
        let mut initial = None;
        let mut output = None;
        let mut source_sha = None;
        while let Some(key) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {key}"))?;
            match key.as_str() {
                "--initial" => {
                    initial = Some(match value.as_str() {
                        "light" => WindowAppearance::Light,
                        "dark" => WindowAppearance::Dark,
                        _ => return Err("initial must be light or dark".into()),
                    });
                }
                "--output" => output = Some(PathBuf::from(value)),
                "--source-sha" => source_sha = Some(value),
                _ => return Err(format!("unexpected argument {key}")),
            }
        }
        Ok(Options {
            initial: initial.ok_or("--initial is required")?,
            output: output.ok_or("--output is required")?,
            source_sha: source_sha.ok_or("--source-sha is required")?,
        })
    }

    fn mode(appearance: WindowAppearance) -> &'static str {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => "light",
            WindowAppearance::Dark | WindowAppearance::VibrantDark => "dark",
        }
    }
    fn theme_mode(mode: &str) -> ThemeMode {
        if mode == "light" {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        }
    }
    fn prepared(records: Records) -> PreparedScriptView {
        let entry = ModuleId::parse("main").unwrap();
        EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), SOURCE.into())])),
            include_str!("../../../../registry/themes/default_dark.rhai"),
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .theme_sources([(
            "light".into(),
            include_str!("../../../../registry/themes/default_light.rhai").into(),
        )])
        .manifest(
            AppManifest::new(entry)
                .with_capability("app.probe", "*")
                .unwrap(),
        )
        .extension(records)
        .prepare()
        .expect("framework-owned appearance fixture prepares")
    }

    fn dispatch(primary: WindowHandle<Host>, cx: &mut AsyncApp, id: &str) -> Result<(), String> {
        primary
            .update(cx, |root, window, cx| {
                root.view
                    .automate(
                        AutomationCommand::Dispatch {
                            locator: AutomationLocator::TestId { id: id.into() },
                            event: "click".into(),
                            payload: None,
                        },
                        window,
                        cx,
                    )
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            })
            .map_err(|error| error.to_string())?
    }

    #[derive(Clone, Copy)]
    struct Expected {
        initial: &'static str,
        mode: &'static str,
        marker: u32,
        effects: u32,
        child: bool,
    }

    fn sample(
        primary: WindowHandle<Host>,
        cx: &mut AsyncApp,
        expected: Expected,
        records: &Records,
    ) -> Result<Value, String> {
        let Expected {
            initial,
            mode: expected,
            marker,
            effects,
            child,
        } = expected;
        let primary_sample = primary.update(cx, |root, window, cx| {
            let appearance = window.appearance();
            let snapshot = root.view.theme_snapshot(cx).map_err(|error| error.to_string())?;
            let semantic = root.view.accessibility_snapshot(cx).map_err(|error| error.to_string())?;
            let statuses = semantic.nodes().filter(|node| node.role == "status").collect::<Vec<_>>();
            let expected_label = format!("primary:1:{initial}:{marker}:{}:{expected}", if expected == "light" {"Light"} else {"Dark"});
            if mode(appearance) != expected || snapshot.variant.mode != theme_mode(expected)
                || statuses.first().is_none_or(|node| node.name != expected_label)
                || statuses.get(1).is_none_or(|node| node.name != format!("effect:{effects}"))
                || statuses.first().and_then(|node| node.geometry).is_none_or(|geometry| geometry.layout.width <= 0.0 || geometry.layout.height <= 0.0)
            {
                return Err(format!("primary not settled: native={appearance:?}, theme={:?}, statuses={:?}", snapshot.variant.mode, statuses.iter().map(|node| &node.name).collect::<Vec<_>>()));
            }
            Ok(json!({"native_appearance":format!("{appearance:?}"),"host_variant":snapshot.variant.name,"host_mode":format!("{:?}",snapshot.variant.mode),"statuses":statuses.iter().map(|node|&node.name).collect::<Vec<_>>(),"geometry_committed":true}))
        }).map_err(|error| error.to_string())??;
        let windows = cx.update(|cx| cx.windows());
        if windows.len() != if child { 2 } else { 1 } {
            return Err(format!("unexpected owned window count {}", windows.len()));
        }
        let mut child_appearance = None;
        if child {
            let handle = windows
                .into_iter()
                .find(|window| *window != *primary)
                .ok_or("child absent")?;
            let appearance = cx
                .update(|cx| cx.update_window(handle, |_, window, _| window.appearance()))
                .map_err(|error| error.to_string())?;
            if mode(appearance) != expected {
                return Err(format!("child appearance {appearance:?} != {expected}"));
            }
            child_appearance = Some(format!("{appearance:?}"));
        }
        for window in if child {
            vec!["primary", "child"]
        } else {
            vec!["primary"]
        } {
            let init = format!("init:{window}:1:{initial}");
            let effect = format!("effect:{window}:{effects}:{expected}");
            let observed = records.0.borrow();
            if observed
                .iter()
                .filter(|record| record.starts_with(&format!("init:{window}:")))
                .count()
                != 1
                || !observed.contains(&init)
                || !observed.contains(&effect)
            {
                return Err(format!("{window} init/effect not settled: {observed:?}"));
            }
        }
        Ok(
            json!({"expected":expected,"primary":primary_sample,"child_native_appearance":child_appearance,"records":records.0.borrow().clone()}),
        )
    }

    async fn wait_sample(
        primary: WindowHandle<Host>,
        cx: &mut AsyncApp,
        expected: Expected,
        records: &Records,
    ) -> Result<Value, String> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(50))
                .await;
            match sample(primary, cx, expected, records) {
                Ok(value) => return Ok(value),
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => {}
            }
        }
    }

    async fn exercise(
        primary: WindowHandle<Host>,
        cx: &mut AsyncApp,
        initial: WindowAppearance,
        records: &Records,
    ) -> Result<Vec<Value>, String> {
        let initial_mode = mode(initial);
        let opposite = if initial_mode == "light" {
            WindowAppearance::Dark
        } else {
            WindowAppearance::Light
        };
        let mut expected = Expected {
            initial: initial_mode,
            mode: initial_mode,
            marker: 7,
            effects: 1,
            child: false,
        };
        let mut samples = vec![wait_sample(primary, cx, expected, records).await?];
        dispatch(primary, cx, "increment")?;
        expected.marker = 8;
        samples.push(wait_sample(primary, cx, expected, records).await?);
        dispatch(primary, cx, "open")?;
        expected.child = true;
        samples.push(wait_sample(primary, cx, expected, records).await?);
        samples.push(idle(primary, cx, records).await?);
        cx.update(|cx| cx.set_window_appearance(Some(opposite)));
        expected.mode = mode(opposite);
        expected.effects = 2;
        samples.push(wait_sample(primary, cx, expected, records).await?);
        samples.push(idle(primary, cx, records).await?);
        cx.update(|cx| cx.set_window_appearance(Some(initial)));
        expected.mode = initial_mode;
        expected.effects = 3;
        samples.push(wait_sample(primary, cx, expected, records).await?);
        Ok(samples)
    }

    async fn idle(
        primary: WindowHandle<Host>,
        cx: &mut AsyncApp,
        records: &Records,
    ) -> Result<Value, String> {
        let _ = primary
            .update(cx, |root, _, cx| root.view.take_performance_snapshot(cx))
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let before = records.0.borrow().clone();
        cx.background_executor()
            .timer(Duration::from_millis(750))
            .await;
        let performance = primary
            .update(cx, |root, _, cx| root.view.take_performance_snapshot(cx))
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let after = records.0.borrow().clone();
        let evidence = json!({"stage":"idle-before-appearance-change","real_timer_ms":750,"primary_timing_count":performance.timings.len(),"shared_dirty_components":performance.dirty_components,"shared_pending_virtual_requests":performance.pending_virtual_requests,"init_effect_records_unchanged":before==after,"timings":performance.timings.iter().map(|timing|format!("{:?}",timing.operation)).collect::<Vec<_>>()});
        if !performance.timings.is_empty()
            || performance.dirty_components != 0
            || performance.pending_virtual_requests
            || before != after
        {
            return Err(format!("native app did not reach genuine idle: {evidence}"));
        }
        Ok(evidence)
    }

    fn cleanup(cx: &mut App) -> (Value, bool) {
        let owned = cx.windows();
        let mut errors = Vec::new();
        for window in &owned {
            if let Err(error) = cx.update_window(*window, |_, window, _| window.remove_window()) {
                errors.push(error.to_string());
            }
        }
        cx.set_window_appearance(None);
        let remaining = cx.windows().len();
        let clean = remaining == 0 && errors.is_empty();
        let evidence = json!({"close_requests":owned.len(),"errors":errors,"remaining_owned_windows":remaining,"owned_windows_closed":clean,"per_app_override_clear_called":true,"effective_app_appearance_after_clear":format!("{:?}",cx.window_appearance())});
        cx.quit();
        (evidence, clean)
    }

    pub(super) fn run() {
        let options = options().unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2)
        });
        let records = Records(Rc::new(RefCell::new(Vec::new())));
        let initial = options.initial;
        let done = Rc::new(Cell::new(false));
        let exit = Rc::new(Cell::new(124));
        let capture_exit = exit.clone();
        gpui_platform::application().run(move |cx| {
            install(cx);
            cx.set_window_appearance(Some(initial));
            let fixture=prepared(records.clone());
            let primary=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds::centered(None,size(px(480.0),px(240.0)),cx))),..WindowOptions::default()},move |window,cx|{
                cx.new(|cx| {
                    let domain=ScriptViewHost::new("primary",cx).unwrap();
                    let view=fixture.mount_window(ScriptViewConfig::new("primary"),domain.clone(),window,cx).unwrap();
                    Host{domain,view}
                })
            }).expect("actual MacPlatform primary opens");
            cx.activate(false);
            let watchdog_done=done.clone();
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(18)).await;
                if !watchdog_done.get(){eprintln!("native appearance probe watchdog elapsed");let (evidence,_) =cx.update(cleanup);eprintln!("watchdog cleanup: {evidence}");}
            }).detach();
            cx.spawn(async move |cx| {
                let result=exercise(primary,cx,initial,&records).await;
                done.set(true);
                let (cleanup_evidence,clean)=cx.update(cleanup);
                capture_exit.set(if result.is_ok()&&clean{0}else{1});
                let report=json!({"platform":"actual macOS MacPlatform (current_platform(false))","gpui_pre":"0.3.7","source_sha":options.source_sha,"initial":mode(initial),"samples":result.as_ref().ok(),"error":result.as_ref().err(),"records":records.0.borrow().clone(),"os_preferences_changed":false,"cleanup":cleanup_evidence,"process_id":std::process::id(),"binary_path":std::env::current_exe().ok()});
                let text=serde_json::to_string_pretty(&report).unwrap();
                println!("{text}");
                if let Err(error)=std::fs::write(&options.output,&text){eprintln!("failed writing {}: {error}",options.output.display());capture_exit.set(1);}
            }).detach();
        });
        std::process::exit(exit.get());
    }
}
