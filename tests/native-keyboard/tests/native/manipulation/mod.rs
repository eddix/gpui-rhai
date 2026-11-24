//! Direct manipulation: drag continuation, resize grips, sliders, transformed selection and no-op
//! contracts.

mod contracts;
mod drag_continuation;
mod file_drops;
mod resizable_grip_cursor;
mod slider;
mod transformed_resize;
// Requires the vendor's transform/shaping probe.
#[cfg(feature = "vendor-gpui")]
mod transformed_selection;
mod transformed_slider;
