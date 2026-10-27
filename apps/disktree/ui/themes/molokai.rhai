// Semantic adaptation of Molokai by Tomas Restrepo (MIT), after Monokai by Wimer Hazenberg.
// Source: https://github.com/tomasr/molokai
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - Cyan is the accent: the Keyword pink is red enough to be the danger status.
// - danger lifts the Keyword pink at its hue, so the background reads on it at 4.5:1.
// - raised and hover are even Oklab steps from the background.
// - text_muted is a light Comment tint, since Comment itself is below 5.2:1 on hover.
// - selection keeps the warm Visual grey one step lighter than hover so the two differ.
fn theme() {
    #{ family: "Molokai", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x1b1d1eff, surface_raised: 0x27292aff, surface_hover: 0x343638ff,
            text_primary: 0xf8f8f2ff, text_muted: 0xc0c7c8ff,
            accent: 0x66d9efff, accent_hover: 0x86e2f3ff, on_accent: 0x1b1d1eff,
            danger: 0xff3d84ff, on_danger: 0x1b1d1eff,
            warning: 0xfd971fff, on_warning: 0x1b1d1eff,
            success: 0xa6e22eff, on_success: 0x1b1d1eff,
            border: 0x5a676aff, focus_ring: 0xae81ffff,
            selection: 0x464343ff, disabled: 0x7e8e91ff,
        },
    } }
}
