use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_rhai::{RuntimeEngine, ScriptView};

fn main() {
    let mut runtime = RuntimeEngine::new();
    let compiled = runtime
        .compile(
            r#"
                fn view() {
                    column([
                        text("GPUI Rhai"),
                        text("Rhai -> UiNode -> GPUI")
                    ])
                }
            "#,
        )
        .expect("phase-0 script should compile");
    let root = runtime
        .render(&compiled)
        .expect("phase-0 view should evaluate");

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(520.0), px(240.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |_, cx| cx.new(|_| ScriptView::new(root.clone())),
        )
        .expect("phase-0 GPUI window should open");
        cx.activate(true);
    });
}
