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
            sm: #{ unit: "pixels", value: 0.0 }, md: #{ unit: "pixels", value: 0.0 },
            lg: #{ unit: "pixels", value: 0.0 },
        },
        typography: #{
            roles: #{
                caption: #{ size: #{ unit: "pixels", value: 10.0 },
                    line_height: #{ unit: "pixels", value: 14.0 }, weight: 400 },
                body_small: #{ size: #{ unit: "pixels", value: 11.0 },
                    line_height: #{ unit: "pixels", value: 14.0 }, weight: 400 },
                body: #{ size: #{ unit: "pixels", value: 12.0 },
                    line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                subtitle: #{ size: #{ unit: "pixels", value: 13.0 },
                    line_height: #{ unit: "pixels", value: 18.0 }, weight: 400 },
                title: #{ size: #{ unit: "pixels", value: 14.0 },
                    line_height: #{ unit: "pixels", value: 20.0 }, weight: 700 },
                heading: #{ size: #{ unit: "pixels", value: 16.0 },
                    line_height: #{ unit: "pixels", value: 22.0 }, weight: 700 },
                display: #{ size: #{ unit: "pixels", value: 24.0 },
                    line_height: #{ unit: "pixels", value: 30.0 }, weight: 700 },
                display_large: #{ size: #{ unit: "pixels", value: 28.0 },
                    line_height: #{ unit: "pixels", value: 34.0 }, weight: 700 },
            },
        },
    } }
}
