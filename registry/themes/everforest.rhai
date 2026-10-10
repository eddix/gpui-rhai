// Semantic adaptation of Omarchy Everforest (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/everforest
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Everforest", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x2d353bff, surface_raised: 0x343f44ff, surface_hover: 0x3d484dff,
            text_primary: 0xd3c6aaff, text_muted: 0xb2b5a6ff,
            accent: 0x7fbbb3ff, accent_hover: 0x9acbc4ff, on_accent: 0x21272cff,
            danger: 0xe67e80ff, on_danger: 0x21272cff,
            warning: 0xdbbc7fff, on_warning: 0x21272cff,
            success: 0xa7c080ff, on_success: 0x21272cff,
            border: 0x59656aff, focus_ring: 0x83c092ff,
            selection: 0x374549ff, disabled: 0x7a8581ff,
        },
    } }
}
