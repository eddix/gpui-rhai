// Catppuccin Mocha semantic adaptation.
// Upstream: https://github.com/catppuccin/catppuccin (MIT)
fn theme() {
    #{
        family: "Catppuccin",
        name: "Mocha",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x1e1e2eff,
                surface_raised: 0x181825ff,
                surface_hover: 0x313244ff,
                text_primary: 0xcdd6f4ff,
                text_muted: 0xa6adc8ff,
                accent: 0x89b4faff,
                accent_hover: 0x74c7ecff,
                on_accent: 0x1e1e2eff,
                danger: 0xf38ba8ff,
                on_danger: 0x1e1e2eff,
                warning: 0xf9e2afff,
                on_warning: 0x1e1e2eff,
                success: 0xa6e3a1ff,
                on_success: 0x1e1e2eff,
                border: 0x45475aff,
                focus_ring: 0x89b4faff,
                selection: 0x89b4fa66,
                disabled: 0x6c7086ff,
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
