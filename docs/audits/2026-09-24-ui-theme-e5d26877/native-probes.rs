use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
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
fn source(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(&std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()).join(path),
    )
    .unwrap()
}
fn mount(
    cx: &mut TestAppContext,
    script: &str,
    overrides: ThemeTokenOverrides,
    ext: impl ScriptViewExtension + 'static,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (entry.clone(), script.to_owned()),
        (
            ModuleId::parse("components/tabs").unwrap(),
            source("registry/components/tabs.rhai"),
        ),
    ]));
    let prepared =
        EmbeddedScriptView::new(entry, scripts, source("registry/themes/default_dark.rhai"))
            .theme_token_overrides(overrides)
            .extension(ext)
            .motion_preference(MotionPreference::None)
            .prepare()
            .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let take = captured.clone();
    let win = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("ui-audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("ui-audit"), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    (win, view)
}
struct Noop;
impl ScriptViewExtension for Noop {}
#[gpui::test]
fn tabs_track_accommodates_valid_theme_inset(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let overrides = ThemeTokenOverrides {
        spacing: BTreeMap::from([("xxs".into(), Length::Pixels(8.))]),
        ..Default::default()
    };
    let script = r#"import "components/tabs" as tabs;fn view(ctx){tabs::Tabs(#{label:"Sections",value:"a",tabs:[#{value:"a",label:"First",content:text("A")},#{value:"b",label:"Second",content:text("B")}]})}"#;
    let (w, view) = mount(cx, script, overrides, Noop);
    let mut visual = VisualTestContext::from_window(*w, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let list = tree
        .find_by_role_and_name("tablist", "Sections")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let tab = tree
        .find_by_role_and_name("tab", "First")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let top = tab.y - list.y;
    let bottom = list.y + list.height - tab.y - tab.height;
    let left = tab.x - list.x;
    println!(
        "THEMED_TABS list={list:?} tab={tab:?} expected_inset=8 left={left} top={top} bottom={bottom}"
    );
    assert!(
        top >= 7.9 && bottom >= 7.9,
        "fixed track height violates theme inset on top/bottom"
    );
}
struct TokenProbe(Rc<RefCell<Vec<Option<Rgba8>>>>);
impl PrimitiveHandler for TokenProbe {
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, String> {
        self.0.borrow_mut().push(theme.color("brand.tint"));
        Ok(gpui::div().into_any_element())
    }
}
struct TokenExtension(Rc<RefCell<Vec<Option<Rgba8>>>>);
impl ScriptViewExtension for TokenExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_primitive(
            PrimitiveDescriptor {
                id: PrimitiveId::parse("audit.token_probe").unwrap(),
                export: "TokenProbe".into(),
                props: BTreeMap::new(),
                events: BTreeMap::new(),
                state: ComponentStateSchema::default(),
                lifecycle: true,
                effect: None,
            },
            TokenProbe(self.0.clone()),
        )
        .map_err(|e| e.to_string())
    }
}
#[gpui::test]
fn native_theme_preserves_valid_namespaced_color(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let expected = Rgba8::from_rgba_hex(0x55aaccff);
    let overrides = ThemeTokenOverrides {
        namespaces: BTreeMap::from([(
            "brand".into(),
            BTreeMap::from([("tint".into(), ThemeTokenValue::Color(expected))]),
        )]),
        ..Default::default()
    };
    let script = r#"fn view(ctx){audit::TokenProbe(#{key:"probe"})}"#;
    let (_w, _view) = mount(cx, script, overrides, TokenExtension(seen.clone()));
    println!(
        "NATIVE_CUSTOM_TOKEN expected={expected:?} captures={:?}",
        seen.borrow()
    );
    assert!(!seen.borrow().is_empty());
    assert_eq!(
        *seen.borrow().last().unwrap(),
        Some(expected),
        "native semantic snapshot dropped a validated namespaced theme color"
    );
}

#[gpui::test]
fn tabs_slot_accommodates_theme_text_line(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let overrides = ThemeTokenOverrides {
        typography: ThemeTypographyOverrides {
            roles: BTreeMap::from([(
                "body".into(),
                TypographyToken {
                    size: Length::Pixels(24.),
                    line_height: Length::Pixels(36.),
                    weight: 400,
                },
            )]),
            ..Default::default()
        },
        ..Default::default()
    };
    let script = r#"import "components/tabs" as tabs;fn view(ctx){tabs::Tabs(#{label:"Sections",value:"a",tabs:[#{value:"a",label:"First",content:text("A")},#{value:"b",label:"Second",content:text("B")}]})}"#;
    let (w, view) = mount(cx, script, overrides, Noop);
    let mut visual = VisualTestContext::from_window(*w, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let tab = tree
        .find_by_role_and_name("tab", "First")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    println!(
        "THEMED_TABS_TEXT line_height=36 border_total=2 actual_tab_height={}",
        tab.height
    );
    assert!(
        tab.height >= 38.,
        "fixed tab slot does not contain its resolved theme line box and borders"
    );
}
