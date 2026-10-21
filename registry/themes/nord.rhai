// Semantic adaptation of Omarchy Nord (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/nord
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Nord", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x2e3440ff, surface_raised: 0x3c414dff, surface_hover: 0x4b505cff,
            text_primary: 0xeaf0fbff, text_muted: 0xc9d1deff,
            accent: 0x81a1c1ff, accent_hover: 0x8cacccff, on_accent: 0x222730ff,
            danger: 0xbf616aff, on_danger: 0x000000ff,
            warning: 0xebcb8bff, on_warning: 0x222730ff,
            success: 0xa3be8cff, on_success: 0x222730ff,
            border: 0x6a6e78ff, focus_ring: 0x88c0d0ff,
            selection: 0x364253ff, disabled: 0x959aa3ff,
        },
    } }
}
