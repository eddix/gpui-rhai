// Catppuccin Mocha semantic adaptation.
// Upstream: https://github.com/catppuccin/catppuccin (MIT)
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Catppuccin",
        name: "Mocha",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x1e1e2eff,
                surface_raised: 0x313244ff,
                surface_hover: 0x45475aff,
                text_primary: 0xcdd6f4ff,
                text_muted: 0xafb6d2ff,
                accent: 0x89b4faff,
                accent_hover: 0x74c7ecff,
                on_accent: 0x1e1e2eff,
                danger: 0xf38ba8ff,
                on_danger: 0x1e1e2eff,
                warning: 0xf9e2afff,
                on_warning: 0x1e1e2eff,
                success: 0xa6e3a1ff,
                on_success: 0x1e1e2eff,
                border: 0x45475aff,
                focus_ring: 0x89b4faff,
                selection: 0x2b3046ff,
                disabled: 0x6c7086ff,
            },
        },
    }
}
