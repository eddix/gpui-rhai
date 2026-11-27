// Semantic adaptation of Molokai by Tomas Restrepo (MIT), after Monokai by Wimer Hazenberg.
// Source: https://github.com/tomasr/molokai
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - Cyan is the accent: the Keyword pink is red enough to be the danger status.
// - danger lifts the Keyword pink at its hue, so the background reads on it at 4.5:1.
// - raised and hover are even Oklab steps from the background.
// - text_muted is a light Comment tint, since Comment itself is below 5.2:1 on hover.
// - selection keeps the warm Visual grey one step lighter than hover so the two differ.
fn theme() {
    #{
        family: "Molokai",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x131516ff,
                surface: 0x1b1d1eff,
                surface_raised: 0x27292aff,
                surface_elevated: 0x313335ff,
                surface_hover: 0x343638ff,
                text_primary: 0xf8f8f2ff,
                text_muted: 0xc0c7c8ff,
                accent: 0x66d9efff,
                accent_hover: 0x86e2f3ff,
                on_accent: 0x1b1d1eff,
                danger: 0xff3d84ff,
                on_danger: 0x1b1d1eff,
                warning: 0xfd971fff,
                on_warning: 0x1b1d1eff,
                success: 0xa6e22eff,
                on_success: 0x1b1d1eff,
                border_subtle: 0x3e4040ff,
                border: 0x5a676aff,
                border_strong: 0x7d8788ff,
                focus_ring: 0xae81ffff,
                selection: 0x464343ff,
                disabled: 0x7e8e91ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x66d9efff, "palette.blue_ink": 0x66d9efff, "palette.blue_soft": 0x344c51ff, "palette.blue_border": 0x41737dff,
                "palette.violet": 0xae81ffff, "palette.violet_ink": 0xbc9bffff, "palette.violet_soft": 0x423b55ff, "palette.violet_border": 0x604e83ff,
                "palette.teal": 0x52d6b3ff, "palette.teal_ink": 0x52d6b3ff, "palette.teal_soft": 0x304c45ff, "palette.teal_border": 0x397264ff,
                "palette.green": 0xa6e22eff, "palette.green_ink": 0xa6e22eff, "palette.green_soft": 0x404e2bff, "palette.green_border": 0x5c772cff,
                "palette.amber": 0xe6db74ff, "palette.amber_ink": 0xe6db74ff, "palette.amber_soft": 0x4d4d39ff, "palette.amber_border": 0x777449ff,
                "palette.rose": 0xf92672ff, "palette.rose_ink": 0xff7092ff, "palette.rose_soft": 0x512838ff, "palette.rose_border": 0x7f2848ff,
                "palette.orange": 0xfd971fff, "palette.orange_ink": 0xff9922ff, "palette.orange_soft": 0x523f28ff, "palette.orange_border": 0x815725ff,
                "palette.slate": 0xa6b2b5ff, "palette.slate_ink": 0xa8b5b8ff, "palette.slate_soft": 0x404446ff, "palette.slate_border": 0x5c6364ff,
            },
        },
    }
}
