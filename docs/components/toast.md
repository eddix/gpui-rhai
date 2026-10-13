# Toast specification

Toast is controlled Rhai source over public `layer`, declarative `timeout`,
Box/Text atoms, semantic component events, and `hover_change` routing. There is
no ToastHost node, native toast queue, or component-specific Entity.

`Toast(props)` accepts a stable `key`, up to 100 caller-owned items, a
`max_visible` value from 1 through 10 (default 3), a `dismiss_label` for the
close buttons (default "Dismiss notification"), and required `on_dismiss`. Each item has
a unique non-empty ID and title plus optional message, semantic variant,
window-corner region, 1ms–24h duration, controlled pause, and dismissibility.
Only the first `max_visible` items declare timers and render.

Each visible ID declares one component-scoped one-shot timeout. Reconciliation
preserves its deadline when the declaration signature is unchanged, restarts it
when duration/payload/callback changes, and remembers completion until the
declaration disappears. Controlled pause and hover pause are independent; both
must clear before the preserved remaining duration resumes. Manual dismissal
completes the timer before emitting `dismiss(id)`.

`layout: "stack"` piles each corner's toasts instead of listing them. Every
item joins its corner's pile and declares its timer (`max_visible` applies to
the flat layout only); the newest, the last of the corner's items, shows whole,
up to two 6px edges peek out behind it, and a count marks its top end corner.
While the pointer is over the pile or keyboard focus is inside it
(`focus_within_change`), the pile expands into one list, newest nearest the
corner, under the `heading` and the number of toasts, with a `clear_label`
button that dismisses every toast of the corner; Escape collapses it until the
pointer or focus enters again. Every timer of the corner is paused meanwhile
(the declaration's pause), and resumes when pointer and focus have left; the
pointer leaving also releases focus that a press left in the pile. All toasts
stay in one keyed list in both states, so focus on a toast or close button
survives the pile opening. The flat layout keeps the per-toast hover pause.

With `on_activate`, a toast is also a button: a click, or Enter while it has
focus, emits `activate(id)`. Close buttons and activatable toasts show the
focus frame.

The four region columns use generic Layer placement (`top_left`, `top_right`,
`bottom_left`, `bottom_right`). A Layer's identity is its declaring component
instance and key (D57), so two Toast instances in one view never share a
region; layers are registered with the shared Host portal, so embedded views
escape local bounds without sharing script state or a hidden Toast queue.

Style parts are `root`, `region` (a corner's column or pile), `toast`, the
four `toast_<variant>` parts, `lamp` (the status lamp), `title`, `message`,
`close`, and for the stack `stack_edge`, `stack_count`, `stack_header`,
`stack_clear` and `stack_list` (480px high at most before it scrolls). Neutral/success/warning/danger colors resolve
through semantic theme tokens. Toasts retain status or alert metadata and a
button label for manual dismissal.
