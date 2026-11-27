//! Direct manipulation: drag continuation, resize grips, sliders, transformed selection and no-op
//! contracts.

mod contracts;
mod drag_continuation;
mod file_drops;
mod resizable_grip_cursor;
mod slider;
// The published profile deliberately uses the untransformed interaction plane.
// The ordinary component contracts above run on both profiles.
#[cfg(feature = "vendor-gpui")]
mod transformed_resize;
#[cfg(feature = "vendor-gpui")]
mod transformed_selection;
#[cfg(feature = "vendor-gpui")]
mod transformed_slider;
