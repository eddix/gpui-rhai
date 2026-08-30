# Select specification

Select is a controlled, form-oriented, single-choice control. It shares a
private native listbox/search/overlay core with Dropdown but deliberately has a
smaller scalar API and a fixed Input-like trigger.

Dropdown remains the advanced choice picker for multiple selection, controlled
query/open state, custom trigger/header/footer/empty slots, and other menu-like
composition. Popover owns arbitrary overlay content; Menu owns commands.

## Public contract

`registry/components/select.rhai` exports `Select(props)` with:

- required `key: string`;
- `options: array<{ value, label, disabled?, group?, keywords? }>`;
- `value: optional<string>`;
- `placeholder`, `search_placeholder`, and `empty_text` strings;
- `clearable` and `searchable` booleans;
- `disabled`, `error`, `size`, `placement`, and `max_visible`;
- optional `width: Length`, default `px(280)`;
- optional `on_change`, whose payload is `optional<string>`.

Rhai `()` is the only empty-value sentinel. Empty-string option values remain
legal and distinct from null; all option values must be unique. An unknown
controlled value is an error. A disabled option may remain selected and visible
but cannot be newly selected.

The optional `group` string is caller-localized display text and group identity.
Groups appear in first-occurrence order and preserve input order within each
group. Group headers are not selectable. Search matches case-insensitive
substrings of label and keywords while preserving the same order.

Style parts include at least `root`, `trigger`, `value`, `placeholder`,
`indicator`, `clear`, `panel`, `search`, `list`, `group`, `option`,
`option_selected`, `option_disabled`, and `empty`.

## Controlled and transient state

The caller owns `value`. The keyed native entity owns open state, focused
option, type-ahead buffer, search query, scroll position, and focus restoration.
Select does not expose controlled `open` or `query` props. Query text clears on
selection, Escape, outside dismissal, and every close, so reopening shows the
complete list.

Clearing emits `()`. Selecting an enabled option emits its scalar value, closes
the panel, and restores focus to the trigger.

## Keyboard and accessibility

- Enter, Space, ArrowUp, or ArrowDown opens the list.
- Up/Down move between enabled options; Home/End move to edges.
- Character input performs type-ahead when search is disabled.
- Search input uses the shared native text input behavior when enabled.
- Enter selects the focused option; Escape closes without a value change.
- Tab closes and continues deterministic window traversal.

The normalized root uses combobox semantics; the panel retains listbox, option,
selected, disabled, expanded, invalid, label, and value metadata.

## Native boundary

Dropdown and Select must not copy two state machines. Rust extracts one private
choice-list core for validation, grouping, filtering, focus movement,
virtualized realization, type-ahead, and overlay coordination. Thin native
adapters preserve their distinct public payloads and visual policy. Search and
grouping ship in the first Select slice; remote asynchronous loading and
multiple selection do not.

## Required evidence

- duplicate/unknown values, groups, disabled options, and filtering tests;
- controlled optional-value and clear behavior tests;
- native mouse, keyboard, type-ahead, search, dismissal, and focus tests;
- form error/disabled/placeholder snapshots and multi-theme baselines;
- a country or region field in `form_showcase`.
