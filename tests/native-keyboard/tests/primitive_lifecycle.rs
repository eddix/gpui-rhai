use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, Window, WindowHandle};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let element = if self.view.state() == ScriptViewState::Active {
            self.view.element().unwrap()
        } else {
            gpui::div().into_any_element()
        };
        self.host.container(element)
    }
}

#[derive(Default)]
struct NativeState {
    active: Cell<bool>,
    fail_resume: Cell<bool>,
    fail_suspend: Cell<bool>,
    resumes: Cell<usize>,
    suspends: Cell<usize>,
}

struct HookPrimitive(Rc<NativeState>);

impl PrimitiveHandler for HookPrimitive {
    fn mount(&mut self, _: &PrimitiveInstance) -> Result<(), String> {
        self.0.active.set(true);
        Ok(())
    }

    fn suspend(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        self.0.suspends.set(self.0.suspends.get() + 1);
        assert!(
            !self.0.fail_suspend.replace(false),
            "injected native suspend failure"
        );
        self.0.active.set(false);
    }

    fn resume(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        self.0.resumes.set(self.0.resumes.get() + 1);
        assert!(
            !self.0.fail_resume.replace(false),
            "injected native resume failure"
        );
        self.0.active.set(true);
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

    fn unmount(&mut self, _: &PrimitiveInstanceId) {
        self.0.active.set(false);
    }
}

struct HookExtension(Vec<Rc<NativeState>>);

impl ScriptViewExtension for HookExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        for (index, state) in self.0.iter().enumerate() {
            engine
                .register_primitive(
                    PrimitiveDescriptor {
                        id: PrimitiveId::parse(format!("test.lifecycle{index}")).unwrap(),
                        export: format!("Lifecycle{index}"),
                        props: BTreeMap::new(),
                        events: BTreeMap::new(),
                        state: ComponentStateSchema::default(),
                        lifecycle: true,
                        effect: None,
                    },
                    HookPrimitive(state.clone()),
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn mount(
    cx: &mut TestAppContext,
    name: &str,
    states: Vec<Rc<NativeState>>,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let source = r#"fn view(ctx){row([test::Lifecycle0(#{key:"a"}).with_key("a"),test::Lifecycle1(#{key:"b"}).with_key("b"),test::Lifecycle2(#{key:"c"}).with_key("c")])}"#;
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .extension(HookExtension(states))
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

#[gpui::test]
fn native_resume_failure_remains_suspended_and_retryable(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    states[1].fail_resume.set(true);
    let (window, view) = mount(cx, "native-resume-failure", states.clone());
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    let result = visual.update(|_, cx| view.resume(cx));
    assert!(result.is_err());
    assert_eq!(view.state(), ScriptViewState::Suspended);
    assert!(states.iter().all(|state| !state.active.get()));
    assert!(matches!(visual.update(|_, cx| view.resume(cx)), Ok(true)));
    assert!(states.iter().all(|state| state.active.get()));
}

#[gpui::test]
fn native_suspend_failure_compensates_every_instance_and_retries(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    states[1].fail_suspend.set(true);
    let (window, view) = mount(cx, "native-suspend-failure", states.clone());
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let result = visual.update(|window, cx| view.suspend(window, cx));
    assert!(result.is_err());
    assert_eq!(view.state(), ScriptViewState::Active);
    assert!(states.iter().all(|state| state.active.get()));
    assert!(matches!(
        visual.update(|window, cx| view.suspend(window, cx)),
        Ok(true)
    ));
    assert!(states.iter().all(|state| !state.active.get()));
}

#[gpui::test]
fn failed_compensation_faults_and_quarantines_the_view(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    let (window, view) = mount(cx, "native-compensation-failure", states.clone());
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    states[1].fail_resume.set(true);
    states[0].fail_suspend.set(true);
    assert!(visual.update(|_, cx| view.resume(cx)).is_err());
    assert_eq!(view.state(), ScriptViewState::Disposed);
    assert!(states.iter().all(|state| !state.active.get()));
    assert!(matches!(
        visual.update(|_, cx| view.resume(cx)),
        Err(ScriptViewError::DisposedView(_))
    ));
}
