// Semantic adaptation of Omarchy Retro 82 (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/retro-82
fn theme() {
    #{ family: "Retro 82", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x05182eff, surface_raised: 0x0a2540ff, surface_hover: 0x134e5aff,
            text_primary: 0xf6dcacff, text_muted: 0xa7c9c6ff,
            accent: 0xfaa968ff, accent_hover: 0xffc28eff, on_accent: 0x031222ff,
            danger: 0xf85525ff, on_danger: 0x031222ff,
            warning: 0xe97b3cff, on_warning: 0x031222ff,
            success: 0x36a6a4ff, on_success: 0x031222ff,
            border: 0x2a6b78ff, focus_ring: 0x8cbfb8ff,
            selection: 0x134e5aff, disabled: 0x5e9694ff,
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
