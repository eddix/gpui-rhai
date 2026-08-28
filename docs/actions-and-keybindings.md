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
EmbeddedScriptApp::new(entry, scripts, theme).key_binding(binding);
```

The same builder is available on `ScriptApp`. GPUI first offers keyboard input
to the focused native primitive; unhandled input then reaches the action system.
Register an app action once (normally only when `ctx.window_id() == "main"`) so
opening another script window does not redefine policy accidentally.
