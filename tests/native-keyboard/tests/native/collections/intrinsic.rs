//! Native async intrinsic changes remeasure a leased row, not its Rhai model.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, TestAppContext, VisualTestContext, Window, px, size,
};
use gpui_rhai::{
    ComponentStateSchema, MotionPreference, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveTheme,
    RuntimeEngine, ScriptViewExtension,
};

use crate::support::{Mount, Step, bounds, dispatch_script_button, last_error, settle, snapshot};

#[derive(Default)]
struct NativeSize {
    height: Cell<f32>,
    layouts: Cell<usize>,
    context: RefCell<Option<PrimitiveContext>>,
}

struct NativeHeight(Rc<NativeSize>);

impl IntoElement for NativeHeight {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NativeHeight {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.0.layouts.set(self.0.layouts.get() + 1);
        let style = gpui::Style {
            size: size(px(100.0).into(), px(self.0.height.get()).into()),
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

struct Handler {
    live: BTreeMap<PrimitiveInstanceId, Rc<NativeSize>>,
    mounted: Rc<RefCell<Vec<Rc<NativeSize>>>>,
}

impl PrimitiveHandler for Handler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        _: &PrimitiveTheme,
        _: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let state = self
            .live
            .entry(instance.id.clone().unwrap())
            .or_insert_with(|| {
                let state = Rc::new(NativeSize::default());
                self.mounted.borrow_mut().push(Rc::clone(&state));
                state
            });
        *state.context.borrow_mut() = Some(context.clone());
        // Explicitly exercise native render calling the hook while both the
        // primitive registry and the enclosing list entity are still borrowed.
        if state.layouts.get() == 0 {
            context.invalidate_intrinsic_size(cx);
        }
        Ok(NativeHeight(Rc::clone(state)).into_any_element())
    }
    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.live.remove(instance);
    }
}

struct Extension(Rc<RefCell<Vec<Rc<NativeSize>>>>);

impl ScriptViewExtension for Extension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("test.native_height").unwrap(),
                    export: "NativeHeight".to_owned(),
                    props: BTreeMap::new(),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: true,
                    effect: None,
                },
                Handler {
                    live: BTreeMap::new(),
                    mounted: Rc::clone(&self.0),
                },
            )
            .map_err(|error| error.to_string())
    }
}

const SOURCE: &str = r#"
fn state_schema(){#{fields:#{epoch:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn replace(ctx,value){ctx.set_state("epoch",ctx.get_state("epoch")+1);}
fn item(ctx,payload){
    if payload.item.key == "native" {
        column([test::NativeHeight(#{key:`native-${payload.item.epoch}`})])
            .accessibility_role("listitem").accessibility_label("Native row")
    } else {
        text("After native").with_style(style().height(px(30)))
            .accessibility_role("listitem").accessibility_label("After native")
    }
}
fn view(ctx){column([
    text("Replace").accessibility_role("button").accessibility_label("Replace").on_click(Fn("replace")),
    virtual_collection(#{key:"intrinsic",data:[#{key:"native",epoch:ctx.get_state("epoch")},#{key:"after"}],
        height:240,estimated_height:30,overdraw_pixels:0},Fn("item"))
])}
"#;

#[gpui::test]
fn native_async_size_commit_remeasures_only_its_live_row_without_rhai(cx: &mut TestAppContext) {
    let instances = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&instances);
    let (mut visual, view) = Mount::new(SOURCE)
        .with(move |view| view.extension(Extension(captured)))
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    assert!(last_error(&mut visual, &view).is_none());
    let before = bounds(&mut visual, &view, "listitem", "After native");
    let state = instances.borrow().last().unwrap().clone();
    visual.update(|_, cx| {
        view.take_performance_snapshot(cx).unwrap();
    });
    visual.update(|_, cx| {
        // A native asynchronous foreground commit, with no ScriptView notify
        // or declarative state write to incidentally invalidate the row.
        cx.spawn(async move |cx| {
            cx.background_spawn(async {}).await;
            cx.update(|cx| {
                state.height.set(120.0);
                let context = state.context.borrow().as_ref().unwrap().clone();
                context.invalidate_intrinsic_size(cx);
                context.invalidate_intrinsic_size(cx);
            });
        })
        .detach();
    });
    settle(&mut visual, Step::Tick, 8);
    let row = bounds(&mut visual, &view, "listitem", "Native row");
    let after = bounds(&mut visual, &view, "listitem", "After native");
    assert!(
        (row.height - 120.0).abs() < 1.0,
        "native measurement: {row:?}"
    );
    assert!(
        (after.y - before.y - 120.0).abs() < 1.0,
        "following row must move by committed native growth: {before:?} -> {after:?}"
    );
    let performance = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert!(
        performance.timings.iter().all(|timing| !matches!(
            timing.operation,
            gpui_rhai::ExecutionOperation::VirtualCollection(_)
        )),
        "native intrinsic invalidation must not rerun virtual-row Rhai"
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn obsolete_native_context_cannot_invalidate_same_row_key_replacement(cx: &mut TestAppContext) {
    let instances = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&instances);
    let (mut visual, view) = Mount::new(SOURCE)
        .with(move |view| view.extension(Extension(captured)))
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    let old = instances.borrow().last().unwrap().clone();
    let obsolete = old.context.borrow().as_ref().unwrap().clone();
    dispatch_script_button(&mut visual, &view, "Replace");
    settle(&mut visual, Step::Tick, 8);
    let current = instances.borrow().last().unwrap().clone();
    assert!(
        !Rc::ptr_eq(&old, &current),
        "same collection row must host a fresh native incarnation"
    );
    let layouts = current.layouts.get();
    visual.update(|_, cx| {
        cx.defer(move |cx| {
            old.height.set(180.0);
            obsolete.invalidate_intrinsic_size(cx);
        });
    });
    visual.run_until_parked();
    assert_eq!(
        current.layouts.get(),
        layouts,
        "obsolete lease must not schedule a replacement row measurement or paint"
    );
    assert!(last_error(&mut visual, &view).is_none());
}

const RETAINED_SOURCE: &str = r#"
fn state_schema(){#{fields:#{show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn toggle(ctx,value){ctx.set_state("show",!ctx.get_state("show"));}
fn item(ctx,payload){
    if payload.item.key == "native" {
        column([test::NativeHeight(#{key:"native"})])
            .accessibility_role("listitem").accessibility_label("Native row")
    } else {
        text("After native").with_style(style().height(px(30)))
            .accessibility_role("listitem").accessibility_label("After native")
    }
}
fn view(ctx){column([
    text("Toggle").accessibility_role("button").accessibility_label("Toggle").on_click(Fn("toggle")),
    virtual_collection(#{key:"intrinsic",data:[#{key:"native"},#{key:"after"}],
        height:240,estimated_height:30,overdraw_pixels:0},Fn("item"))
        .with_key("page").retained_page(ctx.get_state("show")),
    text("Other page").with_key("other").retained_page(!ctx.get_state("show"))
]).with_style(style().width(px(400)).height(px(320)))}
"#;

fn commit_native_height(visual: &mut VisualTestContext, state: Rc<NativeSize>, height: f32) {
    visual.update(|_, cx| {
        cx.spawn(async move |cx| {
            cx.background_spawn(async {}).await;
            cx.update(|cx| {
                state.height.set(height);
                state
                    .context
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .invalidate_intrinsic_size(cx);
            });
        })
        .detach();
    });
}

fn assert_hidden_quiescence(visual: &mut VisualTestContext, state: &NativeSize, layouts: usize) {
    for _ in 0..16 {
        visual
            .executor()
            .advance_clock(std::time::Duration::from_millis(32));
        visual.run_until_parked();
        let callbacks = visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
        assert_eq!(
            callbacks, 0,
            "hidden dirty row must not sustain a frame callback loop"
        );
        assert_eq!(
            state.layouts.get(),
            layouts,
            "hidden native measurement must remain quiescent"
        );
    }
}

#[gpui::test]
fn hidden_retained_native_growth_coalesces_until_presented(cx: &mut TestAppContext) {
    let instances = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&instances);
    let (mut visual, view) = Mount::new(RETAINED_SOURCE)
        .motion(MotionPreference::None)
        .with(move |view| view.extension(Extension(captured)))
        .mount(cx);
    settle(&mut visual, Step::Tick, 8);
    let before = bounds(&mut visual, &view, "listitem", "After native");
    let state = instances.borrow().last().unwrap().clone();
    assert_eq!(state.height.get(), 0.0);
    let mounted = instances.borrow().len();
    dispatch_script_button(&mut visual, &view, "Toggle");
    settle(&mut visual, Step::Tick, 8);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("listitem", "Native row")
            .next()
            .is_none(),
        "retained row must be hidden, not merely covered"
    );
    let layouts = state.layouts.get();
    visual.update(|_, cx| {
        view.take_performance_snapshot(cx).unwrap();
    });
    for height in [120.0, 180.0] {
        commit_native_height(&mut visual, Rc::clone(&state), height);
        settle(&mut visual, Step::Tick, 8);
        assert_eq!(
            state.layouts.get(),
            layouts,
            "a hidden native commit may dirty its row but must not remeasure it"
        );
    }
    assert_hidden_quiescence(&mut visual, &state, layouts);
    let performance = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    assert!(
        !performance.pending_virtual_requests,
        "hidden dirty row must not schedule realization work"
    );
    assert!(
        performance.timings.iter().all(|timing| !matches!(
            timing.operation,
            gpui_rhai::ExecutionOperation::VirtualCollection(_)
        )),
        "hidden size commits must execute no virtual-row Rhai"
    );
    dispatch_script_button(&mut visual, &view, "Toggle");
    settle(&mut visual, Step::Tick, 8);
    assert_eq!(
        instances.borrow().len(),
        mounted,
        "show must preserve the same native instance"
    );
    assert!(Rc::ptr_eq(&state, instances.borrow().last().unwrap()));
    let row = bounds(&mut visual, &view, "listitem", "Native row");
    let after = bounds(&mut visual, &view, "listitem", "After native");
    assert!(
        (row.height - 180.0).abs() < 1.0,
        "show must measure the newest hidden native commit: {row:?}"
    );
    assert!(
        (after.y - before.y - 180.0).abs() < 1.0,
        "show must move the successor by the full retained growth: {before:?} -> {after:?}"
    );
    assert!(last_error(&mut visual, &view).is_none());
}
