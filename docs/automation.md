# Retained automation

`ScriptViewHandle` exposes a production-runtime automation surface. Locators
resolve against the retained accessibility tree by semantic ID, test ID, exact
role/name, or exact visible semantic text. A locator must match exactly one
node; missing and ambiguous matches are errors.

Rhai assigns a non-semantic automation identity with `node.test_id("save")`.
Test IDs are bounded safe identifiers and are separate from
`accessibility_id`, labels, and roles.

```rust
let result = view.automate(
    gpui_rhai::AutomationCommand::Dispatch {
        locator: gpui_rhai::AutomationLocator::TestId {
            id: "save".into(),
        },
        event: "click".into(),
        payload: None,
    },
    window,
    cx,
)?;
```

`Snapshot` and `Query` return stable `NodeId`, semantic state, relationships,
and last committed visual bounds. `Dispatch` follows retained
capture/target/bubble order and invokes the same Script, Host, or
`NativeHandlerRef` callbacks used by mounted nodes. `Action` uses the mounted
semantic action registry. These commands do not fabricate GPUI platform input
or browser-style default behavior; platform keyboard, IME, pointer, and focus
certification continues through GPUI test support and unlocked macOS tests.

`AdvanceTime` advances an injected controllable `RuntimeClock`, then polls the
mounted view so timers and animation observe one deterministic timeline. It
fails against the production system clock.

## JSON-lines protocol

`run_automation_json_lines` is an opt-in synchronous codec for Hosts that want
a language-neutral stdin/socket bridge. The Host owns transport, authorization,
threading, and foreground dispatch. Each line contains a correlation `id` plus
one tagged command and produces one response line:

```json
{"id":"q1","command":"query","locator":{"kind":"role_name","role":"button","name":"Save"}}
{"id":"t1","command":"advance_time","millis":16}
```

Malformed JSON produces `ok:false` with a null correlation ID; command failures
preserve the request ID. The codec performs no I/O beyond the supplied
`BufRead`/`Write` pair and never opens a network or process capability.

GPU screenshot capture remains a separate platform gate because it requires a
painted window and renderer readback rather than a retained semantic snapshot.
