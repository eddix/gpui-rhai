// Semantic adaptation of Absolutely Light, a built-in theme of the OpenAI Codex app.
// Anchors (background, surface, foreground, accent) as republished in
// https://github.com/shaw-baobao/codex-themes (MIT).
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - the accent darkens at its hue: the source terracotta is below 3:1 on the page and
//   carries white at 3:1.
// - danger is rose and warning amber, 30 deg from the accent; the bright amber carries dark
//   ink, which keeps it amber rather than olive.
// - hover is one Oklab step below the page; selection keeps a terracotta tint, lighter than
//   hover so the two differ.
// - text_muted and border darken so secondary text reads at 5.2:1 and dividers show.
fn theme() {
    #{ family: "Absolutely", name: "Light", mode: "light", tokens: #{
        colors: #{
            surface: 0xf9f9f7ff, surface_raised: 0xffffffff, surface_hover: 0xe4e4e2ff,
            text_primary: 0x2d2d2bff, text_muted: 0x575754ff,
            accent: 0xa55a3cff, accent_hover: 0x984e30ff, on_accent: 0xffffffff,
            danger: 0xaf2c53ff, on_danger: 0xffffffff,
            warning: 0xdfb920ff, on_warning: 0x2d2d2bff,
            success: 0x1d7c3aff, on_success: 0xffffffff,
            border: 0xa8a8a5ff, focus_ring: 0x2d2d2bff,
            selection: 0xffeee4ff, disabled: 0x8f8f8cff,
        },
    } }
}
