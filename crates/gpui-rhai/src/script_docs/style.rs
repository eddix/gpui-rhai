//! Script API docs: Style methods. One entry per registered signature;
//! see `super` for the format.

use super::ScriptFnDoc;

pub(super) const DOCS: &[ScriptFnDoc] = &[
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
        doc: "Paints the nested style's background, border color, text color and opacity while the node has keyboard focus, over `hover` paint and under `active`. A focused node's border takes `focus_ring` unless this style sets a border color, also without a `focus` call; it shows only where the node has a border width.",
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
        doc: "Sets the space between children on both axes, in px, rem or a `relative` fraction of the container.",
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
        signature: "grid_rows(_: &mut Style, _: i64) -> core::result::Result<gpui_rhai::style::Style,alloc::boxed::Box<rhai::types::error::EvalAltResult>>",
        params: &["rows"],
        doc: "Makes the node a grid with `rows` equal-height rows, from 1 to 1024.",
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
        doc: "Hides the node but keeps its layout space; `visible` undoes it.",
    },
    ScriptFnDoc {
        signature: "italic(_: &mut Style) -> Style",
        params: &[],
        doc: "Sets the italic font style.",
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
        doc: "Sets one box shadow from `shadow(#{x, y, blur, spread, color})`, in logical pixels, replacing an earlier one.",
    },
    ScriptFnDoc {
        signature: "text_center(_: &mut Style) -> Style",
        params: &[],
        doc: "Centers text horizontally.",
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
