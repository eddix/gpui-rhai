//! The examples' own tests, in one test binary. Each example keeps its `#[cfg(test)]` module
//! next to the view it checks; including the examples here runs those modules without
//! building every example as a test target. Their `main` functions are never called.

#![allow(dead_code)]

#[path = "../examples/artistic_showcase.rs"]
mod artistic_showcase;
#[path = "../examples/code_viewer.rs"]
mod code_viewer;
#[path = "../examples/data_table.rs"]
mod data_table;
#[path = "../examples/diff_viewer.rs"]
mod diff_viewer;
#[path = "../examples/embedded_views.rs"]
mod embedded_views;
#[path = "../examples/form_showcase.rs"]
mod form_showcase;
#[path = "../examples/mini_timeline.rs"]
mod mini_timeline;
#[path = "../examples/internal/performance/table_1000.rs"]
mod table_1000;
#[path = "../examples/variable_height_chat.rs"]
mod variable_height_chat;
