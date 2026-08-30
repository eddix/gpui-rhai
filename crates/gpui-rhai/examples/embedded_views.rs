use std::collections::BTreeMap;

use gpui::{
    App, AppContext, Application, Bounds, Context, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHandle,
    ScriptViewHost, install,
};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const DROPDOWN: &str = include_str!("../../../registry/components/dropdown.rhai");
const TOAST: &str = include_str!("../../../registry/components/toast.rhai");
const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");

const WIDGET: &str = r#"
import "components/button" as button;
import "components/dropdown" as dropdown;
import "components/toast" as toast;

fn state_schema() {
    #{ fields: #{
        count: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } },
        open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: __OPEN__ } },
        toast_visible: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: __TOAST__ } },
    } }
}

fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
fn set_open(ctx, open) { ctx.set_state("open", open); }
fn selected(ctx, values) { ctx.set_state("open", false); }
fn dismiss_toast(ctx, id) { ctx.set_state("toast_visible", false); }

fn view(ctx) {
    let items = [];
    if ctx.get_state("toast_visible") {
        items.push(#{
            id: "shared-toast", title: `Toast from ${ctx.view_id()}`,
            message: "IDs are namespaced by the host.", duration_ms: 60000,
        });
    }
    column([
        toast::Toast(#{ key: "toast-host", items: items, on_dismiss: Fn("dismiss_toast") }),
        text(`View: ${ctx.view_id()}`),
        text(`Window: ${ctx.window_id()}`),
        text(`Responsive: ${ctx.viewport_class()}`),
        text(`Count: ${ctx.get_state("count")}`),
        button::Button(#{ text: "Increment", size: "sm", on_click: Fn("increment") }),
        dropdown::Dropdown(#{
            key: "shared-dropdown",
            options: [
                #{ value: "one", label: "A deliberately wide dropdown option" },
                #{ value: "two", label: "Second option" }
            ],
            selected: [], open: ctx.get_state("open"),
            on_change: Fn("selected"), on_open_change: Fn("set_open")
        })
    ]).with_style(
        style().width(relative(1.0)).height(relative(1.0)).padding(px(12)).gap(px(8))
            .background(theme_color("surface_raised"))
    )
}
"#;

fn prepared_widget(open: bool, toast: bool) -> gpui_rhai::PreparedScriptView {
    let main = WIDGET
        .replace("__OPEN__", if open { "true" } else { "false" })
        .replace("__TOAST__", if toast { "true" } else { "false" });
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("main").unwrap(), main),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
        (
            ModuleId::parse("components/dropdown").unwrap(),
            DROPDOWN.to_owned(),
        ),
        (
            ModuleId::parse("components/toast").unwrap(),
            TOAST.to_owned(),
        ),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, THEME)
        .prepare()
        .expect("embedded widget prepares")
}

struct EmbeddedViewsDemo {
    host: ScriptViewHost,
    first: ScriptViewHandle,
    second: ScriptViewHandle,
    third: Option<ScriptViewHandle>,
}

impl EmbeddedViewsDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        install(cx);
        let host = ScriptViewHost::new("demo-window", cx).expect("valid host");
        let first = prepared_widget(true, true)
            .mount(
                ScriptViewConfig::new("small-widget"),
                host.clone(),
                window,
                cx,
            )
            .expect("small widget mounts");
        let second = prepared_widget(false, true)
            .mount(
                ScriptViewConfig::new("middle-widget"),
                host.clone(),
                window,
                cx,
            )
            .expect("middle widget mounts");
        let third = prepared_widget(false, false)
            .mount(
                ScriptViewConfig::new("third-widget"),
                host.clone(),
                window,
                cx,
            )
            .expect("third widget mounts");
        Self {
            host,
            first,
            second,
            third: Some(third),
        }
    }

    fn toggle_third(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(view) = self.third.take() {
            view.dispose(cx).expect("third widget disposes");
        } else {
            self.third = Some(
                prepared_widget(false, false)
                    .mount(
                        ScriptViewConfig::new("third-widget"),
                        self.host.clone(),
                        window,
                        cx,
                    )
                    .expect("third widget remounts"),
            );
        }
        cx.notify();
    }
}

impl Render for EmbeddedViewsDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let first = self.first.element().expect("first view is live");
        let second = self.second.element().expect("second view is live");
        let third = self.third.as_ref().map_or_else(
            || div().child("Third widget is unmounted").into_any_element(),
            |view| view.element().expect("third view is live"),
        );
        let toggle = div()
            .id("toggle-third")
            .px_3()
            .py_2()
            .bg(gpui::rgb(0x0025_63eb))
            .text_color(gpui::white())
            .on_click(cx.listener(|this, _, window, cx| this.toggle_third(window, cx)))
            .child(if self.third.is_some() {
                "Unmount third widget"
            } else {
                "Remount third widget"
            });
        self.host.container(
            div()
                .size_full()
                .p_4()
                .gap_3()
                .flex()
                .flex_col()
                .child(toggle)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap_3()
                        .items_start()
                        .child(div().w(px(200.0)).h(px(300.0)).child(first))
                        .child(div().w(px(280.0)).h(px(300.0)).child(second))
                        .child(div().flex_1().h(px(300.0)).child(third)),
                ),
        )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        install(cx);
        let bounds = Bounds::centered(None, size(px(900.0), px(420.0)), cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |window, cx| cx.new(|cx| EmbeddedViewsDemo::new(window, cx)),
        )
        .expect("embedded views window opens");
        cx.activate(true);
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_widget_variants_prepare_independently() {
        super::prepared_widget(true, true);
        super::prepared_widget(false, true);
        super::prepared_widget(false, false);
    }
}
