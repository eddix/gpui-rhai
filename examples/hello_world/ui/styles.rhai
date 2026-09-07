// Application-wide component skin. Instance style/part_styles still win.
fn component_styles() {
    #{
        "components/button": #{
            root: style().height(px(34)).padding_x(theme_spacing("md")),
        },
        "components/input": #{
            root: style().height(px(34)),
        },
    }
}
