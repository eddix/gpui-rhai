//! A validated key binding spec converts against a real App: `KeyBinding::load`
//! reads the keyboard mapper and host OS from it on both backends.
use gpui::TestAppContext;
use gpui_rhai::{ActionId, KeyBindingSpec};

#[gpui::test]
fn key_binding_spec_converts_against_the_app(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let binding = KeyBindingSpec::new(
            "cmd-s",
            ActionId::parse("document.save").unwrap(),
            Some("Editor && mode == full".to_owned()),
        )
        .unwrap();
        binding.to_gpui(cx).unwrap();
    });
}