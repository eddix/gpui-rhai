// Semantic adaptation of Absolutely Dark, a built-in theme of the OpenAI Codex app.
// Anchors (background, foreground, accent) as republished in
// https://github.com/0xSero/litter and https://github.com/shaw-baobao/codex-themes (MIT).
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - raised and hover are even Oklab steps from the Codex background.
// - danger is rose and warning amber: red and orange sit within 30 deg of the terracotta
//   accent, which would make every highlight look like an alert.
// - on_accent is dark ink: white on the terracotta is 3:1.
// - selection is a terracotta-tinted step lighter than hover, and text_muted is light
//   enough to read on it at 5.2:1.
fn theme() {
    #{
        family: "Absolutely",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x242423ff,
                surface: 0x2d2d2bff,
                surface_raised: 0x3a3a38ff,
                surface_elevated: 0x454543ff,
                surface_hover: 0x484846ff,
                text_primary: 0xf9f9f7ff,
                text_muted: 0xd8d8d5ff,
                accent: 0xcc7d5eff,
                accent_hover: 0xe59677ff,
                on_accent: 0x1f1f1dff,
                danger: 0xf0759aff,
                on_danger: 0x1f1f1dff,
                warning: 0xe3c452ff,
                on_warning: 0x1f1f1dff,
                success: 0x00c853ff,
                on_success: 0x1f1f1dff,
                border_subtle: 0x4f4f4dff,
                border: 0x6a6a67ff,
                border_strong: 0x898987ff,
                focus_ring: 0xf9f9f7ff,
                selection: 0x664e43ff,
                disabled: 0x8c8c89ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x96c5ffff, "palette.blue_soft": 0x424f5fff, "palette.blue_border": 0x4a6789ff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xc6b7ffff, "palette.violet_soft": 0x504a5fff, "palette.violet_border": 0x685c89ff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x40e0caff, "palette.teal_soft": 0x375953ff, "palette.teal_border": 0x357b71ff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x53e687ff, "palette.green_soft": 0x3d5b46ff, "palette.green_border": 0x417f56ff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xffc537ff, "palette.amber_soft": 0x615534ff, "palette.amber_border": 0x8b7230ff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xffa7afff, "palette.rose_soft": 0x614547ff, "palette.rose_border": 0x8b5158ff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xffb580ff, "palette.orange_soft": 0x614c39ff, "palette.orange_border": 0x8b5f3aff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0xb4c4daff, "palette.slate_soft": 0x4c4f52ff, "palette.slate_border": 0x60666eff,
            },
        },
    }
}
