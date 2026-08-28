import "components/button" as button;
import "components/input" as input;
import "components/label" as label;

fn state_schema() {
    #{
        fields: #{
            name: #{
                schema: #{ type: "string" },
                "default": #{ type: "string", value: "" },
            },
        },
    }
}

fn greet(ctx, payload) {
    print("Hello from a Rhai event handler");
}

fn name_changed(ctx, value) {
    ctx.set_state("name", value);
}

fn view(ctx) {
    column([
        label::Label(#{
            text: "GPUI Rhai",
            description: "Editable Rhai source rendered by a stable Rust runtime"
        }),
        text(ctx.t("common.loading"))
            .with_style(style().text_color(theme_color("text_muted"))),
        input::Input(#{
            key: "name",
            value: ctx.get_state("name"),
            placeholder: "Type your name",
            on_change: Fn("name_changed")
        }),
        button::Button(#{
            text: "Say hello",
            variant: "primary",
            size: "md",
            on_click: Fn("greet")
        })
    ]).with_style(
        style()
            .gap(px(16))
            .padding(px(24))
            .background(theme_color("surface"))
    )
}
