// Semantic adaptation of Omarchy Nord (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/nord
fn theme() {
    #{ family: "Nord", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x2e3440ff, surface_raised: 0x3b4252ff, surface_hover: 0x434c5eff,
            text_primary: 0xd8dee9ff, text_muted: 0xaab2c0ff,
            accent: 0x81a1c1ff, accent_hover: 0x9ab6d0ff, on_accent: 0x222730ff,
            danger: 0xbf616aff, on_danger: 0x000000ff,
            warning: 0xebcb8bff, on_warning: 0x222730ff,
            success: 0xa3be8cff, on_success: 0x222730ff,
            border: 0x4c566aff, focus_ring: 0x88c0d0ff,
            selection: 0x434c5eff, disabled: 0x7b8494ff,
        },
        spacing: #{
            xs: #{ unit: "pixels", value: 4.0 }, sm: #{ unit: "pixels", value: 8.0 },
            md: #{ unit: "pixels", value: 12.0 }, lg: #{ unit: "pixels", value: 16.0 },
        },
        radii: #{
            sm: #{ unit: "pixels", value: 2.0 }, md: #{ unit: "pixels", value: 4.0 },
            lg: #{ unit: "pixels", value: 6.0 },
        },
    } }
}
