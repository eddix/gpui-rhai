use std::collections::BTreeMap;

use gpui::{
    App, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHandle,
    ScriptViewHost, install,
};

const STACK: &str = include_str!("../../../registry/layouts/stack.rhai");
const INLINE: &str = include_str!("../../../registry/layouts/inline.rhai");
const DESCRIPTION_LIST: &str = include_str!("../../../registry/patterns/description_list.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
const TOAST: &str = include_str!("../../../registry/components/toast.rhai");
const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");

const WIDGET: &str = r#"
import "layouts/stack" as stack;
import "layouts/inline" as inline;
import "patterns/description_list" as description_list;
import "components/button" as button;
import "components/combobox" as combobox;
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

fn facts(ctx) {
    description_list::DescriptionList(#{ label: "View", label_width: px(80), items: [
        #{ label: "View", value: ctx.view_id(), identifier: true },
        #{ label: "Window", value: ctx.window_id(), identifier: true },
        #{ label: "Responsive", value: ctx.viewport_class() },
        #{ label: "Count", value: `${ctx.get_state("count")}`, numeric: true },
    ] })
}

fn controls(ctx) {
    let choice = combobox::Combobox(#{
        key: "shared-combobox", label: "Shared choice", width: relative(1.0),
        options: [
            #{ value: "one", label: "A deliberately wide combobox option" },
            #{ value: "two", label: "Second option" }
        ],
        selected: [], open: ctx.get_state("open"), query: "",
        on_change: Fn("selected"), on_open_change: Fn("set_open")
    });
    stack::Stack(#{ children: [
        inline::Inline(#{ children: [
            button::Button(#{ key: "increment", text: "Increment", on_click: Fn("increment") })
        ] }),
        choice
    ] })
}

fn view(ctx) {
    let items = [];
    if ctx.get_state("toast_visible") {
        items.push(#{
            id: "shared-toast", title: `Toast from ${ctx.view_id()}`,
            message: "IDs are namespaced by the host.", duration_ms: 60000,
        });
    }
    // A narrow view switches itself to compact density.
    let density = if ctx.viewport_class() == "compact" { "compact" } else { "comfortable" };
    column([
        toast::Toast(#{ key: "toast-host", items: items, on_dismiss: Fn("dismiss_toast") }),
        stack::Stack(#{ gap: "group", children: [facts(ctx), controls(ctx)] })
    ]).with_style(
        style().width(relative(1.0)).height(relative(1.0)).padding(theme_length("metrics.inset"))
            .background(theme_color("surface_raised"))
    ).env(#{ density: density })
}
"#;

/// Prepare one widget view, optionally with its Combobox open and a Toast.
///
/// # Panics
///
/// Panics if the bundled widget sources fail to prepare.
pub fn prepared_widget(open: bool, toast: bool) -> gpui_rhai::PreparedScriptView {
    let main = WIDGET
        .replace("__OPEN__", if open { "true" } else { "false" })
        .replace("__TOAST__", if toast { "true" } else { "false" });
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("main").unwrap(), main),
        (ModuleId::parse("layouts/stack").unwrap(), STACK.to_owned()),
        (
            ModuleId::parse("layouts/inline").unwrap(),
            INLINE.to_owned(),
        ),
        (
            ModuleId::parse("patterns/description_list").unwrap(),
            DESCRIPTION_LIST.to_owned(),
        ),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/toast").unwrap(),
            TOAST.to_owned(),
        ),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, THEME)
        .token_base(include_str!("../../../registry/tokens.rhai"))
        .asset_sources(registry_icons())
        .prepare()
        .expect("embedded widget prepares")
}

/// Logical window size of the example.
pub const WINDOW: (f32, f32) = (900.0, 420.0);

/// Three independent Rhai views in one Host-owned GPUI layout.
pub struct EmbeddedViewsDemo {
    host: ScriptViewHost,
    first: ScriptViewHandle,
    second: ScriptViewHandle,
    third: Option<ScriptViewHandle>,
}

impl EmbeddedViewsDemo {
    /// Mount the three widgets in one Host.
    ///
    /// # Panics
    ///
    /// Panics if a widget fails to mount.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
        // Host chrome follows the views' theme instead of hard-coding colors.
        let theme = self.first.theme_snapshot(cx).expect("first view is live");
        let color = |token: &str| {
            gpui::rgba(
                theme
                    .variant
                    .tokens
                    .colors
                    .get(token)
                    .map_or(0x0000_00ff, |color| color.as_rgba_hex()),
            )
        };
        let first = self.first.element().expect("first view is live");
        let second = self.second.element().expect("second view is live");
        let third = self.third.as_ref().map_or_else(
            || {
                div()
                    .text_color(color("text_muted"))
                    .child("Third widget is unmounted")
                    .into_any_element()
            },
            |view| view.element().expect("third view is live"),
        );
        let toggle = div()
            .id("toggle-third")
            .px_3()
            .py_1()
            .bg(color("surface_hover"))
            .text_color(color("text_primary"))
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
                .gap_4()
                .flex()
                .flex_col()
                .items_start()
                .bg(color("surface"))
                .child(toggle)
                .child(
                    div()
                        .w_full()
                        .flex_1()
                        .flex()
                        .gap_4()
                        .items_start()
                        .child(div().w(px(200.0)).h(px(300.0)).child(first))
                        .child(div().w(px(280.0)).h(px(300.0)).child(second))
                        .child(div().flex_1().h(px(300.0)).child(third)),
                ),
        )
    }
}

#[allow(dead_code)]
fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        install(cx);
        let bounds = Bounds::centered(None, size(px(WINDOW.0), px(WINDOW.1)), cx);
        cx.on_window_closed(|cx, _| {
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

include!("support/icons.rs");

#[cfg(test)]
mod tests {
    #[test]
    fn all_widget_variants_prepare_independently() {
        super::prepared_widget(true, true);
        super::prepared_widget(false, true);
        super::prepared_widget(false, false);
    }
}
