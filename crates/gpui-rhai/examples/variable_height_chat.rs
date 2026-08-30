use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");

const MAIN: &str = r#"
fn message(index) {
    let height = if index % 7 == 0 { 92 } else if index % 3 == 0 { 58 } else { 36 };
    let content = if index % 7 == 0 {
        `Message ${index}: a longer multi-line-shaped bubble used to exercise variable measurements.`
    } else { `Message ${index}` };
    #{
        key: `message-${index}`,
        node: box([
            text(content).with_key("body")
        ]).with_key(`bubble-${index}`)
            .with_style(style().height(px(height)).padding(px(10)).radius(px(12))
                .background(if index % 2 == 0 {
                    theme_color("surface_raised")
                } else { theme_color("surface_hover") }))
            .accessibility_role("listitem")
            .accessibility_label(content)
    }
}

fn view(ctx) {
    let items = [];
    for index in 0..2000 { items.push(message(index)); }
    box([
        text([span("VARIABLE ").bold(), span("CHAT").color(theme_color("accent"))])
            .with_key("title").with_style(style().font_size(rem(1.3))),
        virtual_list(#{
            key: "messages", label: "Messages", estimated_height: 48,
            height: 520, overdraw_pixels: 240, alignment: "bottom",
            follow_tail: true, items: items
        }).with_key("message-list")
    ]).with_key("chat").with_style(style().flex_col().gap(px(14)).padding(px(24))
        .width(px(620)).background(theme_color("surface")))
}
"#;

fn chat_view() -> EmbeddedScriptView {
    EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(BTreeMap::from([(
            ModuleId::parse("main").unwrap(),
            MAIN.to_owned(),
        )])),
        THEME,
    )
}

fn main() {
    let view = chat_view().prepare().expect("variable chat prepares");
    ScriptApplication::new(view)
        .window_size(700.0, 640.0)
        .run()
        .expect("variable chat runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_height_chat_prepares() {
        chat_view().prepare().unwrap();
        let mut engine = gpui_rhai::RuntimeEngine::new();
        let compiled = engine.compile(MAIN).unwrap();
        let context = gpui_rhai::UiContext::new(
            std::rc::Rc::new(std::cell::RefCell::new(gpui_rhai::UiRuntimeState::new())),
            gpui_rhai::ComponentInstancePath::root("App", "chat-test"),
            Some("main".to_owned()),
            gpui_rhai::ExecutionPhase::Render,
            BTreeMap::new(),
        );
        let root = engine.render_with_context(&compiled, context).unwrap();
        let gpui_rhai::UiNodeKind::Box { children } = root.kind() else {
            panic!("chat root must be Box");
        };
        assert!(matches!(
            children[1].kind(),
            gpui_rhai::UiNodeKind::VirtualList { spec } if spec.items.len() == 2000
        ));
    }
}
