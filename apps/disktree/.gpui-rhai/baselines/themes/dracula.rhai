// Semantic adaptation of Dracula (MIT).
// Source: https://github.com/dracula/dracula-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - raised and hover are even Oklab steps from the background; hover lands on Current Line.
// - text_muted is a light Comment tint, since Comment itself is below 5.2:1 on hover.
// - danger uses the bright red, which carries the background as its foreground.
// - selection is tinted toward Purple and kept darker than hover so the two differ.
fn theme() {
    #{ family: "Dracula", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x282a36ff, surface_raised: 0x353744ff, surface_hover: 0x424552ff,
            text_primary: 0xf8f8f2ff, text_muted: 0xc2c6dcff,
            accent: 0xbd93f9ff, accent_hover: 0xcaa8fbff, on_accent: 0x282a36ff,
            danger: 0xff6e6eff, on_danger: 0x282a36ff,
            warning: 0xffb86cff, on_warning: 0x282a36ff,
            success: 0x50fa7bff, on_success: 0x282a36ff,
            border: 0x6272a4ff, focus_ring: 0xff79c6ff,
            selection: 0x372e62ff, disabled: 0x7b7f96ff,
        },
    } }
}
