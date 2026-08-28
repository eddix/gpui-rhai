# Native virtual-list smoke

Builds 5,000 stable-key item nodes in Rhai while GPUI `uniform_list` realizes
only the visible range. The native view retains scroll/focus identity and maps
Up, Down, Home, and End to a typed `change(string)` callback.

```text
cargo run -p gpui-rhai --example native_virtual_list_smoke
```
