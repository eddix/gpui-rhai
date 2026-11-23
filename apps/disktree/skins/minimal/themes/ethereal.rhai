// Semantic adaptation of Omarchy Ethereal (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/ethereal
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Ethereal", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x060b1eff, surface_raised: 0x111729ff, surface_hover: 0x1e2437ff,
            text_primary: 0xffceadff, text_muted: 0xa5abc4ff,
            accent: 0x7d82d9ff, accent_hover: 0x878de5ff, on_accent: 0x060b1eff,
            danger: 0xed5b5aff, on_danger: 0x060b1eff,
            warning: 0xe9bb4fff, on_warning: 0x060b1eff,
            success: 0x92a593ff, on_success: 0x060b1eff,
            border: 0x444a59ff, focus_ring: 0xa3bfd1ff,
            selection: 0x101336ff, disabled: 0x616878ff,
        },
    } }
}
