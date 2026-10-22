// Semantic adaptation of Omarchy Retro 82 (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/retro-82
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - danger hue moved 17 deg away from the accent/status hues.
// - warning hue moved 35 deg away from the accent/status hues.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{ family: "Retro 82", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x05182eff, surface_raised: 0x13253aff, surface_hover: 0x213248ff,
            text_primary: 0xf6dcacff, text_muted: 0x99bab7ff,
            accent: 0xfaa968ff, accent_hover: 0xffb780ff, on_accent: 0x031222ff,
            danger: 0xf82531ff, on_danger: 0x031222ff,
            warning: 0xe9e03cff, on_warning: 0x031222ff,
            success: 0x36a6a4ff, on_success: 0x031222ff,
            border: 0x455465ff, focus_ring: 0x8cbfb8ff,
            selection: 0x282225ff, disabled: 0x69788aff,
        },
    } }
}
