# component_gallery

An independently runnable embedded-release gallery for the official components
that are mechanisms or dependencies rather than primary surfaces in the four
product examples: Collapsible, Icon, Menu, Skeleton, and Tooltip.

```sh
cargo run -p gpui-rhai --example component_gallery
```

The gallery demonstrates a directional SVG icon, deterministic and animated
Skeleton states, controlled Collapsible content, keyboard-capable Menu, and the
native delayed Tooltip. Visual-test environment variables select theme, locale,
reduced motion, and the fixed open-Menu state.
