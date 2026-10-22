// Semantic adaptation of Aetheria by Dizziee (MIT).
// Source: https://github.com/JJDizz1L/aetheria
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - danger hue moved 22 deg away from the accent/status hues.
fn theme() {
    #{ family: "Aetheria", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x0e091dff, surface_raised: 0x1c1230ff, surface_hover: 0x2a1840ff,
            text_primary: 0x4cd7d2ff, text_muted: 0x9aa3bbff,
            accent: 0xbe3f50ff, accent_hover: 0xe05a6bff, on_accent: 0xfff2e8ff,
            danger: 0xdd0391ff, on_danger: 0xffffffff,
            warning: 0xffbe74ff, on_warning: 0x0e091dff,
            success: 0x7cd699ff, on_success: 0x0e091dff,
            border: 0x4b275fff, focus_ring: 0xff7f41ff,
            selection: 0x183f4aff, disabled: 0x6f6380ff,
        },
    } }
}
