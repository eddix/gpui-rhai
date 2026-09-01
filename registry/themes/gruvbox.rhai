// Semantic adaptation of Omarchy Gruvbox (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/gruvbox
fn theme() {
    #{ family: "Gruvbox", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x282828ff, surface_raised: 0x3c3836ff, surface_hover: 0x504945ff,
            text_primary: 0xd4be98ff, text_muted: 0xa89984ff,
            accent: 0x7daea3ff, accent_hover: 0x9cc7bdff, on_accent: 0x1e1e1eff,
            danger: 0xea6962ff, on_danger: 0x1e1e1eff,
            warning: 0xd8a657ff, on_warning: 0x1e1e1eff,
            success: 0xa9b665ff, on_success: 0x1e1e1eff,
            border: 0x665c54ff, focus_ring: 0x89b482ff,
            selection: 0x504945ff, disabled: 0x7c6f64ff,
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
