//! External-consumer tests of the scoped factory and native ListState hooks.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use gpui::prelude::FluentBuilder as _;

use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, InteractiveElement, IntoElement, IsZero as _,
    ListAlignment, ListState, Modifiers, MouseButton, ParentElement, Pixels, Point, Render,
    ScrollDelta, ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Styled,
    TestAppContext, VisualTestContext, Window, div, list, point, px,
};
use gpui_rhai::{
    ColorValue, ComponentStateSchema, PrimitiveContext, PrimitiveDescriptor, PrimitiveHandler,
    PrimitiveId, PrimitiveInstance, PrimitiveTheme, Rgba8, RuntimeEngine, ScriptViewConfig,
    ScriptViewExtension, ScriptViewHandle, ScriptViewHost, ScrollbarSource, ScrollbarSourceHandle,
    ScrollbarSpec, Style,
};

use crate::support::{ActiveHost, Mount, Step, filled, settle};

const THUMB: u32 = 0xea00_eaff;

struct Probe {
    scroll: ListState,
    height: Cell<f32>,
    starts: Cell<usize>,
    ends: Cell<usize>,
    viewport_override: Cell<Option<Bounds<Pixels>>>,
    source_slot: RefCell<Weak<RefCell<ScrollbarSourceHandle>>>,
    bubbled_wheels: Cell<usize>,
    route_body: Cell<bool>,
}

#[derive(Clone)]
struct Source(Rc<Probe>);

impl ScrollbarSource for Source {
    fn viewport(&self) -> Bounds<Pixels> {
        self.0
            .viewport_override
            .get()
            .unwrap_or_else(|| self.0.scroll.viewport_bounds())
    }
    fn offset(&self) -> Point<Pixels> {
        self.0.scroll.scroll_px_offset_for_scrollbar()
    }
    fn max_offset(&self) -> Point<Pixels> {
        self.0.scroll.max_offset_for_scrollbar()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.0.scroll.set_offset_from_scrollbar(offset);
    }
    fn drag_started(&self) {
        self.0.starts.set(self.0.starts.get() + 1);
        self.0.scroll.scrollbar_drag_started();
    }
    fn drag_ended(&self) {
        self.0.ends.set(self.0.ends.get() + 1);
        self.0.scroll.scrollbar_drag_ended();
    }
}

struct Handler(Rc<Probe>, Rc<RefCell<ScrollbarSourceHandle>>);

impl PrimitiveHandler for Handler {
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        context: &PrimitiveContext,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let state = Rc::clone(&self.0);
        let thumb = Style::new().background(ColorValue::Literal(Rgba8::from_rgba_hex(THUMB)));
        let bars = context
            .scrollbars(theme)
            .with_part_styles(&Style::default(), &thumb, &thumb);
        let spec = ScrollbarSpec::new("hidden", "always")?;
        let scrollbar = if state.route_body.get() {
            bars.element_with_wheel_routing("scrollbar", self.1.borrow().clone(), spec)
        } else {
            bars.element("scrollbar", self.1.borrow().clone(), spec)
        };
        let observer = Rc::clone(&state);
        Ok(div()
            .relative()
            .w(px(300.0))
            .h(px(200.0))
            .overflow_hidden()
            .on_scroll_wheel(move |_, _, _| {
                observer
                    .bubbled_wheels
                    .set(observer.bubbled_wheels.get() + 1)
            })
            .child(
                list(state.scroll.clone(), move |_, _, _| {
                    div().h(px(state.height.get())).into_any_element()
                })
                .size_full(),
            )
            .child(scrollbar)
            .into_any_element())
    }
}

struct Extension(Rc<Probe>);

impl ScriptViewExtension for Extension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        let slot = Rc::new(RefCell::new(ScrollbarSourceHandle::new(Source(Rc::clone(
            &self.0,
        )))));
        *self.0.source_slot.borrow_mut() = Rc::downgrade(&slot);
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("test.scroll_probe").unwrap(),
                    export: "ScrollProbe".to_owned(),
                    props: BTreeMap::new(),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: true,
                    effect: None,
                },
                Handler(Rc::clone(&self.0), slot),
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn mount(cx: &mut TestAppContext, id: &str) -> (VisualTestContext, ScriptViewHandle, Rc<Probe>) {
    let probe = Rc::new(Probe {
        scroll: ListState::new(50, ListAlignment::Top, px(80.0)).with_uniform_item_height(px(40.0)),
        height: Cell::new(40.0),
        starts: Cell::new(0),
        ends: Cell::new(0),
        viewport_override: Cell::new(None),
        source_slot: RefCell::new(Weak::new()),
        bubbled_wheels: Cell::new(0),
        route_body: Cell::new(false),
    });
    let source = Rc::clone(&probe);
    let prepared = Mount::new(
        r#"
fn state_schema(){#{fields:#{tick:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn refresh(ctx,value){ctx.set_state("tick",ctx.get_state("tick")+1);}
fn view(ctx){column([text(`Refresh ${ctx.get_state("tick")}`).with_key("refresh")
    .accessibility_role("button").accessibility_label("Refresh").on_click(Fn("refresh")),
    test::ScrollProbe(#{key:"probe"}).with_key("probe")])}
"#,
    )
    .with(move |view| view.extension(Extension(source)))
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&captured);
    let id = id.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&id, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&id), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        ActiveHost { host, view }
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 4);
    (visual, view, probe)
}

fn begin_drag(visual: &mut VisualTestContext, probe: &Probe) -> Point<Pixels> {
    let thumbs = filled(visual, THUMB);
    assert_eq!(thumbs.len(), 1, "factory renders one themed vertical thumb");
    let thumb = thumbs[0].visible();
    let position = point(
        px(thumb.x + thumb.width / 2.0),
        px(thumb.y + thumb.height / 2.0),
    );
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    assert!(
        probe.scroll.is_scrollbar_dragging(),
        "native height freezes on pointer down"
    );
    position
}

#[gpui::test]
fn scrollbar_freezes_native_measurements_and_releases_on_mouse_up_and_escape(
    cx: &mut TestAppContext,
) {
    let (mut visual, _, probe) = mount(cx, "scrollbar-hooks");
    let position = begin_drag(&mut visual, &probe);
    let frozen = probe.scroll.max_offset_for_scrollbar();
    probe.scroll.splice(50..50, 10);
    probe.height.set(120.0);
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        probe.scroll.max_offset_for_scrollbar(),
        frozen,
        "measurements and appended content cannot move the thumb's extent mid-drag"
    );
    visual.simulate_mouse_move(
        position + point(px(0.0), px(70.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    visual.simulate_mouse_up(
        position + point(px(0.0), px(70.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    assert!(!probe.scroll.is_scrollbar_dragging());
    assert_eq!((probe.starts.get(), probe.ends.get()), (1, 1));
    settle(&mut visual, Step::Tick, 4);
    begin_drag(&mut visual, &probe);
    visual.simulate_keystrokes("escape");
    assert!(!probe.scroll.is_scrollbar_dragging());
    assert_eq!(
        (probe.starts.get(), probe.ends.get()),
        (2, 2),
        "Escape calls exactly one native end hook"
    );
}

#[gpui::test]
fn primitive_scrollbar_drag_cancels_on_suspend_dispose_and_deactivation(cx: &mut TestAppContext) {
    for operation in ["suspend", "dispose", "deactivate"] {
        let (mut visual, view, probe) = mount(cx, &format!("scrollbar-{operation}"));
        begin_drag(&mut visual, &probe);
        match operation {
            "suspend" => {
                visual
                    .update(|window, cx| view.suspend(window, cx))
                    .unwrap();
            }
            "dispose" => {
                visual.update(|_, cx| view.dispose(cx)).unwrap();
            }
            _ => visual.deactivate_window(),
        }
        visual.run_until_parked();
        assert!(
            !probe.scroll.is_scrollbar_dragging(),
            "{operation} must end native measurement freeze"
        );
        assert_eq!(
            (probe.starts.get(), probe.ends.get()),
            (1, 1),
            "{operation} invokes one end hook"
        );
    }
}

#[gpui::test]
fn zero_and_short_native_viewports_do_not_create_oversized_thumbs(cx: &mut TestAppContext) {
    let (mut visual, _, probe) = mount(cx, "scrollbar-small");
    let mut viewport = probe.scroll.viewport_bounds();
    viewport.size.height = px(5.0);
    probe.viewport_override.set(Some(viewport));
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 3);
    let thumb = filled(&mut visual, THUMB);
    assert_eq!(thumb.len(), 1);
    assert!(thumb[0].visible().height <= 5.0);
    viewport.size.height = px(0.0);
    probe.viewport_override.set(Some(viewport));
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 3);
    assert!(
        filled(&mut visual, THUMB).is_empty(),
        "a zero source viewport never falls back to an unrelated container's bounds"
    );
}

#[gpui::test]
fn replacing_a_retained_source_lease_cancels_the_old_thumb_drag(cx: &mut TestAppContext) {
    let (mut visual, view, probe) = mount(cx, "scroll-source-lease");
    begin_drag(&mut visual, &probe);
    let slot = probe.source_slot.borrow().upgrade().unwrap();
    *slot.borrow_mut() = ScrollbarSourceHandle::new(Source(Rc::clone(&probe)));
    crate::support::dispatch_script_button(&mut visual, &view, "Refresh");
    settle(&mut visual, Step::Tick, 5);
    assert!(!probe.scroll.is_scrollbar_dragging());
    assert_eq!(
        (probe.starts.get(), probe.ends.get()),
        (1, 1),
        "a stable element id cannot keep a gesture captured by a retired source lease"
    );
}

#[gpui::test]
fn entirely_unconsumed_track_wheel_events_are_not_swallowed(cx: &mut TestAppContext) {
    let (mut visual, _, probe) = mount(cx, "unconsumed-scroll-wheel");
    let bounds = probe.scroll.viewport_bounds();
    let event = |delta| ScrollWheelEvent {
        position: point(bounds.right() - px(6.0), bounds.top() + px(100.0)),
        delta: ScrollDelta::Pixels(delta),
        ..Default::default()
    };
    visual.simulate_event(event(point(px(-30.0), px(0.0))));
    assert_eq!(
        probe.bubbled_wheels.get(),
        1,
        "an unsupported horizontal wheel reaches the parent handler"
    );
    probe.scroll.scroll_to_end();
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 4);
    visual.simulate_event(event(point(px(0.0), px(-30.0))));
    assert_eq!(
        probe.bubbled_wheels.get(),
        2,
        "an exhausted vertical source leaves its entire delta unconsumed"
    );
}

#[gpui::test]
fn opt_in_body_wheel_consumes_once_before_native_list_and_leaves_unsupported_input(
    cx: &mut TestAppContext,
) {
    for route_body in [false, true] {
        let (mut visual, view, probe) = mount(cx, "body-wheel-opt-in");
        probe.route_body.set(route_body);
        crate::support::dispatch_script_button(&mut visual, &view, "Refresh");
        settle(&mut visual, Step::Tick, 4);
        let viewport = probe.scroll.viewport_bounds();
        let event = |delta| ScrollWheelEvent {
            position: viewport.center(),
            delta: ScrollDelta::Pixels(delta),
            ..Default::default()
        };
        let before = probe.scroll.scroll_px_offset_for_scrollbar();
        visual.simulate_event(event(point(px(0.0), px(-24.0))));
        assert_eq!(
            probe.scroll.scroll_px_offset_for_scrollbar().y,
            before.y - px(24.0),
            "body policy must not apply both shared and automatic native movement"
        );
        assert_eq!(
            probe.bubbled_wheels.get(),
            usize::from(!route_body),
            "ordinary GPUI List bubbles its already-applied delta; opt-in shared body consumes it once"
        );
        settle(&mut visual, Step::Tick, 3);
        let bubbled = probe.bubbled_wheels.get();
        visual.simulate_event(event(point(px(-24.0), px(0.0))));
        assert_eq!(
            probe.bubbled_wheels.get(),
            bubbled + 1,
            "an entirely unsupported body axis propagates unchanged"
        );
    }
}

struct NestedBodyHost {
    host: ScriptViewHost,
    outer: ScrollHandle,
    inner: ScrollHandle,
    outer_source: ScrollbarSourceHandle,
    inner_source: ScrollbarSourceHandle,
    occluded: Rc<Cell<bool>>,
}

impl NestedBodyHost {
    fn content(&self) -> AnyElement {
        let theme = PrimitiveTheme::default();
        let bars = self.host.scrollbars("nested-body", &theme).unwrap();
        let spec = ScrollbarSpec::new("hidden", "hidden").unwrap();
        let inner = bars
            .clone()
            .with_scroll_axis(Some(gpui::Axis::Vertical))
            .with_scroll_parent(self.outer_source.clone(), Some(gpui::Axis::Vertical))
            .element_with_wheel_routing("inner", self.inner_source.clone(), spec);
        div()
            .relative()
            .w(px(300.0))
            .h(px(300.0))
            .overflow_hidden()
            .child(
                div()
                    .id("body-outer")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.outer)
                    .child(
                        div()
                            .relative()
                            .w(px(240.0))
                            .h(px(160.0))
                            .flex_none()
                            .child(
                                div()
                                    .id("body-inner")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.inner)
                                    .child(div().w(px(200.0)).h(px(600.0)).flex_none()),
                            )
                            .child(inner),
                    )
                    .child(div().w(px(200.0)).h(px(600.0)).flex_none()),
            )
            // Deliberately later-painted than the nested body router.
            .child(
                bars.with_scroll_axis(Some(gpui::Axis::Vertical))
                    .element_with_wheel_routing("outer", self.outer_source.clone(), spec),
            )
            .when(self.occluded.get(), |root| {
                root.child(div().absolute().inset_0().occlude())
            })
            .into_any_element()
    }
}

impl Render for NestedBodyHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.content())
    }
}

#[gpui::test]
fn nested_body_region_wins_over_parent_overlay_and_chains_only_boundary_residual(
    cx: &mut TestAppContext,
) {
    let outer = ScrollHandle::new();
    let inner = ScrollHandle::new();
    let outer_state = outer.clone();
    let inner_state = inner.clone();
    let occluded = Rc::new(Cell::new(false));
    let occlusion = Rc::clone(&occluded);
    let window = cx.add_window(move |_, cx| NestedBodyHost {
        host: ScriptViewHost::new("nested-body", cx).unwrap(),
        outer_source: ScrollbarSourceHandle::new(outer_state.clone()),
        inner_source: ScrollbarSourceHandle::new(inner_state.clone()),
        outer: outer_state,
        inner: inner_state,
        occluded: occlusion,
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Tick, 4);
    assert!(inner.max_offset().y > px(24.0));
    assert!(outer.max_offset().y > px(24.0));
    let wheel = |position| ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(-24.0))),
        ..Default::default()
    };
    visual.simulate_event(wheel(inner.bounds().center()));
    assert_eq!(
        inner.offset().y,
        px(-24.0),
        "nearest body source owns its full available delta"
    );
    assert_eq!(
        outer.offset().y,
        px(0.0),
        "later-painted parent overlay must not steal or duplicate body input"
    );
    inner.set_offset(point(px(0.0), -inner.max_offset().y + px(8.0)));
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 3);
    visual.simulate_event(wheel(inner.bounds().center()));
    assert_eq!(inner.offset().y, -inner.max_offset().y);
    assert_eq!(
        outer.offset().y,
        px(-16.0),
        "only the remaining 16px reaches the native ancestor"
    );
    occluded.set(true);
    visual.update(|window, _| window.refresh());
    settle(&mut visual, Step::Tick, 3);
    let before = (inner.offset(), outer.offset());
    visual.simulate_event(wheel(inner.bounds().center()));
    assert_eq!(
        (inner.offset(), outer.offset()),
        before,
        "an occluding overlay blocks both shared body routing and native scrolling"
    );
}

struct CachedBody {
    content: NestedBodyHost,
    renders: Rc<Cell<usize>>,
}

impl Render for CachedBody {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        self.content.content()
    }
}

struct UnrelatedSibling(usize);
impl Render for UnrelatedSibling {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(format!("Unrelated {}", self.0))
    }
}

struct CachedBodyHost {
    host: ScriptViewHost,
    body: gpui::Entity<CachedBody>,
    sibling: gpui::Entity<UnrelatedSibling>,
}

impl Render for CachedBodyHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let style = gpui::StyleRefinement {
            size: gpui::SizeRefinement {
                width: Some(px(300.0).into()),
                height: Some(px(300.0).into()),
            },
            ..Default::default()
        };
        self.host.container(
            div()
                .flex()
                .flex_col()
                .child(self.body.clone().cached(style))
                .child(self.sibling.clone()),
        )
    }
}

#[gpui::test]
fn cached_native_body_replays_current_event_regions_after_unrelated_sibling_updates(
    cx: &mut TestAppContext,
) {
    let outer = ScrollHandle::new();
    let inner = ScrollHandle::new();
    let outer_state = outer.clone();
    let inner_state = inner.clone();
    let renders = Rc::new(Cell::new(0));
    let render_count = Rc::clone(&renders);
    let sibling = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&sibling);
    let window = cx.add_window(move |_, cx| {
        let host = ScriptViewHost::new("cached-body", cx).unwrap();
        let body = cx.new(|_| CachedBody {
            content: NestedBodyHost {
                host: host.clone(),
                outer_source: ScrollbarSourceHandle::new(outer_state.clone()),
                inner_source: ScrollbarSourceHandle::new(inner_state.clone()),
                outer: outer_state,
                inner: inner_state,
                occluded: Rc::new(Cell::new(false)),
            },
            renders: render_count,
        });
        let sibling = cx.new(|_| UnrelatedSibling(0));
        *capture.borrow_mut() = Some(sibling.clone());
        CachedBodyHost {
            host,
            body,
            sibling,
        }
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 3);
    let sibling = sibling.borrow().as_ref().unwrap().clone();
    for boundary in [false, true] {
        if boundary {
            inner.set_offset(point(px(0.0), -inner.max_offset().y + px(8.0)));
            visual.update(|window, _| window.refresh());
            settle(&mut visual, Step::Frame, 3);
        }
        let before = renders.get();
        visual.update(|_, cx| {
            sibling.update(cx, |sibling, cx| {
                sibling.0 += 1;
                cx.notify();
            })
        });
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(
            renders.get(),
            before,
            "precondition: unrelated sibling redraw must reuse cached native prepaint/paint"
        );
        visual.simulate_event(ScrollWheelEvent {
            position: inner.bounds().center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-24.0))),
            ..Default::default()
        });
        assert_eq!(
            inner.offset().y,
            if boundary {
                -inner.max_offset().y
            } else {
                px(-24.0)
            }
        );
        assert_eq!(
            outer.offset().y,
            if boundary { px(-16.0) } else { px(0.0) },
            "cached listeners must register the current region and consume only native residual"
        );
        settle(&mut visual, Step::Frame, 3);
    }
}

struct OverlappingBodyHosts {
    surfaces: [NestedBodyHost; 2],
    stop_capture: Rc<Cell<bool>>,
}

impl Render for OverlappingBodyHosts {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let [back, front] = &self.surfaces;
        let stop = Rc::clone(&self.stop_capture);
        div()
            .relative()
            .w(px(300.0))
            .h(px(300.0))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(back.host.container(back.content())),
            )
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    move |_, (), window, _| {
                        let stop = Rc::clone(&stop);
                        window.on_mouse_event(move |_: &ScrollWheelEvent, phase, _, cx| {
                            if phase == gpui::DispatchPhase::Capture && stop.get() {
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(front.host.container(front.content())),
            )
    }
}

#[gpui::test]
fn overlapping_hosts_collect_every_capture_before_frontmost_body_consumes(cx: &mut TestAppContext) {
    let states = std::array::from_fn::<_, 2, _>(|_| (ScrollHandle::new(), ScrollHandle::new()));
    let sources = states.clone();
    let stop_capture = Rc::new(Cell::new(true));
    let stop = Rc::clone(&stop_capture);
    let window = cx.add_window(move |_, cx| OverlappingBodyHosts {
        surfaces: [0, 1].map(|index| {
            let (outer, inner) = sources[index].clone();
            NestedBodyHost {
                host: ScriptViewHost::new(format!("overlapping-{index}"), cx).unwrap(),
                outer_source: ScrollbarSourceHandle::new(outer.clone()),
                inner_source: ScrollbarSourceHandle::new(inner.clone()),
                outer,
                inner,
                occluded: Rc::new(Cell::new(false)),
            }
        }),
        stop_capture: stop,
    });
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 3);
    let event = || ScrollWheelEvent {
        position: states[1].1.bounds().center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-24.0))),
        ..Default::default()
    };
    visual.simulate_event(event());
    assert!(
        states
            .iter()
            .all(|(outer, inner)| outer.offset().y.is_zero() && inner.offset().y.is_zero()),
        "capture stopped between Hosts must consume no native movement"
    );
    stop_capture.set(false);
    visual.simulate_event(event());
    assert_eq!(
        states[0].1.offset().y,
        px(0.0),
        "back Host must not consume before front Host registers"
    );
    assert_eq!(
        states[1].1.offset().y,
        px(-24.0),
        "only the later-painted front Host consumes"
    );
    assert!(
        states.iter().all(|(outer, _)| outer.offset().y.is_zero()),
        "neither native ancestor duplicates consumed movement"
    );
}

struct NativeHost {
    host: ScriptViewHost,
    source: ScrollHandle,
    direction: gpui_rhai::TextDirection,
}

impl Render for NativeHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = PrimitiveTheme::from_colors(&PrimitiveTheme::default(), self.direction);
        let thumb = Style::new().background(ColorValue::Literal(Rgba8::from_rgba_hex(THUMB)));
        let scrollbar = self
            .host
            .scrollbars("native", &theme)
            .unwrap()
            .with_part_styles(&Style::default(), &thumb, &thumb)
            .element(
                "bars",
                self.source.clone(),
                ScrollbarSpec::new("always", "always").unwrap(),
            );
        self.host.container(
            div()
                .relative()
                .w(px(300.0))
                .h(px(200.0))
                .overflow_hidden()
                .child(
                    div()
                        .id("native-viewport")
                        .size_full()
                        .overflow_scroll()
                        .track_scroll(&self.source)
                        .child(div().w(px(900.0)).h(px(800.0))),
                )
                .child(scrollbar),
        )
    }
}

#[gpui::test]
fn host_scrollhandle_scrollbars_share_rtl_axes_wheel_and_deactivation_policy(
    cx: &mut TestAppContext,
) {
    for direction in [
        gpui_rhai::TextDirection::LeftToRight,
        gpui_rhai::TextDirection::RightToLeft,
    ] {
        let source = ScrollHandle::new();
        let state = source.clone();
        let window = cx.add_window(move |_, cx| NativeHost {
            host: ScriptViewHost::new("native-bars", cx).unwrap(),
            source: state,
            direction,
        });
        cx.refresh().unwrap();
        cx.run_until_parked();
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Tick, 4);
        let thumbs = filled(&mut visual, THUMB);
        assert_eq!(
            thumbs.len(),
            2,
            "the public ScrollHandle adapter exposes both native axes"
        );
        let vertical = thumbs
            .iter()
            .find(|thumb| thumb.bounds.width < thumb.bounds.height)
            .unwrap()
            .bounds;
        let track_x = if direction == gpui_rhai::TextDirection::RightToLeft {
            6.0
        } else {
            294.0
        };
        assert!(
            (vertical.x + vertical.width / 2.0 - track_x).abs() < 1.0,
            "the vertical track follows logical end"
        );
        let vertical_click = point(px(track_x), px(145.0));
        visual.simulate_mouse_down(vertical_click, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_up(vertical_click, MouseButton::Left, Modifiers::default());
        assert!(source.offset().y < px(0.0));
        let old_y = source.offset().y;
        let horizontal_click = point(px(225.0), px(194.0));
        visual.simulate_mouse_down(horizontal_click, MouseButton::Left, Modifiers::default());
        visual.simulate_mouse_up(horizontal_click, MouseButton::Left, Modifiers::default());
        assert!(source.offset().x < px(0.0));
        assert_eq!(
            source.offset().y,
            old_y,
            "a horizontal track click preserves the vertical position"
        );
        settle(&mut visual, Step::Tick, 3);
        visual.simulate_event(ScrollWheelEvent {
            position: point(px(track_x), px(100.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-10.0))),
            ..Default::default()
        });
        assert!(
            source.offset().y < old_y,
            "wheel over the track drives this viewport"
        );
        settle(&mut visual, Step::Tick, 3);
        let vertical = filled(&mut visual, THUMB)
            .into_iter()
            .find(|thumb| thumb.bounds.width < thumb.bounds.height)
            .unwrap()
            .bounds;
        let position = point(px(track_x), px(vertical.y + vertical.height / 2.0));
        visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        let before = source.offset();
        visual.deactivate_window();
        visual.simulate_mouse_move(
            position - point(px(0.0), px(30.0)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert_eq!(
            source.offset(),
            before,
            "a Host-native surface cancels dragging even without a ScriptView in the window"
        );
    }
}
