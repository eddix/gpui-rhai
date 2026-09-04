//! Experimental Grain-to-native function tier.
//!
//! This library target exists so a real application can attach the experiment
//! to a reused [`rhai::grain::Vm`] without copying the benchmark harness. It is
//! intentionally narrow and is not a production Rhai backend.

#[cfg(target_arch = "aarch64")]
pub mod adaptive;
#[cfg(feature = "gpui-adapter")]
pub mod gpui;
#[cfg(target_arch = "aarch64")]
pub mod jit_aarch64;
#[cfg(target_arch = "aarch64")]
pub mod managed_aarch64;
pub mod profile;
mod script_library;
#[cfg(target_arch = "aarch64")]
pub mod tier;
pub mod typed;
