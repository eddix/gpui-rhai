# Actions and keybindings

Semantic actions separate application intent from a physical shortcut or a
particular control. Register app-scoped callbacks during the main window's
`init`:

```rhai
fn save(ctx, payload) { /* update state or call a capability */ }
fn init(ctx) { ctx.register_action("document.save", Fn("save")); }
```

Controls and handlers dispatch by ID with `ctx.dispatch_action(id, payload)`.
Use `ctx.set_action_enabled(id, bool)` to make availability explicit. Dispatch
is generation-bound and has a 64-callback budget to stop accidental cycles.

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
error; repeating the same binding is idempotent. GPUI first offers keyboard input
to the focused native primitive; unhandled input then reaches the action system.
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
