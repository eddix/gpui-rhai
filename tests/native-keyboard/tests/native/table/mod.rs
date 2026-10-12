//! Table: columns, keyboard, context requests, sticky and group headers, extent, focus and virtual
//! rows.

// The 1000-row performance example, shared by the column and virtual-row tests.
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/internal/performance/table_1000.rs"]
mod table_1000_example;

mod boundaries;
mod click_focus;
mod columns;
mod context_requests;
mod extent;
mod frame_demand;
mod group_value_case;
mod header_on_raised_surface;
mod keyboard;
mod sticky_header;
mod virtual_rows;
