# Toast specification

Toast is controlled Rhai source over public `layer`, declarative `timeout`,
Box/Text atoms, semantic component events, and `hover_change` routing. There is
no ToastHost node, native toast queue, or component-specific Entity.

`Toast(props)` accepts a stable `key`, up to 100 caller-owned items, a
`max_visible` value from 1 through 10, and required `on_dismiss`. Each item has
a unique non-empty ID and title plus optional message, semantic variant,
window-corner region, 1ms–24h duration, controlled pause, and dismissibility.
Only the first `max_visible` items declare timers and render.

Each visible ID declares one component-scoped one-shot timeout. Reconciliation
preserves its deadline when the declaration signature is unchanged, restarts it
when duration/payload/callback changes, and remembers completion until the
declaration disappears. Controlled pause and hover pause are independent; both
must clear before the preserved remaining duration resumes. Manual dismissal
completes the timer before emitting `dismiss(id)`.

The four region columns use generic Layer placement (`top_left`, `top_right`,
`bottom_left`, `bottom_right`). Layer IDs are namespaced by mounted `view_id` and
registered with the shared Host portal, so embedded views escape local bounds
without sharing script state or a hidden Toast queue.

Style parts are `root`, `region`, `toast`, the four `toast_<variant>` parts,
`title`, `message`, and `close`. Neutral/success/warning/danger colors resolve
through semantic theme tokens. Toasts retain status or alert metadata and a
button label for manual dismissal.
