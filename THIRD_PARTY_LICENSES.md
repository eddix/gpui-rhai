# Third-party licenses

GPUI Rhai depends on third-party Rust crates under their respective licenses.
Copied themes, icons, palettes, and adapted source must add their attribution to
this file before entering the official registry.

The check and close SVGs are original GPUI Rhai project assets. No third-party
icon or raster image assets are currently included.

## Direct Rust dependencies

- GPUI 0.2.2 — Apache-2.0.
- Rhai 1.26.0 — MIT OR Apache-2.0.
- notify 8.2.0 — CC0-1.0 (development hot reload only).
- image 0.25.10, unicode-segmentation 1.13.3, serde, serde_json,
  semver, thiserror, toml, toml_edit, clap, and tempfile — MIT OR Apache-2.0.

The 2026-08-28 Cargo metadata audit covered 629 host and cross-target packages
and found no missing license metadata. MPL-2.0 dependencies are used under their
file-level terms; dependencies offering LGPL as one option also offer MIT or
Apache-2.0. Cargo source distributions contain the authoritative license text
for every resolved package.

## Theme palette adaptations

- Tokyo Night Night and Storm are adapted from
  [folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim), licensed
  under Apache-2.0.
- Catppuccin Latte and Mocha are adapted from
  [catppuccin/catppuccin](https://github.com/catppuccin/catppuccin), licensed
  under MIT.

The Rhai files map upstream palette values into GPUI Rhai's independent
semantic token contract; they do not copy upstream implementation code.
