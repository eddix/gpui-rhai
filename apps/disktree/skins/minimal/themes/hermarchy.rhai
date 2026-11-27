// Semantic adaptation of Hermarchy by Archer Clawbot (MIT).
// Source: https://github.com/archer-clawbot/omarchy-hermarchy-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Hermarchy",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x040506ff,
                surface: 0x08090aff,
                surface_raised: 0x131415ff,
                surface_elevated: 0x1d1e1fff,
                surface_hover: 0x202122ff,
                text_primary: 0xe8e8e3ff,
                text_muted: 0xb9bbbbff,
                accent: 0x61d6ffff,
                accent_hover: 0x89dfffff,
                on_accent: 0x08090aff,
                danger: 0xe46e6eff,
                on_danger: 0x08090aff,
                warning: 0xe2c275ff,
                on_warning: 0x08090aff,
                success: 0x86d993ff,
                on_success: 0x08090aff,
                border_subtle: 0x2a2b2cff,
                border: 0x474849ff,
                border_strong: 0x6a6b6bff,
                focus_ring: 0x8de4ffff,
                selection: 0x001720ff,
                disabled: 0x626364ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x60a5faff, "palette.blue_soft": 0x223143ff, "palette.blue_border": 0x335175ff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xa78bfaff, "palette.violet_soft": 0x312c43ff, "palette.violet_border": 0x514675ff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x183a37ff, "palette.teal_border": 0x1e655cff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x1e3c2aff, "palette.green_border": 0x2a6942ff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x413618ff, "palette.amber_border": 0x745c1bff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xfb7185ff, "palette.rose_soft": 0x41272bff, "palette.rose_border": 0x743b44ff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x412d1dff, "palette.orange_border": 0x744925ff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x94a3b8ff, "palette.slate_soft": 0x2d3136ff, "palette.slate_border": 0x495059ff,
            },
        },
    }
}
