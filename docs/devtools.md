# Development inspector

Development hosts expose an in-window inspector with `Command-Option-I` or
`F12`. `ScriptApp` enables development mode in debug builds when the
`dev-reload` feature is active; embedded hosts opt in with
`EmbeddedScriptApp::development(true)`.

The inspector reports:

- the live `UiNode` tree, stable keys, handlers, attributes, props summaries,
  typed computed style, and Rhai source line/column;
- exported component prop schemas, slots, parts, and sensitivity markers;
- component and store state, with schema-sensitive fields redacted;
- the selected theme and semantic color tokens;
- pending invalidations, bounded runtime traces, and recent script timings.

Opaque handles never reveal their numeric identity, and trace producers mark
capability inputs and asynchronous work as sensitive by default. The overlay is
owned by Rust and is not part of the Rhai node tree.
