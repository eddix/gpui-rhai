// Semantic adaptation of Omarchy Gruvbox (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/gruvbox
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Gruvbox", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x282828ff, surface_raised: 0x383432ff, surface_hover: 0x48423fff,
            text_primary: 0xedd7b0ff, text_muted: 0xd0bb97ff,
            accent: 0x7daea3ff, accent_hover: 0x88b9aeff, on_accent: 0x1e1e1eff,
            danger: 0xea6962ff, on_danger: 0x1e1e1eff,
            warning: 0xd8a657ff, on_warning: 0x1e1e1eff,
            success: 0xa9b665ff, on_success: 0x1e1e1eff,
            border: 0x69615aff, focus_ring: 0x89b482ff,
            selection: 0x2c3835ff, disabled: 0x948980ff,
        },
    } }
}
