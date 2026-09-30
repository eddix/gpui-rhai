use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
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

fn mount(cx: &mut TestAppContext) -> (gpui::WindowHandle<Host>, ScriptViewHandle) {
    let repo = std::path::Path::new(env!("GPUI_RHAI_AUDIT_ROOT"));
    let entry = ModuleId::parse("main").unwrap();
    let script = r#"
import "components/resizable" as resizable;
fn fail(ctx,payload){throw "audit-tooling-callback-failed";}
fn view(ctx){column([
    text("Fail script").accessibility_role("button").accessibility_label("Fail script").on_click(Fn("fail")),
    resizable::Resizable(#{key:"rect",label:"Audit resize",rect:#{x:20.0,y:20.0,width:200.0,height:100.0},
        handles:["e"],contain:false,content:text("content"),on_resize:Fn("fail")})
        .with_style(style().width(px(500)).height(px(300)))
])}
"#;
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (
                ModuleId::parse("components/resizable").unwrap(),
                std::fs::read_to_string(repo.join("registry/components/resizable.rhai")).unwrap(),
            ),
        ])),
        std::fs::read_to_string(repo.join("registry/themes/default_dark.rhai")).unwrap(),
    )
    .prepare()
    .unwrap();
    let seen = Rc::new(RefCell::new(None));
    let out = seen.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("tooling-round2", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("tooling-round2"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *out.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = seen.borrow().as_ref().unwrap().clone();
    (window, view)
}

#[gpui::test]
fn declarative_automation_failure_control(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let result = visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::RoleName {
                    role: "button".to_owned(),
                    name: "Fail script".to_owned(),
                },
                event: "click".to_owned(),
                payload: None,
            },
            window,
            cx,
        )
    });
    println!("DECLARATIVE immediate={result:?}");
    assert!(result.is_err());
}

#[gpui::test]
fn native_automation_must_report_its_callback_failure(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let result = visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::RoleName {
                    role: "separator".to_owned(),
                    name: "Audit resize: e resize handle".to_owned(),
                },
                event: "key:right".to_owned(),
                payload: None,
            },
            window,
            cx,
        )
    });
    visual.run_until_parked();
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    println!("NATIVE immediate={result:?} settled_error={error:?}");
    assert!(
        error
            .as_ref()
            .is_some_and(|e| e.contains("audit-tooling-callback-failed")),
        "native callback actually runs"
    );
    assert!(
        result.is_err(),
        "public automation reports callback failure, not successful invocation"
    );
}
