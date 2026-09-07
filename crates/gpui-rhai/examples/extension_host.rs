use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div, px, rgba};
use gpui_rhai::{
    AppManifest, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityHandler, CapabilityId,
    CapabilityMethod, ComponentStateSchema, EmbeddedScriptSource, EmbeddedScriptView, ModuleId,
    ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId,
    PrimitiveInstance, PrimitiveValue, RuntimeEngine, ScriptApplication, ScriptViewExtension,
    SubscriptionCapabilityHandler, SubscriptionWork, TaskWork, UiRuntimeState, UiValue,
    ValueSchema,
};
use semver::Version;

const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");

const MAIN: &str = r#"
define_component(#{
    metadata: #{ id: "examples/ticker", "export": "Ticker", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
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
    text(`Subscription tick: ${ctx.get_state("tick")}`)
}
fn Ticker() { render_component("examples/ticker", #{ key: "ticker" }) }

fn init(ctx) {
    let message = ctx.call_capability("app.text_transform", "uppercase", "extension ready");
    ctx.set_state("message", message);
    ctx.start_task("app.delayed_text", "load", "background ready", Fn("loaded"), Fn("failed"));
}

fn view(ctx) {
    column([
        text("Host extension"),
        my_app::StatusCard(#{ message: ctx.get_state("message") }),
        Ticker(),
        text("The card above is rendered by a Rust primitive.")
            .with_style(style().text_color(theme_color("text_muted")))
    ]).with_style(
        style().width(px(520)).padding(px(24)).gap(px(16))
            .background(theme_color("surface"))
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
            ("load", UiValue::String(value)) => Ok(Box::new(move || {
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

struct StatusCard;

impl PrimitiveHandler for StatusCard {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        theme: &gpui_rhai::PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        let Some(PrimitiveValue::Data(UiValue::String(message))) =
            instance.node.props.get("message")
        else {
            return Err("StatusCard.message must be a string".to_owned());
        };

        Ok(div()
            .p_4()
            .rounded(px(10.0))
            .bg(rgba(
                theme
                    .color("accent")
                    .unwrap_or_else(|| gpui_rhai::Rgba8::from_rgba_hex(0x2563_ebff))
                    .as_rgba_hex(),
            ))
            .text_color(rgba(
                theme
                    .color("on_accent")
                    .unwrap_or_else(|| gpui_rhai::Rgba8::from_rgba_hex(0xffff_ffff))
                    .as_rgba_hex(),
            ))
            .child(message.clone())
            .into_any_element())
    }
}

struct DemoExtension;

impl ScriptViewExtension for DemoExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("my_app.status_card")
                        .map_err(|error| error.to_string())?,
                    export: "StatusCard".to_owned(),
                    props: BTreeMap::from([(
                        "message".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    )]),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: false,
                },
                StatusCard,
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

fn main() {
    let entry = ModuleId::parse("main").expect("static module ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    let manifest = AppManifest::new(entry.clone())
        .with_capability("app.text_transform", "*")
        .expect("static capability requirement")
        .with_capability("app.delayed_text", "*")
        .expect("static capability requirement")
        .with_capability("app.ticker", "*")
        .expect("static capability requirement");

    EmbeddedScriptView::new(entry, scripts, DEFAULT_DARK)
        .extension(DemoExtension)
        .manifest(manifest)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(600.0, 320.0)
                .run()
        })
        .expect("extension_host failed");
}
