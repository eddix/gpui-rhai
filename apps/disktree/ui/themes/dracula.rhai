// Semantic adaptation of Dracula (MIT).
// Source: https://github.com/dracula/dracula-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - raised and hover are even Oklab steps from the background; hover lands on Current Line.
// - text_muted is a light Comment tint, since Comment itself is below 5.2:1 on hover.
// - danger uses the bright red, which carries the background as its foreground.
// - selection is tinted toward Purple and kept darker than hover so the two differ.
fn theme() {
    #{
        family: "Dracula",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x20222dff,
                surface: 0x282a36ff,
                surface_raised: 0x353744ff,
                surface_elevated: 0x40424fff,
                surface_hover: 0x424552ff,
                text_primary: 0xf8f8f2ff,
                text_muted: 0xc2c6dcff,
                accent: 0xbd93f9ff,
                accent_hover: 0xcaa8fbff,
                on_accent: 0x282a36ff,
                danger: 0xff6e6eff,
                on_danger: 0x282a36ff,
                warning: 0xffb86cff,
                on_warning: 0x282a36ff,
                success: 0x50fa7bff,
                on_success: 0x282a36ff,
                border_subtle: 0x4a4c57ff,
                border: 0x6272a4ff,
                border_strong: 0x838fb5ff,
                focus_ring: 0xff79c6ff,
                selection: 0x372e62ff,
                disabled: 0x7b7f96ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x8be9fdff, "palette.blue_ink": 0x8be9fdff, "palette.blue_soft": 0x465b69ff, "palette.blue_border": 0x598292ff,
                "palette.violet": 0xbd93f9ff, "palette.violet_ink": 0xd1b3ffff, "palette.violet_soft": 0x504968ff, "palette.violet_border": 0x6e5e90ff,
                "palette.teal": 0x50fa7bff, "palette.teal_ink": 0x50fa7bff, "palette.teal_soft": 0x3a5e4fff, "palette.teal_border": 0x40895bff,
                "palette.green": 0x69ff94ff, "palette.green_ink": 0x69ff94ff, "palette.green_soft": 0x3f5f54ff, "palette.green_border": 0x4b8b66ff,
                "palette.amber": 0xf1fa8cff, "palette.amber_ink": 0xf1fa8cff, "palette.amber_soft": 0x5b5e52ff, "palette.amber_border": 0x848962ff,
                "palette.rose": 0xff79c6ff, "palette.rose_ink": 0xffa3d4ff, "palette.rose_soft": 0x5d445eff, "palette.rose_border": 0x8a537bff,
                "palette.orange": 0xffb86cff, "palette.orange_ink": 0xffbd77ff, "palette.orange_soft": 0x5d514cff, "palette.orange_border": 0x8a6d55ff,
                "palette.slate": 0x95a5d4ff, "palette.slate_ink": 0xb0c1f1ff, "palette.slate_soft": 0x484d61ff, "palette.slate_border": 0x5d6580ff,
            },
        },
    }
}
