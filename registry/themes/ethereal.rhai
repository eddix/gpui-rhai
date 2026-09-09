// Semantic adaptation of Omarchy Ethereal (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/ethereal
fn theme() {
    #{ family: "Ethereal", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x060b1eff, surface_raised: 0x131a3aff, surface_hover: 0x252e56ff,
            text_primary: 0xffceadff, text_muted: 0xa3a9c2ff,
            accent: 0x7d82d9ff, accent_hover: 0x9aa0efff, on_accent: 0x060b1eff,
            danger: 0xed5b5aff, on_danger: 0x060b1eff,
            warning: 0xe9bb4fff, on_warning: 0x060b1eff,
            success: 0x92a593ff, on_success: 0x060b1eff,
            border: 0x4a5685ff, focus_ring: 0xa3bfd1ff,
            selection: 0x252e56ff, disabled: 0x6d7db6ff,
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
                caption: #{ size: #{ unit: "pixels", value: 11.0 },
                    line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                body_small: #{ size: #{ unit: "pixels", value: 12.0 },
                    line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                body: #{ size: #{ unit: "pixels", value: 13.0 },
                    line_height: #{ unit: "pixels", value: 18.0 }, weight: 400 },
                subtitle: #{ size: #{ unit: "pixels", value: 14.0 },
                    line_height: #{ unit: "pixels", value: 20.0 }, weight: 400 },
                title: #{ size: #{ unit: "pixels", value: 16.0 },
                    line_height: #{ unit: "pixels", value: 22.0 }, weight: 700 },
                heading: #{ size: #{ unit: "pixels", value: 18.0 },
                    line_height: #{ unit: "pixels", value: 24.0 }, weight: 700 },
                display: #{ size: #{ unit: "pixels", value: 24.0 },
                    line_height: #{ unit: "pixels", value: 32.0 }, weight: 700 },
                display_large: #{ size: #{ unit: "pixels", value: 28.0 },
                    line_height: #{ unit: "pixels", value: 36.0 }, weight: 700 },
            },
        },
    } }
}
