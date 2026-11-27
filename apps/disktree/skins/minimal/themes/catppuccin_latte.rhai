// Catppuccin Latte semantic adaptation.
// Upstream: https://github.com/catppuccin/catppuccin (MIT)
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - surface_hover stepped toward text for a visible tonal block.
// - text_muted raised to 4.5:1 on hover and selection surfaces.
fn theme() {
    #{
        family: "Catppuccin",
        name: "Latte",
        mode: "light",
        tokens: #{
            colors: #{
                surface_sunken: 0xe7e9edff,
                surface: 0xeff1f5ff,
                surface_raised: 0xf9fbfeff,
                surface_elevated: 0xfcfdffff,
                surface_hover: 0xdbdde0ff,
                text_primary: 0x34364fff,
                text_muted: 0x494c62ff,
                accent: 0x1e66f5ff,
                accent_hover: 0x115ae9ff,
                on_accent: 0xffffffff,
                danger: 0xd20f39ff,
                on_danger: 0xffffffff,
                warning: 0xdf8e1dff,
                on_warning: 0x1e1e2eff,
                success: 0x40a02bff,
                on_success: 0x1e1e2eff,
                border_subtle: 0xe8eaefff,
                border: 0xaaacaeff,
                border_strong: 0x85868dff,
                focus_ring: 0x1e66f5ff,
                selection: 0xb4cefcff,
                disabled: 0x7e7f82ff,
                "shadow.ambient": 0x0000000d,
                "shadow.key": 0x00000018,
                "palette.blue": 0x1e66f5ff, "palette.blue_ink": 0x125be9ff, "palette.blue_soft": 0xdfe9fdff, "palette.blue_border": 0xafc8fbff,
                "palette.violet": 0x8839efff, "palette.violet_ink": 0x8434eaff, "palette.violet_soft": 0xebe4fcff, "palette.violet_border": 0xd3b9f9ff,
                "palette.teal": 0x179299ff, "palette.teal_ink": 0x007278ff, "palette.teal_soft": 0xdeeef2ff, "palette.teal_border": 0xacd7dcff,
                "palette.green": 0x3e9e29ff, "palette.green_ink": 0x1e7700ff, "palette.green_soft": 0xe3f0e4ff, "palette.green_border": 0xb9dbb6ff,
                "palette.amber": 0xc67a00ff, "palette.amber_ink": 0x935a00ff, "palette.amber_soft": 0xf3ece0ff, "palette.amber_border": 0xe8cfa8ff,
                "palette.rose": 0xd461b6ff, "palette.rose_ink": 0xaa3a8fff, "palette.rose_soft": 0xf5e9f5ff, "palette.rose_border": 0xecc7e6ff,
                "palette.orange": 0xee5c00ff, "palette.orange_ink": 0xb24300ff, "palette.orange_soft": 0xf8e8e0ff, "palette.orange_border": 0xf5c5a8ff,
                "palette.slate": 0x6c6f85ff, "palette.slate_ink": 0x63657bff, "palette.slate_soft": 0xe8eaefff, "palette.slate_border": 0xc9cbd5ff,
            },
        },
    }
}
