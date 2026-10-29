//! GPUI-native tests: real GPUI windows, focus, input and paint under the `test-support`
//! platform. One test binary; each directory is an area of the product.
//!
//! Run one area or file with a filter: `cargo test table::keyboard`.

mod support;

mod assets;
mod audit;
mod charts;
mod collections;
mod combobox;
mod components;
mod gallery;
mod host;
mod input;
mod layout;
mod manipulation;
mod motion;
mod overlay;
mod perf;
mod runtime;
mod table;
mod text;
mod theme;
mod windows;
