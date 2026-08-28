# Embedding multiple script views

GPUI Rhai is view-first. A host may mount several independent Rhai views into
one existing GPUI window without transferring ownership of `Application`, the
window, or the surrounding Rust layout.

## Ownership model

```text
GPUI Window
└─ ScriptViewHost (one interaction and overlay domain)
   ├─ ScriptViewHandle A -> independent Engine/Runtime/Lifecycle
   ├─ ScriptViewHandle B -> independent Engine/Runtime/Lifecycle
   └─ ScriptViewHandle C -> independent Engine/Runtime/Lifecycle
```

Views never share Rhai state, app/window stores, themes, locales, tasks,
subscriptions, assets, capabilities, or diagnostics. Share business data only
through explicit host capabilities. The Host shares native interaction
mechanisms that must coordinate across components: Overlay, Tooltip, Toast,
outside dismissal, Escape routing, focus fallback, and approved key bindings.

## Mounting

Install App-level input actions once, create one Host for the window, then mount
single-use prepared views:

```rust
gpui_rhai::install(cx);
let host = gpui_rhai::ScriptViewHost::new("main-window", cx)?;

let first = gpui_rhai::FileScriptView::new("plugins/first/ui/main.rhai")
    .prepare()?
    .mount(
        gpui_rhai::ScriptViewConfig::new("first-widget"),
        host.clone(),
        window,
        cx,
    )?;
```

`PreparedScriptView::mount` consumes the prepared value. Prepare again for a
second instance, even when it uses the same source. This preserves handler and
runtime isolation.

The Rust root must wrap the complete layout containing all sibling views:

```rust
host.container(
    row()
        .child(first.element()?)
        .child(second.element()?)
        .child(third.element()?)
)
```

The container is layout-transparent. It initializes the shared Host once per
frame and installs native capture routing. Rendering a handle outside its Host
container produces a visible diagnostic.

Several Hosts may intentionally coexist in one window. Capture routing is
limited to each Host container, so their Overlay domains do not dismiss one
another. Use one shared Host whenever sibling widgets should coordinate.

## Bounds and overlays

`ScriptViewHandle::element()` automatically measures the bounds allocated by
the host layout. The measured width drives the view's own
`compact`/`regular`/`wide` class. Crossing a breakpoint schedules one additional
view render; ordinary pixel resizing remains native GPUI layout.

The Overlay viewport is separate. It defaults to the complete GPUI window, so a
Dropdown in a 200-point widget can render at its measured 280-point width and
flip/clamp against window edges. `ScriptViewHost::set_overlay_viewport` may set
an explicit absolute rectangle for an intentionally isolated domain.

Every local Overlay, Tooltip, Toast, and focus ID is internally namespaced by
`view_id`. Rhai callbacks continue to receive their original local IDs. Toasts
from every sibling use one Host queue and one per-region maximum (default 3).

Non-modal outside clicks dismiss the topmost Host overlay during native capture
and continue to the clicked sibling control. Modal backdrops consume the click.

## Identity and window commands

`ctx.window_id()` identifies the actual host window and is shared by sibling
views. `ctx.view_id()` identifies the mounted widget and is unique inside the
Host.

Embedded views reject `open_window`, `focus_window`, `close_window`, and close
handler registration immediately with `UnsupportedInEmbeddedView`. The
standalone `ScriptApplication` adapter enables the existing restricted
multi-window implementation.

## Key bindings

Mounting never modifies the App keymap. Inspect `PreparedScriptView::key_bindings`
and explicitly approve them:

```rust
host.bind_keys(prepared.key_bindings().iter().cloned(), cx)?;
```

The operation is idempotent for identical bindings and rejects conflicting
actions for the same keystrokes/context.

## Disposal

Remove a configured widget with `view.dispose(cx)?` before dropping it. Disposal
is idempotent and immediately cancels tasks/subscriptions, removes scoped state,
actions, overlays, tooltips, and toasts, and frees the `view_id` for remounting.
Dropping the final handle is a fallback. Merely omitting `view.element()` from a
temporary page does not dispose it.

See `cargo run -p gpui-rhai --example embedded_views` for three isolated views,
automatic compact sizing, escaping Dropdown placement, duplicate local IDs,
cross-view dismissal, a shared Toast queue, and explicit dispose/remount.
