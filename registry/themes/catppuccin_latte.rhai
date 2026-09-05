// Catppuccin Latte semantic adaptation.
// Upstream: https://github.com/catppuccin/catppuccin (MIT)
fn theme() {
    #{
        family: "Catppuccin",
        name: "Latte",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xeff1f5ff,
                surface_raised: 0xf8f9fbff,
                surface_hover: 0xe6e9efff,
                text_primary: 0x4c4f69ff,
                text_muted: 0x686b80ff,
                accent: 0x1e66f5ff,
                accent_hover: 0x1a5ad7ff,
                on_accent: 0xffffffff,
                danger: 0xd20f39ff,
                on_danger: 0xffffffff,
                warning: 0xdf8e1dff,
                on_warning: 0x1e1e2eff,
                success: 0x40a02bff,
                on_success: 0x1e1e2eff,
                border: 0xbcc0ccff,
                focus_ring: 0x1e66f5ff,
                selection: 0xdce7fcff,
                disabled: 0x9ca0b0ff,
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
                    caption: #{ size: #{ unit: "pixels", value: 10.0 },
                        line_height: #{ unit: "pixels", value: 14.0 }, weight: 400 },
                    body_small: #{ size: #{ unit: "pixels", value: 11.0 },
                        line_height: #{ unit: "pixels", value: 14.0 }, weight: 400 },
                    body: #{ size: #{ unit: "pixels", value: 12.0 },
                        line_height: #{ unit: "pixels", value: 16.0 }, weight: 400 },
                    subtitle: #{ size: #{ unit: "pixels", value: 13.0 },
                        line_height: #{ unit: "pixels", value: 18.0 }, weight: 400 },
                    title: #{ size: #{ unit: "pixels", value: 14.0 },
                        line_height: #{ unit: "pixels", value: 20.0 }, weight: 700 },
                    heading: #{ size: #{ unit: "pixels", value: 16.0 },
                        line_height: #{ unit: "pixels", value: 22.0 }, weight: 700 },
                    display: #{ size: #{ unit: "pixels", value: 24.0 },
                        line_height: #{ unit: "pixels", value: 30.0 }, weight: 700 },
                    display_large: #{ size: #{ unit: "pixels", value: 28.0 },
                        line_height: #{ unit: "pixels", value: 34.0 }, weight: 700 },
                },
            },
        },
    }
}
