// Project-owned Default Dark theme.
fn theme() {
    #{
        family: "Default",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x18181bff,
                surface_raised: 0x27272aff,
                surface_hover: 0x3f3f46ff,
                text_primary: 0xf4f4f5ff,
                text_muted: 0xa1a1aaff,
                accent: 0x3b82f6ff,
                accent_hover: 0x2563ebff,
                on_accent: 0x18181bff,
                danger: 0xef4444ff,
                on_danger: 0x18181bff,
                warning: 0xf59e0bff,
                on_warning: 0x18181bff,
                success: 0x22c55eff,
                on_success: 0x18181bff,
                border: 0x3f3f46ff,
                focus_ring: 0x60a5faff,
                selection: 0x3b82f666,
                disabled: 0x52525bff,
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
