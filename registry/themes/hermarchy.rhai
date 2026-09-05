// Semantic adaptation of Hermarchy by Archer Clawbot (MIT).
// Source: https://github.com/archer-clawbot/omarchy-hermarchy-theme
fn theme() {
    #{ family: "Hermarchy", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x08090aff, surface_raised: 0x101214ff, surface_hover: 0x26333aff,
            text_primary: 0xf1f1ecff, text_muted: 0xa5a7a7ff,
            accent: 0x61d6ffff, accent_hover: 0x8de4ffff, on_accent: 0x08090aff,
            danger: 0xe46e6eff, on_danger: 0x08090aff,
            warning: 0xe2c275ff, on_warning: 0x08090aff,
            success: 0x86d993ff, on_success: 0x08090aff,
            border: 0x3b4044ff, focus_ring: 0x8de4ffff,
            selection: 0x26333aff, disabled: 0x606468ff,
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
