//! External-consumer tests of the scoped factory and native ListState hooks.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use gpui::{
    AnyElement, App, Bounds, Context, InteractiveElement, IntoElement, ListAlignment, ListState,
    Modifiers, MouseButton, ParentElement, Pixels, Point, Render, ScrollDelta, ScrollHandle,
    ScrollWheelEvent, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext,
    Window, div, list, point, px,
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
        let scrollbar = context
            .scrollbars(theme)
            .with_part_styles(&Style::default(), &thumb, &thumb)
            .element(
                "scrollbar",
                self.1.borrow().clone(),
                ScrollbarSpec::new("hidden", "always")?,
            );
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
