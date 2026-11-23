#[path = "support/fonts.rs"]
mod shared_fonts;
#[path = "support/themes.rs"]
mod shared_themes;

use std::collections::BTreeMap;
use std::time::Duration;

use gpui_rhai::gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div};
use gpui_rhai::{
    AppManifest, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityHandler, CapabilityId,
    CapabilityMethod, ComponentStateSchema, EmbeddedScriptSource, HostSlotRegistry, ModuleId,
    ObjectField, PrimitiveContext, PrimitiveDescriptor, PrimitiveHandler, PrimitiveId,
    PrimitiveInstance, PrimitiveValue, RuntimeEngine, ScriptApplication, ScriptView,
    ScriptViewExtension, SubscriptionCapabilityHandler, SubscriptionWork, TaskWork, UiRuntimeState,
    UiValue, ValueSchema,
};
use semver::Version;

const MAIN: &str = r#"
import "components/card" as card;

define_component(#{
    metadata: #{ id: "examples/ticker", "export": "Ticker", version: "0.1.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{ "app.ticker": "*" } },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{ tick: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } } } },
        events: #{}, slots: #{}, parts: [], effects: ["watch"] },
    render: Fn("render_Ticker"),
});

fn state_schema() {
    #{ fields: #{
        message: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Waiting for host extension" } },
    } }
}

fn loaded(ctx, value) { ctx.set_state("message", value); }
fn failed(ctx, error) { ctx.set_state("message", `Host error: ${error}`); }
fn ticker_received(ctx, value) { ctx.set_state("tick", value); }
fn ticker_failed(ctx, error) { ctx.set_state("tick", -1); }
fn start_ticker(ctx, deps) {
    ctx.start_subscription("app.ticker", "watch", (),
        Fn("ticker_received"), Fn("ticker_failed"),
        #{ delivery: "latest", throttle_ms: 10 });
}
fn stop_ticker(ctx, deps) { () }
fn render_Ticker(ctx, props) {
    effect("watch", (), Fn("start_ticker"), Fn("stop_ticker"));
    text(`Subscription tick: ${ctx.get_state("tick")}`).test_id("extension-tick")
}
fn Ticker() { render_component("examples/ticker", #{ key: "ticker" }) }
fn host_slot(name) { gpui_rhai::HostSlot(#{ key: name, name: name }) }

fn init(ctx) {
    let message = ctx.call_capability("app.text_transform", "uppercase", "extension ready");
    ctx.set_state("message", message);
    ctx.start_task("app.delayed_text", "load", "background ready", Fn("loaded"), Fn("failed"));
}

fn view(ctx) {
    column([
        text("Host extension").with_style(style().typography("heading")),
        card::Card(#{key:"status-card",header:text("Host update"),
            content:my_app::StatusMessage(#{message:ctx.get_state("message")})
                .with_key("host-message").accessibility_role("status")
                .accessibility_label("Host extension message").accessibility_value(ctx.get_state("message"))})
            .test_id("extension-status-card"),
        host_slot("native-footer").with_style(style().width(relative(1.0)).height(theme_length("metrics.row"))),
        Ticker(),
        text("Rust owns the content; the official Card owns its design.")
            .with_style(style().text_color(theme_color("text_muted")))
    ]).with_style(
        style().width(relative(1.0)).height(relative(1.0))
            .padding(theme_spacing("lg")).gap(theme_spacing("md")).typography("body")
            .background(theme_color("surface")).text_color(theme_color("text_primary"))
    )
}
"#;

struct UppercaseCapability;

impl CapabilityHandler for UppercaseCapability {
    fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String> {
        match (method, input) {
            ("uppercase", UiValue::String(value)) => Ok(UiValue::String(value.to_uppercase())),
            ("uppercase", _) => Err("uppercase expects a string".to_owned()),
            _ => Err(format!("unknown text transform method `{method}`")),
        }
    }
}

struct DelayedTextCapability;

impl AsyncCapabilityHandler for DelayedTextCapability {
    fn start(&mut self, method: &str, input: UiValue) -> Result<TaskWork, String> {
        match (method, input) {
            ("load", UiValue::String(value)) => Ok(TaskWork::new(move || {
                std::thread::sleep(Duration::from_millis(20));
                Ok(UiValue::String(value.to_uppercase()))
            })),
            ("load", _) => Err("load expects a string".to_owned()),
            _ => Err(format!("unknown delayed text method `{method}`")),
        }
    }
}

struct TickerCapability;

impl SubscriptionCapabilityHandler for TickerCapability {
    fn subscribe(&mut self, method: &str, input: UiValue) -> Result<SubscriptionWork, String> {
        if method != "watch" || input != UiValue::Null {
            return Err("watch expects null input".to_owned());
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("gpui-rhai-example-ticker".to_owned())
            .spawn(move || {
                for tick in 1..=3 {
                    std::thread::sleep(Duration::from_millis(15));
                    if sender.send(UiValue::Integer(tick)).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(SubscriptionWork::from_receiver(receiver))
    }
}

struct StatusMessage;

impl PrimitiveHandler for StatusMessage {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        _: &PrimitiveContext,
        _: &gpui_rhai::PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let Some(PrimitiveValue::Data(UiValue::String(message))) =
            instance.node.props.get("message")
        else {
            return Err("StatusMessage.message must be a string".to_owned());
        };

        // Preserve inherited Card typography and ink. A primitive that only
        // supplies content does not need a second card painter or font recipe.
        Ok(div().child(message.clone()).into_any_element())
    }
}

struct DemoExtension;

impl ScriptViewExtension for DemoExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("my_app.status_message")
                        .map_err(|error| error.to_string())?,
                    export: "StatusMessage".to_owned(),
                    props: BTreeMap::from([(
                        "message".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    )]),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: false,
                    effect: None,
                },
                StatusMessage,
            )
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let id = CapabilityId::parse("app.text_transform").map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: id.clone(),
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "uppercase".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::string(),
                            output: ValueSchema::string(),
                        },
                    )]),
                },
                UppercaseCapability,
            )
            .map_err(|error| error.to_string())?;
        let delayed = CapabilityId::parse("app.delayed_text").map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_async(
                CapabilityDescriptor {
                    id: delayed,
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "load".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::string(),
                            output: ValueSchema::string(),
                        },
                    )]),
                },
                DelayedTextCapability,
            )
            .map_err(|error| error.to_string())?;
        let ticker = CapabilityId::parse("app.ticker").map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_subscription(
                CapabilityDescriptor {
                    id: ticker,
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "watch".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Null,
                            output: ValueSchema::integer(),
                        },
                    )]),
                },
                TickerCapability,
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

/// Build the runnable Host-extension example, also mounted by the native tests.
///
/// # Panics
///
/// Panics if a static module ID, capability requirement or Host-slot name in
/// this example is invalid.
#[must_use]
pub fn view() -> ScriptView {
    let entry = ModuleId::parse("main").expect("static module ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (entry.clone(), MAIN.to_owned()),
        (
            ModuleId::parse("components/card").expect("static Card module ID"),
            include_str!("../../../registry/components/card.rhai").to_owned(),
        ),
    ]));
    let manifest = AppManifest::new(entry.clone())
        .with_capability("app.text_transform", "*")
        .expect("static capability requirement")
        .with_capability("app.delayed_text", "*")
        .expect("static capability requirement")
        .with_capability("app.ticker", "*")
        .expect("static capability requirement");
    let host_slots = HostSlotRegistry::new()
        .with_slot("native-footer", |_, _| {
            Ok(div()
                .size_full()
                .child("Opaque element supplied by the Rust Host")
                .into_any_element())
        })
        .expect("static host slot name");

    ScriptView::sources(entry, scripts, shared_themes::PRIMARY)
        .token_base(include_str!("../../../registry/tokens.rhai"))
        .font_sources(shared_fonts::sources())
        .theme_sources(shared_themes::additional())
        .extension(DemoExtension)
        .extension(host_slots)
        .manifest(manifest)
}

fn main() {
    view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(600.0, 320.0)
                .run()
        })
        .expect("extension_host failed");
}
