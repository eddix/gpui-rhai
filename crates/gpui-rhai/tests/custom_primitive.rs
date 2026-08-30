use std::collections::BTreeMap;

use gpui::{AnyElement, App, IntoElement, Window, div};
use gpui_rhai::{
    ComponentStateSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveValue, RuntimeEngine, UiNodeKind,
    UiValue, ValueSchema,
};

struct DownstreamEditor;

impl PrimitiveHandler for DownstreamEditor {
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        _: &gpui_rhai::PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        Ok(div().into_any_element())
    }
}

#[test]
fn downstream_crate_registers_namespaced_primitive() {
    let mut runtime = RuntimeEngine::new();
    runtime
        .register_primitive(
            PrimitiveDescriptor {
                id: PrimitiveId::parse("my_app.code_editor").unwrap(),
                export: "CodeEditor".to_owned(),
                props: BTreeMap::from([(
                    "value".to_owned(),
                    ObjectField::required(ValueSchema::string()),
                )]),
                events: BTreeMap::new(),
                state: ComponentStateSchema::default(),
                lifecycle: false,
            },
            DownstreamEditor,
        )
        .unwrap();

    let compiled = runtime
        .compile(
            r#"
                fn view() {
                    my_app::CodeEditor(#{ value: "hello" })
                }
            "#,
        )
        .unwrap();
    let root = runtime.render(&compiled).unwrap();
    let UiNodeKind::Custom { primitive } = root.kind() else {
        panic!("custom primitive constructor returned a non-custom node");
    };
    assert_eq!(
        primitive.props.get("value"),
        Some(&PrimitiveValue::Data(UiValue::String("hello".to_owned())))
    );
}
