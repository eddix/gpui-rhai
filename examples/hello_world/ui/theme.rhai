// Project-owned Default Dark theme.
fn theme() {
    #{
        family: "Default",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x111318ff,
                surface_raised: 0x1a1d24ff,
                surface_hover: 0x252a34ff,
                text_primary: 0xd9dee8ff,
                text_muted: 0x929aa8ff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x8db0ffff,
                on_accent: 0x111318ff,
                danger: 0xf7768eff,
                on_danger: 0x111318ff,
                warning: 0xe0af68ff,
                on_warning: 0x111318ff,
                success: 0x9ece6aff,
                on_success: 0x111318ff,
                border: 0x353b48ff,
                focus_ring: 0x7dcfffff,
                selection: 0x33476fff,
                disabled: 0x626a78ff,
            },
            spacing: #{
                xs: #{ unit: "pixels", value: 4.0 },
                sm: #{ unit: "pixels", value: 8.0 },
                md: #{ unit: "pixels", value: 12.0 },
                lg: #{ unit: "pixels", value: 16.0 },
            },
            radii: #{
                sm: #{ unit: "pixels", value: 2.0 },
                md: #{ unit: "pixels", value: 4.0 },
                lg: #{ unit: "pixels", value: 6.0 },
            },
        },
    }
}
