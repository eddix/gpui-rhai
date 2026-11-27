// Semantic adaptation of Omarchy Retro 82 (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/retro-82
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - danger hue moved 17 deg away from the accent/status hues.
// - warning hue moved 35 deg away from the accent/status hues.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Retro 82",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x011025ff,
                surface: 0x05182eff,
                surface_raised: 0x13253aff,
                surface_elevated: 0x1d3045ff,
                surface_hover: 0x213248ff,
                text_primary: 0xf6dcacff,
                text_muted: 0x99bab7ff,
                accent: 0xfaa968ff,
                accent_hover: 0xffb780ff,
                on_accent: 0x031222ff,
                danger: 0xf82531ff,
                on_danger: 0x031222ff,
                warning: 0xe9e03cff,
                on_warning: 0x031222ff,
                success: 0x36a6a4ff,
                on_success: 0x031222ff,
                border_subtle: 0x2c3947ff,
                border: 0x455465ff,
                border_strong: 0x6c7376ff,
                focus_ring: 0x8cbfb8ff,
                selection: 0x282225ff,
                disabled: 0x69788aff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x6cadffff, "palette.blue_soft": 0x223f60ff, "palette.blue_border": 0x335b8bff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xb098ffff, "palette.violet_soft": 0x313960ff, "palette.violet_border": 0x51508bff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x184855ff, "palette.teal_border": 0x1e6f72ff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x1e4a48ff, "palette.green_border": 0x2a7357ff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x414436ff, "palette.amber_border": 0x746631ff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xff7b8dff, "palette.rose_soft": 0x413449ff, "palette.rose_border": 0x74455aff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x413b3aff, "palette.orange_border": 0x74533bff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x9babc0ff, "palette.slate_soft": 0x2d3e53ff, "palette.slate_border": 0x495a6fff,
            },
        },
    }
}
