//! Backend-specific presentation adapters. Layout and interaction identity
//! remain owned by the same SDK in either build.

#[cfg(feature = "vendor-gpui")]
pub(crate) use gpui::{
    BottomFade, ComputedPaint, LeadingFloat, PaintInk, PaintRevealX, ShadowStack, TextExclusion,
    TextExclusionSide, VisualCoordinates,
};
#[cfg(feature = "vendor-gpui")]
pub(crate) type Path = gpui::Path<gpui::Pixels>;
#[cfg(feature = "vendor-gpui")]
pub(crate) const INTRINSIC_ROWS: gpui::GridTemplateMinSize = gpui::GridTemplateMinSize::Intrinsic;

#[cfg(not(feature = "vendor-gpui"))]
mod public;
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) use crate::{BottomFade, PaintRevealX};
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) use public::{
    ComputedPaint, PaintInk, PreviewWindowExt as PublicWindowExt, TextExclusion, TextExclusionSide,
    VisualCoordinates,
};
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) type ShadowStack = Vec<gpui::BoxShadow>;
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) type Path = gpui::Path;
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) const INTRINSIC_ROWS: gpui::GridTemplateMinSize = gpui::GridTemplateMinSize::MaxContent;
