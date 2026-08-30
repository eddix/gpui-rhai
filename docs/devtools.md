# Development inspector

Development hosts expose an in-window inspector with `Command-Option-I` or
`F12`. `FileScriptView` enables development mode in debug builds when the
`dev-reload` feature is active; embedded hosts opt in with
`EmbeddedScriptView::development(true)`. `ScriptApplication` installs the
shortcut for its focused view. Existing GPUI hosts may control one isolated
view through `ScriptViewHandle::set_inspector_open`; mounting never adds a
global shortcut automatically.

The inspector reports:

- the live `UiNode` tree, stable keys, handlers, attributes, props summaries,
  typed computed style, and Rhai source line/column;
- exported component prop schemas, slots, parts, and sensitivity markers;
- component and store state, with schema-sensitive fields redacted;
- the selected theme and semantic color tokens;
- pending invalidations, bounded runtime traces, and recent script timings.

Every successful full, incremental, virtual-realization, or hot-reload commit
adds one `Reconcile` trace with mounted/preserved/moved/unmounted counts and a
bounded sample of stable `NodeId` values. Failed candidates emit no reconcile
trace, so the trace stream describes only committed retained mutations.

Opaque handles never reveal their numeric identity, and trace producers mark
capability inputs and asynchronous work as sensitive by default. The overlay is
owned by Rust and is not part of the Rhai node tree.

Handler entries distinguish `script:<name>` from `host:<label>`. Inspector
snapshots never retain or display a Host closure address or captured value, and
Host callback executions do not enter the Script runtime trace automatically.
