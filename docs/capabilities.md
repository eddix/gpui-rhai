# Capabilities

Capabilities are the only script path to application services such as
persistence, network requests, platform APIs, and long-running work. Each
capability has a namespaced ID, semantic version, method schemas, and a Rust
handler.

Implement `ScriptViewExtension` to configure the host. Register the descriptor
and handler in `configure_runtime`; the application manifest activates exactly
the versions the application declares after all extensions register:

```rust
impl ScriptViewExtension for AppServices {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime.capabilities.register(descriptor(), Handler)
            .map_err(|error| error.to_string())
    }
}
```

Attach it with `.extension(AppServices)`. The complete synchronous, task, and
subscription example is `crates/gpui-rhai/examples/extension_host.rs`. File views load
`ui/app.toml`; embedded applications pass `AppManifest` with `.manifest(...)`.

## Rhai calls

Synchronous methods may run anywhere except `view`: `init`, an event callback,
an effect, `suspend`, `resume`, or `dispose`:

```rhai
let result = ctx.call_capability("app.settings", "load", #{ key: "theme" });
```

Use `ctx.start_task` for one-shot background work. It may start from `init`, an
event callback, an effect start, or `resume`; ordinary component-owned tasks
continue across retained view suspension. Rust work runs off the foreground
thread on a bounded shared worker pool and callbacks return to the GPUI thread.
Implement ordinary work with `TaskWork::new`. For work that can stop early,
use `TaskWork::cancellable` and periodically inspect the supplied
`TaskCancellation`; the runtime never attempts to forcibly interrupt arbitrary
Rust code.

`ctx.start_subscription` is valid only inside a declared formal-component
effect start callback. This makes every long-lived stream structurally owned:
dependency replacement, unmount, suspension, hot reload, and disposal all run
cleanup and cancel the exact effect activation. Starting a subscription from
root `init` or an ordinary event callback is a runtime error.

Subscription delivery is explicit and bounded:

```rhai
ctx.start_subscription(
    "app.events", "watch", (), Fn("received"), Fn("failed"),
    #{ delivery: "all", capacity: 128, throttle_ms: 0 },
);
```

The call above belongs inside a function referenced by
`effect("stream", deps, Fn("start_stream"), Fn("stop_stream"))`, not directly
inside `view` or root lifecycle code.

`delivery: "all"` is the default and preserves FIFO order. Its queue defaults
to 64 values and reports backpressure to the Rust emitter instead of silently
dropping an event. `delivery: "latest"` explicitly coalesces pending values and
is appropriate for progress, sensors, and replaceable state snapshots.
`throttle_ms` paces lossless delivery or defines the coalescing interval for a
latest-only stream. Capacity is bounded to 1–4096 and throttle to 60 seconds.
The built-in `SubscriptionWork::from_receiver` waits for lossless capacity and
wakes on cancellation; it does not reinterpret backpressure as end-of-stream.
Capacity waits and every close path share one mutex/condition protocol, so
scope cancellation, registry teardown, and generation replacement cannot lose
a producer wakeup. A stale generation discards its buffered values and removes
the subscription immediately; a normal producer close still drains values that
were accepted before close.

## What a call responds to

A synchronous handler can override `call_with` to see the context of a call:
what it runs in response to (`InvocationOrigin`), the view, and the calling
component instance. The default forwards to `call`. Gate a method that should
only run for the user, such as switching to another tool, on
`context.origin.is_user_input()`:

```rust
impl CapabilityHandler for Open {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        Err("call_with is the entry point".into())
    }

    fn call_with(&mut self, context: &InvocationContext, method: &str, input: UiValue)
        -> Result<UiValue, String> {
        if !context.origin.is_user_input() {
            return Err("open needs user input".into());
        }
        self.open(method, input)
    }
}
```

| Origin | Set for |
|---|---|
| `UserInput { event }` | a platform input event dispatched to the view (`click`, `pointer_down`, `key_down`, ...), a Host key-binding action (`action`), a window close (`window_close`) |
| `Automation` | `AutomationCommand::Dispatch` and `Action` |
| `Timer { started_by }` | a timer firing; `started_by` is the root origin that scheduled it |
| `TaskCompletion { started_by }` | a task or image decode completing; `started_by` is the root origin that started it |
| `Subscription` | a subscription delivering a value |
| `Effect` | an effect's start or cleanup, a motion timeline callback |
| `Lifecycle` | `init`, `dispose`, suspend, resume and reload |

Callbacks that run inside the same dispatch (component events a callback
emits, the render it causes) share its origin; effects run as `Effect`. A task a
click starts completes as `TaskCompletion { started_by: UserInput { .. } }`, so
`is_user_input()` follows the chain to its root; an automation command is never
user input. The origin is one field set on each dispatch path and read only by
handlers that ask for it.

Task and subscription handlers receive the same context when they start.
`AsyncCapabilityHandler::start_with` and
`SubscriptionCapabilityHandler::subscribe_with` default to `start` and
`subscribe`; override them to decide before any work runs. A save dialog or a
package install started by `ctx.start_task` can refuse automation:

```rust
impl AsyncCapabilityHandler for Save {
    fn start(&mut self, _: &str, _: UiValue) -> Result<TaskWork, String> {
        Err("start_with is the entry point".into())
    }

    fn start_with(&mut self, context: &InvocationContext, method: &str, input: UiValue)
        -> Result<TaskWork, String> {
        if !context.origin.is_user_input() {
            return Err("save needs user input".into());
        }
        self.save(method, input)
    }
}
```

A subscription starts in an effect, so its origin is `Effect`; the context
still names the view and the component. A Host calling the registry directly
uses `CapabilityRegistry::call_with`, `start_task_with` and
`start_subscription_with`; `call`, `start_task` and `start_subscription` pass
`Lifecycle`, no view and the root component.

## Subscription producer lifetime

`SubscriptionWork` owns the complete producer loop. It must remain running for
as long as any external producer may call a cloned `SubscriptionEmitter`.
Returning from the work closes the subscription immediately with
`SubscriptionCloseReason::WorkReturned`; retaining the emitter elsewhere does
not extend that lifetime.

Use `SubscriptionWork::new` for a blocking producer loop. When the application
already has an external producer, bridge it through a channel so the work owns
the receiver loop:

```rust
fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    self.outlets.lock().unwrap().insert("widget".to_owned(), sender);
    Ok(SubscriptionWork::from_receiver(receiver))
}
```

`SubscriptionEmitter::emit` and `emit_error` return
`AsyncRuntimeError::Closed { reason }` after shutdown. Reasons distinguish work
return, producer close, explicit cancellation, scope disposal, stale generation,
transaction rollback, startup failure, and registry drop. The first close wins.
Foreground diagnostics also record `close <capability>.<method>: <reason>` as a
Subscription trace.

The CLI validates capability IDs and version requirements in `ui/app.toml`.
Runtime activation remains explicit so a manifest cannot grant itself a service.
Startup and `gpui-rhai check` also cross-check every installed component header:
its capability requirement must appear explicitly and identically in the app
manifest before any handler can activate.

## Host extensions in `check`

`gpui-rhai check` lints every call against the functions its own engine
registers. It also knows `gpui_rhai::HostSlot`, which a Host installs with
`HostSlotRegistry`. Functions and custom primitives a Host registers itself in
`ScriptViewExtension::configure_engine` are declared under `[host]` in
`ui/app.toml`, so the lint treats calls to them as known and still checks
their arity:

```toml
entry = "main"
runtime_api = 3

[host]
functions = ["open_file/1", "omb::status/0"]
primitives = ["OmbChart"]
```

A function is `name/arity`, called as `open_file(path)` or `path.open_file()`,
or `module::name/arity` for a static module. A primitive is its PascalCase
export: `OmbChart` is called as `gpui_rhai::OmbChart(#{ ... })`, and
`omb::Chart` declares the export `Chart` of the primitive namespace `omb`. A
malformed entry or an unknown key fails the check with its line in
`ui/app.toml`. Only the CLI reads `[host]`; `AppManifest` and the runtime
ignore it.

Only the Host provides these items, so when the scripts call `HostSlot` or a
declared item, `check` validates them statically and skips the headless first
frame; its summary lists the extensions it found. Declarations the scripts do
not call leave the first frame on. This entry passes `check` without a
`[host]` table:

<!-- check: pass -->
```rhai
fn host_slot(name) { gpui_rhai::HostSlot(#{ key: name, name: name }) }
fn view(ctx) { column([text("Editor"), host_slot("editor")]) }
```
