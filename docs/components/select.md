# Select specification

Select is a controlled, form-oriented, single-choice Rhai component. It is a
thin scalar adapter over the public Combobox source component; neither Select
nor Combobox has a privileged Rust node or private state machine.

## Public contract

`registry/components/select.rhai` exports `Select(props)` with:

- required `key: string`;
- `options: array<{ value, label, disabled?, group?, keywords? }>`;
- `value: optional<string>`;
- required controlled `open: bool` and `query: string`;
- `placeholder`, `search_placeholder`, `empty_text`, and `clear_label` strings;
- `clearable`, `searchable`, `disabled`, and `error` booleans;
- `size`, `placement`, `max_visible`, and optional `width: Length`;
- optional `on_change`, `on_open_change`, and `on_query_change` callbacks.

Rhai `()` is the only empty-value sentinel. Empty-string option values remain
legal and distinct from null. Option values must be unique, an externally
controlled value must name an option, and a disabled option may remain selected
but cannot be newly selected.

Groups follow first-occurrence order and preserve input order within each
group. Group headers are not selectable. Search matches case-insensitive
substrings of labels and keywords. The copied source uses data-backed,
variable-height `virtual_collection`, so filtering a large option set does not
build every option node.

## State and interaction

The caller owns `value`, `open`, and `query`. Combobox retains only the
active-option navigation transient under the stable Select key. Closing Select
requests `open_change(false)` and query reset through `query_change("")`.
Clearing emits `()`; selecting emits the scalar value and requests panel close.

Arrow keys skip disabled options; Home and End move to the edges; Enter commits;
Escape and outside click dismiss through the public Overlay mechanism. When
search is disabled, letter keys cycle by the first label character. Searchable
Select delegates editing, IME, selection, and clipboard behavior to the public
Input source component and its generic native text-editing primitive.

Style parts are `root`, `trigger`, `value`, `placeholder`, `indicator`, `clear`,
`panel`, `search`, `list`, `group`, `option`, `option_selected`,
`option_active`, `option_disabled`, and `empty`. Virtual item renderers resolve
the same validated component style snapshot through `ctx.component_style`.

## Native boundary

Rust owns only generic platform mechanisms used by any script author: Overlay
placement/dismissal, Input editing/IME, focus routing, and data-backed virtual
collection measurement. Choice validation, grouping, filtering, selection,
keyboard policy, rendering, and scalar adaptation are inspectable Rhai source.

Required evidence covers identity validation, grouping/filtering, controlled
payloads, keyboard routing, bounded realization, style propagation, CLI
dependency installation, and multi-theme interaction baselines.
