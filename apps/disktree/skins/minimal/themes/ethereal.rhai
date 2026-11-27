// Semantic adaptation of Omarchy Ethereal (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/ethereal
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Ethereal",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x020516ff,
                surface: 0x060b1eff,
                surface_raised: 0x111729ff,
                surface_elevated: 0x1a2134ff,
                surface_hover: 0x1e2437ff,
                text_primary: 0xffceadff,
                text_muted: 0xa5abc4ff,
                accent: 0x7d82d9ff,
                accent_hover: 0x878de5ff,
                on_accent: 0x060b1eff,
                danger: 0xed5b5aff,
                on_danger: 0x060b1eff,
                warning: 0xe9bb4fff,
                on_warning: 0x060b1eff,
                success: 0x92a593ff,
                on_success: 0x060b1eff,
                border_subtle: 0x2b2b38ff,
                border: 0x444a59ff,
                border_strong: 0x6d676bff,
                focus_ring: 0xa3bfd1ff,
                selection: 0x101336ff,
                disabled: 0x616878ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x60a5faff, "palette.blue_soft": 0x213353ff, "palette.blue_border": 0x325381ff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xa78bfaff, "palette.violet_soft": 0x2f2e53ff, "palette.violet_border": 0x504881ff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x173d47ff, "palette.teal_border": 0x1d6668ff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x1c3f3aff, "palette.green_border": 0x296b4eff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x403928ff, "palette.amber_border": 0x735e27ff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xfb7185ff, "palette.rose_soft": 0x40293bff, "palette.rose_border": 0x733d50ff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x40302dff, "palette.orange_border": 0x734b31ff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x94a3b8ff, "palette.slate_soft": 0x2b3346ff, "palette.slate_border": 0x485265ff,
            },
        },
    }
}
