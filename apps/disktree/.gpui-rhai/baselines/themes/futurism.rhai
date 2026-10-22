// Original gpui-rhai semantic palette inspired by Futurism by Bjarne Øverli.
// No Futurism code or artwork is redistributed.
// Inspiration: https://github.com/bjarneo/omarchy-futurism-theme
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - danger hue moved 11 deg away from the accent/status hues.
fn theme() {
    #{ family: "Futurism", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x091225ff, surface_raised: 0x151e31ff, surface_hover: 0x222c3eff,
            text_primary: 0xdfe9f1ff, text_muted: 0xa8bdd3ff,
            accent: 0xf14aa2ff, accent_hover: 0xfe56adff, on_accent: 0x091225ff,
            danger: 0xff6062ff, on_danger: 0x091225ff,
            warning: 0xf2b35dff, on_warning: 0x091225ff,
            success: 0x42d6c7ff, on_success: 0x091225ff,
            border: 0x484f5eff, focus_ring: 0x4dd8ffff,
            selection: 0x300f2fff, disabled: 0x687080ff,
        },
    } }
}
