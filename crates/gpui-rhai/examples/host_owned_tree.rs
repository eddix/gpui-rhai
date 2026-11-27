use std::borrow::Cow;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use gpui_rhai::gpui::{
    App, AppContext, Bounds, Context, IntoElement, Render, Task, WeakEntity, Window, WindowBounds,
    WindowOptions, px, size,
};
use gpui_rhai::{
    ColorValue, EventPropagation, HostCallback, Length, PrimitiveNode, PrimitiveProps,
    PrimitiveRegistry, PrimitiveValue, RuntimeEngine, StaticUiView, Style,
    TextInputPrimitiveHandler, ThemeTokenOverrides, ThemeVariant, UiNode, UiValue, init_text_input,
    load_theme_with_layers, load_token_base, text_input_primitive_descriptor,
};

#[derive(Default)]
struct HostFrame {
    input: String,
    input_revision: u64,
    worker: WorkerFrame,
}

impl HostFrame {
    fn status(&self) -> &str {
        if self.worker.acknowledged_input < self.input_revision {
            "Input sent to the Host worker"
        } else {
            &self.worker.status
        }
    }
}

/// The worker projects business state and acknowledges an edit. The text
/// editor stays controlled by the foreground until a new user edit arrives.
#[derive(Clone)]
struct WorkerFrame {
    selected: usize,
    status: String,
    acknowledged_input: u64,
}

impl Default for WorkerFrame {
    fn default() -> Self {
        Self {
            selected: 0,
            status: "Rust Host owns this frame".to_owned(),
            acknowledged_input: 0,
        }
    }
}

enum HostEvent {
    Input { value: String, revision: u64 },
    Submit(String),
    Select(usize),
}

impl WorkerFrame {
    fn worker_event(&mut self, event: HostEvent) {
        match event {
            HostEvent::Input { value, revision } => {
                self.acknowledged_input = revision;
                if value.is_empty() {
                    "Empty input handled on the Host worker".clone_into(&mut self.status);
                } else {
                    "Input event handled on the Host worker".clone_into(&mut self.status);
                }
            }
            HostEvent::Submit(value) => {
                self.status = format!("Submitted: {value}");
            }
            HostEvent::Select(index) => {
                self.selected = index;
                self.status = format!("Selected row {index}");
            }
        }
    }
}

/// Rust-owned reference application state; UI callbacks retain only weak ownership.
pub struct HostOwnedTree {
    view: gpui_rhai::gpui::Entity<StaticUiView>,
    frame: HostFrame,
    frames: Receiver<WorkerFrame>,
    events: Sender<HostEvent>,
    themes: Vec<ThemeVariant>,
    theme_index: usize,
    _poll: Task<()>,
}

impl HostOwnedTree {
    fn new(themes: Vec<ThemeVariant>, cx: &mut Context<Self>) -> Self {
        let (events, event_receiver) = channel();
        let (frame_sender, frames) = channel();
        spawn_worker(event_receiver, frame_sender);
        Self::with_worker(themes, events, frames, cx)
    }

    fn with_worker(
        themes: Vec<ThemeVariant>,
        events: Sender<HostEvent>,
        frames: Receiver<WorkerFrame>,
        cx: &mut Context<Self>,
    ) -> Self {
        let registry = PrimitiveRegistry::new();
        registry
            .register(
                text_input_primitive_descriptor(),
                TextInputPrimitiveHandler::default(),
            )
            .expect("built-in TextInput primitive registers");
        let initial = HostFrame::default();
        let weak = cx.entity().downgrade();
        let retained =
            StaticUiView::with_primitives(build_tree(&initial, &events, &weak), registry)
                .expect("initial Host-owned tree should reconcile")
                .with_theme(themes[0].clone())
                .expect("bundled theme is valid");
        let view = cx.new(|_| retained);
        let poll_weak = weak.clone();
        let poll = cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
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
            themes,
            theme_index: 0,
            _poll: poll,
        }
    }

    fn poll_frames(&mut self, cx: &mut Context<Self>) {
        let mut latest = None;
        while let Ok(frame) = self.frames.try_recv() {
            latest = Some(frame);
        }
        if let Some(frame) = latest {
            self.frame.worker = frame;
            self.refresh_tree(cx);
        }
    }

    fn echo_controlled_input(&mut self, value: String, cx: &mut Context<Self>) {
        self.frame.input.clone_from(&value);
        self.frame.input_revision += 1;
        let _ = self.events.send(HostEvent::Input {
            value,
            revision: self.frame.input_revision,
        });
        self.refresh_tree(cx);
    }

    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        let root = build_tree(&self.frame, &self.events, &weak);
        self.view
            .update(cx, |view, cx| view.set_root(root, cx))
            .expect("Host-owned frame should reconcile atomically");
    }

    fn next_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = (self.theme_index + 1) % self.themes.len();
        let theme = self.themes[next].clone();
        self.view
            .update(cx, |view, cx| view.set_theme(Some(theme.clone()), cx))
            .expect("bundled theme is valid");
        self.theme_index = next;
        window.set_window_title(&format!(
            "Host-owned tree · {} / {}",
            theme.family, theme.name
        ));
    }
}

impl Render for HostOwnedTree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}

fn spawn_worker(events: Receiver<HostEvent>, frames: Sender<WorkerFrame>) {
    std::thread::Builder::new()
        .name("gpui-rhai-host-owned-tree".to_owned())
        .spawn(move || {
            let mut frame = WorkerFrame::default();
            while let Ok(event) = events.recv() {
                frame.worker_event(event);
                if frames.send(frame.clone()).is_err() {
                    break;
                }
            }
        })
        .expect("Host worker starts");
}

fn build_input(
    frame: &HostFrame,
    events: &Sender<HostEvent>,
    owner: &WeakEntity<HostOwnedTree>,
) -> UiNode {
    let submit_events = events.clone();
    let input_owner = owner.clone();
    UiNode::custom(PrimitiveNode {
        primitive: text_input_primitive_descriptor().id,
        key: Some("host-input".to_owned()),
        props: PrimitiveProps::new()
            .with(
                "typography",
                PrimitiveValue::Data(UiValue::String("control".to_owned())),
            )
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
                            let _ = input_owner.update(app, |owner, cx| {
                                owner.echo_controlled_input(value, cx);
                            });
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
    })
    .with_style(
        &Style::new()
            .min_height(Length::token("metrics.field_h").unwrap())
            .radius(Length::token("metrics.field_radius").unwrap())
            .background(ColorValue::token("surface.field").unwrap())
            .text_color(ColorValue::token("text_primary").unwrap()),
    )
}

fn build_tree(
    frame: &HostFrame,
    events: &Sender<HostEvent>,
    owner: &WeakEntity<HostOwnedTree>,
) -> UiNode {
    let input = build_input(frame, events, owner);
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
            let selected = frame.worker.selected == index;
            UiNode::text(format!("{} Row {index}", if selected { "●" } else { "○" }))
                .with_key(format!("row-{index}"))
                .with_style(
                    &Style::new()
                        .padding(Length::theme_spacing("sm").unwrap())
                        .radius(Length::theme_radius("sm").unwrap())
                        .background(
                            ColorValue::token(if selected {
                                "selection"
                            } else {
                                "surface_hover"
                            })
                            .unwrap(),
                        ),
                )
                .with_host_handler("click", row_callback.clone())
                .with_handler_payload(
                    "click",
                    UiValue::Integer(i64::try_from(index).expect("four rows fit i64")),
                )
        })
        .collect();

    let theme_owner = owner.clone();
    let next_theme = UiNode::text("Next design")
        .with_key("next-design")
        .with_attribute("role", UiValue::String("button".to_owned()))
        .with_style(
            &Style::new()
                .padding(Length::theme_spacing("sm").unwrap())
                .radius(Length::theme_radius("sm").unwrap())
                .background(ColorValue::token("surface_hover").unwrap()),
        )
        .with_host_handler(
            "click",
            HostCallback::new("host.next-design", move |_, window, app| {
                let _ = theme_owner.update(app, |owner, cx| owner.next_theme(window, cx));
                EventPropagation::Handled
            }),
        );
    UiNode::column(vec![
        UiNode::text("Host-owned interactive UiNode tree")
            .with_style(&Style::new().typography("heading").unwrap()),
        UiNode::text("Rust UI and callbacks; theme configuration parsed once at startup")
            .with_style(&Style::new().text_color(ColorValue::token("text_muted").unwrap())),
        next_theme,
        input,
        UiNode::column(rows).with_style(&Style::new().gap(Length::theme_spacing("xs").unwrap())),
        UiNode::text(frame.status().to_owned()),
    ])
    .with_style(
        &Style::new()
            .width(Length::relative(1.0).unwrap())
            .height(Length::relative(1.0).unwrap())
            .typography("body")
            .unwrap()
            .padding(Length::theme_spacing("lg").unwrap())
            .gap(Length::theme_spacing("md").unwrap())
            .background(ColorValue::token("surface").unwrap())
            .text_color(ColorValue::token("text_primary").unwrap()),
    )
}

fn resolved_designs() -> Vec<ThemeVariant> {
    // Parsing is startup configuration only. No Engine or script UI is retained
    // by the Host, StaticUiView, callbacks, worker or frame poller.
    let runtime = RuntimeEngine::new();
    let base = load_token_base(
        runtime.engine(),
        "tokens",
        gpui_rhai_registry::TOKEN_BASE_SOURCE,
    )
    .expect("bundled token base is valid");
    [
        ("modern_dark", gpui_rhai_registry::MODERN_DARK_THEME),
        ("modern_light", gpui_rhai_registry::MODERN_LIGHT_THEME),
        ("clean_dark", gpui_rhai_registry::CLEAN_DARK_THEME),
        ("clean_light", gpui_rhai_registry::CLEAN_LIGHT_THEME),
    ]
    .into_iter()
    .map(|(name, source)| {
        let theme = load_theme_with_layers(
            runtime.engine(),
            Some(&base),
            name,
            source,
            &ThemeTokenOverrides::default(),
        )
        .expect("bundled design is valid");
        theme
            .require(
                "host_owned_tree",
                [
                    "surface",
                    "surface_hover",
                    "text_primary",
                    "text_muted",
                    "selection",
                    "surface.field",
                    "metrics.field_h",
                    "metrics.field_radius",
                    "spacing.xs",
                    "spacing.sm",
                    "spacing.md",
                    "spacing.lg",
                    "radius.sm",
                    "typography.body",
                    "typography.heading",
                    "typography.control",
                ],
            )
            .expect("bundled design provides the Host's vocabulary");
        theme
    })
    .collect()
}

#[test]
fn bundled_designs_supply_the_host_owned_example_vocabulary() {
    let designs = resolved_designs();
    assert_eq!(designs.len(), 4);
    assert_eq!(designs[0].family, "Modern");
    assert_eq!(designs[0].name, "Dark");
}

/// Build the actual Rust-owned application, also mounted by native integration tests.
/// Theme configuration is parsed once here, never by a render or callback.
///
/// # Panics
/// Panics if bundled theme/font data is invalid or the Host worker cannot start.
#[must_use]
pub fn view(cx: &mut Context<HostOwnedTree>) -> HostOwnedTree {
    install_view_fonts(cx);
    HostOwnedTree::new(resolved_designs(), cx)
}

fn install_view_fonts(cx: &mut Context<HostOwnedTree>) {
    init_text_input(cx);
    cx.text_system()
        .add_fonts(
            gpui_rhai_registry::BUNDLED_FONT_SOURCES
                .iter()
                .map(|&(_, bytes)| Cow::Borrowed(bytes))
                .collect(),
        )
        .expect("bundled UI fonts load");
}

fn main() {
    gpui_rhai::gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(568.0), px(440.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |window, cx| {
                window.set_window_title("Host-owned tree · Modern / Dark");
                cx.new(view)
            },
        )
        .expect("host_owned_tree window opens");
        cx.activate(true);
    });
}
