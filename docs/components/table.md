# Table

The official Table is a copied Rhai source component. It has no native Table
node, renderer, Entity, private scrollbar, or asset privilege.

Its header and cells are public Box/Text atoms. Root render normalizes row maps
into UiValue data only; `virtual_collection` invokes `render_table_row` for the
estimated/visible window outside GPUI layout and paint. Realized rows use stable
row keys and component semantic events for sort, selection, and row-click.

The current contract requires exactly one of a positive logical-pixel `height`
or `fill_height: true`; fill mode participates in a parent flex layout and uses
the measured native viewport for virtualization. `estimated_row_height` remains
optional. Cells and headers are single-line, clipped, and ellipsized so fixed
row heights cannot overpaint adjacent rows. Column widths remain tagged
fixed/percent/flex data. The old eager `cell_renderer`, native locale formatter maps, selection
geometry, private horizontal scrollbar configuration, and bundled sort/check
assets were removed. Applications that require richer cells should supply
domain data formatted before the Table boundary or compose a specialized
source component over `virtual_collection`.

Table currently exposes source parts for root/header/header_cell/body/loading/
empty. Lazy row/cell part overrides require the future structural item-context
API and are intentionally not faked through UiValue.
