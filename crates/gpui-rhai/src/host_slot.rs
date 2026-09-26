use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};
use thiserror::Error;

use crate::{
    ComponentStateSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveTheme, PrimitiveValue,
    RuntimeEngine, ScriptViewExtension, ScriptViewHandle, UiValue, ValueSchema,
};

type HostSlotFactory = Rc<dyn Fn(&mut Window, &mut App) -> Result<AnyElement, String>>;

/// Host-owned opaque elements exposed to one prepared script view.
///
/// The registry is installed as a [`ScriptViewExtension`]. Rhai can place a
/// registered element through `gpui_rhai::HostSlot`, but it never receives the
/// element, entity, or lifecycle handle.
#[derive(Clone, Default)]
pub struct HostSlotRegistry {
    slots: BTreeMap<String, HostSlotFactory>,
}

impl fmt::Debug for HostSlotRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostSlotRegistry")
            .field("slots", &self.slots.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl HostSlotRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one named foreground element factory.
    ///
    /// Configure all slots before preparing the script view. The factory runs
    /// only while GPUI renders the opaque slot on the foreground thread.
    ///
    /// # Errors
    ///
    /// Returns [`HostSlotError`] for invalid or duplicate names.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut Window, &mut App) -> Result<AnyElement, String> + 'static,
    ) -> Result<(), HostSlotError> {
        let name = name.into();
        validate_name(&name)?;
        if self.slots.contains_key(&name) {
            return Err(HostSlotError::Duplicate(name));
        }
        self.slots.insert(name, Rc::new(factory));
        Ok(())
    }

    /// Builder form of [`Self::register`].
    ///
    /// # Errors
    ///
    /// Returns [`HostSlotError`] for invalid or duplicate names.
    pub fn with_slot(
        mut self,
        name: impl Into<String>,
        factory: impl Fn(&mut Window, &mut App) -> Result<AnyElement, String> + 'static,
    ) -> Result<Self, HostSlotError> {
        self.register(name, factory)?;
        Ok(self)
    }

    /// Register an existing GPUI entity as opaque slot content.
    ///
    /// # Errors
    ///
    /// Returns [`HostSlotError`] for invalid or duplicate names.
    pub fn with_entity<V: Render + 'static>(
        self,
        name: impl Into<String>,
        entity: Entity<V>,
    ) -> Result<Self, HostSlotError> {
        self.with_slot(name, move |_, _| Ok(entity.clone().into_any_element()))
    }

    /// Register an independently mounted script view as opaque slot content.
    ///
    /// The Host remains responsible for suspending or disposing the nested
    /// view. The outer Rhai program receives no nested-view authority. A view
    /// from another [`crate::ScriptViewHost`] is rendered inside its own Host
    /// frame boundary; a view already inside the active Host reuses it.
    ///
    /// # Errors
    ///
    /// Returns [`HostSlotError`] for invalid or duplicate names.
    pub fn with_script_view(
        self,
        name: impl Into<String>,
        view: ScriptViewHandle,
    ) -> Result<Self, HostSlotError> {
        self.with_slot(name, move |_, _| {
            view.host_slot_item().map_err(|error| error.to_string())
        })
    }

    fn factory(&self, name: &str) -> Option<HostSlotFactory> {
        self.slots.get(name).cloned()
    }
}

impl ScriptViewExtension for HostSlotRegistry {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_primitive(
                host_slot_primitive_descriptor(),
                HostSlotPrimitiveHandler {
                    slots: self.clone(),
                },
            )
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum HostSlotError {
    #[error("host slot name `{0}` is invalid")]
    InvalidName(String),
    #[error("host slot `{0}` is registered more than once")]
    Duplicate(String),
}

fn validate_name(name: &str) -> Result<(), HostSlotError> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        Err(HostSlotError::InvalidName(name.to_owned()))
    } else {
        Ok(())
    }
}

fn host_slot_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.host_slot")
            .expect("built-in HostSlot primitive ID is valid"),
        export: "HostSlot".to_owned(),
        props: BTreeMap::from([(
            "name".to_owned(),
            ObjectField::required(ValueSchema::string()),
        )]),
        events: BTreeMap::new(),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

struct HostSlotPrimitiveHandler {
    slots: HostSlotRegistry,
}

impl PrimitiveHandler for HostSlotPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        _: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let Some(PrimitiveValue::Data(UiValue::String(name))) = instance.node.props.get("name")
        else {
            return Err("HostSlot.name must be a string".to_owned());
        };
        let factory = self
            .slots
            .factory(name)
            .ok_or_else(|| format!("host slot `{name}` is not registered"))?;
        let child = factory(window, cx)?;
        let node = instance
            .id
            .as_ref()
            .map(crate::PrimitiveInstanceId::node)
            .ok_or_else(|| "HostSlot requires retained identity".to_owned())?;
        let mut boundary = div()
            .id(SharedString::from(format!("gpui-rhai-host-slot-{node}")))
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .occlude()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation());
        for button in MouseButton::all() {
            boundary = boundary.on_mouse_up(button, |_, _, cx| cx.stop_propagation());
        }
        Ok(boundary
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(child)
            .into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_invalid_and_duplicate_names() {
        assert_eq!(
            HostSlotRegistry::new()
                .with_slot("bad/name", |_, _| Ok(div().into_any_element()))
                .unwrap_err(),
            HostSlotError::InvalidName("bad/name".to_owned())
        );

        let mut slots = HostSlotRegistry::new();
        slots
            .register("content", |_, _| Ok(div().into_any_element()))
            .unwrap();
        assert_eq!(
            slots
                .register("content", |_, _| Ok(div().into_any_element()))
                .unwrap_err(),
            HostSlotError::Duplicate("content".to_owned())
        );
    }

    #[test]
    fn extension_registers_a_keyed_opaque_primitive_constructor() {
        let slots = HostSlotRegistry::new()
            .with_slot("content", |_, _| Ok(div().into_any_element()))
            .unwrap();
        let mut engine = RuntimeEngine::new();
        slots.configure_engine(&mut engine).unwrap();
        let compiled = engine
            .compile(
                r#"
                    fn view() {
                        gpui_rhai::HostSlot(#{ key: "content", name: "content" })
                    }
                "#,
            )
            .unwrap();
        let root = engine.render(&compiled).unwrap();
        let crate::UiNodeKind::Custom { primitive } = root.kind() else {
            panic!("HostSlot must remain an ordinary custom primitive");
        };
        assert_eq!(primitive.primitive.as_str(), "gpui_rhai.host_slot");
        assert_eq!(primitive.key.as_deref(), Some("content"));
        assert_eq!(
            primitive.props.get("name"),
            Some(&PrimitiveValue::Data(UiValue::String("content".to_owned())))
        );
    }
}
