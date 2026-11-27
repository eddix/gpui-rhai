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
                surface_sunken: 0x1c1f32ff,
                surface: 0x24283bff,
                surface_raised: 0x313547ff,
                surface_elevated: 0x3c4052ff,
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
                border_subtle: 0x43475bff,
                border: 0x5f6371ff,
                border_strong: 0x7d8396ff,
                focus_ring: 0x7dcfffff,
                selection: 0x283458ff,
                disabled: 0x888c9bff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x7aa2f7ff, "palette.blue_ink": 0x9fbfffff, "palette.blue_soft": 0x404b6aff, "palette.blue_border": 0x506391ff,
                "palette.violet": 0xbb9af7ff, "palette.violet_ink": 0xcdb4ffff, "palette.violet_soft": 0x4d496aff, "palette.violet_border": 0x6b5f91ff,
                "palette.teal": 0x73dacaff, "palette.teal_ink": 0x73dacaff, "palette.teal_soft": 0x3e5661ff, "palette.teal_border": 0x4d7a7eff,
                "palette.green": 0x9ece6aff, "palette.green_ink": 0xa4d570ff, "palette.green_soft": 0x47544eff, "palette.green_border": 0x5f7556ff,
                "palette.amber": 0xe0af68ff, "palette.amber_ink": 0xecbb73ff, "palette.amber_soft": 0x544d4eff, "palette.amber_border": 0x7b6855ff,
                "palette.rose": 0xf7768eff, "palette.rose_ink": 0xffa0aeff, "palette.rose_soft": 0x594255ff, "palette.rose_border": 0x845065ff,
                "palette.orange": 0xff9e64ff, "palette.orange_ink": 0xffb082ff, "palette.orange_soft": 0x5a4a4dff, "palette.orange_border": 0x886153ff,
                "palette.slate": 0x9aa5ceff, "palette.slate_ink": 0xb2bee8ff, "palette.slate_soft": 0x464b62ff, "palette.slate_border": 0x5d6480ff,
            },
        },
    }
}
