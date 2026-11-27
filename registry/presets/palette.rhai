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
            "charts.map_missing": theme_color("surface_sunken"),
            "charts.crosshair": theme_color("focus_ring"),

            "surface_sunken": by_mode(
                mix(theme_color("surface"), theme_color("text_primary"), 0.04),
                mix(theme_color("surface"), rgba(0x000000ff), 0.22)
            ),
            "surface_elevated": by_mode(
                mix(theme_color("surface_raised"), rgba(0xffffffff), 0.35),
                mix(theme_color("surface_raised"), theme_color("text_primary"), 0.06)
            ),
            "border_subtle": by_mode(
                mix(theme_color("surface_raised"), theme_color("text_primary"), 0.085),
                mix(theme_color("surface_raised"), theme_color("text_primary"), 0.11)
            ),
            "border_strong": readable(
                mix(theme_color("border"), theme_color("text_primary"), 0.22),
                theme_color("text_primary"), theme_color("surface_raised"), 3.2
            ),
            "shadow.ambient": by_mode(rgba(0x0000000d), rgba(0x00000052)),
            "shadow.key": by_mode(rgba(0x00000018), rgba(0x0000007a)),

            "palette.blue": by_mode(rgb(0x2563eb), rgb(0x60a5fa)),
            "palette.blue_ink": readable(theme_color("palette.blue"), toward, theme_color("surface_raised"), 4.65),
            "palette.blue_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.blue"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.blue"), 0.20)),
            "palette.blue_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.blue"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.blue"), 0.42)),
            "palette.violet": by_mode(rgb(0x7c3aed), rgb(0xa78bfa)),
            "palette.violet_ink": readable(theme_color("palette.violet"), toward, theme_color("surface_raised"), 4.65),
            "palette.violet_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.violet"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.violet"), 0.20)),
            "palette.violet_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.violet"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.violet"), 0.42)),
            "palette.teal": by_mode(rgb(0x0d9488), rgb(0x2dd4bf)),
            "palette.teal_ink": readable(theme_color("palette.teal"), toward, theme_color("surface_raised"), 4.65),
            "palette.teal_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.teal"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.teal"), 0.20)),
            "palette.teal_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.teal"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.teal"), 0.42)),
            "palette.green": by_mode(rgb(0x16a34a), rgb(0x4ade80)),
            "palette.green_ink": readable(theme_color("palette.green"), toward, theme_color("surface_raised"), 4.65),
            "palette.green_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.green"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.green"), 0.20)),
            "palette.green_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.green"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.green"), 0.42)),
            "palette.amber": by_mode(rgb(0xd97706), rgb(0xfbbf24)),
            "palette.amber_ink": readable(theme_color("palette.amber"), toward, theme_color("surface_raised"), 4.65),
            "palette.amber_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.amber"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.amber"), 0.20)),
            "palette.amber_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.amber"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.amber"), 0.42)),
            "palette.rose": by_mode(rgb(0xe11d48), rgb(0xfb7185)),
            "palette.rose_ink": readable(theme_color("palette.rose"), toward, theme_color("surface_raised"), 4.65),
            "palette.rose_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.rose"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.rose"), 0.20)),
            "palette.rose_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.rose"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.rose"), 0.42)),
            "palette.orange": by_mode(rgb(0xea580c), rgb(0xfb923c)),
            "palette.orange_ink": readable(theme_color("palette.orange"), toward, theme_color("surface_raised"), 4.65),
            "palette.orange_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.orange"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.orange"), 0.20)),
            "palette.orange_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.orange"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.orange"), 0.42)),
            "palette.slate": by_mode(rgb(0x64748b), rgb(0x94a3b8)),
            "palette.slate_ink": readable(theme_color("palette.slate"), toward, theme_color("surface_raised"), 4.65),
            "palette.slate_soft": by_mode(mix(theme_color("surface_raised"), theme_color("palette.slate"), 0.12), mix(theme_color("surface_raised"), theme_color("palette.slate"), 0.20)),
            "palette.slate_border": by_mode(mix(theme_color("surface_raised"), theme_color("palette.slate"), 0.34), mix(theme_color("surface_raised"), theme_color("palette.slate"), 0.42)),

            "palette.1": theme_color("palette.blue"),
            "palette.1_ink": theme_color("palette.blue_ink"),
            "palette.1_soft": theme_color("palette.blue_soft"),
            "palette.1_border": theme_color("palette.blue_border"),
            "palette.2": theme_color("palette.violet"),
            "palette.2_ink": theme_color("palette.violet_ink"),
            "palette.2_soft": theme_color("palette.violet_soft"),
            "palette.2_border": theme_color("palette.violet_border"),
            "palette.3": theme_color("palette.teal"),
            "palette.3_ink": theme_color("palette.teal_ink"),
            "palette.3_soft": theme_color("palette.teal_soft"),
            "palette.3_border": theme_color("palette.teal_border"),
            "palette.4": theme_color("palette.green"),
            "palette.4_ink": theme_color("palette.green_ink"),
            "palette.4_soft": theme_color("palette.green_soft"),
            "palette.4_border": theme_color("palette.green_border"),
            "palette.5": theme_color("palette.amber"),
            "palette.5_ink": theme_color("palette.amber_ink"),
            "palette.5_soft": theme_color("palette.amber_soft"),
            "palette.5_border": theme_color("palette.amber_border"),
            "palette.6": theme_color("palette.rose"),
            "palette.6_ink": theme_color("palette.rose_ink"),
            "palette.6_soft": theme_color("palette.rose_soft"),
            "palette.6_border": theme_color("palette.rose_border"),
            "palette.7": theme_color("palette.orange"),
            "palette.7_ink": theme_color("palette.orange_ink"),
            "palette.7_soft": theme_color("palette.orange_soft"),
            "palette.7_border": theme_color("palette.orange_border"),
            "palette.8": theme_color("palette.slate"),
            "palette.8_ink": theme_color("palette.slate_ink"),
            "palette.8_soft": theme_color("palette.slate_soft"),
            "palette.8_border": theme_color("palette.slate_border"),

            "charts.palette_1": theme_color("palette.1"),
            "charts.palette_2": theme_color("palette.2"),
            "charts.palette_3": theme_color("palette.3"),
            "charts.palette_4": theme_color("palette.4"),
            "charts.palette_5": theme_color("palette.5"),
            "charts.palette_6": theme_color("palette.6"),
            "charts.palette_7": theme_color("palette.7"),
            "charts.palette_8": theme_color("palette.8"),
        }
}
