# Combobox specification

Combobox is the public Rhai choice-composition component used directly for
single or multiple selection and indirectly by Select.

It combines generic `overlay`, Input, Box/Text atoms, semantic events, formal
component state, and data-backed `virtual_collection`. Its strictly controlled
props are `selected`, `open`, and `query`; callbacks emit `change`,
`open_change`, and `query_change`. Custom trigger, header, footer, and empty
slots remain ordinary nodes.

`width` accepts every public `Length`. Fixed pixels retain their exact width;
relative widths resolve from the component's real parent allocation, and the
standard instance style can add `flex_grow`/`min_width(0)` for mixed flex rows.
The popup panel follows the realized trigger width rather than reusing the
declared length, so it remains aligned after window and parent-layout changes.

The textual `label` and all three controlled props are required. `label` names
the trigger, listbox, virtual collection, and searchable Input; `placeholder`
remains an empty-value hint. Only the roving active option remains
component-local transient state. This removes dual controlled/uncontrolled
behavior before the first public release and makes Host snapshot/restoration
unambiguous.

Options have stable unique string values, non-empty labels, optional keywords,
disabled state, and optional non-empty groups. Filtering is case-insensitive
over label and keywords. Group order follows first occurrence, keyboard
navigation skips group headers and disabled options, and multiple selection
preserves stable values across filter changes.

The component has no asset requirement and no privileged Rust constructor.
Its public dependencies are only `components/input` plus runtime atoms. See
[Select](select.md) for the scalar form-field adapter and the complete style,
keyboard, accessibility, and native-boundary contract.
