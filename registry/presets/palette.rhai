// Semantic expressions for alternate application palettes.
// This helper is inlined into the generated token base; it is not a theme.
fn palette_colors() {
    // A complete preset may use a translucent hover wash. Calibrate derived
    // text against its composed surface, not the wash's uncomposited RGB.
    let backing = theme_color("surface_hover").over(theme_color("surface_raised"));
    let toward = theme_color("text_primary").over(backing);
#{
            "text.accent": readable(theme_color("accent").over(backing), toward, backing, 4.65),
            "text.danger": readable(theme_color("danger").over(backing), toward, backing, 4.65),
            "text.warning": readable(theme_color("warning").over(backing), toward, backing, 4.65),
            "text.success": readable(theme_color("success").over(backing), toward, backing, 4.65),
            "control.hover": mix(theme_color("surface_hover"), theme_color("text_primary"), 0.06),
            // A control boundary is meaningful, unlike a decorative divider. Keep
            // it at 3:1 on both page and field well, without making every panel loud.
            "control.border": readable(readable(theme_color("border"), theme_color("text_primary"),
                theme_color("surface"), 3.0), theme_color("text_primary"), theme_color("surface_raised"), 3.0),
            "control.danger_hover": readable(mix(theme_color("danger"), theme_color("text_primary"), 0.12),
                theme_color("danger"), theme_color("on_danger"), 4.5),
            "control.warning_hover": readable(mix(theme_color("warning"), theme_color("text_primary"), 0.12),
                theme_color("warning"), theme_color("on_warning"), 4.5),
            "control.success_hover": readable(mix(theme_color("success"), theme_color("text_primary"), 0.12),
                theme_color("success"), theme_color("on_success"), 4.5),
            // Text sits on these two, so each tint backs off toward its base only as far as
            // text_primary needs to keep 4.5:1; palettes with room keep the full tint.
            "tag.facet": readable(mix(theme_color("surface_hover"), theme_color("text_primary"), 0.14),
                theme_color("surface_hover"), theme_color("text_primary"), 4.5),
            "table.selection": readable(mix(theme_color("surface"), theme_color("accent"), 0.28),
                theme_color("surface"), theme_color("text_primary"), 4.5),
            "scrollbar.thumb": readable(mix(theme_color("surface"), theme_color("text_muted"), 0.65),
                theme_color("text_primary"), theme_color("surface_hover"), 3.0),
            "scrollbar.thumb_hover": theme_color("text_muted"),
            "tabs.foreground": readable(theme_color("text_muted"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            // Tabs count capsule: a step from the track toward ink, so it shows on the track,
            // the thumb and the page alike. Its figures are small, so it keeps text_primary at
            // the 5.2:1 margin of secondary reading text rather than the bare 4.5:1.
            "tabs.counter": readable(mix(theme_color("surface_hover"), theme_color("text_primary"), 0.14),
                theme_color("surface_hover"), theme_color("text_primary"), 5.2),
            // Tabs `bar` variant: the bar sits between the page and a tonal block; the selected tab
            // takes the panel's surface so it joins the panel under it.
            "tabbar.background": mix(theme_color("surface"), theme_color("surface_hover"), 0.5),
            "tabbar.active": theme_color("surface"),
            // A third surface between the window canvas and raised cards, for an
            // application's sidebars and title bars; no built-in component takes it.
            "surface.chrome": mix(theme_color("surface"), theme_color("surface_raised"), 0.5),

            "syntax.comment": theme_color("text_muted"),
            "syntax.string": readable(theme_color("success").over(backing), toward, backing, 4.65),
            "syntax.number": readable(theme_color("warning").over(backing), toward, backing, 4.65),
            "syntax.keyword": readable(theme_color("accent").over(backing), toward, backing, 4.65),
            "syntax.function": readable(theme_color("accent_hover").over(backing), toward, backing, 4.65),
            "syntax.type": readable(theme_color("warning").over(backing), toward, backing, 4.65),
            "syntax.variable": theme_color("text_primary"),
            "syntax.constant": readable(theme_color("danger").over(backing), toward, backing, 4.65),
            "syntax.operator": readable(theme_color("accent").over(backing), toward, backing, 4.65),
            "syntax.punctuation": theme_color("text_muted"),
            "syntax.tag": readable(theme_color("danger").over(backing), toward, backing, 4.65),
            "syntax.attribute": readable(theme_color("warning").over(backing), toward, backing, 4.65),

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
        }
}
