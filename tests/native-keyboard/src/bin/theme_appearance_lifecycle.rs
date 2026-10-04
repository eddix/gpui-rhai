//! Bounded, app-only real MacPlatform appearance/lifecycle matrix.
//! Does not modify OS preferences or fake the private TestPlatform appearance API.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("actual macOS appearance lifecycle probe requires macOS");
    std::process::exit(2);
}
#[cfg(target_os = "macos")]
fn main() {
    actual::run();
}

#[cfg(target_os = "macos")]
mod actual {
    use gpui::{
        App, AppContext, AsyncApp, Bounds, Context, IntoElement, ParentElement, Render, Styled,
        Window, WindowAppearance, WindowBounds, WindowHandle, WindowOptions, div, px, size,
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

    const DARK: &str = include_str!("../../../../registry/themes/default_dark.rhai");
    const LIGHT: &str = include_str!("../../../../registry/themes/default_light.rhai");
    const READER: &str = r#"
fn state_schema(){#{fields:#{init_count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},initial:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn init(ctx){ctx.set_state("init_count",ctx.get_state("init_count")+1);ctx.set_state("initial",ctx.theme_variant().mode);ctx.call_capability("app.matrix","record",`TAG:init:${ctx.get_state("init_count")}:${ctx.get_state("initial")}`);}
define_component(#{metadata:#{id:"matrix/reader","export":"Reader",version:"0.1.8",runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{"app.matrix":"*"}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{starts:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{},slots:#{},parts:[],effects:["theme"]},render:Fn("reader")});
fn start(ctx,deps){let n=ctx.get_state("starts")+1;ctx.set_state("starts",n);ctx.call_capability("app.matrix","record",`TAG:effect:${n}:${deps.mode}`);}
fn stop(ctx,deps){}
fn reader(ctx,props){effect("theme",ctx.theme_variant(),Fn("start"),Fn("stop"));text(`effect:${ctx.get_state("starts")}`).accessibility_role("status")}
fn view(ctx){let info=ctx.theme_variant();column([text(`TAG:${ctx.get_state("init_count")}:${ctx.get_state("initial")}:${info.mode}`).accessibility_role("status"),render_component("matrix/reader",#{key:"reader"})]).with_style(style().padding(px(12)).background(theme_color("surface")))}
"#;
    const TOKEN: &str = r#"
fn init(ctx){ctx.call_capability("app.matrix","record","T:init");}
fn view(ctx){text("token-only").accessibility_role("status").with_style(style().padding(px(12)).background(theme_color("surface")).text_color(theme_color("text_primary")))}
"#;

    #[derive(Clone)]
    struct Records {
        values: Rc<RefCell<Vec<String>>>,
        system: bool,
    }
    impl CapabilityHandler for Records {
        fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
            let UiValue::String(input) = input else {
                return Err("record needs a string".into());
            };
            self.values.borrow_mut().push(input);
            Ok(UiValue::Null)
        }
    }
    impl ScriptViewExtension for Records {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            if self.system {
                runtime
                    .theme
                    .as_mut()
                    .unwrap()
                    .set_app(ThemePreference::System {
                        family: "Default".into(),
                    })
                    .map_err(|error| error.to_string())?;
            }
            runtime
                .capabilities
                .register(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.matrix").unwrap(),
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
    fn prepared(
        tag: &str,
        records: &Records,
        token: bool,
        system: bool,
    ) -> Result<PreparedScriptView, String> {
        let entry = ModuleId::parse("main").unwrap();
        let source = if token {
            TOKEN.into()
        } else {
            READER.replace("TAG", tag)
        };
        EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), source)])),
            DARK,
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .theme_sources([("light".into(), LIGHT.into())])
        .manifest(
            AppManifest::new(entry)
                .with_capability("app.matrix", "*")
                .unwrap(),
        )
        .extension(Records {
            values: records.values.clone(),
            system,
        })
        .prepare()
        .map_err(|error| error.to_string())
    }
    struct Host {
        domain: ScriptViewHost,
        views: Vec<ScriptViewHandle>,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.domain.container(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .children(self.views.iter().filter_map(|view| view.flex_item().ok())),
            )
        }
    }
    type Handle = WindowHandle<Host>;
    async fn pause(cx: &mut AsyncApp, millis: u64) {
        cx.background_executor()
            .timer(Duration::from_millis(millis))
            .await;
    }
    fn view_sample(view: &ScriptViewHandle, cx: &mut App) -> Result<Value, String> {
        let theme = view.theme_snapshot(cx).map_err(|error| error.to_string())?;
        let tree = view
            .accessibility_snapshot(cx)
            .map_err(|error| error.to_string())?;
        let nodes = tree
            .nodes()
            .filter(|node| node.role == "status")
            .collect::<Vec<_>>();
        let presented = nodes.iter().all(|node| {
            node.geometry
                .is_some_and(|g| g.layout.width > 0.0 && g.layout.height > 0.0)
        });
        Ok(
            json!({"id":view.view_id(),"state":format!("{:?}",view.state()),"mode":format!("{:?}",theme.variant.mode),
            "labels":nodes.iter().map(|node|&node.name).collect::<Vec<_>>(),"presented":presented}),
        )
    }
    fn sample(handle: Handle, cx: &mut AsyncApp) -> Result<Value, String> {
        handle
            .update(cx, |root, window, cx| {
                let mut views = Vec::new();
                for view in &root.views {
                    if view.state() == ScriptViewState::Active {
                        views.push(view_sample(view, cx)?);
                    }
                }
                Ok(json!({"native":format!("{:?}",window.appearance()),"views":views}))
            })
            .map_err(|error| error.to_string())?
    }
    async fn expect(
        handle: Handle,
        cx: &mut AsyncApp,
        expected: &[(&str, &str)],
        mode: ThemeMode,
    ) -> Result<Value, String> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            pause(cx, 40).await;
            let result = sample(handle, cx)?;
            let views = result["views"].as_array().unwrap();
            let matches = views.len() == expected.len()
                && views.iter().zip(expected).all(|(view, (label, theme))| {
                    view["labels"][0] == *label
                        && view["mode"] == *theme
                        && view["presented"] == true
                });
            let native = if mode == ThemeMode::Light {
                "Light"
            } else {
                "Dark"
            };
            if matches && result["native"] == native {
                return Ok(result);
            }
            if Instant::now() >= deadline {
                return Err(format!("frame did not settle to {expected:?}: {result}"));
            }
        }
    }
    fn timings(handle: Handle, cx: &mut AsyncApp) -> Result<Vec<usize>, String> {
        handle
            .update(cx, |root, _, cx| {
                root.views
                    .iter()
                    .map(|view| {
                        view.take_performance_snapshot(cx)
                            .map(|p| p.timings.len())
                            .map_err(|error| error.to_string())
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())?
    }
    async fn idle(handle: Handle, cx: &mut AsyncApp, records: &Records) -> Result<Value, String> {
        let _ = timings(handle, cx)?;
        let before = records.values.borrow().clone();
        pause(cx, 600).await;
        let counts = timings(handle, cx)?;
        if counts.iter().any(|count| *count != 0) || *records.values.borrow() != before {
            return Err(format!(
                "not idle: timings={counts:?}, records={:?}",
                records.values.borrow()
            ));
        }
        Ok(
            json!({"stage":"genuine-idle","milliseconds":600,"timings":counts,"records_unchanged":true}),
        )
    }
    fn effect_count(records: &Records, tag: &str) -> usize {
        records
            .values
            .borrow()
            .iter()
            .filter(|value| value.starts_with(&format!("{tag}:effect:")))
            .count()
    }
    fn init_count(records: &Records, tag: &str) -> usize {
        records
            .values
            .borrow()
            .iter()
            .filter(|value| value.starts_with(&format!("{tag}:init:")))
            .count()
    }
    fn check_records(records: &Records, expected: &[(&str, usize, usize)]) -> Result<(), String> {
        for (tag, init, effect) in expected {
            if init_count(records, tag) != *init || effect_count(records, tag) != *effect {
                return Err(format!(
                    "{tag} wrong init/effect counts: {:?}",
                    records.values.borrow()
                ));
            }
        }
        Ok(())
    }

    async fn exercise(
        handle: Handle,
        cx: &mut AsyncApp,
        records: &Records,
    ) -> Result<Vec<Value>, String> {
        let mut results = vec![
            expect(
                handle,
                cx,
                &[
                    ("A:1:light:light", "Light"),
                    ("B:1:light:light", "Light"),
                    ("F:1:dark:dark", "Dark"),
                ],
                ThemeMode::Light,
            )
            .await?,
        ];
        check_records(records, &[("A", 1, 1), ("B", 1, 1), ("F", 1, 1)])?;
        results.push(idle(handle, cx, records).await?);
        cx.update(|cx| cx.set_window_appearance(Some(WindowAppearance::Light)));
        results.push(idle(handle, cx, records).await?);
        results.push(json!({"stage":"same-native-mode-deduplicated"}));
        cx.update(|cx| cx.set_window_appearance(Some(WindowAppearance::Dark)));
        results.push(
            expect(
                handle,
                cx,
                &[
                    ("A:1:light:dark", "Dark"),
                    ("B:1:light:dark", "Dark"),
                    ("F:1:dark:dark", "Dark"),
                ],
                ThemeMode::Dark,
            )
            .await?,
        );
        check_records(records, &[("A", 1, 2), ("B", 1, 2), ("F", 1, 1)])?;
        results.push(json!({"stage":"independent-runtimes-one-window-and-fixed-control"}));

        handle
            .update(cx, |root, window, cx| {
                root.views[0]
                    .suspend(window, cx)
                    .map_err(|e| e.to_string())?;
                cx.notify();
                Ok::<_, String>(())
            })
            .map_err(|e| e.to_string())??;
        pause(cx, 120).await;
        let _ = timings(handle, cx)?;
        let before = records
            .values
            .borrow()
            .iter()
            .filter(|value| value.starts_with("A:"))
            .cloned()
            .collect::<Vec<_>>();
        cx.update(|cx| cx.set_window_appearance(Some(WindowAppearance::Light)));
        results.push(
            expect(
                handle,
                cx,
                &[("B:1:light:light", "Light"), ("F:1:dark:dark", "Dark")],
                ThemeMode::Light,
            )
            .await?,
        );
        pause(cx, 200).await;
        let counts = timings(handle, cx)?;
        let after = records
            .values
            .borrow()
            .iter()
            .filter(|value| value.starts_with("A:"))
            .cloned()
            .collect::<Vec<_>>();
        if counts[0] != 0 || before != after {
            return Err(format!("suspended A executed: {counts:?}, {after:?}"));
        }
        results.push(json!({"stage":"suspended-appearance-kept-pending","suspended_timing_count":counts[0],"records_unchanged":true}));
        handle
            .update(cx, |root, _, cx| {
                root.views[0].resume(cx).map_err(|e| e.to_string())?;
                cx.notify();
                Ok::<_, String>(())
            })
            .map_err(|e| e.to_string())??;
        results.push(
            expect(
                handle,
                cx,
                &[
                    ("A:1:light:light", "Light"),
                    ("B:1:light:light", "Light"),
                    ("F:1:dark:dark", "Dark"),
                ],
                ThemeMode::Light,
            )
            .await?,
        );
        check_records(records, &[("A", 1, 3), ("B", 1, 3), ("F", 1, 1)])?;

        let replacement = prepared("A2", records, false, true)?;
        let retired = handle
            .update(cx, |root, window, cx| {
                let retired = root.views[0].clone();
                retired.dispose(cx).map_err(|e| e.to_string())?;
                root.views[0] = replacement
                    .mount(ScriptViewConfig::new("A"), root.domain.clone(), window, cx)
                    .map_err(|e| e.to_string())?;
                cx.notify();
                Ok::<_, String>(retired)
            })
            .map_err(|e| e.to_string())??;
        results.push(
            expect(
                handle,
                cx,
                &[
                    ("A2:1:light:light", "Light"),
                    ("B:1:light:light", "Light"),
                    ("F:1:dark:dark", "Dark"),
                ],
                ThemeMode::Light,
            )
            .await?,
        );
        let old_records = records
            .values
            .borrow()
            .iter()
            .filter(|v| v.starts_with("A:"))
            .cloned()
            .collect::<Vec<_>>();
        results.push(idle(handle, cx, records).await?);
        cx.update(|cx| cx.set_window_appearance(Some(WindowAppearance::Dark)));
        results.push(
            expect(
                handle,
                cx,
                &[
                    ("A2:1:light:dark", "Dark"),
                    ("B:1:light:dark", "Dark"),
                    ("F:1:dark:dark", "Dark"),
                ],
                ThemeMode::Dark,
            )
            .await?,
        );
        let now = records
            .values
            .borrow()
            .iter()
            .filter(|v| v.starts_with("A:"))
            .cloned()
            .collect::<Vec<_>>();
        if retired.state() != ScriptViewState::Disposed || old_records != now {
            return Err("disposed mount received appearance delivery".into());
        }
        check_records(records, &[("A2", 1, 2), ("B", 1, 4), ("F", 1, 1)])?;
        results.push(json!({"stage":"retained-disposed-same-id-remount","old_state":"Disposed","old_records_unchanged":true}));

        let token = prepared("T", records, true, true)?;
        handle
            .update(cx, |root, window, cx| {
                root.views[0].dispose(cx).map_err(|e| e.to_string())?;
                root.views[1].dispose(cx).map_err(|e| e.to_string())?;
                let token = token
                    .mount(ScriptViewConfig::new("T"), root.domain.clone(), window, cx)
                    .map_err(|e| e.to_string())?;
                root.views = vec![token, root.views[2].clone()];
                cx.notify();
                Ok::<_, String>(())
            })
            .map_err(|e| e.to_string())??;
        results.push(
            expect(
                handle,
                cx,
                &[("token-only", "Dark"), ("F:1:dark:dark", "Dark")],
                ThemeMode::Dark,
            )
            .await?,
        );
        results.push(idle(handle, cx, records).await?);
        cx.update(|cx| cx.set_window_appearance(Some(WindowAppearance::Light)));
        results.push(
            expect(
                handle,
                cx,
                &[("token-only", "Light"), ("F:1:dark:dark", "Dark")],
                ThemeMode::Light,
            )
            .await?,
        );
        let counts = timings(handle, cx)?;
        if counts[0] != 0 {
            return Err(format!("token-only appearance ran Rhai: {counts:?}"));
        }
        results.push(json!({"stage":"token-only-native-repaint-and-host-snapshot","rhai_timing_count":counts[0],"geometry_and_theme_presented":true}));
        Ok(results)
    }
    fn cleanup(cx: &mut App) -> Value {
        let owned = cx.windows();
        let mut errors = Vec::new();
        for window in &owned {
            if let Err(error) = cx.update_window(*window, |_, window, _| window.remove_window()) {
                errors.push(error.to_string());
            }
        }
        cx.set_window_appearance(None);
        let result = json!({"close_requests":owned.len(),"remaining_owned_windows":cx.windows().len(),"errors":errors,"per_app_override_clear_called":true});
        cx.quit();
        result
    }
    pub(super) fn run() {
        let mut args = std::env::args().skip(1);
        let mut output = None;
        let mut source_sha = None;
        while let Some(key) = args.next() {
            let value = args.next().expect("option value");
            match key.as_str() {
                "--output" => output = Some(PathBuf::from(value)),
                "--source-sha" => source_sha = Some(value),
                _ => panic!("unexpected {key}"),
            }
        }
        let output = output.expect("--output required");
        let source_sha = source_sha.expect("--source-sha required");
        let records = Records {
            values: Rc::new(RefCell::new(Vec::new())),
            system: true,
        };
        let done = Rc::new(Cell::new(false));
        let exit = Rc::new(Cell::new(124));
        let final_exit = exit.clone();
        gpui_platform::application().run(move|cx|{
            install(cx);cx.set_window_appearance(Some(WindowAppearance::Light));
            let a=prepared("A",&records,false,true).unwrap();let b=prepared("B",&records,false,true).unwrap();let fixed=prepared("F",&records,false,false).unwrap();
            let handle=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds::centered(None,size(px(520.0),px(360.0)),cx))),..Default::default()},move|window,cx|
                cx.new(|cx|{let domain=ScriptViewHost::new("physical",cx).unwrap();let mut views=Vec::new();
                    for (id,prepared) in [("A",a),("B",b),("F",fixed)]{views.push(prepared.mount(ScriptViewConfig::new(id),domain.clone(),window,cx).unwrap());}
                    Host{domain,views}})).expect("one owned native window");
            cx.activate(false);
            let watchdog=done.clone();
            cx.spawn(async move|cx|{pause(cx,20_000).await;if !watchdog.get(){eprintln!("watchdog: {}",cx.update(cleanup));}}).detach();
            cx.spawn(async move|cx|{
                let result=exercise(handle,cx,&records).await;done.set(true);let cleanup=cx.update(cleanup);
                let passed=result.is_ok()&&cleanup["remaining_owned_windows"]==0&&cleanup["errors"].as_array().is_some_and(Vec::is_empty);
                final_exit.set(if passed {0}else{1});
                let report=json!({"source_sha":source_sha,"platform":"actual macOS MacPlatform","gpui_pre":"0.3.7","passed":passed,
                    "samples":result.as_ref().ok(),"error":result.as_ref().err(),"records":records.values.borrow().clone(),"cleanup":cleanup,
                    "os_preferences_changed":false,"maximum_own_windows":1,"maximum_live_views":3,"binary_path":std::env::current_exe().ok()});
                let text=serde_json::to_string_pretty(&report).unwrap();println!("{text}");if let Err(error)=std::fs::write(&output,text){eprintln!("report write failed: {error}");final_exit.set(1);}
            }).detach();
        });
        std::process::exit(exit.get());
    }
}
