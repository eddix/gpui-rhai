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
                surface_raised: 0x2b2b3aff,
                surface_hover: 0x383948ff,
                text_primary: 0xcdd6f4ff,
                text_muted: 0xa9b0cbff,
                accent: 0x89b4faff,
                accent_hover: 0x98bfffff,
                on_accent: 0x1e1e2eff,
                danger: 0xf38ba8ff,
                on_danger: 0x1e1e2eff,
                warning: 0xf9e2afff,
                on_warning: 0x1e1e2eff,
                success: 0xa6e3a1ff,
                on_success: 0x1e1e2eff,
                border: 0x595965ff,
                focus_ring: 0x89b4faff,
                selection: 0x222b47ff,
                disabled: 0x7f808dff,
            },
        },
    }
}
