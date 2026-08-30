# Dropdown specification

Dropdown is the public Rhai choice-composition component used directly for
single or multiple selection and indirectly by Select.

It combines generic `overlay`, Input, Box/Text atoms, semantic events, formal
component state, and data-backed `virtual_collection`. Its controlled-or-local
props are `selected`, `open`, and `query`; callbacks emit `change`,
`open_change`, and `query_change`. Custom trigger, header, footer, and empty
slots remain ordinary nodes.

Options have stable unique string values, non-empty labels, optional keywords,
disabled state, and optional non-empty groups. Filtering is case-insensitive
over label and keywords. Group order follows first occurrence, keyboard
navigation skips group headers and disabled options, and multiple selection
preserves stable values across filter changes.

The component has no asset requirement and no privileged Rust constructor.
Its public dependencies are only `components/input` plus runtime atoms. See
[Select](select.md) for the scalar form-field adapter and the complete style,
keyboard, accessibility, and native-boundary contract.
