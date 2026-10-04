// gpui-rhai design language token base.
//
// This is the lowest token layer. Palette themes (ui/theme.rhai,
// ui/themes/*.rhai) supply colors; a Host or profile may override individual
// entries. Layout tokens live here so switching palette never changes layout.
// See docs/design/themes.md.
fn tokens() {
    #{
        environment: #{
            density: #{ values: ["comfortable", "compact"], "default": "comfortable" },
            size: #{ values: ["xs", "sm", "md", "lg"], "default": "md" },
        },

        // Density-aware spacing scale. Compact shrinks only the outer steps.
        spacing: #{
            xxs: px(2),
            xs: px(4),
            sm: px(8),
            md: by_env("density", #{ comfortable: px(12), compact: px(8) }),
            lg: by_env("density", #{ comfortable: px(16), compact: px(12) }),
            xl: by_env("density", #{ comfortable: px(24), compact: px(16) }),
        },

        // Relationship aliases for layouts: unit < related < group < section.
        space: #{
            unit: px(4),
            related: px(8),
            group: by_env("density", #{ comfortable: px(16), compact: px(12) }),
            section: by_env("density", #{ comfortable: px(24), compact: px(16) }),
        },

        // Radius roles: sm markers, md controls, lg overlays. Square by default;
        // a profile may use up to 4px.
        radius: #{ sm: px(0), md: px(0), lg: px(0) },

        metrics: #{
            control: by_env(["density", "size"], #{
                comfortable: #{ xs: px(24), sm: px(28), md: px(32), lg: px(36) },
                compact: #{ xs: px(20), sm: px(24), md: px(28), lg: px(32) },
            }),
            control_pad: by_env(["density", "size"], #{
                comfortable: #{ xs: px(8), sm: px(12), md: px(16), lg: px(20) },
                compact: #{ xs: px(8), sm: px(8), md: px(12), lg: px(16) },
            }),
            icon: by_env(["density", "size"], #{
                comfortable: #{ xs: px(12), sm: px(14), md: px(16), lg: px(16) },
                compact: #{ xs: px(12), sm: px(14), md: px(14), lg: px(16) },
            }),
            row: by_env("density", #{ comfortable: px(32), compact: px(28) }),
            inset: by_env("density", #{ comfortable: px(12), compact: px(8) }),
            marker: px(20),
            marker_small: px(18),
            titlebar: by_env("density", #{ comfortable: px(36), compact: px(32) }),
            statusbar: by_env("density", #{ comfortable: px(24), compact: px(22) }),
            label_column: px(128),
        },

        typography: #{
            family: (),
            fallbacks: [],
            roles: #{
                caption: #{ size: px(12), line_height: px(18), weight: 400 },
                body_small: #{ size: px(13), line_height: px(20), weight: 400 },
                body: #{ size: px(14), line_height: px(22), weight: 400 },
                subtitle: #{ size: px(16), line_height: px(24), weight: 600 },
                title: #{ size: px(18), line_height: px(26), weight: 600 },
                heading: #{ size: px(20), line_height: px(28), weight: 600 },
                display: #{ size: px(24), line_height: px(32), weight: 600 },
                display_large: #{ size: px(32), line_height: px(40), weight: 600 },
                // Label voice: monospace caption, uppercase Latin set by the
                // component. GPUI has no letter spacing; the mono advance keeps
                // uppercase legible.
                label: #{ size: px(12), line_height: px(16), weight: 400,
                    family: "Menlo", fallbacks: ["DejaVu Sans Mono", "Liberation Mono"] },
                code: #{ size: px(13), line_height: px(20), weight: 400,
                    family: "Menlo", fallbacks: ["DejaVu Sans Mono", "Liberation Mono"] },
                control: by_env("size", #{
                    xs: "body_small", sm: "body", md: "body", lg: "body",
                }),
            },
        },

        // Derived colors, evaluated against the active palette and overrides.
        colors: #{
            "text.accent": readable(theme_color("accent"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            "text.danger": readable(theme_color("danger"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            "text.warning": readable(theme_color("warning"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            "text.success": readable(theme_color("success"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            "control.hover": mix(theme_color("surface_hover"), theme_color("text_primary"), 0.06),
            "tag.key": mix(theme_color("surface_hover"), theme_color("text_primary"), 0.14),
            "table.selection": mix(theme_color("surface"), theme_color("accent"), 0.28),
            "tabs.foreground": readable(theme_color("text_muted"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),

            "syntax.comment": theme_color("text_muted"),
            "syntax.string": theme_color("success"),
            "syntax.number": theme_color("warning"),
            "syntax.keyword": theme_color("accent"),
            "syntax.function": theme_color("accent_hover"),
            "syntax.type": theme_color("warning"),
            "syntax.variable": theme_color("text_primary"),
            "syntax.constant": theme_color("danger"),
            "syntax.operator": theme_color("accent"),
            "syntax.punctuation": theme_color("text_muted"),
            "syntax.tag": theme_color("danger"),
            "syntax.attribute": theme_color("warning"),

            "document.search_match": alpha(theme_color("warning"), 0.33),
            "document.search_current": alpha(theme_color("warning"), 0.67),

            "diff.left_only": alpha(theme_color("danger"), 0.14),
            "diff.right_only": alpha(theme_color("success"), 0.14),
            "diff.modified": alpha(theme_color("accent"), 0.09),
            "diff.inline_left": alpha(theme_color("danger"), 0.4),
            "diff.inline_right": alpha(theme_color("success"), 0.4),
            "diff.gutter": theme_color("surface_raised"),
            "diff.fold": theme_color("surface_hover"),

            "charts.axis": theme_color("text_muted"),
            "charts.grid": alpha(theme_color("border"), 0.33),
            "charts.tooltip_surface": theme_color("surface_raised"),
            "charts.tooltip_text": theme_color("text_primary"),
            "charts.positive": theme_color("success"),
            "charts.negative": theme_color("danger"),
            "charts.selection": theme_color("accent"),
            "charts.map_missing": theme_color("surface_hover"),
            "charts.crosshair": theme_color("focus_ring"),
            "charts.palette_1": theme_color("accent"),
            "charts.palette_2": theme_color("success"),
            "charts.palette_3": theme_color("warning"),
            "charts.palette_4": theme_color("danger"),
            "charts.palette_5": mix(theme_color("accent"), theme_color("success"), 0.5),
            "charts.palette_6": theme_color("accent_hover"),
            "charts.palette_7": theme_color("text_muted"),
            "charts.palette_8": mix(theme_color("accent"), theme_color("danger"), 0.5),
        },
    }
}
