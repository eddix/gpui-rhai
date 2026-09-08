# Third-party licenses

GPUI Rhai depends on third-party Rust crates under their respective licenses.
Copied themes, icons, palettes, and adapted source must add their attribution to
this file before entering the official registry.

The check, close, chevron, disclosure, calendar, date-navigation, and sort SVGs are original GPUI Rhai project assets. No third-party
icon or raster image assets are currently included.

## Direct Rust dependencies

- GPUI 0.2.2 — Apache-2.0.
  `crates/gpui-rhai/src/text_input.rs` adapts GPUI's Apache-2.0
  `examples/input.rs`; the source notice and Zed Industries copyright are
  retained in that module and the distributed Apache license.
- Rhai 1.26.0 — MIT OR Apache-2.0.
- notify 8.2.0 — CC0-1.0 (development hot reload only).
- Jiff 0.2.35 — Unlicense OR MIT.
- similar 3.2.0 — Apache-2.0.
- syntect 5.3.0 — MIT. Its bundled default syntax definitions originate from
  the open-source Sublime Text default package set; GPUI Rhai's Rhai,
  TypeScript/TSX, JSONC, TOML, and Dockerfile definitions are original project
  adaptations.
- image 0.25.10, regex 1.13.1, unicode-segmentation 1.13.3, serde, serde_json,
  semver, thiserror, toml, toml_edit, clap, and tempfile — MIT OR Apache-2.0.

The 2026-09-08 all-features Cargo metadata audit covered 651 resolved packages
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
- Ethereal, Everforest, Gruvbox, Hackerman, Nord, and Retro 82 are semantic
  adaptations of their corresponding palettes in the
  [MIT-licensed Omarchy repository](https://github.com/basecamp/omarchy/tree/master/themes).
- Hermarchy is adapted from
  [Archer Clawbot's Hermarchy theme](https://github.com/archer-clawbot/omarchy-hermarchy-theme),
  licensed under MIT.
- Aetheria is adapted from
  [Dizziee's Aetheria theme](https://github.com/JJDizz1L/aetheria), licensed
  under MIT.
- Futurism is an original GPUI Rhai semantic palette inspired by
  [Bjarne Øverli's Futurism theme](https://github.com/bjarneo/omarchy-futurism-theme).
  No source code or artwork from that repository is redistributed.

The Rhai files map upstream palette values into GPUI Rhai's independent
semantic token contract; they do not copy upstream implementation code.
