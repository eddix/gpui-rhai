//! Host-domain typed application drag/drop primitives.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, Background, Bounds, Context, DispatchPhase, Edges, Element,
    ElementId, Entity, FocusHandle, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent,
    PaintQuad, ParentElement, Pixels, Render, Style, Styled, Window, div, px, relative, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveTheme, Rgba8, UiValue, ValueSchema,
};

#[derive(Clone)]
struct DragSourceConfig {
    id: String,
    source_id: String,
    payload_type: String,
    payload: UiValue,
    operation: crate::interaction::DragOperation,
    threshold: f64,
    keyboard_target: Option<String>,
    disabled: bool,
    focus: Option<FocusHandle>,
    accent: Rgba8,
}

#[derive(Clone, Debug, PartialEq)]
struct DragSourceFingerprint {
    source_id: String,
    payload_type: String,
    payload: UiValue,
    operation: crate::interaction::DragOperation,
    disabled: bool,
}

#[derive(Clone)]
struct DragSourceState(Rc<RefCell<DragSourceFingerprint>>);

impl DragSourceState {
    fn new(config: &DragSourceConfig) -> Self {
        Self(Rc::new(RefCell::new(source_fingerprint(config))))
    }
}

fn source_fingerprint(config: &DragSourceConfig) -> DragSourceFingerprint {
    DragSourceFingerprint {
        source_id: config.source_id.clone(),
        payload_type: config.payload_type.clone(),
        payload: config.payload.clone(),
        operation: config.operation,
        disabled: config.disabled,
    }
}

struct SourcePrepaint {
    hitbox: Hitbox,
}

struct DragSourceElement {
    config: DragSourceConfig,
    context: PrimitiveContext,
}

impl IntoElement for DragSourceElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DragSourceElement {
    type RequestLayoutState = ();
    type PrepaintState = SourcePrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(self.config.id.clone().into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    size: size(relative(1.0).into(), relative(1.0).into()),
                    ..Style::default()
                },
                None,
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> SourcePrepaint {
        let state = window
            .use_state(cx, |_, _| DragSourceState::new(&self.config))
            .read(cx)
            .clone();
        let changed = {
            let next = source_fingerprint(&self.config);
            let mut fingerprint = state.0.borrow_mut();
            let changed = *fingerprint != next;
            *fingerprint = next;
            changed
        };
        let owner = self.context.interaction_owner(&self.config.id);
        if changed {
            self.context.cancel_interaction(&owner, window, cx);
        }
        SourcePrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        prepaint: &mut SourcePrepaint,
        window: &mut Window,
        _: &mut App,
    ) {
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.present_interaction(owner.clone());
        if self.context.app_drag_source_active(&owner) {
            paint_feedback(bounds, self.config.accent, 0x16, window);
        }
        register_source_pointer(prepaint, &self.config, &self.context, window);
    }
}

fn register_source_pointer(
    prepaint: &SourcePrepaint,
    config: &DragSourceConfig,
    context: &PrimitiveContext,
    window: &mut Window,
) {
    let hitbox = prepaint.hitbox.clone();
    let config = config.clone();
    let context = context.clone();
    let view = window.current_view();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || config.disabled
            || !hitbox.is_hovered(window)
        {
            return;
        }
        if let Some(focus) = config.focus.as_ref() {
            focus.focus(window, cx);
        }
        let owner = context.interaction_owner(&config.id);
        let spec = drag_spec(&config, owner.clone(), view);
        let finish_context = context.clone();
        context.begin_application_drag(
            spec,
            event.position,
            config.threshold,
            move |result, cancelled, window, cx| {
                finish_context.propose("drag_end", drag_end_value(&result, cancelled), window, cx);
            },
            None,
            window,
            cx,
        );
        cx.stop_propagation();
    });
}

fn drag_spec(
    config: &DragSourceConfig,
    owner: crate::interaction::InteractionOwner,
    notify: gpui::EntityId,
) -> crate::interaction::ApplicationDragSpec {
    crate::interaction::ApplicationDragSpec::new(
        owner,
        config.source_id.clone(),
        config.payload_type.clone(),
        config.payload.clone(),
        config.operation,
        notify,
    )
}

#[derive(Clone)]
struct DropZoneConfig {
    id: String,
    target_id: String,
    payload_types: BTreeSet<String>,
    operations: BTreeSet<crate::interaction::DragOperation>,
    priority: i64,
    disabled: bool,
    accent: Rgba8,
    danger: Rgba8,
}

struct ZonePrepaint {
    bounds: Bounds<Pixels>,
    hitbox: Hitbox,
}

struct DropZoneElement {
    config: DropZoneConfig,
    context: PrimitiveContext,
}

impl IntoElement for DropZoneElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for DropZoneElement {
    type RequestLayoutState = ();
    type PrepaintState = ZonePrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(self.config.id.clone().into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    size: size(relative(1.0).into(), relative(1.0).into()),
                    ..Style::default()
                },
                None,
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        _: &mut App,
    ) -> ZonePrepaint {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        ZonePrepaint { bounds, hitbox }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        prepaint: &mut ZonePrepaint,
        window: &mut Window,
        _: &mut App,
    ) {
        let owner = self.context.interaction_owner(&self.config.id);
        if self.config.disabled {
            return;
        }
        let Ok(bounds) = crate::GeometryBounds::new(
            f64::from(prepaint.bounds.origin.x),
            f64::from(prepaint.bounds.origin.y),
            f64::from(prepaint.bounds.size.width),
            f64::from(prepaint.bounds.size.height),
        ) else {
            return;
        };
        let target_context = self.context.clone();
        let target_id = self.config.target_id.clone();
        self.context
            .register_drop_target(crate::interaction::DropTargetRegistration::new(
                owner.clone(),
                self.config.target_id.clone(),
                bounds,
                self.config.payload_types.clone(),
                self.config.operations.clone(),
                self.config.priority,
                prepaint.hitbox.clone(),
                self.context.ancestor_scroll_handles(),
                window.current_view(),
                move |drag, position, window, cx| {
                    target_context.propose(
                        "drop",
                        drop_value(drag, &target_id, position),
                        window,
                        cx,
                    );
                },
            ));
        match self.context.drop_target_state(&owner) {
            crate::interaction::DropTargetState::Active => {
                paint_feedback(prepaint.bounds, self.config.accent, 0x28, window);
            }
            crate::interaction::DropTargetState::Eligible => {
                paint_feedback(prepaint.bounds, self.config.accent, 0x0e, window);
            }
            crate::interaction::DropTargetState::Invalid => {
                paint_feedback(prepaint.bounds, self.config.danger, 0x12, window);
            }
            crate::interaction::DropTargetState::Idle => {}
        }
    }
}

fn paint_feedback(bounds: Bounds<Pixels>, color: Rgba8, alpha: u8, window: &mut Window) {
    let translucent = Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | u32::from(alpha));
    window.paint_quad(PaintQuad {
        bounds,
        corner_radii: px(0.0).into(),
        background: Background::from(rgba(translucent.as_rgba_hex())),
        border_widths: Edges::all(px(1.0)),
        border_color: rgba(color.as_rgba_hex()).into(),
        border_style: gpui::BorderStyle::default(),
    });
}

fn drop_value(
    drag: &crate::interaction::ApplicationDragSpec,
    target_id: &str,
    position: gpui::Point<Pixels>,
) -> UiValue {
    UiValue::Map(BTreeMap::from([
        (
            "source_id".to_owned(),
            UiValue::String(drag.source_id().to_owned()),
        ),
        (
            "target_id".to_owned(),
            UiValue::String(target_id.to_owned()),
        ),
        (
            "payload_type".to_owned(),
            UiValue::String(drag.payload_type().to_owned()),
        ),
        ("payload".to_owned(), drag.payload().clone()),
        (
            "operation".to_owned(),
            UiValue::String(drag.operation().as_str().to_owned()),
        ),
        ("x".to_owned(), UiValue::Float(f64::from(position.x))),
        ("y".to_owned(), UiValue::Float(f64::from(position.y))),
    ]))
}

fn drag_end_value(result: &crate::interaction::ApplicationDropResult, cancelled: bool) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("accepted".to_owned(), UiValue::Bool(result.accepted)),
        (
            "target_id".to_owned(),
            result
                .target_id
                .as_ref()
                .map_or(UiValue::Null, |target| UiValue::String(target.clone())),
        ),
        (
            "operation".to_owned(),
            UiValue::String(result.operation.as_str().to_owned()),
        ),
        ("cancelled".to_owned(), UiValue::Bool(cancelled)),
    ]))
}

struct DragSourceEntity {
    focus: FocusHandle,
    config: DragSourceConfig,
    context: PrimitiveContext,
}

impl DragSourceEntity {
    fn new(
        mut config: DragSourceConfig,
        context: PrimitiveContext,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = config.focus.clone().unwrap_or_else(|| cx.focus_handle());
        config.focus = Some(focus.clone());
        Self {
            focus,
            config,
            context,
        }
    }

    fn update(&mut self, mut config: DragSourceConfig, context: PrimitiveContext) {
        config.focus = Some(self.focus.clone());
        self.config = config;
        self.context = context;
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.disabled || !matches!(event.keystroke.key.as_str(), "enter" | "space") {
            return;
        }
        let Some(target) = self.config.keyboard_target.as_deref() else {
            return;
        };
        let owner = self.context.interaction_owner(&self.config.id);
        let spec = drag_spec(&self.config, owner, cx.entity_id());
        let result = self
            .context
            .perform_keyboard_drop(&spec, target, window, cx);
        self.context
            .propose("drag_end", drag_end_value(&result, false), window, cx);
        cx.stop_propagation();
    }
}

impl Render for DragSourceEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus.clone().tab_stop(!self.config.disabled))
            .on_key_down(cx.listener(Self::key_down))
            .child(DragSourceElement {
                config: self.config.clone(),
                context: self.context.clone(),
            })
    }
}

#[derive(Default)]
pub struct DragSourcePrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<DragSourceEntity>>,
}

impl PrimitiveHandler for DragSourcePrimitiveHandler {
    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        theme: &PrimitiveTheme,
        _: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "DragSourcePrimitive requires a stable key".to_owned())?;
        let config = parse_source(
            &instance.node.props,
            instance.focus_handle().cloned(),
            theme,
        )?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| DragSourceEntity::new(config.clone(), context.clone(), cx));
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |source, _| source.update(config, context.clone()));
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

#[derive(Default)]
pub struct DropZonePrimitiveHandler;

impl PrimitiveHandler for DropZonePrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        theme: &PrimitiveTheme,
        _: &mut Window,
        _: &mut App,
    ) -> Result<AnyElement, String> {
        Ok(DropZoneElement {
            config: parse_target(&instance.node.props, theme)?,
            context: context.clone(),
        }
        .into_any_element())
    }
}

fn parse_source(
    props: &PrimitiveProps,
    focus: Option<FocusHandle>,
    theme: &PrimitiveTheme,
) -> Result<DragSourceConfig, String> {
    let source_id = required_safe_string(props, "source_id")?;
    let payload_type = required_safe_string(props, "payload_type")?;
    let operation = props
        .string("operation")
        .and_then(crate::interaction::DragOperation::parse)
        .ok_or_else(|| "drag source operation must be copy or move".to_owned())?;
    let threshold = props.number("threshold").unwrap_or(4.0);
    if !threshold.is_finite() || !(0.0..=64.0).contains(&threshold) {
        return Err("drag source threshold must be finite and in [0,64]".to_owned());
    }
    let payload = props
        .data("payload")
        .cloned()
        .ok_or_else(|| "drag source requires payload".to_owned())?;
    Ok(DragSourceConfig {
        id: format!("gpui-rhai-drag-source:{source_id}"),
        source_id,
        payload_type,
        payload,
        operation,
        threshold,
        keyboard_target: props.string("keyboard_target").map(str::to_owned),
        disabled: props.boolean("disabled").unwrap_or(false),
        focus,
        accent: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
    })
}

fn parse_target(props: &PrimitiveProps, theme: &PrimitiveTheme) -> Result<DropZoneConfig, String> {
    let target_id = required_safe_string(props, "target_id")?;
    let payload_types = string_set(props, "payload_types")?;
    let operations = operation_set(props, "operations")?;
    if payload_types.is_empty() || operations.is_empty() {
        return Err("drop zone requires at least one payload type and operation".to_owned());
    }
    Ok(DropZoneConfig {
        id: format!("gpui-rhai-drop-zone:{target_id}"),
        target_id,
        payload_types,
        operations,
        priority: props.integer("priority").unwrap_or(0),
        disabled: props.boolean("disabled").unwrap_or(false),
        accent: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
        danger: theme
            .color("danger")
            .unwrap_or(Rgba8::from_rgba_hex(0xdc26_26ff)),
    })
}

fn required_safe_string(props: &PrimitiveProps, name: &str) -> Result<String, String> {
    let value = props
        .string(name)
        .ok_or_else(|| format!("{name} is required"))?;
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        Err(format!("{name} must be 1-128 non-control characters"))
    } else {
        Ok(value.to_owned())
    }
}

fn string_set(props: &PrimitiveProps, name: &str) -> Result<BTreeSet<String>, String> {
    let Some(UiValue::Array(values)) = props.data(name) else {
        return Err(format!("{name} must be an array"));
    };
    values
        .iter()
        .map(|value| match value {
            UiValue::String(value)
                if !value.is_empty()
                    && value.len() <= 128
                    && !value.chars().any(char::is_control) =>
            {
                Ok(value.clone())
            }
            _ => Err(format!("{name} entries must be safe strings")),
        })
        .collect()
}

fn operation_set(
    props: &PrimitiveProps,
    name: &str,
) -> Result<BTreeSet<crate::interaction::DragOperation>, String> {
    let Some(UiValue::Array(values)) = props.data(name) else {
        return Err(format!("{name} must be an array"));
    };
    values
        .iter()
        .map(|value| match value {
            UiValue::String(value) => crate::interaction::DragOperation::parse(value)
                .ok_or_else(|| format!("{name} entries must be copy or move")),
            _ => Err(format!("{name} entries must be strings")),
        })
        .collect()
}

fn drag_end_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        (
            "accepted".to_owned(),
            ObjectField::required(ValueSchema::Bool),
        ),
        (
            "target_id".to_owned(),
            ObjectField::required(ValueSchema::optional(ValueSchema::string())),
        ),
        (
            "operation".to_owned(),
            ObjectField::required(ValueSchema::String {
                allowed: vec!["copy".to_owned(), "move".to_owned()],
            }),
        ),
        (
            "cancelled".to_owned(),
            ObjectField::required(ValueSchema::Bool),
        ),
    ]))
}

fn drop_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        (
            "source_id".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "target_id".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "payload_type".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "payload".to_owned(),
            ObjectField::required(ValueSchema::UiValue),
        ),
        (
            "operation".to_owned(),
            ObjectField::required(ValueSchema::String {
                allowed: vec!["copy".to_owned(), "move".to_owned()],
            }),
        ),
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
    ]))
}

/// Build the native typed application drag-source schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn drag_source_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.drag_source").expect("static primitive ID"),
        export: "DragSourcePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "source_id".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Stable identifier of the dragged item; it is `source_id` in the target's `drop` payload.",
                ),
            ),
            (
                "payload_type".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Type name of the payload; only drop zones that list it in `payload_types` accept the drag.",
                ),
            ),
            (
                "payload".to_owned(),
                ObjectField::required(ValueSchema::UiValue).with_doc(
                    "Bounded application data the drag carries; the drop zone gets it back in its `drop` payload.",
                ),
            ),
            (
                "operation".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec!["copy".to_owned(), "move".to_owned()],
                })
                .with_doc(
                    "Whether dropping copies or moves the item; only drop zones that list it in `operations` accept the drag.",
                ),
            ),
            (
                "threshold".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))).with_doc(
                    "Pointer movement in logical pixels before a press becomes a drag; defaults to 4.",
                ),
            ),
            (
                "keyboard_target".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string())).with_doc(
                    "`target_id` of the drop zone that Enter or Space on the focused source drops onto; `()` for no keyboard drop.",
                ),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Ignores presses and keys and removes the source from the tab order."),
            ),
            (
                "on_drag_end".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the outcome when a started drag is dropped or cancelled, or a keyboard drop runs.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "drag_end".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted when a started drag ends or a keyboard drop runs; the payload says whether and where it was accepted, or cancelled."
                        .to_owned(),
                ),
                payload: drag_end_schema(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

/// Build the native typed application drop-target schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn drop_zone_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.drop_zone").expect("static primitive ID"),
        export: "DropZonePrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "target_id".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Stable identifier of this zone; it is `target_id` in `drop` and in the source's `drag_end`.",
                ),
            ),
            (
                "payload_types".to_owned(),
                ObjectField::required(ValueSchema::Array {
                    items: Box::new(ValueSchema::string()),
                    max_items: Some(32),
                })
                .with_doc(
                    "Payload types this zone accepts; a drag of another type shows the invalid highlight and cannot drop.",
                ),
            ),
            (
                "operations".to_owned(),
                ObjectField::required(ValueSchema::Array {
                    items: Box::new(ValueSchema::String {
                        allowed: vec!["copy".to_owned(), "move".to_owned()],
                    }),
                    max_items: Some(2),
                })
                .with_doc("Operations this zone accepts, `copy`, `move` or both; other drags cannot drop here."),
            ),
            (
                "priority".to_owned(),
                ObjectField::optional(ValueSchema::integer()).with_doc(
                    "Rank among overlapping zones: the highest wins, then the smallest area; defaults to 0.",
                ),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Takes the zone out of drag targeting, so it neither highlights nor accepts drops."),
            ),
            (
                "on_drop".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the drop proposal when an accepted drag is released over this zone or a keyboard drop targets it.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "drop".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted when an accepted drag drops here by pointer or key; the payload is the source, its payload and operation, and window `x`, `y`."
                        .to_owned(),
                ),
                payload: drop_schema(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: false,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_target_acceptance_distinguishes_payload_and_operation() {
        let props = PrimitiveProps::new()
            .with(
                "payload_types",
                crate::PrimitiveValue::Data(UiValue::Array(vec![UiValue::String(
                    "card".to_owned(),
                )])),
            )
            .with(
                "operations",
                crate::PrimitiveValue::Data(UiValue::Array(vec![UiValue::String(
                    "move".to_owned(),
                )])),
            );
        assert_eq!(
            string_set(&props, "payload_types").unwrap(),
            BTreeSet::from(["card".to_owned()])
        );
        assert_eq!(
            operation_set(&props, "operations").unwrap(),
            BTreeSet::from([crate::interaction::DragOperation::Move])
        );
    }
}
