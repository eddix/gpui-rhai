// Semantic adaptation of Omarchy Hackerman (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/hackerman
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - success hue moved 43 deg away from the accent/status hues.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Hackerman",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x05060eff,
                surface: 0x0b0c16ff,
                surface_raised: 0x161821ff,
                surface_elevated: 0x20222bff,
                surface_hover: 0x23252fff,
                text_primary: 0xd3edf5ff,
                text_muted: 0xaebadaff,
                accent: 0x82fb9cff,
                accent_hover: 0xacffbbff,
                on_accent: 0x0b0c16ff,
                danger: 0xff667dff,
                on_danger: 0x0b0c16ff,
                warning: 0x50f7d4ff,
                on_warning: 0x0b0c16ff,
                success: 0x7de84fff,
                on_success: 0x0b0c16ff,
                border_subtle: 0x2b2f38ff,
                border: 0x484a51ff,
                border_strong: 0x676e75ff,
                focus_ring: 0x7cf8f7ff,
                selection: 0x011e13ff,
                disabled: 0x676871ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x60a5faff, "palette.blue_soft": 0x25344cff, "palette.blue_border": 0x35537cff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xa78bfaff, "palette.violet_soft": 0x332f4cff, "palette.violet_border": 0x53487cff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x1b3e41ff, "palette.teal_border": 0x206763ff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x204034ff, "palette.green_border": 0x2c6b49ff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x443922ff, "palette.amber_border": 0x765e22ff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xfb7185ff, "palette.rose_soft": 0x442a35ff, "palette.rose_border": 0x763d4bff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x443026ff, "palette.orange_border": 0x764b2cff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x94a3b8ff, "palette.slate_soft": 0x2f343fff, "palette.slate_border": 0x4b5260ff,
            },
        },
    }
}
