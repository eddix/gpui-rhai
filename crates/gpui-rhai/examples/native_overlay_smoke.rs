use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const POPOVER: &str = include_str!("../../../registry/components/popover.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const DROPDOWN: &str = include_str!("../../../registry/components/dropdown.rhai");
const TOAST: &str = include_str!("../../../registry/components/toast.rhai");
const TOOLTIP: &str = include_str!("../../../registry/components/tooltip.rhai");
const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");

const MAIN: &str = r#"
import "components/popover" as popover;
import "components/dialog" as dialog;
import "components/dropdown" as dropdown_component;
import "components/toast" as toast;
import "components/tooltip" as tooltip;

fn state_schema() {
    #{
        fields: #{
            popover_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            nested_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            dialog_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            dropdown_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            selected: #{
                schema: #{ type: "array", max_items: 250, items: #{ type: "string" } },
                "default": #{
                    type: "array",
                    value: [#{ type: "string", value: "theme-42" }]
                },
            },
            query: #{
                schema: #{ type: "string" },
                "default": #{ type: "string", value: "" },
            },
            toast_visible: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
        },
    }
}

fn set_popover(ctx, open) { ctx.set_state("popover_open", open); }
fn set_nested(ctx, open) { ctx.set_state("nested_open", open); }
fn set_dialog(ctx, open) { ctx.set_state("dialog_open", open); }
fn use_tokyo_night(ctx, payload) { ctx.set_theme("Tokyo Night", "Night"); }
fn set_selection(ctx, values) { ctx.set_state("selected", values); }
fn set_dropdown_open(ctx, open) { ctx.set_state("dropdown_open", open); }
fn set_query(ctx, query) { ctx.set_state("query", query); }
fn dismiss_toast(ctx, id) { ctx.set_state("toast_visible", false); }

fn dropdown_options() {
    let options = [];
    for index in 0..250 {
        options.push(#{
            value: `theme-${index}`,
            label: `Theme ${index}`,
            keywords: [`palette-${index}`],
            disabled: index == 41,
        });
    }
    options
}

fn view(ctx) {
    let toasts = [];
    if ctx.get_state("toast_visible") {
        toasts.push(#{
            id: "startup", title: "Overlay runtime ready",
            message: "This toast expires through the native window queue.",
            variant: "success", duration_ms: 350,
        });
    }
    column([
        toast::Toast(#{
            key: "smoke-toasts", items: toasts,
            on_dismiss: Fn("dismiss_toast")
        }),
        text("Switch to Tokyo Night").on_click(Fn("use_tokyo_night")),
        dropdown_component::Dropdown(#{
            key: "theme-picker",
            options: dropdown_options(),
            mode: "single",
            selected: ctx.get_state("selected"),
            open: ctx.get_state("dropdown_open"),
            searchable: true,
            query: ctx.get_state("query"),
            placeholder: "Choose a theme",
            search_placeholder: "Type to filter themes",
            empty_text: "No matching themes",
            max_visible: 8,
            on_change: Fn("set_selection"),
            on_open_change: Fn("set_dropdown_open"),
            on_query_change: Fn("set_query")
        }),
        popover::Popover(#{
            key: "account-help",
            trigger: text("Toggle popover").with_style(
                style()
                    .padding(px(8))
                    .radius(px(6))
                    .background(theme_color("accent"))
            ),
            content: text("Popover content is deferred above the normal tree."),
            open: ctx.get_state("popover_open"),
            placement: "bottom",
            on_open_change: Fn("set_popover")
        }),
        tooltip::Tooltip(#{
            key: "hover-help",
            trigger: text("Hover for tooltip").with_style(
                style().padding(px(8)).radius(px(6)).background(theme_color("surface_raised"))
            ),
            content: text("Tooltip delay completed"),
            placement: "right",
            show_delay_ms: 250,
            hide_delay_ms: 100,
        }),
        dialog::Dialog(#{
            key: "confirm-dialog",
            open: ctx.get_state("dialog_open"),
            title: "Native overlay smoke test",
            content: popover::Popover(#{
                key: "dialog-help",
                parent_overlay: "confirm-dialog",
                trigger: text("Nested popover"),
                content: text("This child shares the dialog overlay coordinator."),
                open: ctx.get_state("nested_open"),
                placement: "right",
                on_open_change: Fn("set_nested")
            }),
            actions: [text("Press Escape")],
            on_open_change: Fn("set_dialog")
        })
    ]).with_style(
        style()
            .padding(px(24))
            .gap(px(16))
            .background(theme_color("surface"))
    )
}
"#;

fn main() {
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("main").expect("main module"),
            MAIN.to_owned(),
        ),
        (
            ModuleId::parse("components/popover").expect("popover module"),
            POPOVER.to_owned(),
        ),
        (
            ModuleId::parse("components/dialog").expect("dialog module"),
            DIALOG.to_owned(),
        ),
        (
            ModuleId::parse("components/dropdown").expect("dropdown module"),
            DROPDOWN.to_owned(),
        ),
        (
            ModuleId::parse("components/toast").expect("toast module"),
            TOAST.to_owned(),
        ),
        (
            ModuleId::parse("components/tooltip").expect("tooltip module"),
            TOOLTIP.to_owned(),
        ),
    ]));
    EmbeddedScriptView::new(
        ModuleId::parse("main").expect("main module"),
        scripts,
        THEME,
    )
    .theme_sources([
        ("tokyo_night.rhai".to_owned(), TOKYO_NIGHT.to_owned()),
        (
            "catppuccin_mocha.rhai".to_owned(),
            CATPPUCCIN_MOCHA.to_owned(),
        ),
    ])
    .prepare()
    .and_then(|prepared| {
        ScriptApplication::new(prepared)
            .window_size(720.0, 480.0)
            .run()
    })
    .expect("native overlay smoke test failed");
}
