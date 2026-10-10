// Semantic adaptation of Omarchy Gruvbox (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/gruvbox
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Gruvbox", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x282828ff, surface_raised: 0x3c3836ff, surface_hover: 0x504945ff,
            text_primary: 0xd4be98ff, text_muted: 0xccb794ff,
            accent: 0x7daea3ff, accent_hover: 0x9cc7bdff, on_accent: 0x1e1e1eff,
            danger: 0xea6962ff, on_danger: 0x1e1e1eff,
            warning: 0xd8a657ff, on_warning: 0x1e1e1eff,
            success: 0xa9b665ff, on_success: 0x1e1e1eff,
            border: 0x665c54ff, focus_ring: 0x89b482ff,
            selection: 0x323837ff, disabled: 0x7c6f64ff,
        },
    } }
}
