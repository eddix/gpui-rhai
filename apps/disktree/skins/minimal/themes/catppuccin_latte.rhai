// Catppuccin Latte semantic adaptation.
// Upstream: https://github.com/catppuccin/catppuccin (MIT)
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - surface_hover stepped toward text for a visible tonal block.
// - text_muted raised to 4.5:1 on hover and selection surfaces.
fn theme() {
    #{
        family: "Catppuccin",
        name: "Latte",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xeff1f5ff,
                surface_raised: 0xf8f9fbff,
                surface_hover: 0xdcdfe7ff,
                text_primary: 0x4c4f69ff,
                text_muted: 0x5e6178ff,
                accent: 0x1e66f5ff,
                accent_hover: 0x1a5ad7ff,
                on_accent: 0xffffffff,
                danger: 0xd20f39ff,
                on_danger: 0xffffffff,
                warning: 0xdf8e1dff,
                on_warning: 0x1e1e2eff,
                success: 0x40a02bff,
                on_success: 0x1e1e2eff,
                border: 0xbcc0ccff,
                focus_ring: 0x1e66f5ff,
                selection: 0xdce7fcff,
                disabled: 0x9ca0b0ff,
            },
        },
    }
}
