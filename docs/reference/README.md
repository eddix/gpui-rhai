# Module reference

Generated from the schemas and header comments of the official modules; do not
edit. Regenerate with
`GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli reference`.

The functions, methods and native primitives a script calls are in the
[script API](script-api.md).

## Components

| Module | Export | Purpose |
|---|---|---|
| [`components/accordion`](components/accordion.md) | `Accordion` | Controlled single/multiple accordion with Rust-side clip motion. |
| [`components/alert`](components/alert.md) | `Alert` | Persistent inline feedback with semantic variants and composable actions. |
| [`components/alert_dialog`](components/alert_dialog.md) | `AlertDialog` | Controlled confirmation dialog with explicit cancel and confirm semantics. |
| [`components/avatar`](components/avatar.md) | `Avatar` | Avatar displays an image handle or name-derived initials with presence. |
| [`components/badge`](components/badge.md) | `Badge` | Badge marks a read-only status: a square lamp followed by its label. |
| [`components/button`](components/button.md) | `Button` | Button presents a desktop action: a large block with a centered label. |
| [`components/button_group`](components/button_group.md) | `ButtonGroup` | Visually joins related buttons into one horizontal or vertical control. |
| [`components/card`](components/card.md) | `Card` | General themed content surface with optional header and footer slots. |
| [`components/checkbox`](components/checkbox.md) | `Checkbox` | Controlled checkbox with checked, unchecked, and indeterminate states. |
| [`components/code_viewer`](components/code_viewer.md) | `CodeViewer` | Native, virtualized, selectable read-only source document surface. |
| [`components/collapsible`](components/collapsible.md) | `Collapsible` | Controlled disclosure with trigger/content slots and Rust clip animation. |
| [`components/combobox`](components/combobox.md) | `Combobox` | Public Rhai choice composition over Overlay, Input, and virtual_collection. |
| [`components/command`](components/command.md) | `Command` | Embeddable keyboard-first command search with deterministic fuzzy ranking. |
| [`components/command_dialog`](components/command_dialog.md) | `CommandDialog` | Controlled modal presentation of Command without registering a global shortcut. |
| [`components/context_menu`](components/context_menu.md) | `ContextMenu` | Controlled pointer-anchored action menu sharing Menu items and keyboard behavior. |
| [`components/date_picker`](components/date_picker.md) | `DatePicker` | Controlled single-date field composed from public date data helpers, atoms, and Overlay. |
| [`components/dialog`](components/dialog.md) | `Dialog` | Dialog presents controlled modal content through the native overlay layer. |
| [`components/diff_viewer`](components/diff_viewer.md) | `DiffViewer` | Neutral two-way, read-only source comparison over the native document surface. |
| [`components/divider`](components/divider.md) | `Divider` | Divider separates adjacent content without owning layout spacing. |
| [`components/drag_source`](components/drag_source.md) | `DragSource` | Typed in-application data drag source. |
| [`components/draggable`](components/draggable.md) | `Draggable` | Controlled in-window positioning. |
| [`components/drop_zone`](components/drop_zone.md) | `DropZone` | Typed same-Host application drop target. |
| [`components/empty`](components/empty.md) | `Empty` | Consistent empty-state composition with optional icon, content, and actions. |
| [`components/form_field`](components/form_field.md) | `FormField` | Associates label, control, description, required state, and error content. |
| [`components/group_box`](components/group_box.md) | `GroupBox` | Labelled group of related controls or settings: a top hairline and a label-voice heading on the parent's content edge, with no surface of its own. |
| [`components/icon`](components/icon.md) | `Icon` | Icon wraps a declarative AssetId or a runtime image handle with consistent sizing. |
| [`components/icon_button`](components/icon_button.md) | `IconButton` | IconButton presents one icon in a square action target. |
| [`components/input`](components/input.md) | `Input` | Input is a controlled single-line native text input. |
| [`components/input_group`](components/input_group.md) | `InputGroup` | Composes an input-like control with integrated prefix and suffix content. |
| [`components/kbd`](components/kbd.md) | `Kbd` | Kbd presents a key legend: a framed keycap, or an inline legend inside menus and buttons. |
| [`components/label`](components/label.md) | `Label` | Label presents a control label with optional required and description text. |
| [`components/list`](components/list.md) | `List` | Controlled virtualized list of keyed rows: the rows Table draws, without columns. |
| [`components/menu`](components/menu.md) | `Menu` | Controlled in-window menu with roving selection and nested submenu nodes. |
| [`components/pagination`](components/pagination.md) | `Pagination` | Pagination is a stateless Rhai composition for one-based controlled page state. |
| [`components/pan_zoom`](components/pan_zoom.md) | `PanZoom` | Controlled Canvas viewport transform with native pan and pointer-anchored zoom. |
| [`components/popover`](components/popover.md) | `Popover` | Popover renders controlled content through the native per-window overlay layer. |
| [`components/progress`](components/progress.md) | `Progress` | Determinate or indeterminate progress indicator animated entirely in Rust. |
| [`components/radio`](components/radio.md) | `Radio` | Controlled radio option. |
| [`components/radio_group`](components/radio_group.md) | `RadioGroup` | Controlled radio group with one tab stop and roving arrow-key selection. |
| [`components/range_slider`](components/range_slider.md) | `RangeSlider` | Controlled two-thumb range selection with native pointer preview and keyboard thumbs. |
| [`components/resizable`](components/resizable.md) | `Resizable` | Controlled single-element rectangle resizing inside one local boundary. |
| [`components/rotatable`](components/rotatable.md) | `Rotatable` | Controlled Canvas rotation around an explicit local pivot. |
| [`components/scroll_area`](components/scroll_area.md) | `ScrollArea` | Sized retained scroll viewport with themeable overlay scrollbars. |
| [`components/select`](components/select.md) | `Select` | Scalar controlled Select composed from the public Rhai Combobox component. |
| [`components/selection_area`](components/selection_area.md) | `SelectionArea` | Controlled Canvas click/range/marquee object selection. |
| [`components/sheet`](components/sheet.md) | `Sheet` | Controlled temporary modal panel attached to a viewport edge. |
| [`components/skeleton`](components/skeleton.md) | `Skeleton` | Content placeholder with optional Rust-side pulse animation. |
| [`components/slider`](components/slider.md) | `Slider` | Controlled single-value range input with native drag preview and commit events. |
| [`components/sortable`](components/sortable.md) | `Sortable` | Controlled keyed ordering with native pointer targets and keyboard moves. |
| [`components/spinner`](components/spinner.md) | `Spinner` | Compact indeterminate activity indicator animated by the native runtime clock. |
| [`components/split_pane`](components/split_pane.md) | `SplitPane` | Controlled, nestable two-panel split layout with a native drag hot lane. |
| [`components/status_bar`](components/status_bar.md) | `StatusBar` | Compact application footer with stable logical start, center, and end regions. |
| [`components/switch`](components/switch.md) | `Switch` | Controlled binary switch with disabled and loading states. |
| [`components/tab_bar`](components/tab_bar.md) | `TabBar` | Controlled document tabs that belong to the panel under them: the selected tab takes the panel's surface (`tabbar.active`) and covers the bar's bottom line, so the panel's extent says what the tabs switch. |
| [`components/table`](components/table.md) | `Table` | Data-backed Table composed entirely from public Box/Text/virtual_collection APIs. |
| [`components/tabs`](components/tabs.md) | `Tabs` | Controlled tabs with one tab stop and orientation-aware arrow selection. |
| [`components/tag`](components/tag.md) | `Tag` | Tag labels a category: a flat tonal tape with an optional facet segment. |
| [`components/textarea`](components/textarea.md) | `Textarea` | Textarea is a controlled multiline field with native IME, selection, wrapping, and auto-grow. |
| [`components/title_bar`](components/title_bar.md) | `TitleBar` | Source-owned application chrome with logical start, center, and end regions. |
| [`components/toast`](components/toast.md) | `Toast` | Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events. |
| [`components/toggle`](components/toggle.md) | `Toggle` | Controlled pressed-state tool button, distinct from Checkbox and Switch. |
| [`components/toggle_group`](components/toggle_group.md) | `ToggleGroup` | ToggleGroup holds pressed tool state: equal-height segments joined inside one frame. |
| [`components/tooltip`](components/tooltip.md) | `Tooltip` | Native delayed tooltip using the per-window overlay scheduler. |
| [`components/tree`](components/tree.md) | `Tree` | Controlled virtualized hierarchical outline over the shared Rust projection. |

## Layouts

| Module | Export | Purpose |
|---|---|---|
| [`layouts/inline`](layouts/inline.md) | `Inline` | Inline places children in a row with one spacing relationship and one control size. |
| [`layouts/region`](layouts/region.md) | `Region` | Region is one working area: a header with the region's title and actions, an optional toolbar, a body that fills the remaining height, and a footer pinned to the bottom. |
| [`layouts/stack`](layouts/stack.md) | `Stack` | Stack places children in a column with one spacing relationship between them. |
| [`layouts/toolbar`](layouts/toolbar.md) | `Toolbar` | Toolbar arranges a region's controls by role: context and filters at the start, secondary actions and the one primary action at the end. |

## Patterns

| Module | Export | Purpose |
|---|---|---|
| [`patterns/app_shell`](patterns/app_shell.md) | `AppShell` | AppShell is the window frame of a productivity tool: title bar, sidebar, main area, optional inspector, status bar, and keyboard regions. |
| [`patterns/data_view`](patterns/data_view.md) | `DataView` | DataView is a browsable collection in one region: toolbar, table or list, and a footer with status, selection, count and data time. |
| [`patterns/description_list`](patterns/description_list.md) | `DescriptionList` | DescriptionList shows facts as label and value pairs on one label column. |
| [`patterns/form_layout`](patterns/form_layout.md) | `FormLayout` | FormLayout aligns labels on one column, groups fields, and lines the submit row up with the fields. |
| [`patterns/inline_state`](patterns/inline_state.md) | `InlineState` | InlineState shows loading, empty, error, stale and refreshing feedback where the content would appear. |
| [`patterns/list_detail`](patterns/list_detail.md) | `ListDetail` | ListDetail shows a list and the selected item's detail, side by side or stacked. |
| [`patterns/section`](patterns/section.md) | `Section` | Section is a titled part of a region: a subtitle, an optional description and actions on the end side, then the content. |
| [`patterns/stat`](patterns/stat.md) | `Stat` | Stat shows one figure: a label-voice label, the value with its unit, and an optional change against a reference. |

## Motion

| Module | Export | Purpose |
|---|---|---|
| [`motion/animated_tabs`](motion/animated_tabs.md) | `AnimatedTabs` | AnimatedTabs is a controlled `Tabs` whose selection indicator slides to the selected tab. |
| [`motion/border_beam`](motion/border_beam.md) | `BorderBeam` | BorderBeam traces an accent stroke around a rectangle in an endless loop. |
| [`motion/marquee`](motion/marquee.md) | `Marquee` | Marquee scrolls its content horizontally in a loop. |
| [`motion/number_ticker`](motion/number_ticker.md) | `NumberTicker` | NumberTicker shows an integer and fades each new value in. |
| [`motion/orbit`](motion/orbit.md) | `Orbit` | Orbit spins a satellite dot around a center dot in an endless loop. |
| [`motion/particles`](motion/particles.md) | `Particles` | Bounded deterministic particle field. |
| [`motion/reorder_list`](motion/reorder_list.md) | `ReorderList` | ReorderList slides its rows to their new places when the caller reorders `items`. |
| [`motion/shared_layout_cards`](motion/shared_layout_cards.md) | `SharedLayoutCards` | SharedLayoutCards moves the selected card from the list to a detail area with a shared-layout animation. |
| [`motion/shimmer`](motion/shimmer.md) | `Shimmer` | Shimmer is a loading placeholder that a beam sweeps across in an endless loop. |
| [`motion/text_reveal`](motion/text_reveal.md) | `TextReveal` | Grapheme-safe native text reveal. |

## Charts

| Module | Export | Purpose |
|---|---|---|
| [`charts/bar_chart`](charts/bar_chart.md) | `BarChart` | Source-owned single-series Bar adapter over Chart. |
| [`charts/chart`](charts/chart.md) | `Chart` | Native composable visualization surface. |
| [`charts/line_chart`](charts/line_chart.md) | `LineChart` | Source-owned single-series Line adapter over Chart. |
| [`charts/map_chart`](charts/map_chart.md) | `MapChart` | Source-owned single-series choropleth adapter over Chart. |
| [`charts/pie_chart`](charts/pie_chart.md) | `PieChart` | Source-owned single-series Pie adapter over Chart. |
