# Actions and keybindings

Semantic actions separate application intent from a physical shortcut or a
particular control. Register app-scoped callbacks during the main window's
`init`:

```rhai
fn save(ctx, payload) { /* update state or call a capability */ }
fn init(ctx) { ctx.register_action("document.save", Fn("save")); }
```

Controls and handlers dispatch by ID with `ctx.dispatch_action(id, payload)`.
Use `ctx.set_action_enabled(id, bool)` to make availability explicit. A first
registration is enabled; registering an ID again (as `init` does on hot reload)
replaces the callback and keeps the enabled state. Dispatch is generation-bound
and has a 64-callback budget to stop accidental cycles.

The Rust host maps physical shortcuts through validated `KeyBindingSpec`:

```rust
let binding = KeyBindingSpec::new(
    "cmd-s",
    ActionId::parse("document.save")?,
    Some("GPUIRhaiHost".to_owned()),
)?;
let prepared = EmbeddedScriptView::new(entry, scripts, theme)
    .key_binding(binding)
    .prepare()?;
let host = ScriptViewHost::new("main", cx)?;
host.bind_keys(prepared.key_bindings().iter().cloned(), cx)?;
```

The same declaration is available on `FileScriptView`, but mounting a view never
changes the App keymap automatically. The host must explicitly approve and bind
the declarations. Binding the same keys/context to different actions is an
error; repeating the same binding is idempotent. In the pinned gpui-pre 0.3.7,
matching Host keybindings may dispatch and consume an action before raw node
Capture/Target/Bubble handlers run. A consumed chord does not also reach node
capture. Character-preferred text input can bypass bindings; this is not a
universal "native input first, actions second" ordering. Leave exact chords to
the Host and use node handlers for unconsumed single-key events.
Register an app action once (normally only when `ctx.window_id() == "main"`) so
opening another script window does not redefine policy accidentally.

CodeViewer and DiffViewer install only focus-scoped GPUI actions. `Cmd+F`,
`Cmd+G`, `Shift+Cmd+G`, `Escape`, `Enter`, and copy apply inside the focused
document surface. Diff additionally binds `Option+Up`/`Option+Down` for hunks,
`Shift+Cmd+E`/`Shift+Cmd+C` for context folds, and `Option+Cmd+C` for the
explicit left-to-right unified patch. None is global. A Rust Host can bind or
dispatch `gpui_rhai::RevealDocumentLine { side, line }`; CodeViewer requires
`side: None`, while DiffViewer accepts `Some(DiffSide::Left)` or
`Some(DiffSide::Right)`.

## Node key handlers

Beside host-bound actions, an element can bind single keys directly and
receive them while it (or a descendant) holds focus:

```rhai
canvas(scene)
    .on_key_value("escape", Fn("close"), ())
    .on_key_value("?", Fn("help"), ())
    .on("key:-", Fn("zoom_out"))
```

All Rhai entries (`on_key_value`, `on`, `on_capture`, `on_bubble`) use one
key-name grammar and lowercase ASCII letters. Thus `on("key:Escape", ...)`
and `on_key_value("Escape", ...)` both register `key:escape`. The generic
namespace prefix must be exactly `key:`; ordinary event namespaces retain
their existing lowercase snake-case rules.

The key segment is either 1–60 ASCII letters, digits or underscores, or **one**
ASCII punctuation character other than `:`. Examples include `escape`, `left`,
`enter`, `?`, `/`, `[`, `]`, `-`, and `=`. The `key:` prefix and segment together
are at most 64 bytes. Empty names, surrounding/internal whitespace, control
characters, non-ASCII names, colon, chords (`cmd-s`, `shift-/`) and multiple
punctuation characters (`??`) are rejected during real script evaluation;
compilation alone does not validate dynamically called Rhai methods.

Matching uses GPUI's platform-normalized `Keystroke.key`, **not** `key_char`.
For example, a normal shifted slash may arrive as `key="?"`; an Alt/dead-key
layout may instead report `key="q", key_char="?"`, which does not match
`key:?`. This is a raw-key API, not a layout-independent typed-character API.
Modifiers are ignored when matching a node key handler, rather than required
to be absent: `key:/` can also match a modified slash if no earlier focus-path
handler consumes it. Use Host actions/keybindings for exact modifier chords
or GPUI's character-aware keybinding matching. `on_key_value` stores the
event's declared payload; a generic handler alone defaults to null. Handlers
sharing the same normalized event name share that payload. It is not a complete
native keyboard event with modifier fields.

Without an explicit handler for the respective key, `enter` and `space` fall
back to the node's `click` handler. An explicit `key:enter`/`key:space` replaces
that fallback; it does not also click. Disabled controls are not normal tab
targets and have no click fallback. A sibling control's node shortcuts do not
receive input focused inside a native text field, including committed IME text.
An ancestor handler is still on that focus path and can consume a matching key:
this API does not automatically suppress ancestor shortcuts while typing.
Prefer narrowly scoped node handlers or focus-context Host actions; node
registration does not install global shortcuts or alter native composition
policy.

Whitespace is not trimmed by any entry point. This intentionally tightens the
old `on_key_value(" escape ", ...)` sugar: use `"escape"` instead. A space key
is named `"space"`, not a literal `" "`.

This extension applies to Rhai `Fn` node handlers. It does not change
`NativeHandlerDescriptor`'s snake-case event protocol; do not infer that a
Rust `native_handler(...)` descriptor now accepts names such as `key:?`.

Node key phases are wired to the real GPUI focus path: `on_capture("key:...", ...)`
runs outer-to-inner during capture; normal `on`/`on_key_value` and `on_bubble`
run inner-to-outer during native bubbling, with target handlers before bubble
handlers on each node. Returning `event_response().stop()` from capture
prevents the later target/bubble route. This makes previously inert key-phase
registrations functional; non-key event routing is unchanged. The Host's
active-interaction Escape interceptor still precedes script capture and owns
gesture cancellation. After that owner releases Escape, ordinary script
handlers may receive it again.
