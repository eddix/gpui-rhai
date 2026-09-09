// Original gpui-rhai semantic palette inspired by Futurism by Bjarne Øverli.
// No Futurism code or artwork is redistributed.
// Inspiration: https://github.com/bjarneo/omarchy-futurism-theme
fn theme() {
    #{ family: "Futurism", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x091225ff, surface_raised: 0x142443ff, surface_hover: 0x20375eff,
            text_primary: 0xedf7ffff, text_muted: 0x9aaec4ff,
            accent: 0xf14aa2ff, accent_hover: 0xff70b8ff, on_accent: 0x091225ff,
            danger: 0xff607fff, on_danger: 0x091225ff,
            warning: 0xf2b35dff, on_warning: 0x091225ff,
            success: 0x42d6c7ff, on_success: 0x091225ff,
            border: 0x385071ff, focus_ring: 0x4dd8ffff,
            selection: 0x1a3155ff, disabled: 0x647891ff,
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
