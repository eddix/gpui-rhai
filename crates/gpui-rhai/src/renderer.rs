use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, BoxShadow, ClickEvent, Context, Div, Element, ElementId,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, LayoutId, ParentElement,
    Pixels, Point, Render, SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div,
    img, point, px, relative, rems, rgba,
};

use crate::date_picker_element::{
    DateChangeHandler, DatePickerCallbacks, DatePickerEntityElement, DatePickerPalette,
};
use crate::dropdown_element::{
    DropdownCallbacks, DropdownEntityElement, DropdownPalette, DropdownSlotRuntime, QueryHandler,
    SelectionHandler,
};
use crate::overlay_element::{ScriptOverlayElement, WindowOverlayCoordinator};
use crate::table_element::{
    TableCallbacks, TableEntityElement, TablePalette, TableRowHandler, TableSelectionHandler,
    TableSortHandler,
};
use crate::toast_element::{ToastDismissHandler, ToastHostElement, ToastPalette, ToastPartStyles};
use crate::virtual_list_element::{VirtualFocusHandler, VirtualListEntityElement};
use crate::{
    Align, AnimationKey, AnimationProperty, AssetRegistry, ColorValue, DatePickerNodeSpec,
    DropdownNodeSpec, EventPropagation, FlexDirection, ImageSourceSpec, InteractionState, Justify,
    Length, NodeId, OverlayNodeSpec, PrimitiveRegistry, PseudoState, RadiusToken, RetainedUiTree,
    Rgba8, ScriptCallback, SpacingToken, Style, StyleProperties, TableNodeSpec, TableSort,
    TableSortDirection, TextDirection, ToastHostSpec, UiEventHandler, UiNode, UiNodeKind, UiValue,
};

type DispatchFn = dyn Fn(ScriptCallback, UiValue, &mut Window, &mut App) -> EventPropagation;

#[derive(Clone)]
pub struct NodeEventDispatcher(Rc<DispatchFn>);

impl NodeEventDispatcher {
    #[must_use]
    pub fn new(
        dispatch: impl Fn(ScriptCallback, UiValue, &mut Window, &mut App) -> EventPropagation + 'static,
    ) -> Self {
        Self(Rc::new(dispatch))
    }

    pub(crate) fn dispatch(
        &self,
        callback: ScriptCallback,
        payload: UiValue,
        window: &mut Window,
        cx: &mut App,
    ) -> EventPropagation {
        (self.0)(callback, payload, window, cx)
    }
}

fn dispatch_ui_event(
    handler: &UiEventHandler,
    payload: UiValue,
    window: &mut Window,
    app: &mut App,
    script_dispatcher: Option<&NodeEventDispatcher>,
) -> EventPropagation {
    match handler {
        UiEventHandler::Script(callback) => script_dispatcher
            .map_or(EventPropagation::Handled, |dispatcher| {
                dispatcher.dispatch(callback.clone(), payload, window, app)
            }),
        UiEventHandler::Host(callback) => callback.invoke(payload, window, app),
    }
}

pub trait ColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8>;

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
            Length::ThemeSpacing(_) | Length::ThemeRadius(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiteralColorResolver;

impl ColorResolver for LiteralColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(_) => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct OwnedColorResolver {
    tokens: BTreeMap<String, Rgba8>,
    spacing: BTreeMap<SpacingToken, Length>,
    radii: BTreeMap<RadiusToken, Length>,
}

impl OwnedColorResolver {
    fn capture(colors: &impl ColorResolver) -> Self {
        const TOKENS: &[&str] = &[
            "surface",
            "surface_raised",
            "surface_hover",
            "text_primary",
            "text_muted",
            "accent",
            "accent_hover",
            "on_accent",
            "danger",
            "on_danger",
            "warning",
            "on_warning",
            "success",
            "on_success",
            "border",
            "focus_ring",
            "disabled",
        ];
        let spacing = [
            SpacingToken::Xs,
            SpacingToken::Sm,
            SpacingToken::Md,
            SpacingToken::Lg,
        ]
        .into_iter()
        .filter_map(|token| {
            colors
                .resolve_length(Length::ThemeSpacing(token))
                .map(|value| (token, value))
        })
        .collect();
        let radii = [RadiusToken::Sm, RadiusToken::Md, RadiusToken::Lg]
            .into_iter()
            .filter_map(|token| {
                colors
                    .resolve_length(Length::ThemeRadius(token))
                    .map(|value| (token, value))
            })
            .collect();
        Self {
            tokens: TOKENS
                .iter()
                .filter_map(|token| {
                    colors
                        .resolve(&ColorValue::Token((*token).to_owned()))
                        .map(|value| ((*token).to_owned(), value))
                })
                .collect(),
            spacing,
            radii,
        }
    }
}

impl ColorResolver for OwnedColorResolver {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(token) => self.tokens.get(token).copied(),
        }
    }

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::ThemeSpacing(token) => self.spacing.get(&token).copied(),
            Length::ThemeRadius(token) => self.radii.get(&token).copied(),
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
        }
    }
}

/// Converts stable runtime nodes into short-lived GPUI elements.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuiNodeRenderer;

struct RenderEnvironment<'a, C> {
    colors: &'a C,
    interaction: &'a InteractionState,
    primitives: &'a PrimitiveRegistry,
    dispatcher: Option<&'a NodeEventDispatcher>,
    assets: Option<&'a AssetRegistry>,
    overlays: &'a WindowOverlayCoordinator,
    animations: &'a BTreeMap<AnimationKey, f64>,
    signals: &'a crate::SignalRegistry,
    direction: TextDirection,
    view_id: &'a str,
    retained: Option<&'a RetainedUiTree>,
}

pub(crate) struct WindowRenderResources<'a> {
    pub assets: &'a AssetRegistry,
    pub dispatcher: &'a NodeEventDispatcher,
    pub overlays: &'a WindowOverlayCoordinator,
    pub animations: &'a BTreeMap<AnimationKey, f64>,
    pub signals: &'a crate::SignalRegistry,
    pub direction: TextDirection,
    pub root_path: &'a str,
    pub view_id: &'a str,
}

impl GpuiNodeRenderer {
    #[must_use]
    pub fn render(node: &UiNode) -> AnyElement {
        Self::render_with_primitives(
            node,
            &LiteralColorResolver,
            &InteractionState::default(),
            &PrimitiveRegistry::new(),
        )
    }

    #[must_use]
    pub fn render_with(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
    ) -> AnyElement {
        Self::render_with_primitives(node, colors, interaction, &PrimitiveRegistry::new())
    }

    #[must_use]
    pub fn render_with_primitives(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
        };
        Self::render_internal(node, &environment, None, "root", None)
    }

    #[must_use]
    pub fn render_retained_with_primitives(
        tree: &RetainedUiTree,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: None,
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: Some(tree),
        };
        tree.root().map_or_else(
            || {
                div()
                    .child("Retained UI tree has no root")
                    .into_any_element()
            },
            |root| Self::render_internal(root, &environment, None, "root", tree.root_id()),
        )
    }

    #[must_use]
    pub fn render_with_dispatcher(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        dispatcher: &NodeEventDispatcher,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(dispatcher),
            assets: None,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            direction: TextDirection::LeftToRight,
            view_id: "standalone",
            retained: None,
        };
        Self::render_internal(node, &environment, None, "root", None)
    }

    #[must_use]
    pub fn render_with_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        assets: &AssetRegistry,
        dispatcher: &NodeEventDispatcher,
    ) -> AnyElement {
        let overlays = WindowOverlayCoordinator::default();
        let animations = BTreeMap::new();
        let signals = crate::SignalRegistry::new();
        let resources = WindowRenderResources {
            assets,
            dispatcher,
            overlays: &overlays,
            animations: &animations,
            signals: &signals,
            direction: TextDirection::LeftToRight,
            root_path: "root",
            view_id: "standalone",
        };
        Self::render_with_window_runtime(node, colors, interaction, primitives, &resources)
    }

    pub(crate) fn render_with_window_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
    ) -> AnyElement {
        Self::render_subtree_with_window_runtime(
            node,
            colors,
            interaction,
            primitives,
            resources,
            resources.root_path,
        )
    }

    pub(crate) fn render_retained_with_window_runtime(
        tree: &RetainedUiTree,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
    ) -> AnyElement {
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            animations: resources.animations,
            signals: resources.signals,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: Some(tree),
        };
        tree.root().map_or_else(
            || {
                div()
                    .child("Retained UI tree has no root")
                    .into_any_element()
            },
            |root| {
                Self::render_internal(
                    root,
                    &environment,
                    None,
                    resources.root_path,
                    tree.root_id(),
                )
            },
        )
    }

    pub(crate) fn render_subtree_with_window_runtime(
        node: &UiNode,
        colors: &impl ColorResolver,
        interaction: &InteractionState,
        primitives: &PrimitiveRegistry,
        resources: &WindowRenderResources<'_>,
        path: &str,
    ) -> AnyElement {
        let environment = RenderEnvironment {
            colors,
            interaction,
            primitives,
            dispatcher: Some(resources.dispatcher),
            assets: Some(resources.assets),
            overlays: resources.overlays,
            animations: resources.animations,
            signals: resources.signals,
            direction: resources.direction,
            view_id: resources.view_id,
            retained: None,
        };
        Self::render_internal(node, &environment, None, path, None)
    }

    fn render_internal<C: ColorResolver>(
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        let local_interaction = if is_disabled(node) {
            environment.interaction.clone().with(PseudoState::Disabled)
        } else {
            environment.interaction.clone()
        };
        let animation = node_animation(environment.animations, path);
        let signals = node_signals(environment.signals, node);
        let mut resolved_style = node.style().resolve(&local_interaction);
        apply_animated_dimensions(&mut resolved_style, animation);
        apply_signal_style(&mut resolved_style, &signals);
        let mut element = apply_style(
            div(),
            &resolved_style,
            environment.colors,
            environment.direction,
        );
        if matches!(node.kind(), UiNodeKind::Custom { .. }) {
            let focus_ring = semantic_color(environment.colors, "focus_ring", 0x003b_82f6);
            let focus_surface = semantic_color(environment.colors, "surface", 0x0018_181b);
            element =
                element.in_focus(move |style| focus_ring_shadow(style, focus_ring, focus_surface));
        }
        if let Some(opacity) = signals.opacity.or(animation.opacity) {
            element = element.opacity(f64_to_f32(opacity.clamp(0.0, 1.0)));
        }
        if animation.clip_height.is_some() || resolved_style.clip == Some(true) {
            element = element.overflow_hidden();
        }
        let populated = Self::populate_with_interactions(
            element,
            node,
            environment,
            boundary_fallback,
            path,
            retained_id,
        );
        translated(
            populated,
            signals.translate_x.or(animation.translate_x),
            signals.translate_y.or(animation.translate_y),
        )
    }

    fn populate_with_interactions<C: ColorResolver>(
        element: Div,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        let click = node.handlers().get("click").map(|callback| {
            (
                callback.clone(),
                node.handler_payload("click")
                    .cloned()
                    .unwrap_or(UiValue::Null),
            )
        });
        let key_handlers = node
            .handlers()
            .iter()
            .filter_map(|(event, callback)| {
                event.strip_prefix("key:").map(|key| {
                    (
                        key.to_owned(),
                        (
                            callback.clone(),
                            node.handler_payload(event)
                                .cloned()
                                .unwrap_or(UiValue::Null),
                        ),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        if is_disabled(node) || (click.is_none() && key_handlers.is_empty()) {
            return Self::populate(
                element,
                node,
                environment,
                boundary_fallback,
                path,
                retained_id,
            );
        }

        let click_dispatcher = environment.dispatcher.cloned();
        let keyboard_dispatcher = environment.dispatcher.cloned();
        let keyboard_click = click.clone();
        let tab_stop = match node.attributes().get("tab_stop") {
            Some(UiValue::Bool(tab_stop)) => *tab_stop,
            _ => true,
        };
        let text_direction = environment.direction;
        let stable_id = retained_id.map_or_else(
            || path.to_owned(),
            |node_id| format!("gpui-rhai-node-{node_id}"),
        );
        let debug_path = path.to_owned();
        let element = apply_pseudo_backgrounds(
            element
                .id(SharedString::from(stable_id))
                .debug_selector(move || debug_path.clone()),
            node.style(),
            environment.colors,
        )
        .tab_index(0)
        .tab_stop(tab_stop)
        .on_click(move |event, window, cx| {
            if matches!(event, ClickEvent::Mouse(_))
                && let Some((callback, payload)) = &click
                && dispatch_ui_event(
                    callback,
                    payload.clone(),
                    window,
                    cx,
                    click_dispatcher.as_ref(),
                ) == EventPropagation::Handled
            {
                cx.stop_propagation();
            }
        })
        .on_key_down(move |event, window, cx| {
            let semantic_key = logical_keyboard_key(event.keystroke.key.as_str(), text_direction);
            let semantic = key_handlers.get(semantic_key).or_else(|| {
                matches!(event.keystroke.key.as_str(), "enter" | "space")
                    .then_some(())
                    .and(keyboard_click.as_ref())
            });
            if let Some((callback, payload)) = semantic
                && dispatch_ui_event(
                    callback,
                    payload.clone(),
                    window,
                    cx,
                    keyboard_dispatcher.as_ref(),
                ) == EventPropagation::Handled
            {
                cx.stop_propagation();
            }
        });
        Self::populate(
            element,
            node,
            environment,
            boundary_fallback,
            path,
            retained_id,
        )
    }

    fn populate<C: ColorResolver>(
        element: impl ParentElement + IntoElement,
        node: &UiNode,
        environment: &RenderEnvironment<'_, C>,
        boundary_fallback: Option<&UiNode>,
        path: &str,
        retained_id: Option<NodeId>,
    ) -> AnyElement {
        match node.kind() {
            UiNodeKind::Text { text } => element.child(text.as_str().to_owned()).into_any_element(),
            UiNodeKind::Container { children } => element
                .children(children.iter().enumerate().map(|(index, child)| {
                    let child_path = child.key().map_or_else(
                        || format!("{path}/{index}"),
                        |key| format!("{path}/{}", key.as_str()),
                    );
                    let child_id =
                        retained_child_id(environment.retained, retained_id, "children", index);
                    Self::render_internal(
                        child,
                        environment,
                        boundary_fallback,
                        &child_path,
                        child_id,
                    )
                }))
                .into_any_element(),
            UiNodeKind::Custom { primitive } => element
                .child(environment.primitives.element(
                    primitive.clone(),
                    boundary_fallback.cloned(),
                    environment.dispatcher.cloned(),
                    crate::PrimitiveTheme::capture(environment.colors),
                ))
                .into_any_element(),
            UiNodeKind::Image { source } => render_image(element, node, source, environment),
            UiNodeKind::DirectionalImage {
                left_to_right,
                right_to_left,
            } => {
                let source = match environment.direction {
                    TextDirection::LeftToRight => left_to_right,
                    TextDirection::RightToLeft => right_to_left,
                };
                render_image(element, node, source, environment)
            }
            UiNodeKind::Overlay {
                trigger,
                content,
                spec,
            } => element
                .child(native_overlay_element(
                    node,
                    trigger,
                    content,
                    spec,
                    environment,
                    boundary_fallback,
                    (path, retained_id),
                ))
                .into_any_element(),
            UiNodeKind::Dropdown { spec } => element
                .child(native_dropdown_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::Select { spec } => element
                .child(native_select_element(node, &spec.choice, environment, path))
                .into_any_element(),
            UiNodeKind::DatePicker { spec } => element
                .child(native_date_picker_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::Table { spec } => element
                .child(native_table_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::ToastHost { spec } => element
                .child(native_toast_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::VirtualList { spec } => element
                .child(native_virtual_list_element(node, spec, environment, path))
                .into_any_element(),
            UiNodeKind::ErrorBoundary { child, fallback } => element
                .child(Self::render_internal(
                    child,
                    environment,
                    Some(fallback),
                    &format!("{path}/boundary"),
                    retained_child_id(environment.retained, retained_id, "child", 0),
                ))
                .into_any_element(),
        }
    }
}

fn retained_child_id(
    tree: Option<&RetainedUiTree>,
    parent: Option<NodeId>,
    group: &str,
    index: usize,
) -> Option<NodeId> {
    tree.and_then(|tree| tree.node(parent?))
        .and_then(|node| {
            node.children()
                .filter(|child| child.group() == group)
                .nth(index)
        })
        .map(crate::RetainedChildLink::node)
}

fn render_image<C: ColorResolver>(
    element: impl ParentElement + IntoElement,
    node: &UiNode,
    source: &ImageSourceSpec,
    environment: &RenderEnvironment<'_, C>,
) -> AnyElement {
    let interaction = if is_disabled(node) {
        environment.interaction.clone().with(PseudoState::Disabled)
    } else {
        environment.interaction.clone()
    };
    let tint = node
        .style()
        .resolve(&interaction)
        .text_color
        .as_ref()
        .and_then(|color| environment.colors.resolve(color));
    environment.assets.map_or_else(
        || div().child("Image registry unavailable").into_any_element(),
        |assets| {
            let handle = match source {
                ImageSourceSpec::Handle(handle) => Ok(handle.clone()),
                ImageSourceSpec::Asset(asset) => assets
                    .cached_image(asset)
                    .map(|image| image.opaque().clone()),
            };
            match handle.and_then(|handle| assets.image_source_tinted(&handle, tint)) {
                Ok(source) => element.child(img(source)).into_any_element(),
                Err(error) => div()
                    .child(format!("Image error: {error}"))
                    .into_any_element(),
            }
        },
    )
}

fn scoped_overlay_spec(spec: &OverlayNodeSpec, view_id: &str) -> OverlayNodeSpec {
    let mut rendered = spec.clone();
    rendered.id = WindowOverlayCoordinator::scoped_id(view_id, &rendered.id);
    rendered.parent = rendered
        .parent
        .as_ref()
        .map(|parent| WindowOverlayCoordinator::scoped_id(view_id, parent));
    rendered
}

fn native_overlay_element<C: ColorResolver>(
    node: &UiNode,
    trigger: &UiNode,
    content: &UiNode,
    spec: &OverlayNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    boundary_fallback: Option<&UiNode>,
    (path, retained_id): (&str, Option<NodeId>),
) -> ScriptOverlayElement {
    let mut rendered_spec = scoped_overlay_spec(spec, environment.view_id);
    if rendered_spec.kind == crate::OverlayKind::Tooltip {
        rendered_spec.open = environment
            .overlays
            .tooltip_visible(&rendered_spec.id, std::time::Instant::now());
    }
    let trigger = GpuiNodeRenderer::render_internal(
        trigger,
        environment,
        boundary_fallback,
        &format!("{path}/trigger"),
        retained_child_id(environment.retained, retained_id, "trigger", 0),
    );
    let content = GpuiNodeRenderer::render_internal(
        content,
        environment,
        boundary_fallback,
        &format!("{path}/content"),
        retained_child_id(environment.retained, retained_id, "content", 0),
    );
    let open_change = node.handlers().get("open_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = environment.dispatcher.cloned();
        Rc::new(move |open, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                UiValue::Bool(open),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as crate::overlay_element::OpenChangeHandler
    });
    let handlers = node
        .handlers()
        .iter()
        .filter_map(|(event, callback)| {
            event
                .strip_prefix("key:")
                .map(|key| (key.to_owned(), callback.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let panel_key = (!handlers.is_empty()).then(|| {
        let dispatcher = environment.dispatcher.cloned();
        let payloads = node
            .handlers()
            .keys()
            .filter_map(|event| {
                event
                    .strip_prefix("key:")
                    .map(|key| (key.to_owned(), node.handler_payload(event).cloned()))
            })
            .collect::<BTreeMap<_, _>>();
        let direction = environment.direction;
        Rc::new(
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                let key = logical_keyboard_key(event.keystroke.key.as_str(), direction);
                let Some(callback) = handlers.get(key) else {
                    return false;
                };
                dispatch_ui_event(
                    callback,
                    payloads
                        .get(key)
                        .and_then(Clone::clone)
                        .unwrap_or(UiValue::Null),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
                true
            },
        ) as crate::overlay_element::PanelKeyHandler
    });
    let restore_focus_on_close = rendered_spec.kind == crate::OverlayKind::Menu;
    let overlay = ScriptOverlayElement::new(
        path,
        trigger,
        content,
        rendered_spec,
        open_change,
        panel_key,
        environment.overlays.clone(),
    );
    let backdrop_style = node.part_style("backdrop").map(|style| {
        let style = style.clone();
        let colors = OwnedColorResolver::capture(environment.colors);
        let direction = environment.direction;
        Rc::new(move |backdrop: Div| apply_style_override(backdrop, &style, &colors, direction))
            as crate::overlay_element::BackdropStyleHandler
    });
    overlay
        .with_backdrop_style(backdrop_style)
        .with_focus_ring(semantic_color(
            environment.colors,
            "focus_ring",
            0x003b_82f6,
        ))
        .with_focus_surface(semantic_color(environment.colors, "surface", 0x0018_181b))
        .restore_focus_on_close(restore_focus_on_close)
}

fn native_toast_element<C: ColorResolver>(
    node: &UiNode,
    spec: &ToastHostSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> ToastHostElement {
    let dismiss = node.handlers().get("dismiss").map_or_else(
        || Rc::new(|_: String, _: &mut Window, _: &mut App| {}) as ToastDismissHandler,
        |handler| {
            let handler = handler.clone();
            let dispatcher = environment.dispatcher.cloned();
            Rc::new(move |id: String, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    UiValue::String(id),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            }) as ToastDismissHandler
        },
    );
    ToastHostElement::new(
        path,
        spec.clone(),
        ToastPalette {
            surface: semantic_color(environment.colors, "surface_raised", 0x0027_272a),
            text: semantic_color(environment.colors, "text_primary", 0x00f4_f4f5),
            muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
            border: semantic_color(environment.colors, "border", 0x003f_3f46),
            success: semantic_color(environment.colors, "success", 0x0022_c55e),
            warning: semantic_color(environment.colors, "warning", 0x00f5_9e0b),
            danger: semantic_color(environment.colors, "danger", 0x00ef_4444),
        },
        environment.overlays.clone(),
        dismiss,
        ToastPartStyles {
            styles: node
                .part_styles()
                .map(|(name, style)| (name.to_owned(), style.clone()))
                .collect(),
            colors: OwnedColorResolver::capture(environment.colors),
            direction: environment.direction,
        },
        environment.view_id,
    )
}

fn native_dropdown_element<C: ColorResolver>(
    node: &UiNode,
    spec: &DropdownNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> DropdownEntityElement {
    let callbacks = dropdown_callbacks(node, environment.dispatcher);
    native_choice_element(node, spec, callbacks, environment, path)
}

fn native_select_element<C: ColorResolver>(
    node: &UiNode,
    spec: &DropdownNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> DropdownEntityElement {
    let callbacks = select_callbacks(node, environment.dispatcher);
    native_choice_element(node, spec, callbacks, environment, path)
}

fn native_date_picker_element<C: ColorResolver>(
    node: &UiNode,
    spec: &DatePickerNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> DatePickerEntityElement {
    let callbacks = date_picker_callbacks(node, environment.dispatcher);
    let palette = DatePickerPalette {
        surface: semantic_color(environment.colors, "surface", 0x0018_181b),
        hover: semantic_color(environment.colors, "surface_hover", 0x003f_3f46),
        text: semantic_color(environment.colors, "text_primary", 0x00f4_f4f5),
        muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
        accent: semantic_color(environment.colors, "accent", 0x003b_82f6),
        on_accent: semantic_color(environment.colors, "on_accent", 0x00ff_ffff),
        focus_ring: semantic_color(environment.colors, "focus_ring", 0x003b_82f6),
    };
    let runtime = DropdownSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: node
            .part_styles()
            .map(|(name, style)| (name.to_owned(), style.clone()))
            .collect(),
    };
    DatePickerEntityElement::new(
        path,
        spec.clone(),
        callbacks,
        palette,
        environment.overlays.clone(),
        runtime,
    )
}

fn native_table_element<C: ColorResolver>(
    node: &UiNode,
    spec: &TableNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> TableEntityElement {
    let callbacks = table_callbacks(node, environment.dispatcher);
    let palette = TablePalette {
        surface: semantic_color(environment.colors, "surface", 0x0018_181b),
        raised: semantic_color(environment.colors, "surface_raised", 0x0027_272a),
        hover: semantic_color(environment.colors, "surface_hover", 0x003f_3f46),
        text: semantic_color(environment.colors, "text_primary", 0x00f4_f4f5),
        muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
        accent: semantic_color(environment.colors, "accent", 0x003b_82f6),
        on_accent: semantic_color(environment.colors, "on_accent", 0x00ff_ffff),
        border: semantic_color(environment.colors, "border", 0x003f_3f46),
    };
    let runtime = DropdownSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: node
            .part_styles()
            .map(|(name, style)| (name.to_owned(), style.clone()))
            .collect(),
    };
    TableEntityElement::new(path, spec.clone(), callbacks, palette, runtime)
}

fn native_choice_element<C: ColorResolver>(
    node: &UiNode,
    spec: &DropdownNodeSpec,
    callbacks: DropdownCallbacks,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> DropdownEntityElement {
    let palette = DropdownPalette {
        surface: semantic_color(environment.colors, "surface", 0x0018_181b),
        hover: semantic_color(environment.colors, "surface_hover", 0x003f_3f46),
        muted: semantic_color(environment.colors, "text_muted", 0x00a1_a1aa),
        accent: semantic_color(environment.colors, "accent", 0x003b_82f6),
        disabled: semantic_color(environment.colors, "disabled", 0x0052_525b),
        focus_ring: semantic_color(environment.colors, "focus_ring", 0x003b_82f6),
    };
    let slot_runtime = DropdownSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: node
            .part_styles()
            .map(|(name, style)| (name.to_owned(), style.clone()))
            .collect(),
    };
    DropdownEntityElement::new(
        path,
        spec.clone(),
        callbacks,
        palette,
        environment.overlays.clone(),
        slot_runtime,
    )
}

fn native_virtual_list_element<C: ColorResolver>(
    node: &UiNode,
    spec: &crate::VirtualListNodeSpec,
    environment: &RenderEnvironment<'_, C>,
    path: &str,
) -> VirtualListEntityElement {
    let focus_change = node.handlers().get("change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = environment.dispatcher.cloned();
        Rc::new(move |key: String, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                UiValue::String(key),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as VirtualFocusHandler
    });
    let runtime = DropdownSlotRuntime {
        colors: OwnedColorResolver::capture(environment.colors),
        primitives: environment.primitives.clone(),
        assets: environment.assets.cloned().unwrap_or_default(),
        dispatcher: environment
            .dispatcher
            .cloned()
            .unwrap_or_else(|| NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled)),
        overlays: environment.overlays.clone(),
        animations: environment.animations.clone(),
        signals: environment.signals.clone(),
        direction: environment.direction,
        base_path: path.to_owned(),
        view_id: environment.view_id.to_owned(),
        part_styles: BTreeMap::new(),
    };
    VirtualListEntityElement::new(path, spec.clone(), runtime, focus_change)
}

#[derive(Clone, Copy, Default)]
struct NodeAnimationValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    clip_height: Option<f64>,
}

#[derive(Clone, Debug, Default)]
struct NodeSignalValues {
    opacity: Option<f64>,
    translate_x: Option<f64>,
    translate_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    background: Option<ColorValue>,
    text_color: Option<ColorValue>,
    border_color: Option<ColorValue>,
}

fn node_signals(registry: &crate::SignalRegistry, node: &UiNode) -> NodeSignalValues {
    let mut values = NodeSignalValues::default();
    for (property, signal) in node.signal_bindings() {
        let Ok(value) = registry.read(signal) else {
            continue;
        };
        match (property, value) {
            (crate::SignalProperty::Opacity, crate::SignalValue::Float(value)) => {
                values.opacity = Some(value);
            }
            (crate::SignalProperty::TranslateX, crate::SignalValue::Float(value)) => {
                values.translate_x = Some(value);
            }
            (crate::SignalProperty::TranslateY, crate::SignalValue::Float(value)) => {
                values.translate_y = Some(value);
            }
            (crate::SignalProperty::Width, crate::SignalValue::Float(value)) => {
                values.width = Some(value);
            }
            (crate::SignalProperty::Height, crate::SignalValue::Float(value)) => {
                values.height = Some(value);
            }
            (crate::SignalProperty::Background, crate::SignalValue::Color(value)) => {
                values.background = Some(value);
            }
            (crate::SignalProperty::TextColor, crate::SignalValue::Color(value)) => {
                values.text_color = Some(value);
            }
            (crate::SignalProperty::BorderColor, crate::SignalValue::Color(value)) => {
                values.border_color = Some(value);
            }
            _ => debug_assert!(false, "validated signal binding changed type"),
        }
    }
    values
}

fn apply_signal_style(style: &mut StyleProperties, values: &NodeSignalValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)));
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
    if let Some(background) = &values.background {
        style.background = Some(background.clone());
    }
    if let Some(text_color) = &values.text_color {
        style.text_color = Some(text_color.clone());
    }
    if let Some(border_color) = &values.border_color {
        style.border_color = Some(border_color.clone());
    }
}

fn node_animation(values: &BTreeMap<AnimationKey, f64>, path: &str) -> NodeAnimationValues {
    let value = |property| values.get(&AnimationKey::for_node(path, property)).copied();
    NodeAnimationValues {
        opacity: value(AnimationProperty::Opacity),
        translate_x: value(AnimationProperty::TranslateX),
        translate_y: value(AnimationProperty::TranslateY),
        width: value(AnimationProperty::Width),
        height: value(AnimationProperty::Height),
        clip_height: value(AnimationProperty::ClipHeight),
    }
}

fn apply_animated_dimensions(style: &mut StyleProperties, values: NodeAnimationValues) {
    if let Some(width) = values.width {
        style.width = Some(Length::Pixels(width.max(0.0)));
    }
    if let Some(height) = values.height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
    if let Some(height) = values.clip_height {
        style.height = Some(Length::Pixels(height.max(0.0)));
    }
}

fn translated(element: AnyElement, x: Option<f64>, y: Option<f64>) -> AnyElement {
    let offset = point(
        px(f64_to_f32(x.unwrap_or(0.0))),
        px(f64_to_f32(y.unwrap_or(0.0))),
    );
    if offset == Point::default() {
        element
    } else {
        TranslatedElement {
            child: Some(element),
            offset,
        }
        .into_any_element()
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

struct TranslatedElement {
    child: Option<AnyElement>,
    offset: Point<Pixels>,
}

impl Element for TranslatedElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("translated element renders once");
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_element_offset(self.offset, |window| child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_element_offset(self.offset, |window| child.paint(window, cx));
    }
}

impl IntoElement for TranslatedElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn dropdown_callbacks(
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
) -> DropdownCallbacks {
    let dispatcher = dispatcher.cloned();
    let selection = node.handlers().get("change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = dispatcher.clone();
        Rc::new(
            move |values: Vec<String>, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    UiValue::Array(values.into_iter().map(UiValue::String).collect()),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            },
        ) as SelectionHandler
    });
    let open = node.handlers().get("open_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = dispatcher.clone();
        Rc::new(move |open, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                UiValue::Bool(open),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as crate::overlay_element::OpenChangeHandler
    });
    let query = node.handlers().get("query_change").map(|handler| {
        let handler = handler.clone();
        Rc::new(move |query: String, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                UiValue::String(query),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as QueryHandler
    });
    DropdownCallbacks {
        selection,
        open,
        query,
    }
}

fn select_callbacks(node: &UiNode, dispatcher: Option<&NodeEventDispatcher>) -> DropdownCallbacks {
    let dispatcher = dispatcher.cloned();
    let selection = node.handlers().get("change").map(|handler| {
        let handler = handler.clone();
        Rc::new(
            move |values: Vec<String>, window: &mut Window, cx: &mut App| {
                let payload = values
                    .into_iter()
                    .next()
                    .map_or(UiValue::Null, UiValue::String);
                dispatch_ui_event(&handler, payload, window, cx, dispatcher.as_ref());
            },
        ) as SelectionHandler
    });
    DropdownCallbacks {
        selection,
        open: None,
        query: None,
    }
}

fn date_picker_callbacks(
    node: &UiNode,
    dispatcher: Option<&NodeEventDispatcher>,
) -> DatePickerCallbacks {
    let dispatcher = dispatcher.cloned();
    let change = node.handlers().get("change").map(|handler| {
        let handler = handler.clone();
        Rc::new(
            move |value: Option<String>, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    value.map_or(UiValue::Null, UiValue::String),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            },
        ) as DateChangeHandler
    });
    DatePickerCallbacks { change }
}

fn table_callbacks(node: &UiNode, dispatcher: Option<&NodeEventDispatcher>) -> TableCallbacks {
    let dispatcher = dispatcher.cloned();
    let sort = node.handlers().get("sort_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = dispatcher.clone();
        Rc::new(
            move |sort: Option<TableSort>, window: &mut Window, cx: &mut App| {
                let payload = sort.map_or(UiValue::Null, |sort| {
                    UiValue::Map(BTreeMap::from([
                        ("key".to_owned(), UiValue::String(sort.key)),
                        (
                            "direction".to_owned(),
                            UiValue::String(
                                match sort.direction {
                                    TableSortDirection::Ascending => "ascending",
                                    TableSortDirection::Descending => "descending",
                                }
                                .to_owned(),
                            ),
                        ),
                    ]))
                });
                dispatch_ui_event(&handler, payload, window, cx, dispatcher.as_ref());
            },
        ) as TableSortHandler
    });
    let selection = node.handlers().get("selection_change").map(|handler| {
        let handler = handler.clone();
        let dispatcher = dispatcher.clone();
        Rc::new(
            move |values: Vec<String>, window: &mut Window, cx: &mut App| {
                dispatch_ui_event(
                    &handler,
                    UiValue::Array(values.into_iter().map(UiValue::String).collect()),
                    window,
                    cx,
                    dispatcher.as_ref(),
                );
            },
        ) as TableSelectionHandler
    });
    let row_click = node.handlers().get("row_click").map(|handler| {
        let handler = handler.clone();
        Rc::new(move |key: String, window: &mut Window, cx: &mut App| {
            dispatch_ui_event(
                &handler,
                UiValue::String(key),
                window,
                cx,
                dispatcher.as_ref(),
            );
        }) as TableRowHandler
    });
    TableCallbacks {
        sort,
        selection,
        row_click,
    }
}

fn semantic_color(colors: &impl ColorResolver, token: &str, fallback: u32) -> Rgba8 {
    colors
        .resolve(&ColorValue::Token(token.to_owned()))
        .unwrap_or_else(|| Rgba8::from_rgb_hex(fallback))
}

fn apply_pseudo_backgrounds(
    mut element: Stateful<Div>,
    style: &Style,
    colors: &impl ColorResolver,
) -> Stateful<Div> {
    if let Some(color) = style
        .hover
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color))
    {
        element = element.hover(move |style| style.bg(rgba(color.as_rgba_hex())));
    }
    if let Some(color) = style
        .active
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color))
    {
        element = element.active(move |style| style.bg(rgba(color.as_rgba_hex())));
    }
    let focus_background = style
        .focus
        .as_ref()
        .and_then(|properties| properties.background.as_ref())
        .and_then(|color| colors.resolve(color));
    let focus_ring = semantic_color(colors, "focus_ring", 0x003b_82f6);
    let focus_surface = semantic_color(colors, "surface", 0x0018_181b);
    element = element.focus(move |style| {
        let style = match focus_background {
            Some(color) => style.bg(rgba(color.as_rgba_hex())),
            None => style,
        };
        focus_ring_shadow(style, focus_ring, focus_surface)
    });
    element
}

fn focus_ring_shadow(
    style: gpui::StyleRefinement,
    color: Rgba8,
    surface: Rgba8,
) -> gpui::StyleRefinement {
    style.shadow(vec![
        BoxShadow {
            color: rgba(color.as_rgba_hex()).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(4.0),
        },
        BoxShadow {
            color: rgba(surface.as_rgba_hex()).into(),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(2.0),
        },
    ])
}

fn is_disabled(node: &UiNode) -> bool {
    node.attributes().get("disabled") == Some(&UiValue::Bool(true))
}

fn apply_style(
    element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    let mut style = style.clone();
    resolve_style_lengths(&mut style, colors);
    let element = apply_layout(element, &style, direction);
    let element = apply_spacing(element, &style, direction);
    apply_paint_and_text(element, &style, colors)
}

fn resolve_style_lengths(style: &mut StyleProperties, resolver: &impl ColorResolver) {
    let resolve = |value: &mut Option<Length>| {
        *value = (*value).and_then(|value| resolver.resolve_length(value));
    };
    for value in [
        &mut style.width,
        &mut style.height,
        &mut style.min_width,
        &mut style.max_width,
        &mut style.min_height,
        &mut style.max_height,
        &mut style.gap,
    ] {
        resolve(value);
    }
    for value in [
        &mut style.padding.top,
        &mut style.padding.right,
        &mut style.padding.bottom,
        &mut style.padding.left,
        &mut style.padding.start,
        &mut style.padding.end,
        &mut style.margin.top,
        &mut style.margin.right,
        &mut style.margin.bottom,
        &mut style.margin.left,
        &mut style.margin.start,
        &mut style.margin.end,
        &mut style.border_width,
        &mut style.radius,
        &mut style.font_size,
    ] {
        resolve(value);
    }
}

pub(crate) fn apply_style_override(
    element: Div,
    style: &Style,
    colors: &impl ColorResolver,
    direction: TextDirection,
) -> Div {
    apply_style(
        element,
        &style.resolve(&InteractionState::default()),
        colors,
        direction,
    )
}

fn apply_layout(mut element: Div, style: &StyleProperties, text_direction: TextDirection) -> Div {
    if let Some(direction) = style.direction {
        element = element.flex();
        element = match (direction, text_direction) {
            (FlexDirection::Row, TextDirection::LeftToRight) => element.flex_row(),
            (FlexDirection::Row, TextDirection::RightToLeft) => element.flex_row_reverse(),
            (FlexDirection::Column, _) => element.flex_col(),
        };
    }
    if let Some(align) = style.align {
        let align = if style.direction == Some(FlexDirection::Column)
            && text_direction == TextDirection::RightToLeft
        {
            match align {
                Align::Start => Align::End,
                Align::End => Align::Start,
                other => other,
            }
        } else {
            align
        };
        element = match align {
            Align::Start => element.items_start(),
            Align::Center => element.items_center(),
            Align::End => element.items_end(),
            Align::Stretch => element,
        };
    }
    if let Some(justify) = style.justify {
        element = match justify {
            Justify::Start => element.justify_start(),
            Justify::Center => element.justify_center(),
            Justify::End => element.justify_end(),
            Justify::Between => element.justify_between(),
            Justify::Around => element.justify_around(),
        };
    }
    if let Some(value) = style.width {
        element = width(element, value);
    }
    if let Some(value) = style.height {
        element = height(element, value);
    }
    if let Some(value) = style.min_width {
        element = min_width(element, value);
    }
    if let Some(value) = style.max_width {
        element = max_width(element, value);
    }
    if let Some(value) = style.min_height {
        element = min_height(element, value);
    }
    if let Some(value) = style.max_height {
        element = max_height(element, value);
    }
    if let Some(value) = style.gap {
        element = gap(element, value);
    }
    if style.flex_grow == Some(true) {
        element = element.flex_grow();
    }
    element
}

fn apply_spacing(mut element: Div, style: &StyleProperties, direction: TextDirection) -> Div {
    if let Some(value) = style.padding.top {
        element = padding_top(element, value);
    }
    let (padding_left_value, padding_right_value) = logical_horizontal_edges(
        style.padding.left,
        style.padding.right,
        style.padding.start,
        style.padding.end,
        direction,
    );
    if let Some(value) = padding_right_value {
        element = padding_right(element, value);
    }
    if let Some(value) = style.padding.bottom {
        element = padding_bottom(element, value);
    }
    if let Some(value) = padding_left_value {
        element = padding_left(element, value);
    }
    if let Some(value) = style.margin.top {
        element = margin_top(element, value);
    }
    let (margin_left_value, margin_right_value) = logical_horizontal_edges(
        style.margin.left,
        style.margin.right,
        style.margin.start,
        style.margin.end,
        direction,
    );
    if let Some(value) = margin_right_value {
        element = margin_right(element, value);
    }
    if let Some(value) = style.margin.bottom {
        element = margin_bottom(element, value);
    }
    if let Some(value) = margin_left_value {
        element = margin_left(element, value);
    }
    element
}

fn logical_horizontal_edges(
    mut left: Option<Length>,
    mut right: Option<Length>,
    start: Option<Length>,
    end: Option<Length>,
    direction: TextDirection,
) -> (Option<Length>, Option<Length>) {
    match direction {
        TextDirection::LeftToRight => {
            if start.is_some() {
                left = start;
            }
            if end.is_some() {
                right = end;
            }
        }
        TextDirection::RightToLeft => {
            if start.is_some() {
                right = start;
            }
            if end.is_some() {
                left = end;
            }
        }
    }
    (left, right)
}

fn logical_keyboard_key(key: &str, direction: TextDirection) -> &str {
    match (key, direction) {
        ("left", TextDirection::RightToLeft) => "right",
        ("right", TextDirection::RightToLeft) => "left",
        _ => key,
    }
}

fn apply_paint_and_text(
    mut element: Div,
    style: &StyleProperties,
    colors: &impl ColorResolver,
) -> Div {
    if let Some(color) = style
        .background
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.bg(rgba(color.as_rgba_hex()));
    }
    if let Some(color) = style
        .text_color
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.text_color(rgba(color.as_rgba_hex()));
    }
    if let Some(color) = style
        .border_color
        .as_ref()
        .and_then(|color| colors.resolve(color))
    {
        element = element.border_color(rgba(color.as_rgba_hex()));
    }
    if let Some(value) = style.border_width {
        element = border(element, value);
    }
    if let Some(value) = style.radius {
        element = radius(element, value);
    }
    if let Some(value) = style.font_size {
        element = font_size(element, value);
    }
    element
}

macro_rules! definite_length_fn {
    ($name:ident, $method:ident) => {
        fn $name(element: Div, value: Length) -> Div {
            match value {
                Length::Pixels(value) => element.$method(px(to_f32(value))),
                Length::Rems(value) => element.$method(rems(to_f32(value))),
                Length::Relative(value) => element.$method(relative(to_f32(value))),
                Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
            }
        }
    };
}

definite_length_fn!(width, w);
definite_length_fn!(height, h);
definite_length_fn!(min_width, min_w);
definite_length_fn!(max_width, max_w);
definite_length_fn!(min_height, min_h);
definite_length_fn!(max_height, max_h);
definite_length_fn!(gap, gap);
definite_length_fn!(padding_top, pt);
definite_length_fn!(padding_right, pr);
definite_length_fn!(padding_bottom, pb);
definite_length_fn!(padding_left, pl);
definite_length_fn!(margin_top, mt);
definite_length_fn!(margin_right, mr);
definite_length_fn!(margin_bottom, mb);
definite_length_fn!(margin_left, ml);

fn border(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.border(px(to_f32(value))),
        Length::Rems(value) => element.border(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

fn radius(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.rounded(px(to_f32(value))),
        Length::Rems(value) => element.rounded(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

fn font_size(element: Div, value: Length) -> Div {
    match value {
        Length::Pixels(value) => element.text_size(px(to_f32(value))),
        Length::Rems(value) => element.text_size(rems(to_f32(value))),
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => element,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX));
    value as f32
}

/// Minimal GPUI view for a previously evaluated Rhai tree or a Host-built tree.
pub struct StaticUiView {
    tree: crate::RetainedUiTree,
    primitives: PrimitiveRegistry,
}

impl StaticUiView {
    /// Create a retained view from one accepted root snapshot.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors such as duplicate sibling keys.
    pub fn new(root: UiNode) -> Result<Self, crate::ReconcileError> {
        Self::with_primitives(root, PrimitiveRegistry::new())
    }

    /// Create a retained view with a custom primitive registry.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors such as duplicate sibling keys.
    pub fn with_primitives(
        root: UiNode,
        primitives: PrimitiveRegistry,
    ) -> Result<Self, crate::ReconcileError> {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(root)?;
        Ok(Self { tree, primitives })
    }

    /// Reconcile and atomically accept a new Host-owned snapshot.
    ///
    /// # Errors
    ///
    /// Returns structural reconciliation errors without changing the live root.
    pub fn set_root(
        &mut self,
        root: UiNode,
        cx: &mut Context<Self>,
    ) -> Result<crate::ReconcileReport, crate::ReconcileError> {
        let report = self.tree.reconcile(root)?;
        cx.notify();
        Ok(report)
    }

    #[must_use]
    pub fn root(&self) -> Option<&UiNode> {
        self.tree.root()
    }

    #[must_use]
    pub const fn retained(&self) -> &crate::RetainedUiTree {
        &self.tree
    }
}

impl Render for StaticUiView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.root().map_or_else(
            || {
                div()
                    .child("Static UI view has no accepted root")
                    .into_any_element()
            },
            |_| {
                GpuiNodeRenderer::render_retained_with_primitives(
                    &self.tree,
                    &LiteralColorResolver,
                    &InteractionState::default(),
                    &self.primitives,
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorValue, Length, Rgba8, Style};

    #[test]
    fn declarative_nodes_and_typed_styles_convert_without_a_gpui_context() {
        let root = UiNode::column(vec![UiNode::text("one"), UiNode::text("two")]).with_style(
            &Style::new()
                .gap(Length::pixels(8.0).unwrap())
                .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0022_2222))),
        );
        let _element = GpuiNodeRenderer::render(&root);
    }

    #[test]
    fn sampled_animation_values_override_dimensions_and_transform_without_rhai() {
        let values = BTreeMap::from([
            (
                AnimationKey::for_node("root/card", AnimationProperty::Width),
                180.0,
            ),
            (
                AnimationKey::for_node("root/card", AnimationProperty::ClipHeight),
                64.0,
            ),
            (
                AnimationKey::for_node("root/card", AnimationProperty::TranslateX),
                12.0,
            ),
        ]);
        let sampled = node_animation(&values, "root/card");
        let mut style = StyleProperties::default();
        apply_animated_dimensions(&mut style, sampled);
        assert_eq!(style.width, Some(Length::Pixels(180.0)));
        assert_eq!(style.height, Some(Length::Pixels(64.0)));
        assert_eq!(sampled.translate_x, Some(12.0));
    }

    #[test]
    fn native_signal_values_override_approved_properties_without_rhai() {
        let component = crate::ComponentInstancePath::root("Meter", "primary");
        let id =
            crate::SignalId::new(component.clone(), "width", crate::SignalKind::Float).unwrap();
        let signal = crate::NativeSignal::new(id.clone());
        let node = UiNode::text("meter")
            .with_signal_binding(crate::SignalProperty::Width, signal.clone())
            .unwrap();
        let mut registry = crate::SignalRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(
                id,
                crate::signal::SignalDescriptor::new(crate::SignalValue::Float(80.0)),
            )]),
        );
        registry
            .write(&signal, crate::SignalValue::Float(144.0))
            .unwrap();
        let values = node_signals(&registry, &node);
        let mut style = StyleProperties::default();
        apply_signal_style(&mut style, &values);
        assert_eq!(style.width, Some(Length::Pixels(144.0)));
    }

    #[test]
    fn logical_spacing_resolves_to_physical_edges_in_both_directions() {
        let start = Length::Pixels(12.0);
        let end = Length::Pixels(4.0);
        assert_eq!(
            logical_horizontal_edges(
                None,
                None,
                Some(start),
                Some(end),
                TextDirection::LeftToRight,
            ),
            (Some(start), Some(end))
        );
        assert_eq!(
            logical_horizontal_edges(
                None,
                None,
                Some(start),
                Some(end),
                TextDirection::RightToLeft,
            ),
            (Some(end), Some(start))
        );
    }

    #[test]
    fn rtl_keyboard_navigation_maps_physical_arrows_to_logical_handlers() {
        assert_eq!(
            logical_keyboard_key("left", TextDirection::RightToLeft),
            "right"
        );
        assert_eq!(
            logical_keyboard_key("right", TextDirection::RightToLeft),
            "left"
        );
        assert_eq!(
            logical_keyboard_key("left", TextDirection::LeftToRight),
            "left"
        );
        assert_eq!(
            logical_keyboard_key("down", TextDirection::RightToLeft),
            "down"
        );
    }
}
