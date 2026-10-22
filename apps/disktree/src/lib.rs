//! disktree-rhai engine: the scan/tree/classify/space/insights core, a
//! faithful port of tobi/disktree's `disktree-core` for Linux.
//!
//! The binary (`src/main.rs`) is the thin host: window plumbing, skins, and
//! the capability bridge.

pub mod capability;
pub mod embedded;
pub mod engine;
