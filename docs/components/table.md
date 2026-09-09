# Table

The official Table is a copied Rhai source component. It has no native Table
node, renderer, Entity, private scrollbar, or asset privilege.

Its header and cells are public Box/Text atoms. `rows` accepts either the
original Rhai Array of maps or a Rust-owned `NativeCollection`. The Array path
normalizes all row maps in Rhai. The native path keeps the complete keyed source
in Rust, caches controlled scalar-field sort/group orders, and projects
selection, striping and cell payloads only for the GPUI viewport. Both paths
feed the same public `virtual_collection` and `render_table_row` Rhai function,
retain stable row keys, and emit the same semantic events.

## Row grouping

Set `group_by` to a declared column key whose row values are non-empty strings:

```rhai
table::Table(#{
    // ordinary Table props...
    group_by: "track",
    collapsed_groups: ctx.get_state("collapsed_groups"),
    sticky_group_headers: true,
    on_group_toggle: Fn("toggle_group"),
})
```

The flattened virtual sequence contains one synthetic header followed by that
group's visible data rows. Headers show the value and total row count, have
stable collision-free internal keys, count toward virtual length/budgets, and
never participate in row selection or `row_click`. `collapsed_groups` is fully
controlled; `group_toggle` emits the original group string and the caller
returns the next array through props. Stale collapsed keys are harmless when
data changes.

Array input preserves caller row order, so group order is first appearance and
row order within each group is the incoming order. NativeCollection first
applies the controlled scalar sort, then partitions that order: rows remain
sorted within each group and group order follows the first sorted member. The
structural order cache excludes selection state, so selecting a row does not
repeat the O(n) grouping pass.

`sticky_group_headers` defaults to true. The generic `virtual_collection`
runtime—not Table—keeps the active section header realized, substitutes a
same-height natural-position placeholder, paints exactly one retained header at
the viewport top, and pushes it away as the next section arrives. Setting the
prop false keeps ordinary scrolling group rows.

The current contract requires exactly one of a positive logical-pixel `height`
or `fill_height: true`; fill mode participates in a parent flex layout and uses
the measured native viewport for virtualization. `estimated_row_height` remains
optional. Cells and headers are single-line, clipped, and ellipsized so fixed
row heights cannot overpaint adjacent rows. Column widths remain tagged
fixed/percent/flex data: fixed values are logical pixels, percent values are a
percentage of the row width, and flex values are positive weights that divide
the space remaining after fixed and percentage columns. A flex value is never
interpreted as pixels. Fixed and percent columns keep their declared width and
horizontal overflow remains available when the complete contract is wider than
the viewport. Header cells and Array/NativeCollection body cells call the same
width function. The old eager `cell_renderer`, native locale formatter maps,
selection geometry, private horizontal scrollbar configuration, and bundled
sort/check assets were removed. Applications that require richer cells should
supply domain data formatted before the Table boundary or compose a specialized
source component over `virtual_collection`.

Table exposes source parts for `root`, `header`, `header_cell`, `body`,
`group_header`, `group_indicator`, `group_label`, `group_count`, `loading`, and
`empty`. Lazy ordinary row/cell part overrides require the future structural
item-context API and are intentionally not faked through UiValue.

See [Rust-owned collections](../native-collections.md) for host registration,
live replacement, dependency tracking, and the exact Rust/Rhai ownership
boundary.
