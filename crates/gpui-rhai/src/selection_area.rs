//! Controlled Canvas object selection with native click/range/marquee preview.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Element, ElementId, Entity, FocusHandle,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    Modifiers, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, Style, Styled,
    Window, div, point, px, rgba, size,
};

use crate::{
    ComponentStateSchema, EventSchema, GeometryBounds, ObjectField, PrimitiveContext,
    PrimitiveDescriptor, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitiveProps, PrimitiveTheme, Rgba8, UiValue, ValueSchema,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MarqueePolicy {
    Intersect,
    Enclose,
}

#[derive(Clone, Debug)]
struct SelectionTarget {
    key: String,
    bounds: GeometryBounds,
    disabled: bool,
}

#[derive(Clone)]
struct SelectionConfig {
    id: String,
    targets: Vec<SelectionTarget>,
    selected: BTreeSet<String>,
    active: Option<String>,
    anchor: Option<String>,
    multiple: bool,
    marquee: MarqueePolicy,
    threshold: f64,
    disabled: bool,
    canvas_ref: crate::ElementRef,
    focus: Option<FocusHandle>,
    accent: Rgba8,
}

struct SelectionAreaEntity {
    focus: FocusHandle,
    config: SelectionConfig,
    context: PrimitiveContext,
    root_bounds: Option<Bounds<Pixels>>,
    marquee_window: Option<Bounds<Pixels>>,
}

impl SelectionAreaEntity {
    fn new(mut config: SelectionConfig, context: PrimitiveContext, cx: &mut Context<Self>) -> Self {
        let focus = config.focus.clone().unwrap_or_else(|| cx.focus_handle());
        config.focus = Some(focus.clone());
        Self {
            focus,
            config,
            context,
            root_bounds: None,
            marquee_window: None,
        }
    }

    fn update(&mut self, mut config: SelectionConfig, context: PrimitiveContext) {
        config.focus = Some(self.focus.clone());
        self.config = config;
        self.context = context;
    }

    fn local_point(&self, point: Point<Pixels>, cx: &App) -> Option<(f64, f64)> {
        self.context
            .canvas_local_point(&self.config.canvas_ref, point, cx)
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.disabled || event.button != MouseButton::Left {
            return;
        }
        let Some(local_start) = self.local_point(event.position, cx) else {
            return;
        };
        self.focus.focus(window, cx);
        let start_window = event.position;
        let modifiers = event.modifiers;
        let entity = cx.entity();
        let update_entity = entity.clone();
        let update = move |gesture: crate::interaction::GestureUpdate,
                           _: &mut Window,
                           cx: &mut App| {
            update_entity.update(cx, |selection, cx| {
                if gesture.moved() {
                    selection.marquee_window = Some(pixel_bounds(start_window, gesture.current()));
                    cx.notify();
                }
            });
            crate::interaction::InteractionFlow::Continue
        };
        let finish_entity = entity.clone();
        let finish =
            move |gesture: crate::interaction::GestureUpdate, window: &mut Window, cx: &mut App| {
                finish_entity.update(cx, |selection, cx| {
                    let proposal = if gesture.moved() {
                        let marquee = pixel_bounds(start_window, gesture.current());
                        marquee_proposal(
                            &selection.config,
                            &window_quad(start_window, gesture.current())
                                .into_iter()
                                .filter_map(|point| selection.local_point(point, cx))
                                .collect::<Vec<_>>(),
                            marquee.size.width > px(0.0) && marquee.size.height > px(0.0),
                            modifiers,
                        )
                    } else {
                        click_proposal(&selection.config, local_start, modifiers)
                    };
                    selection.marquee_window = None;
                    if let Some(proposal) = proposal {
                        selection.context.propose(
                            "selection_change",
                            proposal_value(proposal),
                            window,
                            cx,
                        );
                    }
                    cx.notify();
                });
            };
        let cancel = move |_: &mut Window, cx: &mut App| {
            entity.update(cx, |selection, cx| {
                selection.marquee_window = None;
                cx.notify();
            });
        };
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.begin_interaction(
            crate::interaction::NativeGesture::new(
                owner,
                event.position,
                cx.entity_id(),
                update,
                finish,
                cancel,
            )
            .with_threshold(self.config.threshold),
            window,
            cx,
        );
        cx.stop_propagation();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.disabled {
            return;
        }
        let eligible = self
            .config
            .targets
            .iter()
            .filter(|target| !target.disabled)
            .map(|target| target.key.as_str())
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            return;
        }
        let current = self
            .config
            .active
            .as_deref()
            .and_then(|active| eligible.iter().position(|key| *key == active))
            .unwrap_or(0);
        let next = match event.keystroke.key.as_str() {
            "left" | "up" => current.saturating_sub(1),
            "right" | "down" => (current + 1).min(eligible.len() - 1),
            "home" => 0,
            "end" => eligible.len() - 1,
            "space" => {
                let key = eligible[current];
                let mut selected = if self.config.multiple {
                    self.config.selected.clone()
                } else {
                    BTreeSet::new()
                };
                if self.config.multiple && !selected.insert(key.to_owned()) {
                    selected.remove(key);
                } else {
                    selected.insert(key.to_owned());
                }
                self.emit(
                    selection_proposal(selected, Some(key.to_owned()), self.config.anchor.clone()),
                    window,
                    cx,
                );
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        let key = eligible[next].to_owned();
        let proposal = if event.keystroke.modifiers.shift && self.config.multiple {
            range_proposal(&self.config, &key)
        } else {
            selection_proposal(
                BTreeSet::from([key.clone()]),
                Some(key.clone()),
                Some(key.clone()),
            )
        };
        self.emit(proposal, window, cx);
        cx.stop_propagation();
    }

    fn emit(&self, proposal: SelectionProposal, window: &mut Window, cx: &mut App) {
        if proposal.selected != self.config.selected
            || proposal.active != self.config.active
            || proposal.anchor != self.config.anchor
        {
            self.context
                .propose("selection_change", proposal_value(proposal), window, cx);
        }
    }
}

impl Render for SelectionAreaEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = self.context.interaction_owner(&self.config.id);
        self.context.present_interaction(owner.clone());
        if self.config.disabled {
            self.context.cancel_interaction(&owner, window, cx);
        }
        let mut root = div()
            .size_full()
            .relative()
            .track_focus(&self.focus.clone().tab_stop(!self.config.disabled))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_key_down(cx.listener(Self::key_down));
        if let (Some(marquee), Some(root_bounds)) = (self.marquee_window, self.root_bounds) {
            let color = self.config.accent.as_rgba_hex();
            root = root.child(
                div()
                    .absolute()
                    .left(marquee.left() - root_bounds.left())
                    .top(marquee.top() - root_bounds.top())
                    .w(marquee.size.width)
                    .h(marquee.size.height)
                    .bg(rgba((color & 0xffff_ff00) | 0x20))
                    .border_1()
                    .border_color(rgba(color)),
            );
        }
        root.child(SelectionBoundsRecorder {
            selection: cx.entity(),
        })
    }
}

struct SelectionBoundsRecorder {
    selection: Entity<SelectionAreaEntity>,
}

impl Element for SelectionBoundsRecorder {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
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
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        _: &mut Window,
        cx: &mut App,
    ) {
        self.selection
            .update(cx, |selection, _| selection.root_bounds = Some(bounds));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        (): &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
}

impl IntoElement for SelectionBoundsRecorder {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct SelectionAreaPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<SelectionAreaEntity>>,
}

impl PrimitiveHandler for SelectionAreaPrimitiveHandler {
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
            .ok_or_else(|| "SelectionAreaPrimitive requires a stable key".to_owned())?;
        let config = parse_config(
            &instance.node.props,
            instance.focus_handle().cloned(),
            theme,
        )?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| SelectionAreaEntity::new(config.clone(), context.clone(), cx));
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |selection, _| selection.update(config, context.clone()));
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectionProposal {
    selected: BTreeSet<String>,
    active: Option<String>,
    anchor: Option<String>,
}

fn selection_proposal(
    selected: BTreeSet<String>,
    active: Option<String>,
    anchor: Option<String>,
) -> SelectionProposal {
    SelectionProposal {
        selected,
        active,
        anchor,
    }
}

fn click_proposal(
    config: &SelectionConfig,
    point: (f64, f64),
    modifiers: Modifiers,
) -> Option<SelectionProposal> {
    let hit = config
        .targets
        .iter()
        .rev()
        .find(|target| !target.disabled && contains(target.bounds, point));
    let Some(hit) = hit else {
        return (!config.selected.is_empty() || config.active.is_some())
            .then(|| selection_proposal(BTreeSet::new(), None, None));
    };
    if modifiers.shift && config.multiple {
        return Some(range_proposal(config, &hit.key));
    }
    let toggle = (modifiers.platform || modifiers.control) && config.multiple;
    let mut selected = if toggle {
        config.selected.clone()
    } else {
        BTreeSet::new()
    };
    if toggle && selected.contains(&hit.key) {
        selected.remove(&hit.key);
    } else {
        selected.insert(hit.key.clone());
    }
    Some(selection_proposal(
        selected,
        Some(hit.key.clone()),
        Some(hit.key.clone()),
    ))
}

fn range_proposal(config: &SelectionConfig, key: &str) -> SelectionProposal {
    let anchor = config.anchor.as_deref().unwrap_or(key);
    let positions = config
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| (target.key.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let Some(&start) = positions.get(anchor) else {
        return selection_proposal(
            BTreeSet::from([key.to_owned()]),
            Some(key.to_owned()),
            Some(key.to_owned()),
        );
    };
    let Some(&end) = positions.get(key) else {
        return selection_proposal(
            config.selected.clone(),
            config.active.clone(),
            config.anchor.clone(),
        );
    };
    let (start, end) = (start.min(end), start.max(end));
    let selected = config.targets[start..=end]
        .iter()
        .filter(|target| !target.disabled)
        .map(|target| target.key.clone())
        .collect();
    selection_proposal(selected, Some(key.to_owned()), Some(anchor.to_owned()))
}

fn marquee_proposal(
    config: &SelectionConfig,
    polygon: &[(f64, f64)],
    has_window_area: bool,
    modifiers: Modifiers,
) -> Option<SelectionProposal> {
    if polygon.len() != 4 || !has_window_area {
        return None;
    }
    let mut hits = config
        .targets
        .iter()
        .filter(|target| !target.disabled)
        .filter(|target| match config.marquee {
            MarqueePolicy::Intersect => polygon_intersects_rect(polygon, target.bounds),
            MarqueePolicy::Enclose => rect_inside_polygon(target.bounds, polygon),
        })
        .map(|target| target.key.clone())
        .collect::<BTreeSet<_>>();
    if !config.multiple
        && hits.len() > 1
        && let Some(key) = config
            .targets
            .iter()
            .rev()
            .find(|target| hits.contains(&target.key))
            .map(|target| target.key.clone())
    {
        hits = BTreeSet::from([key]);
    }
    let additive = config.multiple && (modifiers.shift || modifiers.platform || modifiers.control);
    let selected = if additive {
        config.selected.union(&hits).cloned().collect()
    } else {
        hits
    };
    let active = config
        .targets
        .iter()
        .rev()
        .find(|target| selected.contains(&target.key))
        .map(|target| target.key.clone());
    let proposal = selection_proposal(selected, active, config.anchor.clone());
    (proposal.selected != config.selected || proposal.active != config.active).then_some(proposal)
}

fn window_quad(start: Point<Pixels>, end: Point<Pixels>) -> [Point<Pixels>; 4] {
    let left = start.x.min(end.x);
    let right = start.x.max(end.x);
    let top = start.y.min(end.y);
    let bottom = start.y.max(end.y);
    [
        point(left, top),
        point(right, top),
        point(right, bottom),
        point(left, bottom),
    ]
}

fn rect_corners(bounds: GeometryBounds) -> [(f64, f64); 4] {
    [
        (bounds.x, bounds.y),
        (bounds.x + bounds.width, bounds.y),
        (bounds.x + bounds.width, bounds.y + bounds.height),
        (bounds.x, bounds.y + bounds.height),
    ]
}

fn point_in_polygon(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    let Some(anchor) = polygon.first() else {
        return false;
    };
    let twice_area = (0..polygon.len())
        .map(|index| {
            let next = (index + 1) % polygon.len();
            let a = (polygon[index].0 - anchor.0, polygon[index].1 - anchor.1);
            let b = (polygon[next].0 - anchor.0, polygon[next].1 - anchor.1);
            a.0.mul_add(b.1, -b.0 * a.1)
        })
        .sum::<f64>();
    let coordinate_scale = polygon
        .iter()
        .flat_map(|(x, y)| [(x - anchor.0).abs(), (y - anchor.1).abs()])
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    let point_count = f64::from(u32::try_from(polygon.len()).unwrap_or(u32::MAX));
    let area_epsilon = f64::EPSILON * coordinate_scale * coordinate_scale * point_count * 16.0;
    if twice_area.abs() <= area_epsilon {
        return false;
    }
    let mut sign = 0.0_f64;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let cross = (b.0 - a.0).mul_add(point.1 - a.1, -(b.1 - a.1) * (point.0 - a.0));
        let cross_epsilon = f64::EPSILON
            * ((b.0 - a.0).abs() * (point.1 - a.1).abs()
                + (b.1 - a.1).abs() * (point.0 - a.0).abs())
            * 16.0;
        if cross.abs() <= cross_epsilon {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if sign * cross < 0.0 {
            return false;
        }
    }
    true
}

fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
        let lhs = (b.0 - a.0) * (c.1 - a.1);
        let rhs = (b.1 - a.1) * (c.0 - a.0);
        let cross = (b.0 - a.0).mul_add(c.1 - a.1, -rhs);
        if cross.abs() <= f64::EPSILON * (lhs.abs() + rhs.abs()) * 16.0 {
            0.0
        } else {
            cross
        }
    }
    let (o1, o2, o3, o4) = (
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    );
    let on_segment = |a: (f64, f64), b: (f64, f64), point: (f64, f64)| {
        let epsilon = f64::EPSILON * (a.0 - b.0).abs().max((a.1 - b.1).abs()) * 16.0;
        point.0 >= a.0.min(b.0) - epsilon
            && point.0 <= a.0.max(b.0) + epsilon
            && point.1 >= a.1.min(b.1) - epsilon
            && point.1 <= a.1.max(b.1) + epsilon
    };
    (o1 == 0.0 && on_segment(a, b, c))
        || (o2 == 0.0 && on_segment(a, b, d))
        || (o3 == 0.0 && on_segment(c, d, a))
        || (o4 == 0.0 && on_segment(c, d, b))
        || ((o1 > 0.0 && o2 < 0.0 || o1 < 0.0 && o2 > 0.0)
            && (o3 > 0.0 && o4 < 0.0 || o3 < 0.0 && o4 > 0.0))
}

fn rect_inside_polygon(bounds: GeometryBounds, polygon: &[(f64, f64)]) -> bool {
    rect_corners(bounds)
        .into_iter()
        .all(|corner| point_in_polygon(corner, polygon))
}

fn polygon_intersects_rect(polygon: &[(f64, f64)], bounds: GeometryBounds) -> bool {
    let corners = rect_corners(bounds);
    corners
        .iter()
        .copied()
        .any(|corner| point_in_polygon(corner, polygon))
        || polygon.iter().copied().any(|point| contains(bounds, point))
        || (0..polygon.len()).any(|polygon_edge| {
            (0..corners.len()).any(|rect_edge| {
                segments_intersect(
                    polygon[polygon_edge],
                    polygon[(polygon_edge + 1) % polygon.len()],
                    corners[rect_edge],
                    corners[(rect_edge + 1) % corners.len()],
                )
            })
        })
}

fn pixel_bounds(start: Point<Pixels>, end: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(start.x.min(end.x), start.y.min(end.y)),
        size((start.x - end.x).abs(), (start.y - end.y).abs()),
    )
}

fn contains(bounds: GeometryBounds, point: (f64, f64)) -> bool {
    point.0 >= bounds.x
        && point.0 <= bounds.x + bounds.width
        && point.1 >= bounds.y
        && point.1 <= bounds.y + bounds.height
}

fn parse_config(
    props: &PrimitiveProps,
    focus: Option<FocusHandle>,
    theme: &PrimitiveTheme,
) -> Result<SelectionConfig, String> {
    let targets = parse_targets(props)?;
    let selected = string_set(props, "selected_keys")?;
    let active = optional_string(props, "active_key")?;
    let anchor = optional_string(props, "anchor_key")?;
    if selected
        .iter()
        .any(|key| !targets.iter().any(|target| &target.key == key))
        || active
            .as_ref()
            .is_some_and(|key| !targets.iter().any(|target| &target.key == key))
        || anchor
            .as_ref()
            .is_some_and(|key| !targets.iter().any(|target| &target.key == key))
    {
        return Err("selection keys must reference declared targets".to_owned());
    }
    let marquee = match props.string("marquee") {
        None | Some("intersect") => MarqueePolicy::Intersect,
        Some("enclose") => MarqueePolicy::Enclose,
        Some(_) => return Err("selection marquee must be intersect or enclose".to_owned()),
    };
    let threshold = props.number("threshold").unwrap_or(4.0);
    if !threshold.is_finite() || !(0.0..=64.0).contains(&threshold) {
        return Err("selection threshold must be finite and in [0,64]".to_owned());
    }
    let canvas_ref = props
        .element_ref("canvas_ref")
        .cloned()
        .ok_or_else(|| "selection canvas_ref is required".to_owned())?;
    Ok(SelectionConfig {
        id: format!(
            "gpui-rhai-selection:{}:{}",
            canvas_ref.id().component(),
            canvas_ref.id().key()
        ),
        targets,
        selected,
        active,
        anchor,
        multiple: props.boolean("multiple").unwrap_or(true),
        marquee,
        threshold,
        disabled: props.boolean("disabled").unwrap_or(false),
        canvas_ref,
        focus,
        accent: theme
            .color("accent")
            .unwrap_or(Rgba8::from_rgba_hex(0x3b82_f6ff)),
    })
}

fn parse_targets(props: &PrimitiveProps) -> Result<Vec<SelectionTarget>, String> {
    let Some(UiValue::Array(values)) = props.data("targets") else {
        return Err("selection targets must be an array".to_owned());
    };
    if values.len() > 10_000 {
        return Err("selection targets exceed 10000".to_owned());
    }
    let mut keys = BTreeSet::new();
    values
        .iter()
        .map(|value| {
            let UiValue::Map(value) = value else {
                return Err("selection target must be an object".to_owned());
            };
            let key = value
                .get("key")
                .and_then(ui_string)
                .filter(|key| !key.is_empty() && key.len() <= 128)
                .ok_or_else(|| "selection target key is invalid".to_owned())?
                .to_owned();
            if !keys.insert(key.clone()) {
                return Err(format!("duplicate selection target `{key}`"));
            }
            let number = |name: &str| {
                value
                    .get(name)
                    .and_then(ui_number)
                    .ok_or_else(|| format!("selection target {name} is required"))
            };
            Ok(SelectionTarget {
                key,
                bounds: GeometryBounds::new(
                    number("x")?,
                    number("y")?,
                    number("width")?,
                    number("height")?,
                )
                .map_err(|error| error.to_string())?,
                disabled: value.get("disabled").and_then(ui_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn string_set(props: &PrimitiveProps, name: &str) -> Result<BTreeSet<String>, String> {
    let Some(UiValue::Array(values)) = props.data(name) else {
        return Err(format!("{name} must be an array"));
    };
    values
        .iter()
        .map(|value| {
            ui_string(value)
                .map(str::to_owned)
                .ok_or_else(|| format!("{name} entries must be strings"))
        })
        .collect()
}

fn ui_string(value: &UiValue) -> Option<&str> {
    match value {
        UiValue::String(value) => Some(value),
        _ => None,
    }
}

fn ui_number(value: &UiValue) -> Option<f64> {
    match value {
        UiValue::Float(value) => Some(*value),
        UiValue::Integer(value) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn ui_bool(value: &UiValue) -> Option<bool> {
    match value {
        UiValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn optional_string(props: &PrimitiveProps, name: &str) -> Result<Option<String>, String> {
    match props.data(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::String(value)) => Ok(Some(value.clone())),
        _ => Err(format!("{name} must be null or a string")),
    }
}

fn proposal_value(proposal: SelectionProposal) -> UiValue {
    UiValue::Map(BTreeMap::from([
        (
            "selected_keys".to_owned(),
            UiValue::Array(proposal.selected.into_iter().map(UiValue::String).collect()),
        ),
        (
            "active_key".to_owned(),
            proposal.active.map_or(UiValue::Null, UiValue::String),
        ),
        (
            "anchor_key".to_owned(),
            proposal.anchor.map_or(UiValue::Null, UiValue::String),
        ),
    ]))
}

fn proposal_schema() -> ValueSchema {
    ValueSchema::object(BTreeMap::from([
        (
            "selected_keys".to_owned(),
            ObjectField::required(ValueSchema::Array {
                items: Box::new(ValueSchema::string()),
                max_items: Some(10_000),
            }),
        ),
        (
            "active_key".to_owned(),
            ObjectField::required(ValueSchema::optional(ValueSchema::string())),
        ),
        (
            "anchor_key".to_owned(),
            ObjectField::required(ValueSchema::optional(ValueSchema::string())),
        ),
    ]))
}

/// Build the native controlled Canvas selection schema.
///
/// # Panics
///
/// Panics only if the static primitive ID becomes invalid.
#[must_use]
pub fn selection_area_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.selection_area").expect("static primitive ID"),
        export: "SelectionAreaPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "targets".to_owned(),
                ObjectField::required(ValueSchema::Array {
                    items: Box::new(ValueSchema::Map {
                        values: Box::new(ValueSchema::UiValue),
                    }),
                    max_items: Some(10_000),
                })
                .with_doc(
                    "Selectable `{key, x, y, width, height, disabled}` rectangles in Canvas-local logical pixels; list order sets range and stacking.",
                ),
            ),
            (
                "selected_keys".to_owned(),
                ObjectField::required(ValueSchema::Array {
                    items: Box::new(ValueSchema::string()),
                    max_items: Some(10_000),
                })
                .with_doc(
                    "Controlled keys of the selected targets; the caller stores the `selection_change` payload and passes it back.",
                ),
            ),
            (
                "active_key".to_owned(),
                ObjectField::required(ValueSchema::optional(ValueSchema::string())).with_doc(
                    "Controlled key of the active target that arrow keys move from and Space toggles, or `()`.",
                ),
            ),
            (
                "anchor_key".to_owned(),
                ObjectField::required(ValueSchema::optional(ValueSchema::string()))
                    .with_doc("Controlled key a Shift range extends from, or `()`."),
            ),
            (
                "multiple".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(true))
                    .with_doc(
                        "Allows more than one selected key; `false` turns off toggle, range and additive marquee selection.",
                    ),
            ),
            (
                "marquee".to_owned(),
                ObjectField::optional(ValueSchema::String {
                    allowed: vec!["intersect".to_owned(), "enclose".to_owned()],
                })
                .with_default(UiValue::String("intersect".to_owned()))
                .with_doc(
                    "`intersect` selects targets the marquee touches; `enclose` only targets entirely inside it.",
                ),
            ),
            (
                "threshold".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.0), Some(64.0))).with_doc(
                    "Pointer movement in logical pixels before a press becomes a marquee; defaults to 4.",
                ),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc(
                        "Ignores presses and keys, cancels a running marquee and removes the area from the tab order.",
                    ),
            ),
            (
                "canvas_ref".to_owned(),
                ObjectField::required(ValueSchema::Ref).with_doc(
                    "Ref to the Canvas the targets live on; pointer positions go through its inverse transform, pan, zoom and rotation included.",
                ),
            ),
            (
                "on_selection_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc(
                    "Called with the proposed selection on a click, a marquee release, or an arrow, Home, End or Space key.",
                ),
            ),
        ]),
        events: BTreeMap::from([(
            "selection_change".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted on a click, a marquee release or a selection key; the payload is the next `selected_keys`, `active_key` and `anchor_key`."
                        .to_owned(),
                ),
                payload: proposal_schema(),
            },
        )]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marquee_policies_distinguish_intersection_and_enclosure() {
        let marquee = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
        let crossing = GeometryBounds::new(15.0, 15.0, 20.0, 20.0).unwrap();
        assert!(polygon_intersects_rect(&marquee, crossing));
        assert!(!rect_inside_polygon(crossing, &marquee));
    }

    #[test]
    fn degenerate_marquee_does_not_contain_far_collinear_points() {
        let line = vec![(5.0, 10.0), (50.0, 10.0), (50.0, 10.0), (5.0, 10.0)];
        assert!(!point_in_polygon((100.0, 10.0), &line));
        assert!(!polygon_intersects_rect(
            &line,
            GeometryBounds::new(95.0, 5.0, 10.0, 10.0).unwrap()
        ));
    }

    #[test]
    fn polygon_predicates_preserve_translation_and_scale() {
        for origin in [0.0, 499.0, 1000.0] {
            for scale in [1.0, 1e-4, 1e-6] {
                let polygon = vec![
                    (origin, origin),
                    (origin + 20.0 * scale, origin),
                    (origin + 20.0 * scale, origin + 20.0 * scale),
                    (origin, origin + 20.0 * scale),
                ];
                let inside = GeometryBounds::new(
                    origin + 5.0 * scale,
                    origin + 5.0 * scale,
                    5.0 * scale,
                    5.0 * scale,
                )
                .unwrap();
                let outside = GeometryBounds::new(
                    origin + 25.0 * scale,
                    origin + 5.0 * scale,
                    5.0 * scale,
                    5.0 * scale,
                )
                .unwrap();
                assert!(rect_inside_polygon(inside, &polygon));
                assert!(polygon_intersects_rect(&polygon, inside));
                assert!(!polygon_intersects_rect(&polygon, outside));
            }
        }
    }
}
