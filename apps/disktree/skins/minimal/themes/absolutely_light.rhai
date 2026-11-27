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
    #{
        family: "Absolutely",
        name: "Light",
        mode: "light",
        tokens: #{
            colors: #{
                surface_sunken: 0xf1f1efff,
                surface: 0xf9f9f7ff,
                surface_raised: 0xffffffff,
                surface_elevated: 0xffffffff,
                surface_hover: 0xe4e4e2ff,
                text_primary: 0x2d2d2bff,
                text_muted: 0x575754ff,
                accent: 0xa55a3cff,
                accent_hover: 0x984e30ff,
                on_accent: 0xffffffff,
                danger: 0xaf2c53ff,
                on_danger: 0xffffffff,
                warning: 0xdfb920ff,
                on_warning: 0x2d2d2bff,
                success: 0x1d7c3aff,
                on_success: 0xffffffff,
                border_subtle: 0xedededff,
                border: 0xa8a8a5ff,
                border_strong: 0x8c8b89ff,
                focus_ring: 0x2d2d2bff,
                selection: 0xffeee4ff,
                disabled: 0x8f8f8cff,
                "shadow.ambient": 0x0000000d,
                "shadow.key": 0x00000018,
                "palette.blue": 0x2563ebff, "palette.blue_ink": 0x215ee6ff, "palette.blue_soft": 0xe5ecfdff, "palette.blue_border": 0xb5caf8ff,
                "palette.violet": 0x7c3aedff, "palette.violet_ink": 0x7c3aedff, "palette.violet_soft": 0xefe7fdff, "palette.violet_border": 0xd2bcf9ff,
                "palette.teal": 0x0d9488ff, "palette.teal_ink": 0x00786eff, "palette.teal_soft": 0xe2f2f1ff, "palette.teal_border": 0xaddbd7ff,
                "palette.green": 0x16a34aff, "palette.green_ink": 0x007c34ff, "palette.green_soft": 0xe3f4e9ff, "palette.green_border": 0xb0e0c1ff,
                "palette.amber": 0xd97706ff, "palette.amber_ink": 0xa45800ff, "palette.amber_soft": 0xfaefe1ff, "palette.amber_border": 0xf2d1aaff,
                "palette.rose": 0xe11d48ff, "palette.rose_ink": 0xcf003dff, "palette.rose_soft": 0xfbe4e9ff, "palette.rose_border": 0xf5b2c1ff,
                "palette.orange": 0xea580cff, "palette.orange_ink": 0xbb4200ff, "palette.orange_soft": 0xfcebe2ff, "palette.orange_border": 0xf8c6acff,
                "palette.slate": 0x64748bff, "palette.slate_ink": 0x5c6b82ff, "palette.slate_soft": 0xeceef1ff, "palette.slate_border": 0xcad0d8ff,
            },
        },
    }
}
