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
    #{ family: "Absolutely", name: "Dark", mode: "dark", tokens: #{
        colors: #{
            surface: 0x2d2d2bff, surface_raised: 0x3a3a38ff, surface_hover: 0x484846ff,
            text_primary: 0xf9f9f7ff, text_muted: 0xd8d8d5ff,
            accent: 0xcc7d5eff, accent_hover: 0xe59677ff, on_accent: 0x1f1f1dff,
            danger: 0xf0759aff, on_danger: 0x1f1f1dff,
            warning: 0xe3c452ff, on_warning: 0x1f1f1dff,
            success: 0x00c853ff, on_success: 0x1f1f1dff,
            border: 0x6a6a67ff, focus_ring: 0xf9f9f7ff,
            selection: 0x664e43ff, disabled: 0x8c8c89ff,
        },
    } }
}
