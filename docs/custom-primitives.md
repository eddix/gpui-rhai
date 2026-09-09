# Custom Rust primitives

Use a custom primitive when a mechanism needs native GPUI APIs—for example an
editor, canvas, media surface, or domain-specific control. Composition and
policy should remain in copied Rhai components where possible.

Register primitives in `ScriptViewExtension::configure_engine`, before scripts
compile. A `PrimitiveDescriptor` declares a namespaced ID, PascalCase Rhai
export, prop/event schemas, optional state schema, and lifecycle requirement.
The `PrimitiveHandler` receives a validated `PrimitiveInstance`, normalized
event emitter, read-only `PrimitiveTheme`, `Window`, and `App`, then returns
`AnyElement`. Resolve component-owned native paint through `PrimitiveTheme`;
handlers must not cache a mutable theme manager or hard-code palette colors.
The snapshot resolves required semantic colors, spacing/radius `Length` tokens,
and typography roles. It changes on the next render after a live theme switch.
Use `PrimitiveTheme::typography(role)` for native text shaping so custom
primitives share the same family, fallback stack, size, line height, and weight.

```rust
engine.register_primitive(
    PrimitiveDescriptor {
        id: PrimitiveId::parse("my_app.status_card")?,
        export: "StatusCard".into(),
        props,
        events,
        state: ComponentStateSchema::default(),
        lifecycle: false,
    },
    StatusCard,
)?;
```

Rhai imports no Rust types and calls the generated namespace constructor:

```rhai
my_app::StatusCard(#{ message: "Ready" })
```

Trusted Rust Hosts can construct `PrimitiveProps` directly. A callback-typed
prop accepts `PrimitiveValue::Callback(UiEventHandler::Host(callback))`; emitted
payloads still pass through the descriptor event schema before the Host closure
runs. Rhai conversion of `ValueSchema::Callback` remains Script-only.

Lifecycle/stateful primitives require a stable component-local `key` and must
render through a `RetainedUiTree`. Reconciliation combines the primitive ID,
key, and stable `NodeId` into `PrimitiveInstanceId`; identical local keys in
different components therefore retain independent native Entities, while keyed
reorder preserves them. Direct ephemeral rendering rejects lifecycle
primitives instead of falling back to a view-global key.

Every retained `PrimitiveInstance` also exposes `instance.resources()`. Register
each durable native task, subscription, watcher, capture helper, or other
resource with a safe label and cancellation closure:

```rust
fn mount(&mut self, instance: &PrimitiveInstance) -> Result<(), String> {
    let resources = instance.resources().expect("lifecycle primitive");
    let cancel = self.start_watcher()?;
    resources
        .own("document_watcher", move || cancel())
        .map_err(|error| error.to_string())?;
    Ok(())
}
```

The same scope is preserved across keyed updates. Runtime unmount invokes the
handler and then cancels all remaining resources in reverse registration order.
Resources added by a mount/update/render attempt are rolled back if that attempt
fails; a newly mounted handler also receives `unmount` if its first render
fails. One panicking cleanup is diagnosed but does not prevent later cleanups.
Dropping the last scope is also a cleanup backstop. Do not capture the primitive
registry or resource scope itself in a cleanup closure. Native changes made by
`update` cannot be cloned by the runtime: stage them until success or make
`update(previous, next)` idempotent so a failed outer render can safely retry.

Emit only declared events through `PrimitiveEventEmitter`; the runtime validates
payloads and dispatches the generation-bound callback. The emitter keeps only a
weak reference back to the registry, so an Entity whose callbacks retain the
emitter cannot form `Registry -> Entity -> Registry` ownership cycles. Use GPUI
through `gpui_rhai::gpui` so the host and runtime cannot link incompatible GPUI
type versions.

`ValueSchema::Signal` and `ValueSchema::Ref` props remain typed native handles
as `PrimitiveValue::Signal` and `PrimitiveValue::Ref`; they are not flattened
into durable `UiValue`. A foreground primitive may write a passed signal with
`PrimitiveEventEmitter::write_signal`, which requests a GPUI repaint without
executing Rhai. It may query the last committed layout rectangle of a passed
ref with `PrimitiveEventEmitter::element_bounds`. Both operations fail closed
when the owning component or view has unmounted.

Primitive emissions do not currently receive renderer-owned target geometry:
`ctx.event_target_bounds()` and `NativeEvent::target` are `()`/`None` on that
path. When a native mechanism exposes coordinates to its consumer, include a
bounded, schema-checked geometry value in its declared event payload. Passing a
genuine component-declared `ElementRef` for internal native layout work does not
add geometry to the emitted event contract.

See `extension_host.rs` for rendering and `tests/custom_primitive.rs` for the
downstream registration contract.
