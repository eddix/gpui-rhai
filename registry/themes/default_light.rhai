// Project-owned Default Light theme.
fn theme() {
    #{
        family: "Default",
        name: "Light",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xfafafaff,
                surface_raised: 0xffffffff,
                surface_hover: 0xf4f4f5ff,
                text_primary: 0x18181bff,
                text_muted: 0x71717aff,
                accent: 0x2563ebff,
                accent_hover: 0x1d4ed8ff,
                on_accent: 0xffffffff,
                danger: 0xdc2626ff,
                on_danger: 0xffffffff,
                warning: 0xd97706ff,
                on_warning: 0x18181bff,
                success: 0x16a34aff,
                on_success: 0x18181bff,
                border: 0xe4e4e7ff,
                focus_ring: 0x3b82f6ff,
                selection: 0x3b82f64d,
                disabled: 0xa1a1aaff,
            },
            spacing: #{
                xs: #{ unit: "pixels", value: 4.0 },
                sm: #{ unit: "pixels", value: 8.0 },
                md: #{ unit: "pixels", value: 12.0 },
                lg: #{ unit: "pixels", value: 16.0 },
            },
            radii: #{
                sm: #{ unit: "pixels", value: 4.0 },
                md: #{ unit: "pixels", value: 8.0 },
                lg: #{ unit: "pixels", value: 12.0 },
            },
        },
    }
}
