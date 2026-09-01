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
            sm: #{ unit: "pixels", value: 2.0 }, md: #{ unit: "pixels", value: 4.0 },
            lg: #{ unit: "pixels", value: 6.0 },
        },
    } }
}
