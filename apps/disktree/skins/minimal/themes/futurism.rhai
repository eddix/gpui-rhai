// Original gpui-rhai semantic palette inspired by Futurism by Bjarne Øverli.
// No Futurism code or artwork is redistributed.
// Inspiration: https://github.com/bjarneo/omarchy-futurism-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - danger hue moved 11 deg away from the accent/status hues.
fn theme() {
    #{
        family: "Futurism",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x040a1cff,
                surface: 0x091225ff,
                surface_raised: 0x151e31ff,
                surface_elevated: 0x1f283cff,
                surface_hover: 0x222c3eff,
                text_primary: 0xdfe9f1ff,
                text_muted: 0xa8bdd3ff,
                accent: 0xf14aa2ff,
                accent_hover: 0xfe56adff,
                on_accent: 0x091225ff,
                danger: 0xff6062ff,
                on_danger: 0x091225ff,
                warning: 0xf2b35dff,
                on_warning: 0x091225ff,
                success: 0x42d6c7ff,
                on_success: 0x091225ff,
                border_subtle: 0x2b3446ff,
                border: 0x484f5eff,
                border_strong: 0x69717eff,
                focus_ring: 0x4dd8ffff,
                selection: 0x300f2fff,
                disabled: 0x687080ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x61a7fcff, "palette.blue_soft": 0x243959ff, "palette.blue_border": 0x355785ff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xac91ffff, "palette.violet_soft": 0x323459ff, "palette.violet_border": 0x524c85ff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x1a424dff, "palette.teal_border": 0x1f6a6dff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x204441ff, "palette.green_border": 0x2b6f52ff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x433e2eff, "palette.amber_border": 0x76622cff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xfe7387ff, "palette.rose_soft": 0x432f42ff, "palette.rose_border": 0x764154ff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x433533ff, "palette.orange_border": 0x764f36ff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x96a6bbff, "palette.slate_soft": 0x2e394cff, "palette.slate_border": 0x4a566aff,
            },
        },
    }
}
