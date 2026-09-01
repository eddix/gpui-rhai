// Semantic adaptation of Omarchy Hackerman (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/hackerman
fn theme() {
    #{ family: "Hackerman", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x0b0c16ff, surface_raised: 0x151828ff, surface_hover: 0x1f253aff,
            text_primary: 0xddf7ffff, text_muted: 0x9aa5c5ff,
            accent: 0x82fb9cff, accent_hover: 0xa5ffb8ff, on_accent: 0x0b0c16ff,
            danger: 0xff667dff, on_danger: 0x0b0c16ff,
            warning: 0x50f7d4ff, on_warning: 0x0b0c16ff,
            success: 0x4fe88fff, on_success: 0x0b0c16ff,
            border: 0x2d3450ff, focus_ring: 0x7cf8f7ff,
            selection: 0x1f253aff, disabled: 0x6a6e95ff,
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
