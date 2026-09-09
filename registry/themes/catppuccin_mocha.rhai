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
                surface_raised: 0x313244ff,
                surface_hover: 0x45475aff,
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
                selection: 0x45475aff,
                disabled: 0x6c7086ff,
            },
            spacing: #{
                xs: #{ unit: "pixels", value: 4.0 },
                sm: #{ unit: "pixels", value: 8.0 },
                md: #{ unit: "pixels", value: 12.0 },
                lg: #{ unit: "pixels", value: 16.0 },
            },
            radii: #{
                sm: #{ unit: "pixels", value: 0.0 },
                md: #{ unit: "pixels", value: 0.0 },
                lg: #{ unit: "pixels", value: 0.0 },
            },
            typography: #{
                roles: #{
                    caption: #{ size: #{ unit: "pixels", value: 11.0 },
                        line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                    body_small: #{ size: #{ unit: "pixels", value: 12.0 },
                        line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                    body: #{ size: #{ unit: "pixels", value: 13.0 },
                        line_height: #{ unit: "pixels", value: 18.0 }, weight: 400 },
                    subtitle: #{ size: #{ unit: "pixels", value: 14.0 },
                        line_height: #{ unit: "pixels", value: 20.0 }, weight: 400 },
                    title: #{ size: #{ unit: "pixels", value: 16.0 },
                        line_height: #{ unit: "pixels", value: 22.0 }, weight: 700 },
                    heading: #{ size: #{ unit: "pixels", value: 18.0 },
                        line_height: #{ unit: "pixels", value: 24.0 }, weight: 700 },
                    display: #{ size: #{ unit: "pixels", value: 24.0 },
                        line_height: #{ unit: "pixels", value: 32.0 }, weight: 700 },
                    display_large: #{ size: #{ unit: "pixels", value: 28.0 },
                        line_height: #{ unit: "pixels", value: 36.0 }, weight: 700 },
                },
            },
        },
    }
}
