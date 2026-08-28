# Virtual lists

`virtual_list(config)` is the general one-dimensional native behavior primitive.
It accepts up to 10,000 stable-key item nodes, an equal `row_height`, a fixed
viewport `height`, an accessibility label, and an overscan diagnostic setting.

```rhai
virtual_list(#{
    key: "files", label: "Files", row_height: 28, height: 420, overscan: 2,
    items: [
        #{ key: "readme", node: text("README.md") },
        #{ key: "cargo", node: text("Cargo.toml") }
    ]
}).on_change(Fn("focused"))
```

GPUI `uniform_list` creates elements only for its requested visible range. A
keyed Rust Entity retains the scroll handle and focused item across ordinary
renders, reorder, and filtering. Up/Down/Home/End move logical focus, scroll it
into view, and emit the item key. Item nodes should have predictable height and
must not perform effects during `view`.

The `native_virtual_list_smoke` example builds 5,000 items and enters real GPUI
layout; unit metrics separately assert bounded realization and focus identity.
Two-dimensional tables remain out of scope.
