// A shell that places its side regions by the width it has (#218): it measures its own frame
// with ctx.element_size, never the window, and its breakpoints are its own. Measuring needs an
// element ref, which only a formal component declares, so the shell lives in one.
import "patterns/app_shell" as app_shell;
import "components/button" as button;
import "components/toggle_group" as toggle_group;
import "layouts/stack" as stack;

define_component(#{
    metadata: #{ id: "gallery/shell_demo", "export": "ShellDemo", version: "0.4.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["patterns/app_shell", "components/button", "components/toggle_group", "layouts/stack"],
        capabilities: #{}, tokens: ["border", "text_muted", "spacing.lg", "space.related", "space.group", "typography.caption"], environment: [] },
    schema: #{ props: #{},
        state: #{ fields: #{
            width: #{ schema: #{ type: "string", allowed: ["wide", "medium", "narrow"] },
                "default": #{ type: "string", value: "wide" } },
            sidebar_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            inspector_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        } },
        events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_ShellDemo")
});

fn ShellDemo(props) { render_component("gallery/shell_demo", props) }

fn choose_width(ctx, values) { if values.len > 0 { ctx.set_state("width", values[0]); } }
fn set_flag(name, value, ctx, payload) { ctx.set_state(name, value); }

// Below 800px the inspector becomes a drawer, below 600px the sidebar too.
fn region_mode(width, dock_from, open) {
    if width >= dock_from { "dock" } else if open { "overlay" } else { "hidden" }
}

fn render_ShellDemo(ctx, props) {
    let choice = ctx.get_state("width");
    let frame_width = if choice == "narrow" { px(480) } else if choice == "medium" { px(680) } else { px(900) };
    let frame = element_ref("frame");
    let measured = ctx.element_size(frame);
    let width = if measured == () { 900.0 } else { measured.width };
    let sidebar_mode = region_mode(width, 600.0, ctx.get_state("sidebar_open"));
    let inspector_mode = region_mode(width, 800.0, ctx.get_state("inspector_open"));
    let actions = [];
    if sidebar_mode != "dock" {
        actions.push(button::Button(#{ key: "open-sidebar", text: "Pages", size: "sm",
            on_click: Fn("set_flag").curry("sidebar_open").curry(true) }));
    }
    if inspector_mode != "dock" {
        actions.push(button::Button(#{ key: "open-inspector", text: "Details", size: "sm",
            on_click: Fn("set_flag").curry("inspector_open").curry(true) }));
    }
    let main = column([
        text(`${width.to_int()}px wide: sidebar ${sidebar_mode}, inspector ${inspector_mode}`),
        row(actions).with_style(style().gap(theme_length("space.related"))),
    ]).with_style(style().flex_col().gap(theme_length("space.group")).padding(theme_spacing("lg")));
    let shell = app_shell::AppShell(#{ key: "shell-demo", label: "Demo shell",
        sidebar: column([text("Pages")]).with_style(style().padding(theme_spacing("lg"))),
        main: main,
        inspector: column([text("Details")]).with_style(style().padding(theme_spacing("lg"))),
        sidebar_width: px(180), inspector_width: px(240),
        sidebar_mode: sidebar_mode, inspector_mode: inspector_mode,
        on_sidebar_dismiss: Fn("set_flag").curry("sidebar_open").curry(false),
        on_inspector_dismiss: Fn("set_flag").curry("inspector_open").curry(false) });
    stack::Stack(#{ align: "start", children: [
        text("Below 800px the inspector becomes a drawer, below 600px the sidebar too: the shell measures its own frame, not the window. A drawer is modal and opens from the window's edge; Escape or a press on the scrim closes it.")
            .with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        toggle_group::ToggleGroup(#{ key: "width", label: "Shell width", values: [choice],
            allow_empty: false, size: "sm", on_change: Fn("choose_width"),
            items: [#{ value: "wide", label: "900" }, #{ value: "medium", label: "680" }, #{ value: "narrow", label: "480" }] }),
        column([shell]).with_key("frame").with_ref(frame).with_style(style().width(frame_width).height(px(320))
            .border(px(1)).border_color(theme_color("border"))),
    ] }).with_style(ctx.component_style("root", style()))
        // A window shell in miniature: its regions keep their own spacing, not the page's.
        .audit_allow(["spacing-not-nested"])
}
