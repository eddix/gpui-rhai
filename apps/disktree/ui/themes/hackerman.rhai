// Semantic adaptation of Omarchy Hackerman (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/hackerman
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - success hue moved 43 deg away from the accent/status hues.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Hackerman", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x0b0c16ff, surface_raised: 0x161821ff, surface_hover: 0x23252fff,
            text_primary: 0xd3edf5ff, text_muted: 0xaebadaff,
            accent: 0x82fb9cff, accent_hover: 0xacffbbff, on_accent: 0x0b0c16ff,
            danger: 0xff667dff, on_danger: 0x0b0c16ff,
            warning: 0x50f7d4ff, on_warning: 0x0b0c16ff,
            success: 0x7de84fff, on_success: 0x0b0c16ff,
            border: 0x484a51ff, focus_ring: 0x7cf8f7ff,
            selection: 0x011e13ff, disabled: 0x676871ff,
        },
    } }
}
