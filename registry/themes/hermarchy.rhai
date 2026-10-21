// Semantic adaptation of Hermarchy by Archer Clawbot (MIT).
// Source: https://github.com/archer-clawbot/omarchy-hermarchy-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Hermarchy", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x08090aff, surface_raised: 0x131415ff, surface_hover: 0x202122ff,
            text_primary: 0xe8e8e3ff, text_muted: 0xb9bbbbff,
            accent: 0x61d6ffff, accent_hover: 0x89dfffff, on_accent: 0x08090aff,
            danger: 0xe46e6eff, on_danger: 0x08090aff,
            warning: 0xe2c275ff, on_warning: 0x08090aff,
            success: 0x86d993ff, on_success: 0x08090aff,
            border: 0x474849ff, focus_ring: 0x8de4ffff,
            selection: 0x001720ff, disabled: 0x626364ff,
        },
    } }
}
