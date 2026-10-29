//! Render-path cost and equivalence: counters that tie a frame's cost to what
//! changed, and two windows that run the same script through the same events
//! committing and presenting the same frame.

mod invariants;
mod islands;
mod lockstep;
