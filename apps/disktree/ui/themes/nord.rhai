// Semantic adaptation of Omarchy Nord (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/nord
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Nord",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x252b37ff,
                surface: 0x2e3440ff,
                surface_raised: 0x3c414dff,
                surface_elevated: 0x474c59ff,
                surface_hover: 0x4b505cff,
                text_primary: 0xeaf0fbff,
                text_muted: 0xc9d1deff,
                accent: 0x81a1c1ff,
                accent_hover: 0x8cacccff,
                on_accent: 0x222730ff,
                danger: 0xbf616aff,
                on_danger: 0x000000ff,
                warning: 0xebcb8bff,
                on_warning: 0x222730ff,
                success: 0xa3be8cff,
                on_success: 0x222730ff,
                border_subtle: 0x4f5460ff,
                border: 0x6a6e78ff,
                border_strong: 0x8b919bff,
                focus_ring: 0x88c0d0ff,
                selection: 0x364253ff,
                disabled: 0x959aa3ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x81a1c1ff, "palette.blue_ink": 0xacceefff, "palette.blue_soft": 0x4a5464ff, "palette.blue_border": 0x59697eff,
                "palette.violet": 0xb48eadff, "palette.violet_ink": 0xe4bcdcff, "palette.violet_soft": 0x545060ff, "palette.violet_border": 0x6e6175ff,
                "palette.teal": 0x88c0d0ff, "palette.teal_ink": 0xa2daebff, "palette.teal_soft": 0x4b5a67ff, "palette.teal_border": 0x5c7684ff,
                "palette.green": 0xa3be8cff, "palette.green_ink": 0xbedaa6ff, "palette.green_soft": 0x515a5aff, "palette.green_border": 0x677667ff,
                "palette.amber": 0xebcb8bff, "palette.amber_ink": 0xf6d595ff, "palette.amber_soft": 0x5f5d59ff, "palette.amber_border": 0x867b67ff,
                "palette.rose": 0xcf6f77ff, "palette.rose_ink": 0xffacb2ff, "palette.rose_soft": 0x594a55ff, "palette.rose_border": 0x7a545fff,
                "palette.orange": 0xd08770ff, "palette.orange_ink": 0xffb7a0ff, "palette.orange_soft": 0x5a4f54ff, "palette.orange_border": 0x7a5e5cff,
                "palette.slate": 0x9aa6baff, "palette.slate_ink": 0xc1cde2ff, "palette.slate_soft": 0x4f5563ff, "palette.slate_border": 0x636b7bff,
            },
        },
    }
}
