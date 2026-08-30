# Table specification

Table is a controlled, read-only, data-driven table with fixed-height native
row virtualization. It is not a spreadsheet or a two-dimensional virtualized
editor. Rhai owns data and policy; Rust owns column measurement, scroll/focus,
visible-row realization, and normalized interaction.

## Public contract

`registry/components/table.rhai` exports `Table(props)` with:

- required `key: string`;
- required localized `label: string` for table semantics;
- required `rows: array<map<UiValue>>`;
- required `row_key: string`, naming a unique stable string field in every row;
- required bounded `height: Length`;
- required `columns` as specified below;
- `size: "xs" | "sm" | "md" | "lg"` and `striped: bool`;
- `loading: bool` plus optional `loading_content` and `empty` node slots;
- `selection_mode: "none" | "single" | "multiple"`;
- controlled `selected_keys: array<string>`, default `[]`;
- controlled `sort: optional<{ key, direction }>`;
- optional `on_sort_change`, `on_selection_change`, and `on_row_click`.

Missing, empty, or duplicate row identities are construction errors. Array
indices are never identity fallbacks. Row maps may contain any value accepted by
`UiValue`, including nested maps and arrays. A default cell accepts only
null/bool/number/string; a complex value requires a custom renderer.

### Column definitions

Every column declares:

- `key: string`, the row field;
- `title: string`, already localized by the caller;
- `width`, one of:
  - `#{ kind: "fixed", value: positive_number }`;
  - `#{ kind: "percent", value: fraction_between_zero_and_one }`;
  - `#{ kind: "flex", value: positive_weight }`;
- optional `align: "start" | "center" | "end"`;
- optional `format`;
- `sortable: bool`;
- optional `cell_renderer: callback`.

Built-in format descriptors are:

```rhai
#{ kind: "text" }
#{ kind: "number", min_fraction_digits: 0, max_fraction_digits: 2, grouping: true }
#{ kind: "date", style: "short" }
```

Text defaults to logical start alignment; numbers and dates default to logical
end. `start`/`end` follow RTL. Date cells require strict ISO values. Number and
date formatting use the common locale service.

A custom renderer receives one extensible map and returns a `UiNode`:

```rhai
fn render_cell(cell) {
    // cell.row, cell.value, cell.row_key, cell.row_index, cell.column_key
}
```

Custom renderer callbacks execute during the complete Rhai view transaction for
all rows in custom columns. GPUI still instantiates only visible row elements.
Ordinary scalar columns do not build per-cell Rhai nodes.
The product of custom columns and rows is capped at 10,000 eager nodes; split
larger custom presentations into controlled pages rather than hiding their cost.

## Layout and virtualization

- Size selects one fixed row height and padding scale. Cells are single-line and
  truncate; custom cells must fit the same height.
- The header remains outside the vertical `uniform_list`, so it is sticky.
- Fixed widths are exact. Percent widths use the visible table width. Flex
  tracks divide remaining space by weight and retain the component source's
  shared minimum width when fixed/percent tracks already overflow.
- When fixed/percent/minimum content exceeds the viewport, header and body share
  one horizontal scroll offset. Declared widths are not silently squeezed.
- An overflowing Table reserves space inside its declared height for a visible,
  draggable horizontal scrollbar. Its track and thumb are styled through the
  `horizontal_scrollbar` and `horizontal_scrollbar_thumb` parts; a fitting Table
  renders neither.
- Ordinary vertical wheel input remains on the vertical list. Only a real
  horizontal delta (including the platform's Shift+wheel mapping) changes the
  horizontal offset, so nested vertical and horizontal scroll containers do not
  consume the same single-axis gesture.
- Vertical realization is limited to the viewport plus overscan. The Table
  implementation reuses the established `VirtualListSpec` range and identity
  logic but consumes row data directly instead of prebuilt item nodes.

`loading == true` takes precedence over rows. The default loading slot renders
Skeleton rows sized to the viewport. When not loading and rows are empty, the
default empty state uses a locale message. Both states are replaceable slots.

Striped rows resolve semantic theme colors or pseudo-state parts; Rust contains
no palette literals or component geometry constants.

## Sorting, selection, and events

Table never mutates or reorders rows. A sortable header cycles
`none -> ascending -> descending -> none` and emits the complete optional sort
descriptor. The caller supplies newly sorted rows.

Selection is controlled:

- single mode selects a row with click or Enter;
- multiple mode toggles only through the selection column or Space;
- ordinary multi-select row clicks focus the row and may emit `on_row_click` but
  do not change selection;
- the header checkbox represents all rows currently passed to Table, never
  undisclosed rows on other server pages;
- `on_selection_change` emits the complete next array of stable row keys;
- emitted selected keys follow the current controlled row order;
- `on_row_click` emits only the row key, not the row map.

The selection indicator is native Table behavior styled through
`selection_cell` and `selection_indicator` parts. Table does not instantiate a
Rhai Checkbox for every row.

## Keyboard and accessibility

Table uses a read-only row-navigation model, not spreadsheet cell focus:

- Tab reaches sortable headers, the select-all control, the body, and visible
  interactive custom cells in deterministic order;
- Up/Down/Home/End/PageUp/PageDown move the active row and scroll it into view;
- Enter emits row click; Space follows the selection-mode rules;
- custom child controls retain their own normal keyboard behavior.

The normalized tree retains table, row, column-header, cell, sort, selected,
disabled, row-count, and column-count semantics subject to pinned GPUI's public
AccessKit limitations.

## Composition and exclusions

Table never owns pagination, filtering, or data fetching. It renders exactly the
controlled rows supplied by the caller. Pagination is a separate component.

The first contract excludes variable-height rows, wrapped cells, resizable or
frozen columns, grouped headers, inline editing, expandable rows, nested
tables, and spreadsheet-style two-dimensional cell navigation.

## Required evidence

- width allocation, row identity, formatting, sort cycling, and selection tests;
- bounded realization for 10,000 scalar rows, the runtime's current array cap;
- explicit measurement of custom-renderer Rhai node cost;
- synchronized horizontal header/body, axis-restricted wheel input, draggable
  overflow scrollbar, and sticky-header native tests;
- keyboard/focus and semantic assertions;
- multi-theme/loading/empty/striped/RTL visual baselines;
- a `data_table` example with sorting, selection, custom cells, and Pagination.
