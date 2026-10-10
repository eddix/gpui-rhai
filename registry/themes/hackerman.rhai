// Semantic adaptation of Omarchy Hackerman (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/hackerman
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - success hue moved 43 deg away from the accent/status hues.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Hackerman", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x0b0c16ff, surface_raised: 0x151828ff, surface_hover: 0x1f253aff,
            text_primary: 0xddf7ffff, text_muted: 0x9aa5c5ff,
            accent: 0x82fb9cff, accent_hover: 0xa5ffb8ff, on_accent: 0x0b0c16ff,
            danger: 0xff667dff, on_danger: 0x0b0c16ff,
            warning: 0x50f7d4ff, on_warning: 0x0b0c16ff,
            success: 0x7de84fff, on_success: 0x0b0c16ff,
            border: 0x2d3450ff, focus_ring: 0x7cf8f7ff,
            selection: 0x2b3f46ff, disabled: 0x6a6e95ff,
        },
    } }
}
