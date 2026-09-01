// Semantic adaptation of Omarchy Everforest (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/everforest
fn theme() {
    #{ family: "Everforest", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x2d353bff, surface_raised: 0x343f44ff, surface_hover: 0x3d484dff,
            text_primary: 0xd3c6aaff, text_muted: 0xa7b0a4ff,
            accent: 0x7fbbb3ff, accent_hover: 0x9acbc4ff, on_accent: 0x21272cff,
            danger: 0xe67e80ff, on_danger: 0x21272cff,
            warning: 0xdbbc7fff, on_warning: 0x21272cff,
            success: 0xa7c080ff, on_success: 0x21272cff,
            border: 0x59656aff, focus_ring: 0x83c092ff,
            selection: 0x3d484dff, disabled: 0x7a8581ff,
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
