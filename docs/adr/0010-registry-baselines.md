# ADR 0010: Bundled registry and committed baselines

Status: Accepted

The CLI ships a versioned offline registry snapshot. Installed editable source
lives under `ui/`; pristine upstream source and install metadata live under
`.gpui-rhai/` and are committed to version control.

Updates never silently overwrite local changes. Clean files update atomically;
modified files require an explicit merge/conflict workflow.
