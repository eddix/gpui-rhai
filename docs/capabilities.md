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

Use `ctx.start_task` for one-shot background work and
`ctx.start_subscription` for streams. Rust work runs off the foreground thread;
callbacks return to the GPUI thread. App, window, and component scopes define
cancellation lifetime. Closing a window cancels its window/component work but
does not cancel app-scoped work.

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
