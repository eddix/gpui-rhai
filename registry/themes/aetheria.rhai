// Semantic adaptation of Aetheria by Dizziee (MIT).
// Source: https://github.com/JJDizz1L/aetheria
fn theme() {
    #{ family: "Aetheria", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x0e091dff, surface_raised: 0x1c1230ff, surface_hover: 0x2a1840ff,
            text_primary: 0x4cd7d2ff, text_muted: 0xa79ab7ff,
            accent: 0xbe3f50ff, accent_hover: 0xe05a6bff, on_accent: 0xfff2e8ff,
            danger: 0xe20342ff, on_danger: 0xffffffff,
            warning: 0xffbe74ff, on_warning: 0x0e091dff,
            success: 0x7cd699ff, on_success: 0x0e091dff,
            border: 0x4b275fff, focus_ring: 0xff7f41ff,
            selection: 0x183f4aff, disabled: 0x6f6380ff,
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
