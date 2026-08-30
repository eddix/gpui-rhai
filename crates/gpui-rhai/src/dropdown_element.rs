use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Element, ElementId, Entity, Focusable,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    ParentElement, Pixels, Render, ScrollStrategy, SharedString, StatefulInteractiveElement,
    Styled, UniformListScrollHandle, Window, div, img, px, relative, rems, rgba, uniform_list,
};

use crate::overlay_element::{
    OpenChangeHandler, PanelKeyHandler, ScriptOverlayElement, WindowOverlayCoordinator,
};
use crate::renderer::{
    GpuiNodeRenderer, OwnedColorResolver, WindowRenderResources, apply_style_override,
};
use crate::text_input::{TextInputCallbacks, TextInputEntity};
use crate::{
    AnimationKey, AssetId, AssetRegistry, ColorResolver, DropdownKey, DropdownNodeSpec,
    DropdownOption, DropdownOutcome, DropdownState, DropdownVisibleRow, InteractionState, Length,
    NodeEventDispatcher, OverlayDismissPolicy, OverlayId, OverlayKind, OverlayNodeSpec,
    PrimitiveRegistry, Rgba8, Style, UiNode,
};

pub(crate) type SelectionHandler = Rc<dyn Fn(Vec<String>, &mut Window, &mut App)>;
pub(crate) type QueryHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;

#[derive(Clone, Default)]
pub(crate) struct DropdownCallbacks {
    pub selection: Option<SelectionHandler>,
    pub open: Option<OpenChangeHandler>,
    pub query: Option<QueryHandler>,
}

#[derive(Clone, Copy)]
pub(crate) struct DropdownPalette {
    pub surface: Rgba8,
    pub hover: Rgba8,
    pub muted: Rgba8,
    pub accent: Rgba8,
    pub disabled: Rgba8,
    pub focus_ring: Rgba8,
}

#[derive(Clone)]
pub(crate) struct DropdownSlotRuntime {
    pub colors: OwnedColorResolver,
    pub primitives: PrimitiveRegistry,
    pub assets: AssetRegistry,
    pub dispatcher: NodeEventDispatcher,
    pub overlays: WindowOverlayCoordinator,
    pub animations: BTreeMap<AnimationKey, f64>,
    pub signals: crate::SignalRegistry,
    pub direction: crate::TextDirection,
    pub base_path: String,
    pub view_id: String,
    pub part_styles: BTreeMap<String, Style>,
}

impl DropdownSlotRuntime {
    pub(crate) fn render(&self, node: &UiNode, slot: &str) -> AnyElement {
        let resources = WindowRenderResources {
            assets: &self.assets,
            dispatcher: &self.dispatcher,
            overlays: &self.overlays,
            animations: &self.animations,
            signals: &self.signals,
            direction: self.direction,
            root_path: &self.base_path,
            view_id: &self.view_id,
        };
        GpuiNodeRenderer::render_subtree_with_window_runtime(
            node,
            &self.colors,
            &InteractionState::default(),
            &self.primitives,
            &resources,
            &format!("{}/{slot}", self.base_path),
        )
    }

    pub(crate) fn style(&self, element: gpui::Div, part: &str) -> gpui::Div {
        if let Some(style) = self.part_styles.get(part) {
            apply_style_override(element, style, &self.colors, self.direction)
        } else {
            element
        }
    }

    pub(crate) fn part_color(&self, part: &str, fallback: Rgba8) -> Rgba8 {
        self.part_styles
            .get(part)
            .and_then(|style| {
                style
                    .resolve(&InteractionState::default())
                    .text_color
                    .and_then(|color| self.colors.resolve(&color))
            })
            .unwrap_or(fallback)
    }
}

pub(crate) struct DropdownEntityElement {
    id: ElementId,
    spec: DropdownNodeSpec,
    callbacks: DropdownCallbacks,
    palette: DropdownPalette,
    coordinator: WindowOverlayCoordinator,
    slot_runtime: DropdownSlotRuntime,
}

impl DropdownEntityElement {
    pub(crate) fn new(
        path: &str,
        spec: DropdownNodeSpec,
        callbacks: DropdownCallbacks,
        palette: DropdownPalette,
        coordinator: WindowOverlayCoordinator,
        slot_runtime: DropdownSlotRuntime,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/dropdown-entity")).into(),
            spec,
            callbacks,
            palette,
            coordinator,
            slot_runtime,
        }
    }
}

struct DropdownElementState {
    view: Entity<DropdownView>,
}

pub(crate) struct DropdownFrame {
    element: AnyElement,
}

impl Element for DropdownEntityElement {
    type RequestLayoutState = DropdownFrame;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            global_id.expect("dropdown entity element is keyed"),
            |state, window| {
                let state = state.unwrap_or_else(|| DropdownElementState {
                    view: cx.new(|cx| {
                        DropdownView::new(
                            self.spec.clone(),
                            self.callbacks.clone(),
                            self.palette,
                            self.coordinator.clone(),
                            self.slot_runtime.clone(),
                            cx,
                        )
                    }),
                });
                let search_sync = state.view.update(cx, |view, cx| {
                    view.synchronize(
                        self.spec.clone(),
                        self.callbacks.clone(),
                        self.palette,
                        self.coordinator.clone(),
                        self.slot_runtime.clone(),
                        cx,
                    )
                });
                if let Some(search_sync) = search_sync {
                    search_sync.entity.update(cx, |input, input_cx| {
                        input.update_props(
                            &search_sync.value,
                            &search_sync.placeholder,
                            false,
                            false,
                            search_sync.callbacks,
                            input_cx,
                        );
                    });
                }
                let mut element = state.view.clone().into_any_element();
                let layout = element.request_layout(window, cx);
                ((layout, DropdownFrame { element }), state)
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.paint(window, cx);
    }
}

impl IntoElement for DropdownEntityElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct DropdownView {
    spec: DropdownNodeSpec,
    state: DropdownState,
    pending_controlled_open: Option<bool>,
    callbacks: DropdownCallbacks,
    palette: DropdownPalette,
    scroll: UniformListScrollHandle,
    search_input: Option<Entity<TextInputEntity>>,
    coordinator: WindowOverlayCoordinator,
    slot_runtime: DropdownSlotRuntime,
}

struct SearchInputSync {
    entity: Entity<TextInputEntity>,
    value: String,
    placeholder: String,
    callbacks: TextInputCallbacks,
}

impl DropdownView {
    fn new(
        spec: DropdownNodeSpec,
        callbacks: DropdownCallbacks,
        palette: DropdownPalette,
        coordinator: WindowOverlayCoordinator,
        slot_runtime: DropdownSlotRuntime,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut state = DropdownState::new(
            spec.options.clone(),
            spec.mode,
            spec.selected.clone().unwrap_or_default(),
        )
        .expect("validated dropdown node has valid options");
        if let Some(query) = &spec.query {
            state
                .set_query(query.clone())
                .expect("validated dropdown query preserves unique keys");
        }
        if spec.open == Some(true) {
            state.open();
        }
        let search_input = spec.behavior.searchable.then(|| {
            Self::create_search_input(
                state.query(),
                &spec.search_placeholder,
                Self::search_callbacks(cx),
                cx,
            )
        });
        Self {
            spec,
            state,
            pending_controlled_open: None,
            callbacks,
            palette,
            scroll: UniformListScrollHandle::new(),
            search_input,
            coordinator,
            slot_runtime,
        }
    }

    fn synchronize(
        &mut self,
        spec: DropdownNodeSpec,
        callbacks: DropdownCallbacks,
        palette: DropdownPalette,
        coordinator: WindowOverlayCoordinator,
        slot_runtime: DropdownSlotRuntime,
        cx: &mut Context<Self>,
    ) -> Option<SearchInputSync> {
        self.state
            .synchronize(spec.options.clone(), spec.mode, spec.selected.as_deref())
            .expect("validated dropdown synchronization preserves option invariants");
        if let Some(query) = &spec.query {
            self.state
                .set_query(query.clone())
                .expect("validated dropdown query preserves unique keys");
        }
        if let Some(open) = spec.open {
            if self.pending_controlled_open == Some(open) {
                self.pending_controlled_open = None;
                self.state.set_controlled_open(open);
            } else if self.pending_controlled_open.is_none() {
                self.state.set_controlled_open(open);
            }
        } else {
            self.pending_controlled_open = None;
        }
        self.spec = spec;
        self.callbacks = callbacks;
        self.palette = palette;
        self.coordinator = coordinator;
        self.slot_runtime = slot_runtime;
        self.synchronize_search_input(cx)
    }

    fn search_callbacks(cx: &mut Context<Self>) -> TextInputCallbacks {
        let change_parent = cx.entity().downgrade();
        let tab_parent = cx.entity().downgrade();
        TextInputCallbacks {
            change: Some(Rc::new(move |query, window, app| {
                if let Ok(emission) =
                    change_parent.update(app, |view, cx| view.set_query(query.clone(), cx))
                {
                    emission.emit(window, app);
                }
            })),
            submit: None,
            tab: Some(Rc::new(move |shift, window, app| {
                if let Ok(emission) = tab_parent.update(app, |view, cx| view.set_open(false, cx)) {
                    emission.emit(window, app);
                }
                if shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
            })),
            ..TextInputCallbacks::default()
        }
    }

    fn create_search_input(
        value: &str,
        placeholder: &str,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) -> Entity<TextInputEntity> {
        cx.new(|input_cx| {
            TextInputEntity::new(value, placeholder, false, false, callbacks, input_cx)
        })
    }

    fn synchronize_search_input(&mut self, cx: &mut Context<Self>) -> Option<SearchInputSync> {
        if !self.spec.behavior.searchable {
            self.search_input = None;
            return None;
        }
        let callbacks = Self::search_callbacks(cx);
        if let Some(input) = &self.search_input {
            Some(SearchInputSync {
                entity: input.clone(),
                value: self.state.query().to_owned(),
                placeholder: self.spec.search_placeholder.clone(),
                callbacks,
            })
        } else {
            self.search_input = Some(Self::create_search_input(
                self.state.query(),
                &self.spec.search_placeholder,
                callbacks,
                cx,
            ));
            None
        }
    }

    fn selected_label(&self) -> String {
        let labels = self
            .spec
            .options
            .iter()
            .filter(|option| self.state.selected().contains(&option.value))
            .map(|option| option.label.as_str())
            .collect::<Vec<_>>();
        if labels.is_empty() {
            self.spec.placeholder.clone()
        } else {
            labels.join(", ")
        }
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) -> DropdownEmission {
        let changed = self.state.set_controlled_open(open);
        let query = if !open && self.spec.behavior.reset_query_on_close {
            self.state
                .clear_query()
                .expect("clearing choice-list query preserves option identity")
                .then(String::new)
        } else {
            None
        };
        if changed && self.spec.open.is_some() {
            self.pending_controlled_open = Some(open);
        }
        if changed {
            cx.notify();
        }
        DropdownEmission {
            callbacks: self.callbacks.clone(),
            open: changed.then_some(open),
            query,
            ..DropdownEmission::default()
        }
    }

    fn activate(&mut self, value: &str, cx: &mut Context<Self>) -> DropdownEmission {
        let outcome = self
            .state
            .select_value(value)
            .expect("rendered dropdown option remains valid");
        self.emission(outcome, true, cx)
    }

    fn clear_selection(&mut self, cx: &mut Context<Self>) -> DropdownEmission {
        let selection_changed = self.state.clear_selection();
        let open_changed = self.state.close();
        let query = if self.spec.behavior.reset_query_on_close {
            self.state
                .clear_query()
                .expect("clearing choice-list query preserves option identity")
                .then(String::new)
        } else {
            None
        };
        if selection_changed || open_changed || query.is_some() {
            cx.notify();
        }
        DropdownEmission {
            callbacks: self.callbacks.clone(),
            selection: selection_changed.then(Vec::new),
            open: open_changed.then_some(false),
            query,
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) -> DropdownEmission {
        let changed = self
            .state
            .set_query(query.clone())
            .expect("native search preserves unique option keys");
        if changed {
            cx.notify();
        }
        DropdownEmission {
            callbacks: self.callbacks.clone(),
            query: changed.then_some(query),
            ..DropdownEmission::default()
        }
    }

    fn search_focused(&self, window: &Window, cx: &App) -> bool {
        self.search_input
            .as_ref()
            .is_some_and(|input| input.focus_handle(cx).is_focused(window))
    }

    fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        text_editing: bool,
        cx: &mut Context<Self>,
    ) -> (bool, DropdownEmission) {
        if text_editing
            && !matches!(
                event.keystroke.key.as_str(),
                "up" | "down" | "enter" | "escape"
            )
        {
            return (false, DropdownEmission::default());
        }
        let key = match event.keystroke.key.as_str() {
            "up" => Some(DropdownKey::ArrowUp),
            "down" => Some(DropdownKey::ArrowDown),
            "home" => Some(DropdownKey::Home),
            "end" => Some(DropdownKey::End),
            "enter" => Some(DropdownKey::Enter),
            "escape" => return (false, DropdownEmission::default()),
            "backspace" if self.spec.behavior.searchable => {
                let mut query = self.state.query().to_owned();
                query.pop();
                let changed = self
                    .state
                    .set_query(query.clone())
                    .expect("dropdown search preserves unique option keys");
                if changed {
                    cx.notify();
                }
                return (
                    true,
                    DropdownEmission {
                        callbacks: self.callbacks.clone(),
                        query: changed.then_some(query),
                        ..DropdownEmission::default()
                    },
                );
            }
            _ => None,
        };
        if let Some(key) = key {
            let now = cx.background_executor().now();
            let outcome = self
                .state
                .handle_key(key, now)
                .expect("dropdown keyboard navigation preserves option identity");
            return (true, self.emission(outcome, true, cx));
        }
        let Some(text) =
            event.keystroke.key_char.as_deref().filter(|_| {
                !event.keystroke.modifiers.control && !event.keystroke.modifiers.platform
            })
        else {
            return (false, DropdownEmission::default());
        };
        if self.spec.behavior.searchable {
            let query = format!("{}{text}", self.state.query());
            let changed = self
                .state
                .set_query(query.clone())
                .expect("dropdown search preserves unique option keys");
            if changed {
                cx.notify();
            }
            (
                true,
                DropdownEmission {
                    callbacks: self.callbacks.clone(),
                    query: changed.then_some(query),
                    ..DropdownEmission::default()
                },
            )
        } else {
            let mut outcome = DropdownOutcome::default();
            let now = cx.background_executor().now();
            for character in text.chars() {
                outcome = self
                    .state
                    .handle_key(DropdownKey::Character(character), now)
                    .expect("dropdown type-ahead preserves option identity");
            }
            (true, self.emission(outcome, true, cx))
        }
    }

    fn emission(
        &mut self,
        mut outcome: DropdownOutcome,
        notify_for_focus: bool,
        cx: &mut Context<Self>,
    ) -> DropdownEmission {
        if outcome.open_changed && !self.state.is_open() && self.spec.behavior.reset_query_on_close
        {
            outcome.query_changed = self
                .state
                .clear_query()
                .expect("clearing choice-list query preserves option identity");
        }
        if outcome.open_changed && self.spec.open.is_some() {
            self.pending_controlled_open = Some(self.state.is_open());
        }
        if notify_for_focus
            || outcome.selection_changed
            || outcome.open_changed
            || outcome.query_changed
        {
            if let Some(index) = self.state.focused_visible_index() {
                self.scroll.scroll_to_item(index, ScrollStrategy::Center);
            }
            cx.notify();
        }
        DropdownEmission {
            callbacks: self.callbacks.clone(),
            selection: outcome
                .selection_changed
                .then(|| self.state.selected().iter().cloned().collect()),
            open: outcome.open_changed.then_some(self.state.is_open()),
            query: outcome.query_changed.then(|| self.state.query().to_owned()),
        }
    }

    fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = self.state.visible_rows().collect::<Vec<_>>();
        let selected = self.state.selected().clone();
        let focused = self.state.focused().map(ToOwned::to_owned);
        let weak = cx.entity().downgrade();
        let palette = self.palette;
        let check_asset = self.spec.check_asset.clone();
        let option_runtime = self.slot_runtime.clone();
        let list = if rows.is_empty() {
            self.spec.empty_slot.as_deref().map_or_else(
                || {
                    self.slot_runtime
                        .style(
                            div()
                                .h(px(to_f32(self.spec.row_height)))
                                .flex()
                                .items_center()
                                .child(self.spec.empty_text.clone()),
                            "empty",
                        )
                        .into_any_element()
                },
                |empty| self.slot_runtime.render(empty, "empty_slot"),
            )
        } else {
            let count = rows.len();
            let row_height = to_f32(self.spec.row_height);
            uniform_list(
                SharedString::from(format!("dropdown-{}-options", self.spec.id)),
                count,
                move |range, _window, _cx| {
                    range
                        .map(|index| match &rows[index] {
                            DropdownVisibleRow::Group(group) => option_runtime
                                .style(
                                    div()
                                        .h(px(row_height))
                                        .flex()
                                        .items_center()
                                        .child(group.clone()),
                                    "group",
                                )
                                .into_any_element(),
                            DropdownVisibleRow::Option(option) => render_option_row(
                                index,
                                option,
                                &selected,
                                focused.as_deref(),
                                &weak,
                                &OptionRowVisual {
                                    palette,
                                    row_height,
                                    check_asset: check_asset.clone(),
                                },
                                &option_runtime,
                            ),
                        })
                        .collect::<Vec<_>>()
                },
            )
            .track_scroll(self.scroll.clone())
            .h(px(rows_height(
                count.min(self.spec.max_visible),
                self.spec.row_height,
            )))
            .w_full()
            .into_any_element()
        };
        self.slot_runtime
            .style(div().child(list), "list")
            .into_any_element()
    }

    fn render_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let list = self.render_list(cx);
        let search_input = self.search_input.clone();
        let header = self
            .spec
            .header_slot
            .as_deref()
            .map(|header| self.slot_runtime.render(header, "header_slot"));
        let footer = self
            .spec
            .footer_slot
            .as_deref()
            .map(|footer| self.slot_runtime.render(footer, "footer_slot"));
        self.slot_runtime
            .style(
                div()
                    .w(px(to_f32(self.spec.panel_width)))
                    .when_some(self.spec.panel_extra_height, |panel, extra| {
                        panel.max_h(px(rows_height(self.spec.max_visible, self.spec.row_height)
                            + to_f32(extra)))
                    })
                    .children(header)
                    .when_some(search_input, |panel, search_input| {
                        panel.child(
                            self.slot_runtime.style(
                                div()
                                    .h(px(to_f32(self.spec.row_height)))
                                    .child(search_input),
                                "search",
                            ),
                        )
                    })
                    .child(list)
                    .children(footer),
                "panel",
            )
            .into_any_element()
    }

    fn render_trigger(&self, cx: &mut Context<Self>) -> AnyElement {
        let label = self.selected_label();
        let palette = self.palette;
        let trigger = self.spec.trigger_slot.as_deref().map_or_else(
            || {
                let clearable = self.spec.behavior.clearable && !self.state.selected().is_empty();
                let weak_clear = cx.entity().downgrade();
                let clear_icon = choice_asset_element(
                    &self.slot_runtime,
                    &self.spec.clear_asset,
                    self.slot_runtime.part_color("clear", palette.muted),
                    self.spec.trigger_height * 0.45,
                );
                let indicator_icon = choice_asset_element(
                    &self.slot_runtime,
                    &self.spec.indicator_asset,
                    self.slot_runtime.part_color("indicator", palette.muted),
                    self.spec.trigger_height * 0.45,
                );
                let has_value = !self.state.selected().is_empty();
                let value = self.slot_runtime.style(
                    div().child(label),
                    if has_value { "value" } else { "placeholder" },
                );
                apply_trigger_width(
                    div(),
                    self.slot_runtime
                        .colors
                        .resolve_length(self.spec.trigger_width),
                )
                .h(px(to_f32(self.spec.trigger_height)))
                .flex()
                .items_center()
                .justify_between()
                .when(self.spec.disabled, |trigger| {
                    trigger.bg(rgba(palette.disabled.as_rgba_hex()))
                })
                .child(value)
                .when(clearable, |trigger| {
                    trigger.child(
                        self.slot_runtime
                            .style(div().child(clear_icon), "clear")
                            .id("choice-list-clear")
                            .on_click(move |_, window, app| {
                                if let Ok(emission) =
                                    weak_clear.update(app, DropdownView::clear_selection)
                                {
                                    emit_from_current_view(
                                        weak_clear.clone(),
                                        emission,
                                        window,
                                        app,
                                    );
                                }
                                app.stop_propagation();
                            }),
                    )
                })
                .child(
                    self.slot_runtime
                        .style(div().child(indicator_icon), "indicator"),
                )
                .into_any_element()
            },
            |trigger| self.slot_runtime.render(trigger, "trigger_slot"),
        );
        self.slot_runtime
            .style(div().child(trigger), "trigger")
            .into_any_element()
    }

    fn overlay_spec(&self, open: bool) -> OverlayNodeSpec {
        let overlay_id = WindowOverlayCoordinator::scoped_id(
            &self.slot_runtime.view_id,
            &OverlayId::new(self.spec.id.clone()),
        );
        let parent = self.spec.parent_overlay.as_ref().map(|parent| {
            WindowOverlayCoordinator::scoped_id(
                &self.slot_runtime.view_id,
                &OverlayId::new(parent.clone()),
            )
        });
        OverlayNodeSpec {
            id: overlay_id,
            parent,
            kind: OverlayKind::Dropdown,
            placement: self.spec.placement,
            open,
            gap: self.spec.overlay_gap,
            modal: false,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
        }
    }
}

fn choice_asset_element(
    runtime: &DropdownSlotRuntime,
    asset: &AssetId,
    tint: Rgba8,
    size: f64,
) -> AnyElement {
    runtime.assets.cached_image(asset).map_or_else(
        |error| {
            div()
                .child(format!("Asset error: {error}"))
                .into_any_element()
        },
        |handle| {
            runtime
                .assets
                .image_source_tinted(handle.opaque(), Some(tint))
                .map_or_else(
                    |error| {
                        div()
                            .child(format!("Image error: {error}"))
                            .into_any_element()
                    },
                    |source| {
                        img(source)
                            .w(px(to_f32(size)))
                            .h(px(to_f32(size)))
                            .into_any_element()
                    },
                )
        },
    )
}

impl Render for DropdownView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let trigger = self.render_trigger(cx);
        let content = self.render_content(cx);
        let open = self.state.is_open() && !self.spec.disabled;
        let weak_open = cx.entity().downgrade();
        let weak_focus = weak_open.clone();
        let open_change = (!self.spec.disabled).then(|| {
            Rc::new(move |open, window: &mut Window, app: &mut App| {
                if let Ok(emission) = weak_open.update(app, |view, cx| view.set_open(open, cx)) {
                    if open
                        && let Ok(Some(focus)) = weak_focus.read_with(app, |view, cx| {
                            view.search_input
                                .as_ref()
                                .map(|input| input.focus_handle(cx))
                        })
                    {
                        focus.focus(window);
                    }
                    emission.emit(window, app);
                }
            }) as OpenChangeHandler
        });
        let weak_key = cx.entity().downgrade();
        let panel_key = Rc::new(
            move |event: &KeyDownEvent, window: &mut Window, app: &mut App| {
                let text_editing = weak_key
                    .read_with(app, |view, cx| view.search_focused(window, cx))
                    .unwrap_or(false);
                match weak_key.update(app, |view, cx| view.handle_key(event, text_editing, cx)) {
                    Ok((handled, emission)) => {
                        emit_from_current_view(weak_key.clone(), emission, window, app);
                        handled
                    }
                    Err(_) => false,
                }
            },
        ) as PanelKeyHandler;
        let overlay_spec = self.overlay_spec(open);
        ScriptOverlayElement::new(
            &format!("dropdown/{}", self.spec.id),
            trigger,
            content,
            overlay_spec,
            open_change,
            Some(panel_key),
            self.coordinator.clone(),
        )
        .with_focus_ring(self.palette.focus_ring)
        .with_focus_surface(self.palette.surface)
        .with_open_key("up")
        .with_open_key("down")
        .restore_focus_on_close(true)
        .into_any_element()
    }
}

fn render_option_row(
    index: usize,
    option: &DropdownOption,
    selected: &std::collections::BTreeSet<String>,
    focused: Option<&str>,
    weak: &gpui::WeakEntity<DropdownView>,
    visual: &OptionRowVisual,
    runtime: &DropdownSlotRuntime,
) -> AnyElement {
    let value = option.value.clone();
    let label = option.label.clone();
    let disabled = option.disabled;
    let is_selected = selected.contains(&option.value);
    let is_focused = focused == Some(option.value.as_str());
    let weak = weak.clone();
    let check = if is_selected {
        choice_asset_element(
            runtime,
            &visual.check_asset,
            runtime.part_color("option_selected", visual.palette.accent),
            f64::from(visual.row_height) * 0.45,
        )
    } else {
        div().into_any_element()
    };
    let option = runtime.style(
        div()
            .h(px(visual.row_height))
            .flex()
            .items_center()
            .justify_between(),
        "option",
    );
    let option = if disabled {
        runtime.style(option, "option_disabled")
    } else if is_selected {
        runtime.style(option, "option_selected")
    } else {
        option
    };
    let selected_color = runtime.part_color("option_selected", visual.palette.accent);
    option
        .id(("dropdown-option", index))
        .when(is_focused, |row| {
            row.bg(rgba(visual.palette.hover.as_rgba_hex()))
        })
        .when(is_selected && !disabled, |row| {
            row.text_color(rgba(selected_color.as_rgba_hex()))
        })
        .when(!disabled, |row| {
            row.cursor_pointer()
                .hover(move |style| style.bg(rgba(visual.palette.hover.as_rgba_hex())))
                .on_click(move |_, window, app| {
                    if let Ok(emission) = weak.update(app, |view, cx| view.activate(&value, cx)) {
                        emit_from_current_view(weak.clone(), emission, window, app);
                    }
                })
        })
        .child(label)
        .child(check)
        .into_any_element()
}

#[derive(Clone)]
struct OptionRowVisual {
    palette: DropdownPalette,
    row_height: f32,
    check_asset: AssetId,
}

fn rows_height(rows: usize, row_height: f64) -> f32 {
    f32::from(u16::try_from(rows).expect("choice-list visible row count is at most 32"))
        * to_f32(row_height)
}

fn to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(f32::MAX)
}

fn apply_trigger_width(element: gpui::Div, width: Option<Length>) -> gpui::Div {
    match width {
        Some(Length::Pixels(value)) => element.w(px(to_f32(value))),
        Some(Length::Rems(value)) => element.w(rems(to_f32(value))),
        Some(Length::Relative(value)) => element.w(relative(to_f32(value))),
        Some(Length::ThemeSpacing(_) | Length::ThemeRadius(_)) | None => element.w_full(),
    }
}

#[derive(Clone, Default)]
struct DropdownEmission {
    callbacks: DropdownCallbacks,
    selection: Option<Vec<String>>,
    open: Option<bool>,
    query: Option<String>,
}

fn emit_from_current_view(
    view: gpui::WeakEntity<DropdownView>,
    mut emission: DropdownEmission,
    window: &mut Window,
    cx: &mut App,
) {
    let restore_controlled_close = emission.open == Some(false) && emission.selection.is_some();
    if restore_controlled_close {
        emission.open = None;
    }
    emission.emit(window, cx);
    if restore_controlled_close {
        window.defer(cx, move |window, cx| {
            if let Ok(Some(handler)) = view.read_with(cx, |view, _| view.callbacks.open.clone()) {
                handler(false, window, cx);
            }
        });
    }
}

impl DropdownEmission {
    fn emit(self, window: &mut Window, cx: &mut App) {
        if let (Some(handler), Some(open)) = (self.callbacks.open, self.open) {
            handler(open, window, cx);
        }
        if let (Some(handler), Some(selection)) = (self.callbacks.selection, self.selection) {
            handler(selection, window, cx);
        }
        if let (Some(handler), Some(query)) = (self.callbacks.query, self.query) {
            handler(query, window, cx);
        }
    }
}
