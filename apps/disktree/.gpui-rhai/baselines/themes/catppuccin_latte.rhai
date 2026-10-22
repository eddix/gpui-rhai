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
                surface_raised: 0xf9fbfeff,
                surface_hover: 0xdbdde0ff,
                text_primary: 0x34364fff,
                text_muted: 0x494c62ff,
                accent: 0x1e66f5ff,
                accent_hover: 0x115ae9ff,
                on_accent: 0xffffffff,
                danger: 0xd20f39ff,
                on_danger: 0xffffffff,
                warning: 0xdf8e1dff,
                on_warning: 0x1e1e2eff,
                success: 0x40a02bff,
                on_success: 0x1e1e2eff,
                border: 0xaaacaeff,
                focus_ring: 0x1e66f5ff,
                selection: 0xb4cefcff,
                disabled: 0x7e7f82ff,
            },
        },
    }
}
