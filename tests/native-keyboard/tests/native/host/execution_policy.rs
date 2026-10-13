use gpui::{AppContext, Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::rhai;
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, rc::Rc};

use crate::support::{by_test_id, dispatch, last_error};

const THEME: &str = gpui_rhai_registry::DEFAULT_THEME;
const HEAVY_VIEW: &str = r#"
fn view(ctx){let n=0;for i in 0..450000{n+=1;}text(`${n}`).accessibility_role("status")}
"#;

struct Host {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.as_ref().map_or_else(
            || gpui::div().into_any_element(),
            |view| view.element().unwrap(),
        ))
    }
}

#[derive(Clone)]
struct PolicyProbe {
    seen: Rc<RefCell<Vec<Policy>>>,
    override_policy: Option<Policy>,
}
type Policy = (u64, (usize, usize));
impl ScriptViewExtension for PolicyProbe {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        self.seen
            .borrow_mut()
            .push((engine.operation_limit(), engine.expression_depth_limits()));
        if let Some((quota, (global, functions))) = self.override_policy {
            engine.set_operation_limit(quota);
            engine.set_expression_depth_limits(global, functions);
        }
        Ok(())
    }
}

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new(source: &str) -> Self {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // Parallel tests can read the same clock value; a counter keeps roots unique.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gpui-rhai-host-policy-{}-{suffix}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("main.rhai"), source).unwrap();
        std::fs::write(root.join("theme.rhai"), THEME).unwrap();
        std::fs::write(root.join("app.toml"), "entry = \"main\"\nruntime_api = 3\n").unwrap();
        Self { root }
    }
    fn builder(&self) -> FileScriptView {
        FileScriptView::new(self.root.join("main.rhai")).development(false)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(
            self.root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("gpui-rhai-host-policy-")
        );
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn embedded(source: &str) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.into())])),
        THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
}

fn mount_result(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    name: &str,
    commands: bool,
) -> (
    gpui::WindowHandle<Host>,
    Result<ScriptViewHandle, ScriptViewError>,
) {
    cx.update(gpui_rhai::install);
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let result = if commands {
            prepared.mount_window(ScriptViewConfig::new("owner"), host.clone(), window, cx)
        } else {
            prepared.mount(ScriptViewConfig::new("owner"), host.clone(), window, cx)
        };
        let view = result.as_ref().ok().cloned();
        *capture.borrow_mut() = Some(result);
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let result = captured.borrow_mut().take().unwrap();
    (window, result)
}
fn remove(window: gpui::WindowHandle<Host>, cx: &mut TestAppContext) {
    window
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn file_and_embedded_mount_actual_over_default_work_with_host_quota(cx: &mut TestAppContext) {
    for file_source in [false, true] {
        for quota in [DEFAULT_SCRIPT_OPERATION_LIMIT, 2_000_000, 500_000] {
            let fixture = Fixture::new(HEAVY_VIEW);
            let prepared = if file_source {
                fixture.builder().operation_limit(quota).prepare()
            } else {
                embedded(HEAVY_VIEW).operation_limit(quota).prepare()
            }
            .unwrap();
            let (window, result) = mount_result(cx, prepared, "policy", false);
            assert_eq!(
                result.is_ok(),
                quota == 2_000_000,
                "file={file_source} quota={quota}: {:?}",
                result.as_ref().err()
            );
            match result {
                Ok(view) => {
                    let mut visual = VisualTestContext::from_window(*window, cx);
                    let names = visual.update(|_, cx| {
                        view.accessibility_snapshot(cx)
                            .unwrap()
                            .nodes()
                            .filter(|node| node.role == "status")
                            .map(|node| node.name.clone())
                            .collect::<Vec<_>>()
                    });
                    assert_eq!(names, vec!["450000"]);
                    let snapshot =
                        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
                    let render = snapshot
                        .timings
                        .iter()
                        .find(|timing| timing.operation == ExecutionOperation::Render)
                        .unwrap();
                    assert!(render.operations > DEFAULT_SCRIPT_OPERATION_LIMIT);
                    assert_eq!(render.operation_limit, quota);
                    assert!(render.succeeded);
                }
                Err(error) => {
                    let diagnostic = error.diagnostic().expect("runtime mount diagnostic");
                    let budget = diagnostic
                        .operation_budget
                        .as_ref()
                        .expect("actual failed execution quota");
                    assert_eq!(diagnostic.error_kind, Some(DiagnosticErrorKind::Terminated));
                    assert_eq!(budget.maximum, quota);
                    assert!(budget.consumed > quota);
                    assert_eq!(
                        diagnostic.token,
                        Some(UiValue::String(format!(
                            "script operation budget exceeded: {} > {quota}",
                            budget.consumed
                        )))
                    );
                    assert!(!diagnostic.execution.as_ref().unwrap().succeeded);
                }
            }
            remove(window, cx);
        }
    }
}

#[gpui::test]
fn child_engine_reapplies_builder_then_extension_policy(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
fn open(ctx,payload){ctx.open_window("child","Child",400,300,false);}
fn view(ctx){if ctx.window_id()=="child"{let n=0;for i in 0..450000{n+=1;}text(`${n}`)}
 else{text("Open").test_id("open").on_click(Fn("open"))}}
"#;
    for file_source in [false, true] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let probe = PolicyProbe {
            seen: seen.clone(),
            override_policy: Some((2_000_000, (64, 64))),
        };
        let fixture = Fixture::new(SOURCE);
        let prepared = if file_source {
            fixture
                .builder()
                .operation_limit(300_000)
                .expression_depth_limits(64, 48)
                .extension(probe)
                .prepare()
        } else {
            embedded(SOURCE)
                .operation_limit(300_000)
                .expression_depth_limits(64, 48)
                .extension(probe)
                .prepare()
        }
        .unwrap();
        let (window, result) = mount_result(cx, prepared, "root", true);
        let view = result.unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        dispatch(&mut visual, &view, by_test_id("open"), "click").unwrap();
        visual.run_until_parked();
        assert_eq!(
            cx.windows().len(),
            2,
            "child must actually mount 1.35M+ operations"
        );
        assert_eq!(view.state(), ScriptViewState::Active);
        assert_eq!(last_error(&mut visual, &view), None);
        assert_eq!(
            *seen.borrow(),
            vec![(300_000, (64, 48)); 2],
            "each independent engine sees builder policy before extension override"
        );
        for child in cx.windows().into_iter().filter(|handle| *handle != *window) {
            cx.update(|cx| {
                cx.update_window(child, |_, window, _| window.remove_window())
                    .unwrap()
            });
        }
        remove(window, cx);
    }
}

#[gpui::test]
fn expression_depth_host_policy_mounts_the_same_file_and_embedded_tree(cx: &mut TestAppContext) {
    let source = format!(
        "fn view(ctx){{text({}\"depth-ok\"{}).accessibility_role(\"status\")}}",
        "(".repeat(24),
        ")".repeat(24)
    );
    for file_source in [false, true] {
        let fixture = Fixture::new(&source);
        let prepared = if file_source {
            fixture.builder().expression_depth_limits(64, 64).prepare()
        } else {
            embedded(&source).expression_depth_limits(64, 64).prepare()
        }
        .unwrap();
        let (window, result) = mount_result(cx, prepared, "depth", false);
        let view = result.unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        let names = visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .filter(|node| node.role == "status")
                .map(|node| node.name.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(names, vec!["depth-ok"]);
        assert_eq!(view.state(), ScriptViewState::Active);
        remove(window, cx);
    }
}

/// A Host's policy for the Rhai standard library, applied with `configure_rhai_engine`:
/// `eval` disabled, every overload of `sleep`, `exit` and `timestamp` replaced with one that
/// fails, and `print` routed to the Host. `host_engine()` names the engine it configured, so
/// a script window shows which engine renders it.
#[derive(Clone, Default)]
struct StdPolicy {
    engines: Rc<std::cell::Cell<usize>>,
    printed: Rc<RefCell<Vec<String>>>,
}

impl ScriptViewExtension for StdPolicy {
    fn configure_rhai_engine(&self, engine: &mut rhai::Engine) -> Result<(), String> {
        use rhai::{Dynamic, EvalAltResult};
        type Refused = Result<(), Box<EvalAltResult>>;
        fn refuse(name: &str) -> Refused {
            Err(format!("`{name}` is not available in a view").into())
        }
        self.engines.set(self.engines.get() + 1);
        let engine_number = i64::try_from(self.engines.get()).unwrap();
        engine.disable_symbol("eval");
        engine.register_fn("sleep", |_: f64| -> Refused { refuse("sleep") });
        engine.register_fn("sleep", |_: i64| -> Refused { refuse("sleep") });
        engine.register_fn("exit", || -> Refused { refuse("exit") });
        engine.register_fn("exit", |_: Dynamic| -> Refused { refuse("exit") });
        engine.register_fn("timestamp", || -> Refused { refuse("timestamp") });
        let printed = self.printed.clone();
        engine.on_print(move |text| printed.borrow_mut().push(text.to_owned()));
        engine.register_fn("host_engine", move || engine_number);
        Ok(())
    }
}

fn statuses(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "status")
            .map(|node| node.name.clone())
            .collect()
    })
}

#[gpui::test]
fn a_host_replaces_standard_library_functions_in_file_and_embedded_views(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
fn view(ctx) {
    print("printed by the view");
    let results = [];
    try { sleep(0.05); results.push("sleep(0.05) ran"); } catch (error) { results.push(error); }
    try { sleep(1); results.push("sleep(1) ran"); } catch (error) { results.push(error); }
    try { exit(); results.push("exit() ran"); } catch (error) { results.push(error); }
    try { exit(1); results.push("exit(1) ran"); } catch (error) { results.push(error); }
    try { timestamp(); results.push("timestamp() ran"); } catch (error) { results.push(error); }
    let nodes = [];
    for result in results { nodes.push(text(result).accessibility_role("status")); }
    column(nodes)
}
"#;
    for file_source in [false, true] {
        let policy = StdPolicy::default();
        let fixture = Fixture::new(SOURCE);
        let prepared = if file_source {
            fixture.builder().extension(policy.clone()).prepare()
        } else {
            embedded(SOURCE).extension(policy.clone()).prepare()
        }
        .unwrap();
        let (window, result) = mount_result(cx, prepared, "std", false);
        let view = result.unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        assert_eq!(
            statuses(&mut visual, &view),
            [
                "`sleep` is not available in a view",
                "`sleep` is not available in a view",
                "`exit` is not available in a view",
                "`exit` is not available in a view",
                "`timestamp` is not available in a view",
            ],
            "file={file_source}"
        );
        assert!(
            policy
                .printed
                .borrow()
                .iter()
                .any(|text| text == "printed by the view"),
            "file={file_source}: print reaches the Host: {:?}",
            policy.printed.borrow()
        );
        remove(window, cx);

        // A disabled keyword fails preparation.
        let eval = "fn view(ctx) { text(`${eval(\"40 + 2\")}`) }";
        let fixture = Fixture::new(eval);
        let error = if file_source {
            fixture.builder().extension(StdPolicy::default()).prepare()
        } else {
            embedded(eval).extension(StdPolicy::default()).prepare()
        }
        .err()
        .expect("eval is disabled")
        .to_string();
        assert!(
            error.contains("'eval' is disabled"),
            "file={file_source}: {error}"
        );
    }
}

#[gpui::test]
fn a_script_window_gets_its_own_configured_engine(cx: &mut TestAppContext) {
    const SOURCE: &str = r#"
fn open(ctx,payload){ctx.open_window("child","Child",400,300,false);}
fn view(ctx){if ctx.window_id()=="child"{text(`engine ${host_engine()}`)}
 else{text(`engine ${host_engine()}`).test_id("open").on_click(Fn("open"))}}
"#;
    for file_source in [false, true] {
        let policy = StdPolicy::default();
        let fixture = Fixture::new(SOURCE);
        let prepared = if file_source {
            fixture.builder().extension(policy.clone()).prepare()
        } else {
            embedded(SOURCE).extension(policy.clone()).prepare()
        }
        .unwrap();
        let (window, result) = mount_result(cx, prepared, "root", true);
        let view = result.unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        dispatch(&mut visual, &view, by_test_id("open"), "click").unwrap();
        visual.run_until_parked();
        // The child renders `host_engine()`, which only `configure_rhai_engine` registers.
        assert_eq!(cx.windows().len(), 2, "file={file_source}");
        assert_eq!(last_error(&mut visual, &view), None);
        assert_eq!(
            policy.engines.get(),
            2,
            "preparation, then the child window"
        );
        for child in cx.windows().into_iter().filter(|handle| *handle != *window) {
            cx.update(|cx| {
                cx.update_window(child, |_, window, _| window.remove_window())
                    .unwrap()
            });
        }
        remove(window, cx);
    }
}
