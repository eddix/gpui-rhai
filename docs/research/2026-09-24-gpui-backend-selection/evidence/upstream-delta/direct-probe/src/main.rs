use gpui::{Context, IntoElement, InteractiveElement, ParentElement, Render, Role, StatefulInteractiveElement, Styled, Window, div};

struct Probe;
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().id("root").child(
            div().id("probe-button")
                .role(Role::Button)
                .accessibility_id("rhai.probe.button")
                .aria_label("Native semantic probe")
                .aria_description("Core GPUI APIs, without Kit or Base")
                .aria_selected(false)
                .size(gpui::px(48.0))
        )
    }
}

fn main() {
    let _native_application = gpui_platform::application();
    let _root = Probe;
}
