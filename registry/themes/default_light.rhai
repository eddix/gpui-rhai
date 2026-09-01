// Project-owned Default Light theme.
fn theme() {
    #{
        family: "Default",
        name: "Light",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xf5f5f2ff,
                surface_raised: 0xffffffff,
                surface_hover: 0xe7e8e3ff,
                text_primary: 0x202226ff,
                text_muted: 0x686e78ff,
                accent: 0x4169a8ff,
                accent_hover: 0x34598fff,
                on_accent: 0xffffffff,
                danger: 0xb4233bff,
                on_danger: 0xffffffff,
                warning: 0xa65f00ff,
                on_warning: 0xffffffff,
                success: 0x397a2fff,
                on_success: 0xffffffff,
                border: 0xc8cbc4ff,
                focus_ring: 0x4169a8ff,
                selection: 0xcbd9efff,
                disabled: 0x92979fff,
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
