use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
const THEME: &str = r#"fn theme(){#{family:"Audit",name:"Dark",mode:"dark",tokens:#{}}}"#;
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            self.view
                .element()
                .unwrap_or_else(|_| gpui::div().into_any_element()),
        )
    }
}
fn mount_prepared(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let saved = Rc::new(RefCell::new(None));
    let s = saved.clone();
    let name = name.to_owned();
    let win = cx.add_window(move |w, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), w, cx)
            .unwrap();
        *s.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    (win, view)
}
fn mount(cx: &mut TestAppContext, src: &str, name: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let id = ModuleId::parse("main").unwrap();
    mount_prepared(
        cx,
        EmbeddedScriptView::new(
            id.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(id, src.to_owned())])),
            THEME,
        )
        .prepare()
        .unwrap(),
        name,
    )
}
fn geometry(v: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) -> GeometryBounds {
    v.update(|w, cx| w.simulate_next_frame(cx));
    v.run_until_parked();
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.test_id.as_deref() == Some(id))
            .unwrap()
            .geometry
            .unwrap()
            .visual
    })
}
#[gpui::test]
fn stretch_width_preserves_horizontal_margins(cx: &mut TestAppContext) {
    let mut widths = vec![];
    for (name, explicit) in [("control", ".self_stretch()"), ("implicit", "")] {
        let src = format!(
            r#"fn view(ctx){{column([box([text("Child")]).accessibility_role("group").test_id("child").with_style(style().height(px(30)).margin_x(px(20)){explicit})]).accessibility_role("group").test_id("parent").with_style(style().width(px(300)).height(px(100)))}}"#
        );
        let (w, view) = mount(cx, &src, name);
        let mut v = VisualTestContext::from_window(*w, cx);
        let p = geometry(&mut v, &view, "parent");
        let c = geometry(&mut v, &view, "child");
        println!("MARGIN {name}: parent={p:?},child={c:?}");
        widths.push(c.width);
    }
    assert!((widths[0] - 260.0).abs() < 0.1);
    assert!(
        (widths[1] - 260.0).abs() < 0.1,
        "implicit stretch must subtract both margins: {widths:?}"
    );
}
#[gpui::test]
fn modified_key_capture_and_target_use_the_same_matching(cx: &mut TestAppContext) {
    let src = r#"fn state_schema(){#{fields:#{trace:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
 fn cap(ctx,p){ctx.set_state("trace",ctx.get_state("trace")+"C");event_response()}
 fn target(ctx,p){ctx.set_state("trace",ctx.get_state("trace")+"T");event_response()}
 fn view(ctx){column([text("Key").tab_stop(true).on_capture("key:shift+f6",Fn("cap")).on("key:shift+f6",Fn("target")),text(ctx.get_state("trace")).test_id("trace")])}"#;
    let (w, view) = mount(cx, src, "keys");
    let mut v = VisualTestContext::from_window(*w, cx);
    v.update(|w, cx| w.focus_next(cx));
    v.simulate_keystrokes("shift-f6");
    v.run_until_parked();
    let trace = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.test_id.as_deref() == Some("trace"))
            .unwrap()
            .name
            .clone()
    });
    println!("MODIFIED KEY trace={trace}");
    assert_eq!(trace, "CT");
}
#[test]
fn incomplete_environment_table_is_rejected() {
    let engine = RuntimeEngine::new();
    let good_base=load_token_base(engine.engine(),"complete.rhai",r#"fn tokens(){#{environment:#{density:#{values:["comfortable","compact"],"default":"compact"}},metrics:#{row:by_env("density",#{comfortable:px(32),compact:px(28)})}}}"#).unwrap();
    let good = load_theme_with_layers(
        engine.engine(),
        Some(&good_base),
        "good-theme.rhai",
        THEME,
        &ThemeTokenOverrides::default(),
    )
    .unwrap();
    assert_eq!(
        good.tokens
            .resolve_length(Length::token("metrics.row").unwrap(), &Environment::EMPTY),
        Some(Length::Pixels(28.0))
    );
    println!("COMPLETE default_resolution=Some(Pixels(28.0))");
    let base=load_token_base(engine.engine(),"incomplete.rhai",r#"fn tokens(){#{environment:#{density:#{values:["comfortable","compact"],"default":"compact"}},metrics:#{row:by_env("density",#{comfortable:px(32)})}}}"#).unwrap();
    let outcome = load_theme_with_layers(
        engine.engine(),
        Some(&base),
        "theme.rhai",
        THEME,
        &ThemeTokenOverrides::default(),
    );
    if let Ok(theme) = &outcome {
        println!(
            "INCOMPLETE required_token_validation={:?}",
            theme.require("components/probe", ["metrics.row"])
        );
        let value = theme
            .tokens
            .resolve_length(Length::token("metrics.row").unwrap(), &Environment::EMPTY);
        println!("INCOMPLETE accepted=true default_resolution={value:?}");
    }
    assert!(
        outcome.is_err(),
        "declared default has no value but preparation accepted the theme"
    );
}
const COMPONENT: &str = r#"/* gpui-rhai
{"id":"components/probe","export":"Probe","version":"0.2.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"tokens":["metrics.unit"],"environment":[]}
*/
define_component(#{metadata:#{id:"components/probe","export":"Probe",version:"0.2.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},tokens:["metrics.unit"],environment:[]},schema:#{props:#{},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_probe")});
fn Probe(props){render_component("components/probe",props)}
fn render_probe(ctx,props){box([text("Token")]).accessibility_role("group").test_id("token-cell").with_style(style().width(px(100)).height(theme_length("metrics.unit")))}"#;
#[gpui::test]
fn file_token_reload_keeps_requirement_validation(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("gpui-pr111-reload-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("components")).unwrap();
    std::fs::write(dir.join("app.toml"), "entry = \"main\"\nruntime_api = 3\n").unwrap();
    std::fs::write(
        dir.join("main.rhai"),
        "import \"components/probe\" as p; fn view(ctx){p::Probe(#{})}",
    )
    .unwrap();
    std::fs::write(dir.join("components/probe.rhai"), COMPONENT).unwrap();
    std::fs::write(dir.join("theme.rhai"), THEME).unwrap();
    std::fs::write(
        dir.join("tokens.rhai"),
        "fn tokens(){#{metrics:#{unit:px(40)}}}",
    )
    .unwrap();
    let entry = dir.join("main.rhai");
    let p = FileScriptView::new(&entry)
        .development(true)
        .prepare()
        .unwrap();
    let (w, view) = mount_prepared(cx, p, "reload");
    let mut v = VisualTestContext::from_window(*w, cx);
    assert_eq!(geometry(&mut v, &view, "token-cell").height, 40.0);
    let settle = |v: &mut VisualTestContext| {
        for _ in 0..15 {
            std::thread::sleep(Duration::from_millis(100));
            v.executor().advance_clock(Duration::from_millis(120));
            v.run_until_parked();
        }
    };
    settle(&mut v);
    std::fs::write(
        dir.join("tokens.rhai"),
        "fn tokens(){#{metrics:#{unit:px(50)}}}",
    )
    .unwrap();
    settle(&mut v);
    let control = geometry(&mut v, &view, "token-cell").height;
    println!(
        "RELOAD valid height={control} error={:?} theme={:?}",
        v.update(|_, cx| view.last_error(cx).unwrap()),
        v.update(|_, cx| view
            .theme_snapshot(cx)
            .unwrap()
            .variant
            .tokens
            .length_token("metrics.unit")
            .cloned())
    );
    assert_eq!(control, 50.0);
    std::fs::write(dir.join("tokens.rhai"), "fn tokens(){#{}} ").unwrap();
    settle(&mut v);
    let height = geometry(&mut v, &view, "token-cell").height;
    let failure = v.update(|_, cx| view.last_error(cx).unwrap());
    let cold = FileScriptView::new(&entry).development(false).prepare();
    println!(
        "RELOAD missing height={height} last_error={failure:?} fresh_prepare_error={:?}",
        cold.err().map(|e| e.to_string())
    );
    v.update(|_, cx| view.dispose(cx)).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(
        failure.is_some() && (height - 50.0).abs() < 0.1,
        "invalid token reload must retain last-good and expose an error"
    );
}
#[derive(Clone)]
struct Rows;
impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        let rows = (0..3).map(|i| {
            BTreeMap::from([
                ("id".into(), UiValue::String(format!("r{i}"))),
                ("label".into(), UiValue::String(format!("Row {i}"))),
            ])
        });
        r.native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|e| e.to_string())
    }
}
#[gpui::test]
fn table_keyboard_supports_native_data_as_well_as_array(cx: &mut TestAppContext) {
    let template = r#"import "components/table" as table;
 fn state_schema(){#{fields:#{selected:#{schema:#{type:"string"},"default":#{type:"string",value:"r0"}}}}}
 fn changed(ctx,p){ctx.set_state("selected",p[0]);}
 fn view(ctx){column([table::Table(#{key:"rows",label:"Table",row_key:"id",height:120,rows:ROWS,columns:[#{key:"label",title:"Label",width:#{kind:"fixed",value:200}}],selection_mode:"single",selected_keys:[ctx.get_state("selected")],on_selection_change:Fn("changed")}),text(ctx.get_state("selected")).test_id("selected")]).with_style(style().width(px(300)))}"#;
    let mut results = vec![];
    for (name, rows) in [
        (
            "array",
            "[#{id:\"r0\",label:\"Row 0\"},#{id:\"r1\",label:\"Row 1\"},#{id:\"r2\",label:\"Row 2\"}]",
        ),
        ("native", "ctx.get_native_collection(\"rows\")"),
    ] {
        let mut modules = BTreeMap::from([(
            ModuleId::parse("main").unwrap(),
            template.replace("ROWS", rows),
        )]);
        for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
            modules.insert(ModuleId::parse(*id).unwrap(), source.to_string());
        }
        let prepared = EmbeddedScriptView::new(
            ModuleId::parse("main").unwrap(),
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
        .extension(Rows)
        .prepare()
        .unwrap();
        let (w, view) = mount_prepared(cx, prepared, &format!("table-{name}"));
        let mut v = VisualTestContext::from_window(*w, cx);
        v.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.focus_next(cx)
        });
        v.simulate_keystrokes("down");
        v.run_until_parked();
        v.executor().advance_clock(Duration::from_millis(32));
        v.run_until_parked();
        let selected = v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|n| n.test_id.as_deref() == Some("selected"))
                .unwrap()
                .name
                .clone()
        });
        println!("TABLE KEY {name} selected={selected}");
        results.push(selected);
    }
    assert_eq!(results, vec!["r1", "r1"]);
}
