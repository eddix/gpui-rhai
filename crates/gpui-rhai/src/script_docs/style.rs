//! Script API docs: Style methods. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
    ScriptFnDoc {
        signature: "overflow_row(_: &mut Style) -> Style",
        params: &[],
        doc: "Uses a centered, nonwrapping intrinsic row for Box children. Cells keep their measured native widths/heights, not flex grow/shrink distribution. Insufficient width omits lower overflow_priority cohorts from native layout, paint, keyboard and presented semantics; required cells stay. Equal priorities collapse together. Reserves the full intrinsic line height and preserves logical ownership/state/effects; this is not a retained-page lifecycle pause. Decisions happen before paint without a Rhai resize loop. Grid, wrapping, baseline or stretching layouts override the mode. Fragment styles do not create a cell: wrap a group in row/box to give it one priority.",
    },
    ScriptFnDoc {
        signature: "overflow_row(_: &mut Style, _: bool) -> Style",
        params: &["enabled"],
        doc: "Enables overflow_row, or explicitly clears a lower-layer overflow layout with false. Disabling leaves ordinary layout declarations unchanged.",
    },
    ScriptFnDoc {
        signature: "overflow_priority(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["priority"],
        doc: "Marks an intrinsic overflow-row cell optional, with priority 0..65535. Lower priorities collapse first; equal-priority cells form an atomic cohort, useful for a whole set of page numbers. Only affects layout-bearing children of an enabled overflow_row. A later overflow_required() restores a required cell.",
    },
    ScriptFnDoc {
        signature: "overflow_required(_: &mut Style) -> Style",
        params: &[],
        doc: "Clears a lower-layer overflow_priority; the overflow row must retain this cell even when required content cannot fit.",
    },
    ScriptFnDoc {
        signature: "grid_cell(_: &mut Style, _: i64, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["column", "row"],
        doc: "Places an item at a one-based grid cell (1..32767). Multiple children can share a cell; native intrinsic sizing includes all of them, allowing normal/bold labels or other overlays to reserve their maximum size. Spans may be combined with this placement. Opacity 0 retains measurement; display none does not.",
    },
    ScriptFnDoc {
        signature: "focus_ring(_: &mut Style, _: Length, _: Length, _: ColorValue) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "offset", "color"],
        doc: "Draws a non-layout focus ring when this node owns visible focus, following the view's focus-indicator policy. Inside an active group_focus/focus_within refinement, a visual part follows that explicitly borrowed focus scope instead. Merely inheriting focus never activates a base ring on every descendant. Width/offset accept px, rem or theme lengths, not percentages. The gap remains transparent and the ring follows the node's physical/logical radii and translation. Zero width clears an inherited ring. Does not enlarge the hit area; ancestor clipping still applies.",
    },
    ScriptFnDoc {
        signature: "focus_ring(_: &mut Style, _: Length, _: SignedLength, _: ColorValue) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["width", "offset", "color"],
        doc: "The signed-offset focus ring: offset_px(-2) with a 2px stroke paints inside the border box, without consuming layout space or a component-specific overlay. Signed rem offsets are also supported; auto and relative offsets are invalid. Completely collapsed inset geometry emits no paint.",
    },
    ScriptFnDoc {
        signature: "auto_grid_columns(_: &mut Style, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["minimum", "preferred", "maximum"],
        doc: "Lays out Box children in responsive equal-width columns (1 <= minimum <= preferred <= maximum <= 1000000 logical pixels). Each bound is a number of logical pixels or a Length: px, rems or a theme token such as theme_length(\"metrics.label_column\"), resolved natively in the node's environment on every frame, so the grid follows the tokens its cards use through theme and density changes. Bounds that resolve out of order lay out at the nearest valid ones (the minimum holds) and report a presentation diagnostic. Minimum determines auto-fit breakpoints, preferred supplies the unmeasured natural width hint, and maximum caps growth. Native width refinement after mount/resize does not run Rhai. Intrinsic row heights and child aspect_ratio use GPUI; the last row keeps the same tracks. One track per child; use explicit grids for spans. Not virtualized.",
    },
    ScriptFnDoc {
        signature: "aspect_ratio(_: &mut Style, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["ratio"],
        doc: "Sets the positive width/height ratio for GPUI layout. Leave one size automatic, e.g. width(relative(1.0)).aspect_ratio(1) for a square; two explicit dimensions take precedence.",
    },
    ScriptFnDoc {
        signature: "absolute(_: &mut Style) -> Style",
        params: &[],
        doc: "Takes the node out of the flow and places it by its insets (`top`, `left`, `inset_start`, ...) within its parent.",
    },
    ScriptFnDoc {
        signature: "active(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Paints the nested style's background, border color, text color and opacity while the pointer is pressed on the node, over `hover` and `focus` paint.",
    },
    ScriptFnDoc {
        signature: "background(_: &mut Style, _: ColorValue) -> Style",
        params: &["color"],
        doc: "Fills the node with a solid color, literal or theme token; replaces a gradient set earlier on this style.",
    },
    ScriptFnDoc {
        signature: "background(_: &mut Style, _: PaintColor) -> Style",
        params: &["color"],
        doc: "Replaces the whole background with native-context ink, e.g. alpha(current_color(),0.1).",
    },
    ScriptFnDoc {
        signature: "block(_: &mut Style) -> Style",
        params: &[],
        doc: "Uses block layout: children stack vertically in normal flow and flex alignment does not apply.",
    },
    ScriptFnDoc {
        signature: "border(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the border width of all four edges in px or rem, replacing per-edge widths; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "border_bottom(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the bottom border width in px or rem; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "border_color(_: &mut Style, _: ColorValue) -> Style",
        params: &["color"],
        doc: "Sets one border color, literal or theme token, for every edge.",
    },
    ScriptFnDoc {
        signature: "border_color(_: &mut Style, _: PaintColor) -> Style",
        params: &["color"],
        doc: "Sets the border ink from the final native text color without changing the border widths.",
    },
    ScriptFnDoc {
        signature: "border_dashed(_: &mut Style) -> Style",
        params: &[],
        doc: "Draws the borders dashed; one line style applies to every edge.",
    },
    ScriptFnDoc {
        signature: "border_end(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the border width of the logical end edge (right in LTR, left in RTL); overrides the physical edge there.",
    },
    ScriptFnDoc {
        signature: "border_left(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the left border width in px or rem; `border_start`/`border_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "border_right(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the right border width in px or rem; `border_start`/`border_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "border_solid(_: &mut Style) -> Style",
        params: &[],
        doc: "Draws the borders solid, the default; one line style applies to every edge.",
    },
    ScriptFnDoc {
        signature: "border_start(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the border width of the logical start edge (left in LTR, right in RTL); overrides the physical edge there.",
    },
    ScriptFnDoc {
        signature: "border_top(_: &mut Style, _: Length) -> Style",
        params: &["width"],
        doc: "Sets the top border width in px or rem; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "bottom(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the bottom inset of a positioned node to auto.",
    },
    ScriptFnDoc {
        signature: "bottom(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the bottom inset of a positioned node in px, rem or a `relative` fraction of the parent's height.",
    },
    ScriptFnDoc {
        signature: "bottom(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the bottom inset of a positioned node to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "clip(_: &mut Style) -> Style",
        params: &[],
        doc: "Clips the node's children to its bounds without making it scrollable.",
    },
    ScriptFnDoc {
        signature: "col_span(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["span"],
        doc: "Makes a grid item span `span` columns, from 1 to 1024.",
    },
    ScriptFnDoc {
        signature: "cursor_crosshair(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the crosshair cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_default(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the default arrow cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_move(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the open-hand (grab) cursor while the pointer is over the node, like an idle Draggable handle.",
    },
    ScriptFnDoc {
        signature: "cursor_not_allowed(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the not-allowed cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_pointer(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the pointing-hand cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_resize_x(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the left-right resize cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_resize_y(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the up-down resize cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "cursor_text(_: &mut Style) -> Style",
        params: &[],
        doc: "Shows the text I-beam cursor while the pointer is over the node.",
    },
    ScriptFnDoc {
        signature: "disabled(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Merges the nested style over the node while it or an ancestor is disabled; `hover`, `active` and `focus` paint then stop.",
    },
    ScriptFnDoc {
        signature: "flex_basis(_: &mut Style, _: AutoLength) -> Style",
        params: &["basis"],
        doc: "Sets the flex basis to auto: the node starts from its own size before growing or shrinking.",
    },
    ScriptFnDoc {
        signature: "flex_basis(_: &mut Style, _: Length) -> Style",
        params: &["basis"],
        doc: "Sets the main-axis size the node starts from before growing or shrinking, in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "flex_col(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node a flex container that stacks its children vertically.",
    },
    ScriptFnDoc {
        signature: "flex_grow(_: &mut Style) -> Style",
        params: &[],
        doc: "Lets the node grow into free main-axis space with factor 1, replacing an earlier weight.",
    },
    ScriptFnDoc {
        signature: "flex_grow(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["weight"],
        doc: "Lets the node grow into free main-axis space by a positive `weight`, relative to its siblings' weights.",
    },
    ScriptFnDoc {
        signature: "flex_grow(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["weight"],
        doc: "Lets the node grow into free main-axis space by a positive `weight`, relative to its siblings' weights.",
    },
    ScriptFnDoc {
        signature: "flex_nowrap(_: &mut Style) -> Style",
        params: &[],
        doc: "Keeps a flex container's children on one line.",
    },
    ScriptFnDoc {
        signature: "flex_row(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node a flex container that lays its children out in a row from the logical start, mirrored in RTL.",
    },
    ScriptFnDoc {
        signature: "flex_shrink(_: &mut Style, _: bool) -> Style",
        params: &["shrink"],
        doc: "Sets whether the node may shrink below its flex basis when space runs out: `true` lets it, `false` forbids it.",
    },
    ScriptFnDoc {
        signature: "flex_shrink_weight(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["weight"],
        doc: "Sets a finite shrink factor from 0 to 1000. In an overfull flex row or column, a higher factor gives up more of this child's flex basis; zero keeps its basis.",
    },
    ScriptFnDoc {
        signature: "flex_shrink_weight(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["weight"],
        doc: "Sets a finite shrink factor from 0 to 1000. In an overfull flex row or column, a higher factor gives up more of this child's flex basis; zero keeps its basis.",
    },
    ScriptFnDoc {
        signature: "flex_wrap(_: &mut Style) -> Style",
        params: &[],
        doc: "Lets a flex container wrap its children onto further lines.",
    },
    ScriptFnDoc {
        signature: "flex_wrap_reverse(_: &mut Style) -> Style",
        params: &[],
        doc: "Lets a flex container wrap its children onto further lines, stacked in reverse cross-axis order.",
    },
    ScriptFnDoc {
        signature: "focus(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Paints the nested style's background, border color, text color, shadow and gradient while the node has visible focus. Without an exterior focus_ring declaration or an explicit focus border color, an existing border takes the focus_ring token; no border space is added. Use focus_ring(width, offset, color) for a non-layout exterior outline instead.",
    },
    ScriptFnDoc {
        signature: "focus_within(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Merges the nested style over the node while it or any descendant has keyboard focus.",
    },
    ScriptFnDoc {
        signature: "font_fallbacks(_: &mut Style, _: array) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["families"],
        doc: "Sets the ordered fallback font families, 1 to 16 unique non-empty names, used for glyphs the primary font lacks.",
    },
    ScriptFnDoc {
        signature: "font_family(_: &mut Style, _: string) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["family"],
        doc: "Sets the font family by name, a system or registered font of at most 256 bytes; overrides the typography role's family.",
    },
    ScriptFnDoc {
        signature: "font_feature(_: &mut Style, _: string, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["tag", "value"],
        doc: "Sets an OpenType feature: `tag` is four ASCII letters or digits like `\"liga\"`, `value` 0 to 65535; calls accumulate.",
    },
    ScriptFnDoc {
        signature: "font_size(_: &mut Style, _: Length) -> Style",
        params: &["size"],
        doc: "Sets the font size in px or rem, overriding the typography role's; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "font_weight(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["weight"],
        doc: "Sets the numeric font weight, 1 to 1000 (400 regular, 700 bold), overriding the typography role's.",
    },
    ScriptFnDoc {
        signature: "gap(_: &mut Style, _: Length) -> Style",
        params: &["gap"],
        doc: "Sets spacing on both axes, clearing earlier gap_x/gap_y declarations on this layer or lower layers. Accepts px, rem, theme lengths or a relative fraction of the container.",
    },
    ScriptFnDoc {
        signature: "gap_x(_: &mut Style, _: Length) -> Style",
        params: &["gap"],
        doc: "Sets only horizontal spacing (column-gap) between children, in px, rem, theme lengths or relative units. Physical axis, unchanged by RTL or flex direction. A later gap() resets both axes. Applies to flex, wrapped rows, explicit grids and auto_grid_columns.",
    },
    ScriptFnDoc {
        signature: "gap_y(_: &mut Style, _: Length) -> Style",
        params: &["gap"],
        doc: "Sets only vertical spacing (row-gap), including between wrapped flex lines, in px, rem, theme lengths or relative units. A later gap() resets both axes. Applies to flex, explicit grids and auto_grid_columns.",
    },
    ScriptFnDoc {
        signature: "grid(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node a grid container; `grid_cols` and `grid_rows` set its tracks.",
    },
    ScriptFnDoc {
        signature: "grid_cols(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["columns"],
        doc: "Makes the node a grid with `columns` equal-width columns, from 1 to 1024.",
    },
    ScriptFnDoc {
        signature: "grid_cols_max_content(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["columns"],
        doc: "Makes the node a grid with 1..1024 independently max-content-sized columns, rather than equal fractional widths. Follows native text measurement and works with grid_cell(column,row). A later grid_cols or auto_grid_columns replaces this sizing mode.",
    },
    ScriptFnDoc {
        signature: "grid_rows(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["rows"],
        doc: "Makes the node a grid with `rows` equal-height rows, from 1 to 1024.",
    },
    ScriptFnDoc {
        signature: "grid_rows_intrinsic(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["rows"],
        doc: "Makes the node a grid with 1..1024 independently content-sized rows, with a min-content floor and max-content ceiling per track. A later grid_rows restores equal fractional rows.",
    },
    ScriptFnDoc {
        signature: "group_focus(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Merges the nested style over the node while its nearest focusable ancestor-or-self with a `focus` style has keyboard focus.",
    },
    ScriptFnDoc {
        signature: "group_hover(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Paints the nested style's background, border color, text color and opacity while the nearest ancestor with a `hover` style is hovered.",
    },
    ScriptFnDoc {
        signature: "height(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Sets the height to auto, so content and layout decide it.",
    },
    ScriptFnDoc {
        signature: "height(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the height in px, rem or a `relative` fraction of the parent's height.",
    },
    ScriptFnDoc {
        signature: "hidden(_: &mut Style) -> Style",
        params: &[],
        doc: "Removes the node from layout and paint (display none); `invisible` keeps its space instead.",
    },
    ScriptFnDoc {
        signature: "hover(_: &mut Style, _: Style) -> Style",
        params: &["style"],
        doc: "Paints the nested style's background, border color, text color and opacity while the pointer is over the node; `focus` and `active` paint win over it.",
    },
    ScriptFnDoc {
        signature: "inset_end(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical end edge (right in LTR, left in RTL) to auto; overrides `left`/`right` on that side.",
    },
    ScriptFnDoc {
        signature: "inset_end(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical end edge (right in LTR, left in RTL) in px, rem or a `relative` fraction; overrides `left`/`right` there.",
    },
    ScriptFnDoc {
        signature: "inset_end(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical end edge (right in LTR, left in RTL) to a signed `offset_*` length; overrides `left`/`right`.",
    },
    ScriptFnDoc {
        signature: "inset_start(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical start edge (left in LTR, right in RTL) to auto; overrides `left`/`right` on that side.",
    },
    ScriptFnDoc {
        signature: "inset_start(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical start edge (left in LTR, right in RTL) in px, rem or a `relative` fraction; overrides `left`/`right` there.",
    },
    ScriptFnDoc {
        signature: "inset_start(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the inset from the logical start edge (left in LTR, right in RTL) to a signed `offset_*` length; overrides `left`/`right`.",
    },
    ScriptFnDoc {
        signature: "invisible(_: &mut Style) -> Style",
        params: &[],
        doc: "Hides the node but keeps its layout space; like CSS `visibility: hidden`, its subtree also leaves the accessibility tree. `visible` undoes it.",
    },
    ScriptFnDoc {
        signature: "italic(_: &mut Style) -> Style",
        params: &[],
        doc: "Sets the italic font style.",
    },
    ScriptFnDoc {
        signature: "items_baseline(_: &mut Style) -> Style",
        params: &[],
        doc: "In a row, sets each text child's first baseline on the baseline of the row's own text style, as words in one line are set, so a smaller key legend sits on its label's baseline; other children align to the start.",
    },
    ScriptFnDoc {
        signature: "items_center(_: &mut Style) -> Style",
        params: &[],
        doc: "Centers children on the container's cross axis.",
    },
    ScriptFnDoc {
        signature: "items_end(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns children to the cross-axis end: the bottom of a row, the logical end of a column (left in RTL).",
    },
    ScriptFnDoc {
        signature: "items_start(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns children to the cross-axis start: the top of a row, the logical start of a column (right in RTL).",
    },
    ScriptFnDoc {
        signature: "justify_around(_: &mut Style) -> Style",
        params: &[],
        doc: "Distributes free main-axis space evenly around each child.",
    },
    ScriptFnDoc {
        signature: "justify_between(_: &mut Style) -> Style",
        params: &[],
        doc: "Puts the first and last children at the main-axis ends and spreads the free space evenly between children.",
    },
    ScriptFnDoc {
        signature: "justify_center(_: &mut Style) -> Style",
        params: &[],
        doc: "Centers children along the main axis.",
    },
    ScriptFnDoc {
        signature: "justify_end(_: &mut Style) -> Style",
        params: &[],
        doc: "Packs children toward the end of the main axis; in an RTL row that is the left.",
    },
    ScriptFnDoc {
        signature: "justify_start(_: &mut Style) -> Style",
        params: &[],
        doc: "Packs children toward the start of the main axis; in an RTL row that is the right.",
    },
    ScriptFnDoc {
        signature: "left(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the left inset of a positioned node to auto; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "left(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the left inset of a positioned node in px, rem or a `relative` fraction of the parent's width; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "left(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the left inset of a positioned node to a signed `offset_*` length, which may be negative; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "letter_spacing_em(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spacing"],
        doc: "Sets signed font-relative tracking, computed at the declaring element’s final font size.",
    },
    ScriptFnDoc {
        signature: "letter_spacing_em(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spacing"],
        doc: "Sets signed font-relative tracking, computed at the declaring element’s final font size.",
    },
    ScriptFnDoc {
        signature: "letter_spacing_px(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spacing"],
        doc: "Sets signed tracking in logical pixels. Zero clears previously declared tracking.",
    },
    ScriptFnDoc {
        signature: "letter_spacing_px(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["spacing"],
        doc: "Sets signed tracking in logical pixels. Zero clears previously declared tracking.",
    },
    ScriptFnDoc {
        signature: "line_clamp(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["lines"],
        doc: "Limits text to `lines` lines, from 1 to 10000, and hides the rest; `text_ellipsis` marks the cut.",
    },
    ScriptFnDoc {
        signature: "line_height(_: &mut Style, _: Length) -> Style",
        params: &["height"],
        doc: "Sets the line height in px or rem, overriding the typography role's; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "layered_gradient(_: &mut Style, _: LayeredGradient) -> Style",
        params: &["gradient"],
        doc: "Replaces the whole background with a premultiplied sRGB layer stack. This also works in paint-only state styles.",
    },
    ScriptFnDoc {
        signature: "linear_gradient(_: &mut Style, _: LinearGradientSpec) -> Style",
        params: &["gradient"],
        doc: "Fills the node with a two-stop gradient from `linear_gradient(#{angle, from, to})`; replaces a background set earlier on this style.",
    },
    ScriptFnDoc {
        signature: "margin(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets all four margins to auto, centering the node in the free space, and replaces per-edge margins.",
    },
    ScriptFnDoc {
        signature: "margin(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets all four margins in px, rem or a `relative` fraction, replacing per-edge margins set earlier.",
    },
    ScriptFnDoc {
        signature: "margin(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets all four margins to a signed `offset_*` length, which may be negative, replacing per-edge margins.",
    },
    ScriptFnDoc {
        signature: "margin_bottom(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the bottom margin to auto, taking up the free space there.",
    },
    ScriptFnDoc {
        signature: "margin_bottom(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the bottom margin in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "margin_bottom(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the bottom margin to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "margin_end(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the logical end margin (right in LTR, left in RTL) to auto, taking the free space there; overrides the physical margin.",
    },
    ScriptFnDoc {
        signature: "margin_end(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the logical end margin (right in LTR, left in RTL) in px, rem or a `relative` fraction; overrides the physical margin there.",
    },
    ScriptFnDoc {
        signature: "margin_end(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the logical end margin (right in LTR, left in RTL) to a signed `offset_*` length; overrides the physical margin there.",
    },
    ScriptFnDoc {
        signature: "margin_left(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the left margin to auto, taking up the free space there; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_left(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the left margin in px, rem or a `relative` fraction; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_left(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the left margin to a signed `offset_*` length, which may be negative; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_right(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the right margin to auto, taking up the free space there; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_right(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the right margin in px, rem or a `relative` fraction; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_right(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the right margin to a signed `offset_*` length, which may be negative; `margin_start`/`margin_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "margin_start(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the logical start margin (left in LTR, right in RTL) to auto, taking the free space there; overrides the physical margin.",
    },
    ScriptFnDoc {
        signature: "margin_start(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the logical start margin (left in LTR, right in RTL) in px, rem or a `relative` fraction; overrides the physical margin there.",
    },
    ScriptFnDoc {
        signature: "margin_start(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the logical start margin (left in LTR, right in RTL) to a signed `offset_*` length; overrides the physical margin there.",
    },
    ScriptFnDoc {
        signature: "margin_top(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the top margin to auto, taking up the free space there.",
    },
    ScriptFnDoc {
        signature: "margin_top(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the top margin in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "margin_top(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the top margin to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "margin_x(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the left and right margins to auto, centering the node horizontally.",
    },
    ScriptFnDoc {
        signature: "margin_x(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the left and right margins in px, rem or a `relative` fraction; `margin_start`/`margin_end` still override.",
    },
    ScriptFnDoc {
        signature: "margin_x(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the left and right margins to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "margin_y(_: &mut Style, _: AutoLength) -> Style",
        params: &["margin"],
        doc: "Sets the top and bottom margins to auto, centering the node vertically.",
    },
    ScriptFnDoc {
        signature: "margin_y(_: &mut Style, _: Length) -> Style",
        params: &["margin"],
        doc: "Sets the top and bottom margins in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "margin_y(_: &mut Style, _: SignedLength) -> Style",
        params: &["margin"],
        doc: "Sets the top and bottom margins to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "max_height(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Removes a max height limit by setting it to auto.",
    },
    ScriptFnDoc {
        signature: "max_height(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the max height in px, rem or a `relative` fraction of the parent's height.",
    },
    ScriptFnDoc {
        signature: "max_width(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Removes a max width limit by setting it to auto.",
    },
    ScriptFnDoc {
        signature: "max_width(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the max width in px, rem or a `relative` fraction of the parent's width.",
    },
    ScriptFnDoc {
        signature: "merge(_: &mut Style, _: Style) -> Style",
        params: &["overlay"],
        doc: "Lays every property `overlay` sets over this style; pseudo-state styles merge state by state.",
    },
    ScriptFnDoc {
        signature: "min_height(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Sets the min height to auto, the content-based minimum of a flex item.",
    },
    ScriptFnDoc {
        signature: "min_height(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the min height in px, rem or a `relative` fraction of the parent's height.",
    },
    ScriptFnDoc {
        signature: "min_width(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Sets the min width to auto, the content-based minimum of a flex item.",
    },
    ScriptFnDoc {
        signature: "min_width(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the min width in px, rem or a `relative` fraction of the parent's width.",
    },
    ScriptFnDoc {
        signature: "not_italic(_: &mut Style) -> Style",
        params: &[],
        doc: "Sets the upright font style, undoing `italic`.",
    },
    ScriptFnDoc {
        signature: "occlude(_: &mut Style) -> Style",
        params: &[],
        doc: "Blocks all pointer input, scrolling included, from reaching elements painted behind the node.",
    },
    ScriptFnDoc {
        signature: "occlude_except_scroll(_: &mut Style) -> Style",
        params: &[],
        doc: "Blocks pointer input from reaching elements painted behind the node but lets scroll wheel input through.",
    },
    ScriptFnDoc {
        signature: "transform(_: &mut Style, _: map) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["affine"],
        doc: "Sets a visual affine without changing layout. Options: rotate, scale_x, scale_y, skew_x, skew_y, origin_x and origin_y. Angles are degrees, origins are border-box fractions (default 0.5). An empty map clears a lower-priority transform.",
    },
    ScriptFnDoc {
        signature: "isolated_opacity(_: &mut Style, _: bool) -> Style",
        params: &["enabled"],
        doc: "Opt this base style into applying its opacity to the composited subtree once. Does not change layout, mouse targets, keyboard focus, or descendants' own opacity. Ordinary opacity retains per-primitive semantics without this switch.",
    },
    ScriptFnDoc {
        signature: "bottom_fade(_: &mut Style, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["start", "end"],
        doc: "Fade the composed foreground to transparent between two border-box y fractions (0 <= start < end <= 1). This is a paint-only alpha mask, not a hit-test or accessibility clip.",
    },
    ScriptFnDoc {
        signature: "edge_fade(_: &mut Style, _: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["axis", "leading", "trailing"],
        doc: "Apply physical x or y alpha ramps to the already-composed subtree. Each distance is a logical-pixel value, zero leaves that edge unchanged, and short boxes shrink both ramps proportionally. Replaces a lower-priority fade without clipping input.",
    },
    ScriptFnDoc {
        signature: "scroll_edge_fade(_: &mut Style, _: string, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["axis", "leading", "trailing"],
        doc: "Fade physical viewport edges only while more content is hidden in that direction. Distances are logical pixels, and the node must scroll on the selected axis. Paint follows its native scroll handle without changing layout or hit geometry.",
    },
    ScriptFnDoc {
        signature: "glyph_sweep(_: &mut Style, _: ColorValue, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["ink", "half_width", "phase"],
        doc: "Overlays a triangular tint on already-shaped monochrome glyphs, without changing base ink or layout. Half width is 0.01–0.5 of the text extent; phase runs from 0 (outside left) to 1 (outside right). Animate phase with the glyph_sweep property. Apple rendering only in this candidate.",
    },
    ScriptFnDoc {
        signature: "glyph_ink_sweep(_: &mut Style, _: ColorValue, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["ink", "half_width", "travel", "phase"],
        doc: "Replaces part of the original monochrome ink with a triangular highlight, interpolating premultiplied color before applying atlas coverage once. Width and travel are text-box fractions; travel must be at least 1 plus twice the half width and at most 3. Uses the same glyph_sweep animation property. Apple rendering only in this candidate.",
    },
    ScriptFnDoc {
        signature: "without_glyph_sweep(_: &mut Style) -> Style",
        params: &[],
        doc: "Clears a glyph sweep set by a lower style layer.",
    },
    ScriptFnDoc {
        signature: "paint_reveal_x(_: &mut Style, _: types::dynamic::Dynamic, _: types::dynamic::Dynamic) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["radius", "progress"],
        doc: "Shows a rounded left-origin fraction of composed paint. Radius is logical pixels, progress runs from 0 (empty) to 1 (ordinary paint), and animated samples use paint_reveal_x. Layout and pointer/focus geometry are unchanged. Apple renderer only in the candidate.",
    },
    ScriptFnDoc {
        signature: "without_paint_reveal_x(_: &mut Style) -> Style",
        params: &[],
        doc: "Clear a rounded paint reveal from a lower-priority style.",
    },
    ScriptFnDoc {
        signature: "without_bottom_fade(_: &mut Style) -> Style",
        params: &[],
        doc: "Clear the alpha fade (one-way or both edges) from a lower-priority style.",
    },
    ScriptFnDoc {
        signature: "foreground_blur(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["sigma"],
        doc: "Gaussian-blurs the composed foreground subtree using sigma in 0..=64 logical pixels. Zero clears the filter. Leaves layout and input bounds unchanged; does not sample the backdrop.",
    },
    ScriptFnDoc {
        signature: "foreground_blur(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["sigma"],
        doc: "Gaussian-blurs the composed foreground subtree using integer sigma in 0..=64 logical pixels. Zero clears the filter. Leaves layout and input bounds unchanged; does not sample the backdrop.",
    },
    ScriptFnDoc {
        signature: "foreground_grayscale(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["amount"],
        doc: "Desaturates the composed foreground subtree in sRGB without changing its alpha. Zero clears the filter; one applies full grayscale. Layout and input geometry stay unchanged. Apple rendering only in this candidate.",
    },
    ScriptFnDoc {
        signature: "foreground_grayscale(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["amount"],
        doc: "Clears foreground grayscale with zero or sets full grayscale with one. The effect composes once across the subtree.",
    },
    ScriptFnDoc {
        signature: "backdrop_blur(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["sigma"],
        doc: "Gaussian-blurs the already-painted destination underneath this rounded border box, in the current composition surface. Sigma is 0..=64 logical pixels. Zero clears the filter; content stays sharp.",
    },
    ScriptFnDoc {
        signature: "backdrop_blur(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["sigma"],
        doc: "Gaussian-blurs the already-painted destination underneath this rounded border box, in the current composition surface. Integer sigma is 0..=64 logical pixels. Zero clears the filter; content stays sharp.",
    },
    ScriptFnDoc {
        signature: "opacity(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["opacity"],
        doc: "Sets the node's opacity from 0 (transparent) to 1 (opaque); a bound signal or animation overrides it.",
    },
    ScriptFnDoc {
        signature: "opacity(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["opacity"],
        doc: "Sets the node's opacity to 0 (transparent) or 1 (opaque); a bound signal or animation overrides it.",
    },
    ScriptFnDoc {
        signature: "overflow_hidden(_: &mut Style) -> Style",
        params: &[],
        doc: "Clips children to the node's bounds on both axes, without scrolling.",
    },
    ScriptFnDoc {
        signature: "overflow_scroll(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node scroll on both axes, clipping its children to its bounds.",
    },
    ScriptFnDoc {
        signature: "overflow_x_scroll(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node scroll horizontally; wheel input stays on that axis unless the node calls `translate_wheel()`.",
    },
    ScriptFnDoc {
        signature: "overflow_y_scroll(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node scroll vertically; wheel input stays on that axis unless the node calls `translate_wheel()`.",
    },
    ScriptFnDoc {
        signature: "padding(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets all four paddings in px, rem or a `relative` fraction, replacing per-edge paddings set earlier.",
    },
    ScriptFnDoc {
        signature: "padding_bottom(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the bottom padding in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "padding_end(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the padding of the logical end edge (right in LTR, left in RTL); overrides the physical padding there.",
    },
    ScriptFnDoc {
        signature: "padding_left(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the left padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "padding_right(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the right padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "padding_start(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the padding of the logical start edge (left in LTR, right in RTL); overrides the physical padding there.",
    },
    ScriptFnDoc {
        signature: "padding_top(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the top padding in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "padding_x(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the left and right padding in px, rem or a `relative` fraction; `padding_start`/`padding_end` still override.",
    },
    ScriptFnDoc {
        signature: "padding_y(_: &mut Style, _: Length) -> Style",
        params: &["padding"],
        doc: "Sets the top and bottom padding in px, rem or a `relative` fraction.",
    },
    ScriptFnDoc {
        signature: "radius(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds all four corners in px or rem, replacing per-corner radii; a `relative` length is ignored.",
    },
    ScriptFnDoc {
        signature: "radius_bottom_left(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds the bottom-left corner in px or rem; `radius_start`/`radius_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "radius_bottom_right(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds the bottom-right corner in px or rem; `radius_start`/`radius_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "radius_end(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds both corners of the logical end side (right in LTR, left in RTL); overrides the physical corners there.",
    },
    ScriptFnDoc {
        signature: "radius_start(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds both corners of the logical start side (left in LTR, right in RTL); overrides the physical corners there.",
    },
    ScriptFnDoc {
        signature: "radius_top_left(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds the top-left corner in px or rem; `radius_start`/`radius_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "radius_top_right(_: &mut Style, _: Length) -> Style",
        params: &["radius"],
        doc: "Rounds the top-right corner in px or rem; `radius_start`/`radius_end` override it on that side.",
    },
    ScriptFnDoc {
        signature: "relative(_: &mut Style) -> Style",
        params: &[],
        doc: "Keeps the node in normal flow, shifted by any insets; this is the default and undoes `absolute`.",
    },
    ScriptFnDoc {
        signature: "right(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the right inset of a positioned node to auto; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "right(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the right inset of a positioned node in px, rem or a `relative` fraction of the parent's width; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "right(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the right inset of a positioned node to a signed `offset_*` length, which may be negative; `inset_start`/`inset_end` win there.",
    },
    ScriptFnDoc {
        signature: "row_span(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["span"],
        doc: "Makes a grid item span `span` rows, from 1 to 1024.",
    },
    ScriptFnDoc {
        signature: "self_center(_: &mut Style) -> Style",
        params: &[],
        doc: "Centers this node on its parent's cross axis, overriding the parent's `items_*`.",
    },
    ScriptFnDoc {
        signature: "self_end(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns this node to the end of its parent's cross axis, overriding `items_*`: the bottom in a row, the logical end in a column (left in RTL).",
    },
    ScriptFnDoc {
        signature: "self_start(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns this node to the start of its parent's cross axis, overriding `items_*`: the top in a row, the logical start in a column (right in RTL).",
    },
    ScriptFnDoc {
        signature: "self_stretch(_: &mut Style) -> Style",
        params: &[],
        doc: "Stretches this node across its parent's cross axis, overriding the parent's `items_*`.",
    },
    ScriptFnDoc {
        signature: "shadow(_: &mut Style, _: ShadowSpec) -> Style",
        params: &["shadow"],
        doc: "Sets one box shadow from `shadow(#{x, y, blur, spread, color, inset})`, in logical pixels, replacing an earlier stack.",
    },
    ScriptFnDoc {
        signature: "shadows(_: &mut Style, _: array) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["layers"],
        doc: "Replaces the shadow stack with at most 32 shadow(...) values, topmost first. Supports inset and signed spread. An empty array clears the stack. Shadows consume no layout space and work in hover/active/focus refinements.",
    },
    ScriptFnDoc {
        signature: "without_hover(_: &mut Style) -> Style",
        params: &[],
        doc: "Removes a recipe's hover refinement, retaining idle paint and other states.",
    },
    ScriptFnDoc {
        signature: "without_active(_: &mut Style) -> Style",
        params: &[],
        doc: "Removes a recipe's active refinement, retaining idle paint and other states.",
    },
    ScriptFnDoc {
        signature: "text_center(_: &mut Style) -> Style",
        params: &[],
        doc: "Centers text horizontally.",
    },
    ScriptFnDoc {
        signature: "inherit_text_color(_: &mut Style) -> Style",
        params: &[],
        doc: "Removes a lower-layer text color so this node inherits its parent's text color. A later text_color declaration replaces this reset; this is not a contextual currentColor paint or an isolated opacity group.",
    },
    ScriptFnDoc {
        signature: "text_color(_: &mut Style, _: ColorValue) -> Style",
        params: &["color"],
        doc: "Sets the text color, literal or theme token; descendants inherit it.",
    },
    ScriptFnDoc {
        signature: "text_ellipsis(_: &mut Style) -> Style",
        params: &[],
        doc: "Truncates text that overflows the available width with an ellipsis at the end.",
    },
    ScriptFnDoc {
        signature: "text_end(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns text to the logical end: right in LTR, left in RTL.",
    },
    ScriptFnDoc {
        signature: "text_left(_: &mut Style) -> Style",
        params: &[],
        doc: "Same as `text_start`; mirrors in RTL, so it aligns text right there.",
    },
    ScriptFnDoc {
        signature: "text_right(_: &mut Style) -> Style",
        params: &[],
        doc: "Same as `text_end`; mirrors in RTL, so it aligns text left there.",
    },
    ScriptFnDoc {
        signature: "text_start(_: &mut Style) -> Style",
        params: &[],
        doc: "Aligns text to the logical start: left in LTR, right in RTL.",
    },
    ScriptFnDoc {
        signature: "top(_: &mut Style, _: AutoLength) -> Style",
        params: &["inset"],
        doc: "Sets the top inset of a positioned node to auto.",
    },
    ScriptFnDoc {
        signature: "top(_: &mut Style, _: Length) -> Style",
        params: &["inset"],
        doc: "Sets the top inset of a positioned node in px, rem or a `relative` fraction of the parent's height.",
    },
    ScriptFnDoc {
        signature: "top(_: &mut Style, _: SignedLength) -> Style",
        params: &["inset"],
        doc: "Sets the top inset of a positioned node to a signed `offset_*` length, which may be negative.",
    },
    ScriptFnDoc {
        signature: "translate_x(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["offset"],
        doc: "Moves the painted node right by `offset` logical pixels (negative moves left, not mirrored in RTL); layout is unchanged.",
    },
    ScriptFnDoc {
        signature: "translate_x(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["offset"],
        doc: "Moves the painted node right by `offset` logical pixels (negative moves left, not mirrored in RTL); layout is unchanged.",
    },
    ScriptFnDoc {
        signature: "translate_y(_: &mut Style, _: f64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["offset"],
        doc: "Moves the painted node down by `offset` logical pixels (negative moves up); layout is unchanged.",
    },
    ScriptFnDoc {
        signature: "translate_y(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["offset"],
        doc: "Moves the painted node down by `offset` logical pixels (negative moves up); layout is unchanged.",
    },
    ScriptFnDoc {
        signature: "typography(_: &mut Style, _: string) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["role"],
        doc: "Applies a theme typography role such as `\"body\"` or `\"code\"`; explicit family, fallbacks, size, weight or line height win.",
    },
    ScriptFnDoc {
        signature: "visible(_: &mut Style) -> Style",
        params: &[],
        doc: "Makes the node visible again, undoing `invisible`.",
    },
    ScriptFnDoc {
        signature: "whitespace_normal(_: &mut Style) -> Style",
        params: &[],
        doc: "Wraps text at the available width.",
    },
    ScriptFnDoc {
        signature: "whitespace_nowrap(_: &mut Style) -> Style",
        params: &[],
        doc: "Keeps text on one line without wrapping.",
    },
    ScriptFnDoc {
        signature: "width(_: &mut Style, _: AutoLength) -> Style",
        params: &["length"],
        doc: "Sets the width to auto, so content and layout decide it.",
    },
    ScriptFnDoc {
        signature: "width(_: &mut Style, _: Length) -> Style",
        params: &["length"],
        doc: "Sets the width in px, rem or a `relative` fraction of the parent's width.",
    },
];
