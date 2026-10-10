// Tokyo Night Night semantic adaptation.
// Upstream: https://github.com/folke/tokyonight.nvim (Apache-2.0)
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Tokyo Night",
        name: "Night",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x1a1b26ff,
                surface_raised: 0x24283bff,
                surface_hover: 0x292e42ff,
                text_primary: 0xc0caf5ff,
                text_muted: 0xa9b1d6ff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x89b4faff,
                on_accent: 0x1a1b26ff,
                danger: 0xf7768eff,
                on_danger: 0x1a1b26ff,
                warning: 0xe0af68ff,
                on_warning: 0x1a1b26ff,
                success: 0x9ece6aff,
                on_success: 0x1a1b26ff,
                border: 0x3b4261ff,
                focus_ring: 0x7dcfffff,
                selection: 0x333c58ff,
                disabled: 0x565f89ff,
            },
        },
    }
}
