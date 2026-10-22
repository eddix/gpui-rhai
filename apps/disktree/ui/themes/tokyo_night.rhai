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
                surface_raised: 0x262732ff,
                surface_hover: 0x343540ff,
                text_primary: 0xc0caf5ff,
                text_muted: 0xa1a9ceff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x87adffff,
                on_accent: 0x1a1b26ff,
                danger: 0xf7768eff,
                on_danger: 0x1a1b26ff,
                warning: 0xe0af68ff,
                on_warning: 0x1a1b26ff,
                success: 0x9ece6aff,
                on_success: 0x1a1b26ff,
                border: 0x54555eff,
                focus_ring: 0x7dcfffff,
                selection: 0x1d2743ff,
                disabled: 0x7a7b85ff,
            },
        },
    }
}
