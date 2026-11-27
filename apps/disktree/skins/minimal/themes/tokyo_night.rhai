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
                surface_sunken: 0x12131eff,
                surface: 0x1a1b26ff,
                surface_raised: 0x262732ff,
                surface_elevated: 0x30313dff,
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
                border_subtle: 0x373947ff,
                border: 0x54555eff,
                border_strong: 0x717585ff,
                focus_ring: 0x7dcfffff,
                selection: 0x1d2743ff,
                disabled: 0x7a7b85ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x7aa2f7ff, "palette.blue_ink": 0x87aeffff, "palette.blue_soft": 0x374059ff, "palette.blue_border": 0x495b85ff,
                "palette.violet": 0xbb9af7ff, "palette.violet_ink": 0xc1a0fdff, "palette.violet_soft": 0x443e59ff, "palette.violet_border": 0x655785ff,
                "palette.teal": 0x73dacaff, "palette.teal_ink": 0x73dacaff, "palette.teal_soft": 0x354b50ff, "palette.teal_border": 0x467272ff,
                "palette.green": 0x9ece6aff, "palette.green_ink": 0x9ece6aff, "palette.green_soft": 0x3e483dff, "palette.green_border": 0x586d4aff,
                "palette.amber": 0xe0af68ff, "palette.amber_ink": 0xe0af68ff, "palette.amber_soft": 0x4b423dff, "palette.amber_border": 0x746049ff,
                "palette.rose": 0xf7768eff, "palette.rose_ink": 0xff879bff, "palette.rose_soft": 0x503744ff, "palette.rose_border": 0x7e4859ff,
                "palette.orange": 0xff9e64ff, "palette.orange_ink": 0xff9e64ff, "palette.orange_soft": 0x513f3cff, "palette.orange_border": 0x815947ff,
                "palette.slate": 0x9aa5ceff, "palette.slate_ink": 0xa2aed7ff, "palette.slate_soft": 0x3d4051ff, "palette.slate_border": 0x575c74ff,
            },
        },
    }
}
