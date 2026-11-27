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
                surface_sunken: 0x161625ff,
                surface: 0x1e1e2eff,
                surface_raised: 0x2b2b3aff,
                surface_elevated: 0x363645ff,
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
                border_subtle: 0x3d3e4eff,
                border: 0x595965ff,
                border_strong: 0x767988ff,
                focus_ring: 0x89b4faff,
                selection: 0x222b47ff,
                disabled: 0x7f808dff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x89b4faff, "palette.blue_ink": 0x8db9ffff, "palette.blue_soft": 0x3e4660ff, "palette.blue_border": 0x52658bff,
                "palette.violet": 0xcba6f7ff, "palette.violet_ink": 0xcea9faff, "palette.violet_soft": 0x4b4460ff, "palette.violet_border": 0x6e5f89ff,
                "palette.teal": 0x94e2d5ff, "palette.teal_ink": 0x94e2d5ff, "palette.teal_soft": 0x405059ff, "palette.teal_border": 0x57787bff,
                "palette.green": 0xa6e3a1ff, "palette.green_ink": 0xa6e3a1ff, "palette.green_soft": 0x44504fff, "palette.green_border": 0x5f7865ff,
                "palette.amber": 0xf9e2afff, "palette.amber_ink": 0xf9e2afff, "palette.amber_soft": 0x545051ff, "palette.amber_border": 0x82786bff,
                "palette.rose": 0xf5c2e7ff, "palette.rose_ink": 0xf5c2e7ff, "palette.rose_soft": 0x53495dff, "palette.rose_border": 0x806a83ff,
                "palette.orange": 0xfab387ff, "palette.orange_ink": 0xfab387ff, "palette.orange_soft": 0x544649ff, "palette.orange_border": 0x82645aff,
                "palette.slate": 0x9399b2ff, "palette.slate_ink": 0xaab0c9ff, "palette.slate_soft": 0x404152ff, "palette.slate_border": 0x57596cff,
            },
        },
    }
}
