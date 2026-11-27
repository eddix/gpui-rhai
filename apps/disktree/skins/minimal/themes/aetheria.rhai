// Semantic adaptation of Aetheria by Dizziee (MIT).
// Source: https://github.com/JJDizz1L/aetheria
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - danger hue moved 22 deg away from the accent/status hues.
fn theme() {
    #{
        family: "Aetheria",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x070315ff,
                surface: 0x0e091dff,
                surface_raised: 0x191529ff,
                surface_elevated: 0x231f34ff,
                surface_hover: 0x262236ff,
                text_primary: 0x4cd7d2ff,
                text_muted: 0x8e97aeff,
                accent: 0xbe3f50ff,
                accent_hover: 0xb23346ff,
                on_accent: 0xfff2e8ff,
                danger: 0xdd0391ff,
                on_danger: 0xffffffff,
                warning: 0xffbe74ff,
                on_warning: 0x0e091dff,
                success: 0x7cd699ff,
                on_success: 0x0e091dff,
                border_subtle: 0x1f2a3cff,
                border: 0x4b4857ff,
                border_strong: 0x506d78ff,
                focus_ring: 0xff7f41ff,
                selection: 0x2b0b1eff,
                disabled: 0x696677ff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x60a5faff, "palette.blue_ink": 0x60a5faff, "palette.blue_soft": 0x273253ff, "palette.blue_border": 0x375181ff,
                "palette.violet": 0xa78bfaff, "palette.violet_ink": 0xa78bfaff, "palette.violet_soft": 0x352d53ff, "palette.violet_border": 0x554781ff,
                "palette.teal": 0x2dd4bfff, "palette.teal_ink": 0x2dd4bfff, "palette.teal_soft": 0x1d3b47ff, "palette.teal_border": 0x216568ff,
                "palette.green": 0x4ade80ff, "palette.green_ink": 0x4ade80ff, "palette.green_soft": 0x233d3aff, "palette.green_border": 0x2e694eff,
                "palette.amber": 0xfbbf24ff, "palette.amber_ink": 0xfbbf24ff, "palette.amber_soft": 0x463728ff, "palette.amber_border": 0x785c27ff,
                "palette.rose": 0xfb7185ff, "palette.rose_ink": 0xfb7185ff, "palette.rose_soft": 0x46273bff, "palette.rose_border": 0x783c50ff,
                "palette.orange": 0xfb923cff, "palette.orange_ink": 0xfb923cff, "palette.orange_soft": 0x462e2dff, "palette.orange_border": 0x784a31ff,
                "palette.slate": 0x94a3b8ff, "palette.slate_ink": 0x94a3b8ff, "palette.slate_soft": 0x323146ff, "palette.slate_border": 0x4d5165ff,
            },
        },
    }
}
