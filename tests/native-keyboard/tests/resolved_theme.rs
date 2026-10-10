use gpui::{
    AppContext, Context, IntoElement, Render, TestAppContext, VisualTestContext, Window,
    WindowHandle,
};
use gpui_rhai::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

const DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
struct Themes {
    system: bool,
    none: bool,
}
impl ScriptViewExtension for Themes {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        if self.none {
            runtime.theme = None;
        } else if self.system {
            runtime
                .theme
                .as_mut()
                .unwrap()
                .set_app(ThemePreference::System {
                    family: "Default".into(),
                })
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}
fn builder(source: &str, system: bool, none: bool) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.into())])),
        DARK,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .theme_sources([("light".into(), LIGHT.into())])
    .extension(Themes { system, none })
}
fn mount(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    name: &str,
    commands: bool,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let config = ScriptViewConfig::new(&name);
        let view = if commands {
            prepared.mount_window(config, host.clone(), window, cx)
        } else {
            prepared.mount(config, host.clone(), window, cx)
        }
        .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow_mut().take().unwrap();
    (window, view)
}
fn settle(visual: &mut VisualTestContext) {
    for _ in 0..4 {
        visual
            .background_executor
            .advance_clock(std::time::Duration::from_millis(32));
        visual.run_until_parked();
        visual.update(|window, _| window.refresh());
    }
}
fn labels(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "status")
            .map(|node| node.name.clone())
            .collect()
    })
}
fn dispatch(visual: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId { id: id.into() },
                    event: "click".into(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    settle(visual);
}

const INIT: &str = r#"
fn state_schema(){#{fields:#{starts:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},seen:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn init(ctx){ctx.set_state("starts",ctx.get_state("starts")+1);ctx.set_state("seen",ctx.theme_variant().mode);}
fn light(ctx,payload){ctx.set_theme("Default","Light");}
fn dark(ctx,payload){ctx.set_theme("Default","Dark");}
fn view(ctx){let theme=ctx.theme_variant();column([text(`${ctx.get_state("starts")}:${ctx.get_state("seen")}:${theme.family}:${theme.name}:${theme.mode}`).accessibility_role("status"),text("Light").test_id("light").on_click(Fn("light")),text("Dark").test_id("dark").on_click(Fn("dark"))])}
"#;

#[gpui::test]
fn first_system_light_init_and_render_match_host_without_replaying_init(cx: &mut TestAppContext) {
    let (window, view) = mount(
        cx,
        builder(INIT, true, false).prepare().unwrap(),
        "initial-light",
        false,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    assert_eq!(
        visual.update(|window, _| window.appearance()),
        gpui::WindowAppearance::Light
    );
    assert_eq!(
        labels(&mut visual, &view),
        vec!["1:light:Default:Light:light"]
    );
    assert_eq!(
        visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant.mode),
        ThemeMode::Light
    );
    visual
        .update(|_, cx| view.select_theme("Default", "Dark", cx))
        .unwrap();
    settle(&mut visual);
    assert_eq!(
        labels(&mut visual, &view),
        vec!["1:light:Default:Dark:dark"]
    );
    dispatch(&mut visual, &view, "light");
    assert_eq!(
        labels(&mut visual, &view),
        vec!["1:light:Default:Light:light"]
    );
    dispatch(&mut visual, &view, "dark");
    assert_eq!(
        labels(&mut visual, &view),
        vec!["1:light:Default:Dark:dark"]
    );
}

#[gpui::test]
fn fixed_dark_and_host_token_overrides_preserve_resolved_identity(cx: &mut TestAppContext) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("background".into(), Rgba8::from_rgb_hex(0x00ff_ffff))]),
        ..ThemeTokenOverrides::default()
    };
    let (window, view) = mount(
        cx,
        builder(INIT, false, false)
            .theme_token_overrides(overrides)
            .prepare()
            .unwrap(),
        "override",
        false,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    assert_eq!(labels(&mut visual, &view), vec!["1:dark:Default:Dark:dark"]);
    let snapshot = visual.update(|_, cx| view.theme_snapshot(cx).unwrap());
    assert_eq!(snapshot.variant.mode, ThemeMode::Dark);
    assert_eq!(
        snapshot.variant.tokens.color("background"),
        Some(Rgba8::from_rgb_hex(0x00ff_ffff))
    );
}

const EFFECT: &str = r#"
define_component(#{metadata:#{id:"tests/theme_probe","export":"ThemeProbe",version:"0.1.8",runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{starts:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},seen:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}},events:#{},slots:#{},parts:[],effects:["observe"]},render:Fn("probe")});
fn start(ctx,deps){let theme=if deps==(){ctx.theme_variant()}else{deps};ctx.set_state("starts",ctx.get_state("starts")+1);ctx.set_state("seen",theme.mode);}
fn stop(ctx,deps){}
fn probe(ctx,props){effect("observe",(),Fn("start"),Fn("stop"));text(`${ctx.get_state("starts")}:${ctx.get_state("seen")}`).accessibility_role("status")}
fn view(ctx){render_component("tests/theme_probe",#{key:"probe"})}
"#;
#[gpui::test]
fn effect_body_only_does_not_restart_but_explicit_render_deps_do(cx: &mut TestAppContext) {
    for explicit in [false, true] {
        let source = if explicit {
            EFFECT.replace(
                "effect(\"observe\",(),",
                "effect(\"observe\",ctx.theme_variant(),",
            )
        } else {
            EFFECT.into()
        };
        let (window, view) = mount(
            cx,
            builder(&source, false, false).prepare().unwrap(),
            if explicit { "explicit" } else { "body-only" },
            false,
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual);
        assert_eq!(labels(&mut visual, &view), vec!["1:dark"]);
        visual
            .update(|_, cx| view.select_theme("Default", "Light", cx))
            .unwrap();
        settle(&mut visual);
        assert_eq!(
            labels(&mut visual, &view),
            vec![if explicit { "2:light" } else { "1:dark" }]
        );
        assert_eq!(
            visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant.mode),
            ThemeMode::Light
        );
        // Reapplying the same resolved identity does not create a new activation.
        assert!(
            !visual
                .update(|_, cx| view.select_theme("Default", "Light", cx))
                .unwrap()
        );
        settle(&mut visual);
        assert_eq!(
            labels(&mut visual, &view),
            vec![if explicit { "2:light" } else { "1:dark" }]
        );
    }
}

#[gpui::test]
fn window_and_local_setters_keep_sibling_scopes_separate(cx: &mut TestAppContext) {
    let source = r#"
define_component(#{metadata:#{id:"tests/panel","export":"Panel",version:"0.1.8",runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("panel")});
fn local_light(ctx,payload){ctx.set_local_theme("Default","Light");}
fn local_dark(ctx,payload){ctx.set_local_theme("Default","Dark");}
fn window_light(ctx,payload){ctx.set_window_theme("Default","Light");}
fn panel(ctx,props){let theme=ctx.theme_variant();column([text(`${props.key}:${theme.name}:${theme.mode}`).accessibility_role("status"),text("Local light").test_id(`${props.key}-light`).on_click(Fn("local_light")),text("Local dark").test_id(`${props.key}-dark`).on_click(Fn("local_dark"))])}
fn view(ctx){let theme=ctx.theme_variant();column([text(`root:${theme.name}:${theme.mode}`).accessibility_role("status"),text("Window light").test_id("window-light").on_click(Fn("window_light")),render_component("tests/panel",#{key:"a"}),render_component("tests/panel",#{key:"b"})])}
"#;
    let (window, view) = mount(
        cx,
        builder(source, false, false).prepare().unwrap(),
        "scope",
        false,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    assert_eq!(
        labels(&mut visual, &view),
        vec!["root:Dark:dark", "a:Dark:dark", "b:Dark:dark"]
    );
    dispatch(&mut visual, &view, "a-light");
    assert_eq!(
        labels(&mut visual, &view),
        vec!["root:Dark:dark", "a:Light:light", "b:Dark:dark"]
    );
    dispatch(&mut visual, &view, "window-light");
    assert_eq!(
        labels(&mut visual, &view),
        vec!["root:Light:light", "a:Light:light", "b:Light:light"]
    );
    dispatch(&mut visual, &view, "a-dark");
    assert_eq!(
        labels(&mut visual, &view),
        vec!["root:Light:light", "a:Dark:dark", "b:Light:light"]
    );
}

#[gpui::test]
fn missing_theme_is_unit_in_a_mounted_script(cx: &mut TestAppContext) {
    let(window,view)=mount(cx,builder(r#"fn view(ctx){text(if ctx.theme_variant()==(){"none"}else{"unexpected"}).accessibility_role("status")}"#,false,true).prepare().unwrap(),"none",false);
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    assert_eq!(labels(&mut visual, &view), vec!["none"]);
    assert_eq!(view.state(), ScriptViewState::Active);
}

#[derive(Clone)]
struct Capture {
    records: Rc<RefCell<Vec<String>>>,
    fail_next: Rc<Cell<bool>>,
}
impl CapabilityHandler for Capture {
    fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String> {
        match (method, input) {
            ("record", UiValue::String(value)) => {
                self.records.borrow_mut().push(value);
                Ok(UiValue::Null)
            }
            ("fail_next", UiValue::Null) => Ok(UiValue::Bool(self.fail_next.replace(false))),
            _ => Err("invalid capture request".into()),
        }
    }
}
impl ScriptViewExtension for Capture {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: CapabilityId::parse("app.capture").unwrap(),
                    version: semver::Version::new(1, 0, 0),
                    methods: BTreeMap::from([
                        (
                            "record".into(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::Null,
                            },
                        ),
                        (
                            "fail_next".into(),
                            CapabilityMethod {
                                input: ValueSchema::Null,
                                output: ValueSchema::Bool,
                            },
                        ),
                    ]),
                },
                self.clone(),
            )
            .map_err(|err| err.to_string())
    }
}
const CHILD: &str = r#"
fn init(ctx){ctx.call_capability("app.capture","record",`init:${ctx.window_id()}:${ctx.theme_variant().mode}`);if ctx.window_id()=="child"&&ctx.call_capability("app.capture","fail_next",()){ctx.open_window("leaked","Leaked",400,300,false);throw "child-startup-failure";}}
fn open(ctx,payload){ctx.open_window("child","Child",400,300,false);}
define_component(#{metadata:#{id:"tests/label","export":"Label",version:"0.1.8",runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{"app.capture":"*"}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[],effects:["label"]},render:Fn("label")});
fn start_label(ctx,deps){ctx.call_capability("app.capture","record",`view:${ctx.window_id()}:${deps.mode}`);}
fn stop_label(ctx,deps){}
fn label(ctx,props){let theme=ctx.theme_variant();effect("label",theme,Fn("start_label"),Fn("stop_label"));text(theme.mode).accessibility_role("status")}
fn view(ctx){column([render_component("tests/label",#{key:"label"}),text("Open").test_id("open").on_click(Fn("open"))])}
"#;
#[gpui::test]
fn secondary_initial_theme_and_failed_startup_keep_native_authority_closed(
    cx: &mut TestAppContext,
) {
    for fail_first in [false, true] {
        let capture = Capture {
            records: Rc::new(RefCell::new(Vec::new())),
            fail_next: Rc::new(Cell::new(fail_first)),
        };
        let manifest = AppManifest::new(ModuleId::parse("main").unwrap())
            .with_capability("app.capture", "*")
            .unwrap();
        let prepared = builder(CHILD, true, false)
            .manifest(manifest)
            .extension(capture.clone())
            .prepare()
            .unwrap();
        let (window, view) = mount(
            cx,
            prepared,
            if fail_first { "retry" } else { "root" },
            true,
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual);
        assert_eq!(capture.records.borrow().len(), 2);
        if fail_first {
            let error = visual
                .update(|window, cx| {
                    view.automate(
                        AutomationCommand::Dispatch {
                            locator: AutomationLocator::TestId { id: "open".into() },
                            event: "click".into(),
                            payload: None,
                        },
                        window,
                        cx,
                    )
                })
                .unwrap_err();
            assert!(
                error.to_string().contains("child-startup-failure"),
                "{error}"
            );
            settle(&mut visual);
            assert_eq!(
                cx.windows().len(),
                1,
                "failed child and its queued leaked window must not survive"
            );
            assert_eq!(view.state(), ScriptViewState::Active);
            dispatch(&mut visual, &view, "open");
        } else {
            dispatch(&mut visual, &view, "open");
        }
        assert_eq!(
            cx.windows().len(),
            2,
            "same child reservation is reusable after cancellation"
        );
        let expected_root = if fail_first { "retry" } else { "root" };
        let mut expected = vec![
            format!("init:{expected_root}:light"),
            format!("view:{expected_root}:light"),
        ];
        if fail_first {
            expected.push("init:child:light".into());
        }
        expected.extend(["init:child:light".into(), "view:child:light".into()]);
        assert_eq!(
            *capture.records.borrow(),
            expected,
            "init/effect sees native appearance exactly once per attempt"
        );
        assert_eq!(
            visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant.mode),
            ThemeMode::Light
        );
        for handle in cx.windows() {
            cx.update(|cx| {
                cx.update_window(handle, |_, window, _| window.remove_window())
                    .unwrap()
            });
        }
        cx.run_until_parked();
    }
}
