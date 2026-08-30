use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use gpui::{
    App, AppContext, Application, Bounds, Context, IntoElement, Render, Task, Timer, WeakEntity,
    Window, WindowBounds, WindowOptions, px, size,
};
use gpui_rhai::{
    ColorValue, EventPropagation, HostCallback, Length, PrimitiveNode, PrimitiveProps,
    PrimitiveRegistry, PrimitiveValue, Rgba8, StaticUiView, Style, TextInputPrimitiveHandler,
    UiNode, UiValue, init_text_input, text_input_primitive_descriptor,
};

#[derive(Clone, Debug, Default)]
struct HostFrame {
    input: String,
    selected: usize,
    status: String,
}

enum HostEvent {
    Input(String),
    Submit(String),
    Select(usize),
}

struct HostOwnedTree {
    view: gpui::Entity<StaticUiView>,
    frame: HostFrame,
    frames: Receiver<HostFrame>,
    events: Sender<HostEvent>,
    _poll: Task<()>,
}

impl HostOwnedTree {
    fn new(cx: &mut Context<Self>) -> Self {
        let (events, event_receiver) = channel();
        let (frame_sender, frames) = channel();
        spawn_worker(event_receiver, frame_sender);

        let registry = PrimitiveRegistry::new();
        registry
            .register(
                text_input_primitive_descriptor(),
                TextInputPrimitiveHandler::default(),
            )
            .expect("built-in TextInput primitive registers");
        let initial = HostFrame {
            status: "Rust Host owns this frame".to_owned(),
            ..HostFrame::default()
        };
        let weak = cx.entity().downgrade();
        let retained =
            StaticUiView::with_primitives(build_tree(&initial, &events, &weak), registry)
                .expect("initial Host-owned tree should reconcile");
        let view = cx.new(|_| retained);
        let poll_weak = weak.clone();
        let poll = cx.spawn(async move |_, cx| {
            loop {
                Timer::after(Duration::from_millis(16)).await;
                if poll_weak.update(cx, HostOwnedTree::poll_frames).is_err() {
                    break;
                }
            }
        });
        Self {
            view,
            frame: initial,
            frames,
            events,
            _poll: poll,
        }
    }

    fn poll_frames(&mut self, cx: &mut Context<Self>) {
        let mut latest = None;
        while let Ok(frame) = self.frames.try_recv() {
            latest = Some(frame);
        }
        if let Some(frame) = latest {
            self.set_frame(frame, cx);
        }
    }

    fn set_frame(&mut self, frame: HostFrame, cx: &mut Context<Self>) {
        self.frame = frame;
        self.refresh_tree(cx);
    }

    fn echo_controlled_input(&mut self, value: String, cx: &mut Context<Self>) {
        self.frame.input = value;
        "Input sent to the Host worker".clone_into(&mut self.frame.status);
        self.refresh_tree(cx);
    }

    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        let root = build_tree(&self.frame, &self.events, &weak);
        self.view
            .update(cx, |view, cx| view.set_root(root, cx))
            .expect("Host-owned frame should reconcile atomically");
    }
}

impl Render for HostOwnedTree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}

fn spawn_worker(events: Receiver<HostEvent>, frames: Sender<HostFrame>) {
    std::thread::Builder::new()
        .name("gpui-rhai-host-owned-tree".to_owned())
        .spawn(move || {
            let mut frame = HostFrame {
                status: "Rust Host owns this frame".to_owned(),
                ..HostFrame::default()
            };
            while let Ok(event) = events.recv() {
                match event {
                    HostEvent::Input(value) => {
                        frame.input = value;
                        "Input event handled on the Host worker".clone_into(&mut frame.status);
                    }
                    HostEvent::Submit(value) => {
                        frame.status = format!("Submitted: {value}");
                    }
                    HostEvent::Select(index) => {
                        frame.selected = index;
                        frame.status = format!("Selected row {index}");
                    }
                }
                if frames.send(frame.clone()).is_err() {
                    break;
                }
            }
        })
        .expect("Host worker starts");
}

fn build_tree(
    frame: &HostFrame,
    events: &Sender<HostEvent>,
    owner: &WeakEntity<HostOwnedTree>,
) -> UiNode {
    let input_events = events.clone();
    let submit_events = events.clone();
    let input_owner = owner.clone();
    let input = UiNode::custom(PrimitiveNode {
        primitive: text_input_primitive_descriptor().id,
        key: Some("host-input".to_owned()),
        props: PrimitiveProps::new()
            .with(
                "value",
                PrimitiveValue::Data(UiValue::String(frame.input.clone())),
            )
            .with(
                "placeholder",
                PrimitiveValue::Data(UiValue::String("Type into a Host-owned tree".to_owned())),
            )
            .with(
                "on_change",
                PrimitiveValue::Callback(
                    HostCallback::new("host.input-change", move |payload, _, app| {
                        if let UiValue::String(value) = payload {
                            let event_value = value.clone();
                            let _ = input_owner.update(app, |owner, cx| {
                                owner.echo_controlled_input(value, cx);
                            });
                            let _ = input_events.send(HostEvent::Input(event_value));
                        }
                        EventPropagation::Handled
                    })
                    .into(),
                ),
            )
            .with(
                "on_submit",
                PrimitiveValue::Callback(
                    HostCallback::new("host.input-submit", move |payload, _, _| {
                        if let UiValue::String(value) = payload {
                            let _ = submit_events.send(HostEvent::Submit(value));
                        }
                        EventPropagation::Handled
                    })
                    .into(),
                ),
            ),
    });

    let row_events = events.clone();
    let row_callback = HostCallback::new("host.row-click", move |payload, _, _| {
        if let UiValue::Integer(index) = payload
            && let Ok(index) = usize::try_from(index)
        {
            let _ = row_events.send(HostEvent::Select(index));
        }
        EventPropagation::Handled
    });
    let rows = (0..4)
        .map(|index| {
            let selected = frame.selected == index;
            UiNode::text(format!("{} Row {index}", if selected { "●" } else { "○" }))
                .with_key(format!("row-{index}"))
                .with_style(
                    &Style::new()
                        .padding(Length::pixels(8.0).unwrap())
                        .radius(Length::pixels(6.0).unwrap())
                        .background(ColorValue::Literal(Rgba8::from_rgb_hex(if selected {
                            0x003b_82f6
                        } else {
                            0x0027_272a
                        }))),
                )
                .with_host_handler("click", row_callback.clone())
                .with_handler_payload(
                    "click",
                    UiValue::Integer(i64::try_from(index).expect("four rows fit i64")),
                )
        })
        .collect();

    UiNode::column(vec![
        UiNode::text("Host-owned interactive UiNode tree")
            .with_style(&Style::new().font_size(Length::rems(1.25).unwrap())),
        UiNode::text("No Rhai engine, lifecycle, capability, or subscription"),
        input,
        UiNode::column(rows).with_style(&Style::new().gap(Length::pixels(6.0).unwrap())),
        UiNode::text(frame.status.clone()),
    ])
    .with_style(
        &Style::new()
            .width(Length::pixels(520.0).unwrap())
            .padding(Length::pixels(24.0).unwrap())
            .gap(Length::pixels(14.0).unwrap())
            .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0018_181b)))
            .text_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x00f4_f4f5))),
    )
}

fn main() {
    Application::new().run(|cx: &mut App| {
        init_text_input(cx);
        let bounds = Bounds::centered(None, size(px(568.0), px(440.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |_, cx| cx.new(HostOwnedTree::new),
        )
        .expect("host_owned_tree window opens");
        cx.activate(true);
    });
}
