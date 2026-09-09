// Tokyo Night Storm semantic adaptation.
// Upstream: https://github.com/folke/tokyonight.nvim (Apache-2.0)
fn theme() {
    #{
        family: "Tokyo Night",
        name: "Storm",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x24283bff,
                surface_raised: 0x292e42ff,
                surface_hover: 0x3b4261ff,
                text_primary: 0xc0caf5ff,
                text_muted: 0xa9b1d6ff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x8db0ffff,
                on_accent: 0x24283bff,
                danger: 0xf7768eff,
                on_danger: 0x24283bff,
                warning: 0xe0af68ff,
                on_warning: 0x24283bff,
                success: 0x9ece6aff,
                on_success: 0x24283bff,
                border: 0x414868ff,
                focus_ring: 0x7dcfffff,
                selection: 0x3b4261ff,
                disabled: 0x565f89ff,
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
