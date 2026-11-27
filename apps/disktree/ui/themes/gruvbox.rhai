// Semantic adaptation of Omarchy Gruvbox (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/gruvbox
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Gruvbox",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x202020ff,
                surface: 0x282828ff,
                surface_raised: 0x383432ff,
                surface_elevated: 0x433f3dff,
                surface_hover: 0x48423fff,
                text_primary: 0xedd7b0ff,
                text_muted: 0xd0bb97ff,
                accent: 0x7daea3ff,
                accent_hover: 0x88b9aeff,
                on_accent: 0x1e1e1eff,
                danger: 0xea6962ff,
                on_danger: 0x1e1e1eff,
                warning: 0xd8a657ff,
                on_warning: 0x1e1e1eff,
                success: 0xa9b665ff,
                on_success: 0x1e1e1eff,
                border_subtle: 0x4c4640ff,
                border: 0x69615aff,
                border_strong: 0x8c8172ff,
                focus_ring: 0x89b482ff,
                selection: 0x2c3835ff,
                disabled: 0x948980ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x83a598ff, "palette.blue_ink": 0xa1c3b6ff, "palette.blue_soft": 0x474b46ff, "palette.blue_border": 0x58635dff,
                "palette.violet": 0xd3869bff, "palette.violet_ink": 0xf3a4b9ff, "palette.violet_soft": 0x574447ff, "palette.violet_border": 0x79565eff,
                "palette.teal": 0x8ec07cff, "palette.teal_ink": 0x9ccf8aff, "palette.teal_soft": 0x495041ff, "palette.teal_border": 0x5c6f51ff,
                "palette.green": 0xb8bb26ff, "palette.green_ink": 0xc5c93aff, "palette.green_soft": 0x524f30ff, "palette.green_border": 0x6e6d2dff,
                "palette.amber": 0xfabd2fff, "palette.amber_ink": 0xfabd2fff, "palette.amber_soft": 0x5f4f31ff, "palette.amber_border": 0x896e31ff,
                "palette.rose": 0xfb4934ff, "palette.rose_ink": 0xff9482ff, "palette.rose_soft": 0x5f3832ff, "palette.rose_border": 0x8a3d33ff,
                "palette.orange": 0xfe8019ff, "palette.orange_ink": 0xffa66eff, "palette.orange_soft": 0x60432dff, "palette.orange_border": 0x8b5428ff,
                "palette.slate": 0xa89984ff, "palette.slate_ink": 0xc8b8a2ff, "palette.slate_soft": 0x4e4842ff, "palette.slate_border": 0x675e54ff,
            },
        },
    }
}
