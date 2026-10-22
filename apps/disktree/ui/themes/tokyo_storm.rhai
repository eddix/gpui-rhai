// Tokyo Night Storm semantic adaptation.
// Upstream: https://github.com/folke/tokyonight.nvim (Apache-2.0)
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Tokyo Night",
        name: "Storm",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x24283bff,
                surface_raised: 0x313547ff,
                surface_hover: 0x404456ff,
                text_primary: 0xd2dbffff,
                text_muted: 0xb6bee3ff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x87adffff,
                on_accent: 0x24283bff,
                danger: 0xf7768eff,
                on_danger: 0x24283bff,
                warning: 0xe0af68ff,
                on_warning: 0x24283bff,
                success: 0x9ece6aff,
                on_success: 0x24283bff,
                border: 0x5f6371ff,
                focus_ring: 0x7dcfffff,
                selection: 0x283458ff,
                disabled: 0x888c9bff,
            },
        },
    }
}
