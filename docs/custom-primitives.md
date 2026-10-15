# Custom Rust primitives

Use a custom primitive when a mechanism needs native GPUI APIs—for example an
editor, canvas, media surface, or domain-specific control. Composition and
policy should remain in copied Rhai components where possible.

When Rhai only needs to position an already-host-owned element or view, use
`HostSlotRegistry` instead of defining another primitive protocol. It is a thin
adapter over this same primitive boundary: the Host supplies an element factory
and retains lifecycle authority; the script receives only a keyed opaque box.

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
        effect: None,
    },
    StatusCard,
)?;
```

Typed GPU/effect extensions set `effect: Some(EffectPrimitiveDescriptor { .. })`
and must declare supported platforms, positive instance and per-instance cost
budgets, scoped lifecycle, reduced-motion support, and quality-tier support.
Registration fails on an unsupported current platform. Mounting beyond the
declared instance budget fails deterministically. `PrimitiveTheme` exposes the
effective `MotionPreference`, `MotionQuality`, and resolved `ThemeMotion`
tokens, so Host effects follow the same policy as Rhai-authored motion. This is
the 0.1.3 ShaderSurface extension seam; it does not accept arbitrary shader
source.

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

An asynchronous native commit can change a primitive's intrinsic size without
changing any script data. After committing that size, call
`context.invalidate_intrinsic_size(cx)` as well as notifying your native Entity.
The context defers and coalesces measurement invalidation for its current virtual
row and any enclosing rows; it does not run Rhai or scan the collection. Keep the
current context, not one captured by an obsolete job. Old primitive/row leases
and script generations are ignored. Hidden retained rows keep the dirty
measurement until presentation resumes, without starting a repaint timer.

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

When native content inside a virtual row changes its intrinsic size without a
Rhai state change, call `context.invalidate_intrinsic_size(cx)` once **after**
committing the new native content or width-dependent display. Use the current
`PrimitiveContext` held by the live native viewer, not a context captured at the
start of an asynchronous job. This hook does not request a script render: GPUI
remeasures only the current leased row and preserves its native scroll anchor.
Nested virtual rows propagate invalidation to their enclosing row in turn.

The hook defers/coalesces foreground work and is safe from native render,
prepaint, or Entity updates; it never synchronously reenters a parent Entity.
Primitive resource identity, script generation, stable row key and retained
row identity are checked when deferred work executes. Removed/replaced leases
are ignored, even if a new instance reuses the same local key. Outside a
virtual row (or for an ephemeral primitive) the hook is a no-op. Hidden retained
rows may become dirty, but repeated commits do not start timers or a repaint
loop; measurement resumes when they are presented. This is not a per-frame
notification API: call only after a meaningful committed size change.

Emit only declared events through `PrimitiveContext`; the runtime validates
payloads and dispatches the generation-bound callback. The emitter keeps only a
weak reference back to the registry, so an Entity whose callbacks retain the
emitter cannot form `Registry -> Entity -> Registry` ownership cycles. Use GPUI
through `gpui_rhai::gpui` so the host and runtime cannot link incompatible GPUI
type versions.

`ValueSchema::Signal` and `ValueSchema::Ref` props remain typed native handles
as `PrimitiveValue::Signal` and `PrimitiveValue::Ref`; they are not flattened
into durable `UiValue`. A foreground primitive may write a passed signal with
`PrimitiveContext::write_signal`, which requests a GPUI repaint without
executing Rhai. It may query the last committed layout rectangle of a passed
ref with `PrimitiveContext::element_bounds`. Both operations fail closed
when the owning component or view has unmounted.

Primitive emissions do not currently receive renderer-owned target geometry:
`ctx.event_target_bounds()` and `NativeEvent::target` are `()`/`None` on that
path. When a native mechanism exposes coordinates to its consumer, include a
bounded, schema-checked geometry value in its declared event payload. Passing a
genuine component-declared `ElementRef` for internal native layout work does not
add geometry to the emitted event contract.

See `extension_host.rs` for rendering and `tests/custom_primitive.rs` for the
downstream registration contract.

## Shared native scrollbars

Native surfaces do not need to copy scrollbar geometry or the private gesture
coordinator. `PrimitiveContext::scrollbars(theme)` returns a scoped
`ScrollbarContext`; `element(id, source, spec)` accepts a `ScrollHandle`,
`ListState`, `UniformListScrollHandle`, or a Host-defined `ScrollbarSource`.
Render the viewport first and the returned overlay last in a relative, clipped
container. IDs must be stable and unique within that primitive's scope.

```rust
let bars = context.scrollbars(theme).with_part_styles(
    &track_style, &thumb_style, &thumb_hover_style,
);
let overlay = bars.element("transcript-scrollbar", list_state.clone(),
    ScrollbarSpec::new("hidden", "auto")?);
```

The three part backgrounds override transparent track / `scrollbar.thumb` /
`scrollbar.thumb_hover` defaults. `SCROLLBAR_TRACK_WIDTH` exposes the logical
12px pointer-track width for optional row insets. Direction comes from the
theme snapshot; vertical bars occupy the logical end edge. Short/zero
viewports and non-scrollable axes never produce oversized thumbs.

For a non-primitive Host view use `host.scrollbars("unique-surface", &theme)`
and render under `host.container(...)`. A Host can capture its own read-only
resolver with `PrimitiveTheme::from_colors(&resolver, direction)`.
The same window coordinator arbitrates drag, Escape, disappearance, suspension,
disposal and window deactivation. `ListState`'s native measured-height freeze
starts on thumb down and ends exactly once on release/cancellation; a drop
backstop also releases it when a window goes away. The scrollbar never takes
focus or claims Home/End: keyboard behavior belongs to the viewport/component.
Wheel input over its track drives that viewport, not an enclosing scroller.

Track wheel routing consumes the actual clamped movement per axis, then sends
only residual delta through the SDK ancestor chain. Unsupported axes and fully
unconsumed boundary input bubble normally. For a raw Host-native hierarchy,
append measured parents nearest-first with
`with_scroll_parent(parent_source, Some(gpui::Axis::Vertical))` (`None` permits
both axes). `with_scroll_axis(...)` restricts the viewport's own wheel axes
independently of bar visibility. This is scoped track routing, not a replacement
for GPUI's whole-window wheel infrastructure.

GPUI's raw handles do not expose comparable native identity. If a source can
be replaced under a stable scrollbar id, retain a
`ScrollbarSourceHandle::new(source)` alongside it and pass clones to `element`.
Replacing that adapter retires an old captured gesture and ends its freeze.
Alternatively change the scrollbar id when replacing a raw source. Built-in
virtual collections retire their source epoch automatically.
