# Custom Rust primitives

Use a custom primitive when a mechanism needs native GPUI APIs—for example an
editor, canvas, media surface, or domain-specific control. Composition and
policy should remain in copied Rhai components where possible.

Register primitives in `ScriptViewExtension::configure_engine`, before scripts
compile. A `PrimitiveDescriptor` declares a namespaced ID, PascalCase Rhai
export, prop/event schemas, optional state schema, and lifecycle requirement.
The `PrimitiveHandler` receives a validated `PrimitiveInstance`, normalized
event emitter, `Window`, and `App`, then returns `AnyElement`.

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

Lifecycle primitives require a stable `key`. Emit only declared events through
`PrimitiveEventEmitter`; the runtime validates payloads and dispatches the
generation-bound callback. Use GPUI through `gpui_rhai::gpui` so the host and
runtime cannot link incompatible GPUI type versions.

See `extension_host.rs` for rendering and `tests/custom_primitive.rs` for the
downstream registration contract.
