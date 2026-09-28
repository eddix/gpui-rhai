//! Keyed controlled sorting over the shared application drag runtime.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, Background, Bounds, Context, DispatchPhase, Edges, Element,
    ElementId, Entity, FocusHandle, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent,
    PaintQuad, ParentElement, Pixels, Point, Render, Style, Styled, Window, div, point, px,
    relative, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, GeometryBounds, ObjectField, PrimitiveContext,
    PrimitiveDescriptor, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitiveProps, PrimitiveTheme, Rgba8, UiValue, ValueSchema,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortDirection {
    Vertical,
    Horizontal,
    Grid,
}

impl SortDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "vertical" => Some(Self::Vertical),
            "horizontal" => Some(Self::Horizontal),
            "grid" => Some(Self::Grid),
            _ => None,
        }
    }

    const fn uses_vertical_targets(self) -> bool {
        matches!(self, Self::Vertical)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Placement {
    Before,
    After,
}

impl Placement {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

#[derive(Clone)]
struct SortableConfig {
    id: String,
    list_id: String,
    item_key: String,
    previous_key: Option<String>,
    next_key: Option<String>,
    first_key: String,
    last_key: String,
    direction: SortDirection,
    threshold: f64,
    disabled: bool,
    item_ref: crate::ElementRef,
    focus: Option<FocusHandle>,
    accent: Rgba8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SortableFingerprint {
    list_id: String,
    item_key: String,
    previous_key: Option<String>,
    next_key: Option<String>,
    direction: SortDirection,
    disabled: bool,
}

#[derive(Clone)]
struct SortableState(Rc<RefCell<SortableFingerprint>>);

impl SortableState {
    fn new(config: &SortableConfig) -> Self {
        Self(Rc::new(RefCell::new(sortable_fingerprint(config))))
    }
}

fn sortable_fingerprint(config: &SortableConfig) -> SortableFingerprint {
    SortableFingerprint {
        list_id: config.list_id.clone(),
        item_key: config.item_key.clone(),
        previous_key: config.previous_key.clone(),
        next_key: config.next_key.clone(),
        direction: config.direction,
        disabled: config.disabled,
    }
}

struct SortablePrepaint {
    hitbox: Hitbox,
    item_bounds: Bounds<Pixels>,
}

struct SortableElement {
    config: SortableConfig,
    context: PrimitiveContext,
}

impl IntoElement for SortableElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SortableElement {
    type RequestLayoutState = ();
    type PrepaintState = SortablePrepaint;

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
    ) -> SortablePrepaint {
        let state = window
            .use_state(cx, |_, _| SortableState::new(&self.config))
            .read(cx)
            .clone();
        let changed = {
            let next = sortable_fingerprint(&self.config);
            let mut fingerprint = state.0.borrow_mut();
            let changed = *fingerprint != next;
            *fingerprint = next;
            changed
        };
        let source = source_owner(&self.context, &self.config);
        if changed {
            self.context.cancel_interaction(&source, window, cx);
        }
        let item_bounds = self
            .context
            .element_bounds(&self.config.item_ref, cx)
            .map_or(bounds, geometry_bounds);
        SortablePrepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
            item_bounds,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        prepaint: &mut SortablePrepaint,
        window: &mut Window,
        _: &mut App,
    ) {
        let source = source_owner(&self.context, &self.config);
        self.context.present_interaction(source.clone());
        register_target(
            &self.context,
            &self.config,
            prepaint.item_bounds,
            Placement::Before,
            window,
        );
        register_target(
            &self.context,
            &self.config,
            prepaint.item_bounds,
            Placement::After,
            window,
        );
        paint_sortable_feedback(&self.context, &self.config, prepaint.item_bounds, window);
        if !self.config.disabled {
            register_pointer_source(prepaint, &self.config, &self.context, window);
        }
    }
}

fn geometry_bounds(bounds: GeometryBounds) -> Bounds<Pixels> {
    Bounds::new(
        point(px(f64_to_f32(bounds.x)), px(f64_to_f32(bounds.y))),
        size(px(f64_to_f32(bounds.width)), px(f64_to_f32(bounds.height))),
    )
}

fn source_owner(
    context: &PrimitiveContext,
    config: &SortableConfig,
) -> crate::interaction::InteractionOwner {
    context.interaction_owner(&config.id).child("source")
}

fn target_owner(
    context: &PrimitiveContext,
    config: &SortableConfig,
    placement: Placement,
) -> crate::interaction::InteractionOwner {
    context
        .interaction_owner(&config.id)
        .child(placement.as_str())
}

fn payload_type(config: &SortableConfig) -> String {
    format!("gpui-rhai/sortable/{}", config.list_id)
}

fn target_id(list_id: &str, anchor: &str, placement: Placement) -> String {
    format!("{list_id}:{}:{anchor}", placement.as_str())
}

fn drag_spec(
    context: &PrimitiveContext,
    config: &SortableConfig,
    notify: gpui::EntityId,
) -> crate::interaction::ApplicationDragSpec {
    crate::interaction::ApplicationDragSpec::new(
        source_owner(context, config),
        config.item_key.clone(),
        payload_type(config),
        UiValue::Map(BTreeMap::from([(
            "key".to_owned(),
            UiValue::String(config.item_key.clone()),
        )])),
        crate::interaction::DragOperation::Move,
        notify,
    )
}

fn target_half(
    bounds: Bounds<Pixels>,
    direction: SortDirection,
    placement: Placement,
) -> Bounds<Pixels> {
    if direction.uses_vertical_targets() {
        let half = bounds.size.height / 2.0;
        Bounds::new(
            point(
                bounds.origin.x,
                bounds.origin.y
                    + if placement == Placement::After {
                        half
                    } else {
                        px(0.0)
                    },
            ),
            size(bounds.size.width, half),
        )
    } else {
        let half = bounds.size.width / 2.0;
        Bounds::new(
            point(
                bounds.origin.x
                    + if placement == Placement::After {
                        half
                    } else {
                        px(0.0)
                    },
                bounds.origin.y,
            ),
            size(half, bounds.size.height),
        )
    }
}

fn register_target(
    context: &PrimitiveContext,
    config: &SortableConfig,
    bounds: Bounds<Pixels>,
    placement: Placement,
    window: &mut Window,
) {
    let half = target_half(bounds, config.direction, placement);
    let Ok(bounds) = GeometryBounds::new(
        f64::from(half.origin.x),
        f64::from(half.origin.y),
        f64::from(half.size.width),
        f64::from(half.size.height),
    ) else {
        return;
    };
    let target_context = context.clone();
    let anchor = config.item_key.clone();
    let previous = config.previous_key.clone();
    let next = config.next_key.clone();
    context.register_drop_target(crate::interaction::DropTargetRegistration::new(
        target_owner(context, config, placement),
        target_id(&config.list_id, &config.item_key, placement),
        bounds,
        BTreeSet::from([payload_type(config)]),
        BTreeSet::from([crate::interaction::DragOperation::Move]),
        100,
        context.ancestor_scroll_handles(),
        window.current_view(),
        move |drag, position, window, cx| {
            let source = drag.source_id();
            let no_op = source == anchor
                || match placement {
                    Placement::Before => previous.as_deref() == Some(source),
                    Placement::After => next.as_deref() == Some(source),
                };
            if no_op {
                return;
            }
            target_context.propose(
                "reorder",
                reorder_value(source, &anchor, placement, position),
                window,
                cx,
            );
        },
    ));
}

fn register_pointer_source(
    prepaint: &SortablePrepaint,
    config: &SortableConfig,
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
            || !hitbox.is_hovered(window)
        {
            return;
        }
        if let Some(focus) = config.focus.as_ref() {
            focus.focus(window, cx);
        }
        let spec = drag_spec(&context, &config, view);
        context.begin_application_drag(
            spec,
            event.position,
            config.threshold,
            |_, _, _, _| {},
            window,
            cx,
        );
        cx.stop_propagation();
    });
}

fn reorder_value(
    source: &str,
    anchor: &str,
    placement: Placement,
    position: Point<Pixels>,
) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("source_key".to_owned(), UiValue::String(source.to_owned())),
        ("anchor_key".to_owned(), UiValue::String(anchor.to_owned())),
        (
            "placement".to_owned(),
            UiValue::String(placement.as_str().to_owned()),
        ),
        ("x".to_owned(), UiValue::Float(f64::from(position.x))),
        ("y".to_owned(), UiValue::Float(f64::from(position.y))),
    ]))
}

fn paint_sortable_feedback(
    context: &PrimitiveContext,
    config: &SortableConfig,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    if context.app_drag_source_active(&source_owner(context, config)) {
        window.paint_quad(PaintQuad {
            bounds,
            corner_radii: px(0.0).into(),
            background: Background::from(rgba((config.accent.as_rgba_hex() & 0xffff_ff00) | 0x12)),
            border_widths: Edges::all(px(1.0)),
            border_color: rgba(config.accent.as_rgba_hex()).into(),
            border_style: gpui::BorderStyle::default(),
        });
    }
    for placement in [Placement::Before, Placement::After] {
        if context.drop_target_state(&target_owner(context, config, placement))
            != crate::interaction::DropTargetState::Active
        {
            continue;
        }
        let indicator = if config.direction.uses_vertical_targets() {
            let y = if placement == Placement::Before {
                bounds.top()
            } else {
                bounds.bottom() - px(2.0)
            };
            Bounds::new(point(bounds.left(), y), size(bounds.size.width, px(2.0)))
        } else {
            let x = if placement == Placement::Before {
                bounds.left()
            } else {
                bounds.right() - px(2.0)
            };
            Bounds::new(point(x, bounds.top()), size(px(2.0), bounds.size.height))
        };
        window.paint_quad(PaintQuad {
            bounds: indicator,
            corner_radii: px(0.0).into(),
            background: Background::from(rgba(config.accent.as_rgba_hex())),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x0000_0000).into(),
            border_style: gpui::BorderStyle::default(),
        });
    }
}

struct SortableEntity {
    focus: FocusHandle,
    config: SortableConfig,
    context: PrimitiveContext,
}

impl SortableEntity {
    fn new(mut config: SortableConfig, context: PrimitiveContext, cx: &mut Context<Self>) -> Self {
        let focus = config.focus.clone().unwrap_or_else(|| cx.focus_handle());
        config.focus = Some(focus.clone());
        Self {
            focus,
            config,
            context,
        }
    }

    fn update(&mut self, mut config: SortableConfig, context: PrimitiveContext) {
        config.focus = Some(self.focus.clone());
        self.config = config;
        self.context = context;
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.disabled || !event.keystroke.modifiers.alt {
            return;
        }
        let target = match event.keystroke.key.as_str() {
            "up" | "left" => self
                .config
                .previous_key
                .as_ref()
                .map(|anchor| target_id(&self.config.list_id, anchor, Placement::Before)),
            "down" | "right" => self
                .config
                .next_key
                .as_ref()
                .map(|anchor| target_id(&self.config.list_id, anchor, Placement::After)),
            "home" => Some(target_id(
                &self.config.list_id,
                &self.config.first_key,
                Placement::Before,
            )),
            "end" => Some(target_id(
                &self.config.list_id,
                &self.config.last_key,
                Placement::After,
            )),
            _ => return,
        };
        let Some(target) = target else {
            cx.stop_propagation();
            return;
        };
        let spec = drag_spec(&self.context, &self.config, cx.entity_id());
        self.context
            .perform_keyboard_drop(&spec, &target, window, cx);
        cx.stop_propagation();
    }
}

impl Render for SortableEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus.clone().tab_stop(!self.config.disabled))
            .on_key_down(cx.listener(Self::key_down))
            .child(SortableElement {
                config: self.config.clone(),
                context: self.context.clone(),
            })
    }
}

#[derive(Default)]
pub struct SortablePrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<SortableEntity>>,
}

impl PrimitiveHandler for SortablePrimitiveHandler {
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
            .ok_or_else(|| "SortableItemPrimitive requires a stable key".to_owned())?;
        let config = parse_config(
            &instance.node.props,
            instance.focus_handle().cloned(),
            theme,
        )?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| SortableEntity::new(config.clone(), context.clone(), cx));
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |sortable, _| {
            sortable.update(config, context.clone());
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_config(
    props: &PrimitiveProps,
    focus: Option<FocusHandle>,
    theme: &PrimitiveTheme,
) -> Result<SortableConfig, String> {
    let list_id = required_safe_string(props, "list_id")?;
    let item_key = required_safe_string(props, "item_key")?;
    let first_key = required_safe_string(props, "first_key")?;
    let last_key = required_safe_string(props, "last_key")?;
    let direction = props
        .string("direction")
        .and_then(SortDirection::parse)
        .ok_or_else(|| "sortable direction must be vertical, horizontal, or grid".to_owned())?;
    let threshold = props.number("threshold").unwrap_or(4.0);
    if !threshold.is_finite() || !(0.0..=64.0).contains(&threshold) {
        return Err("sortable threshold must be finite and in [0,64]".to_owned());
    }
    Ok(SortableConfig {
        id: format!("gpui-rhai-sortable:{list_id}:{item_key}"),
        list_id,
        item_key,
        previous_key: optional_string(props, "previous_key")?,
        next_key: optional_string(props, "next_key")?,
        first_key,
        last_key,
        direction,
        threshold,
        disabled: props.boolean("disabled").unwrap_or(false),
        item_ref: props
            .element_ref("item_ref")
            .cloned()
            .ok_or_else(|| "sortable item_ref is required".to_owned())?,
        focus,
        accent: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
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

fn optional_string(props: &PrimitiveProps, name: &str) -> Result<Option<String>, String> {
    match props.data(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::String(value))
            if !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control) =>
        {
            Ok(Some(value.clone()))
        }
        _ => Err(format!("{name} must be null or a safe string")),
    }
}

fn reorder_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        (
            "source_key".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "anchor_key".to_owned(),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            "placement".to_owned(),
            ObjectField::required(ValueSchema::String {
                allowed: vec!["before".to_owned(), "after".to_owned()],
            }),
        ),
        ("x".to_owned(), ObjectField::required(ValueSchema::number())),
        ("y".to_owned(), ObjectField::required(ValueSchema::number())),
    ]))
}

/// Build the native keyed sortable-item interaction schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn sortable_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_string = || ValueSchema::optional(ValueSchema::string());
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.sortable_item").expect("static primitive ID"),
        export: "SortableItemPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "list_id".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "item_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "previous_key".to_owned(),
                ObjectField::required(optional_string()),
            ),
            (
                "next_key".to_owned(),
                ObjectField::required(optional_string()),
            ),
            (
                "first_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "last_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "direction".to_owned(),
                ObjectField::required(ValueSchema::String {
                    allowed: vec![
                        "vertical".to_owned(),
                        "horizontal".to_owned(),
                        "grid".to_owned(),
                    ],
                }),
            ),
            (
                "threshold".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            (
                "item_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref),
            ),
            (
                "on_reorder".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
        ]),
        events: BTreeMap::from([(
            "reorder".to_owned(),
            EventSchema {
                payload: reorder_schema(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

fn f64_to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or_else(|_| {
        if value.is_sign_negative() {
            f32::MIN
        } else {
            f32::MAX
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn adjacent_and_self_moves_are_no_ops() {
        let before = |source: &str, anchor: &str, previous: Option<&str>| {
            source == anchor || previous == Some(source)
        };
        let after = |source: &str, anchor: &str, next: Option<&str>| {
            source == anchor || next == Some(source)
        };
        assert!(before("b", "b", Some("a")));
        assert!(before("a", "b", Some("a")));
        assert!(after("b", "b", Some("c")));
        assert!(after("c", "b", Some("c")));
        assert!(!before("c", "b", Some("a")));
        assert!(!after("a", "b", Some("c")));
    }
}
