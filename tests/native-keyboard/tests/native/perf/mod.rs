//! Render-path cost and equivalence: counters that tie a frame's cost to what
//! changed, and two windows that run the same script through the same events
//! committing and presenting the same frame.

mod invariants;
mod lockstep;
// Cache islands are off on the gpui-fast backend; these cover nothing else.
#[cfg(feature = "gpui-pre")]
mod islands;
#[cfg(feature = "gpui-pre")]
mod host_width;
#[cfg(feature = "gpui-pre")]
mod coverage;
