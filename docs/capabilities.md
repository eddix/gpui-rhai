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

Synchronous methods may run only in `init`, `dispose`, or an event callback:

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
