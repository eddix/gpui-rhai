use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, BoxShadow, Context, Element, ElementId, Entity,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    ParentElement, Pixels, Render, ScrollStrategy, SharedString, Styled, UniformListScrollHandle,
    Window, div, point, px, rgba, uniform_list,
};

use crate::dropdown_element::DropdownSlotRuntime;
use crate::{ColorResolver, ColorValue, Rgba8, VirtualListNodeSpec, VirtualListState};

pub(crate) type VirtualFocusHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;

pub(crate) struct VirtualListEntityElement {
    id: ElementId,
    spec: VirtualListNodeSpec,
    runtime: DropdownSlotRuntime,
    focus_change: Option<VirtualFocusHandler>,
}

impl VirtualListEntityElement {
    pub(crate) fn new(
        path: &str,
        spec: VirtualListNodeSpec,
        runtime: DropdownSlotRuntime,
        focus_change: Option<VirtualFocusHandler>,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/virtual-list-entity")).into(),
            spec,
            runtime,
            focus_change,
        }
    }
}

struct VirtualListElementState {
    view: Entity<VirtualListView>,
}

pub(crate) struct VirtualListFrame {
    element: AnyElement,
}

impl Element for VirtualListEntityElement {
    type RequestLayoutState = VirtualListFrame;
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
            global_id.expect("virtual list element is keyed"),
            |state, window| {
                let state = state.unwrap_or_else(|| VirtualListElementState {
                    view: cx.new(|_| {
                        VirtualListView::new(
                            self.spec.clone(),
                            self.runtime.clone(),
                            self.focus_change.clone(),
                        )
                    }),
                });
                state.view.update(cx, |view, cx| {
                    view.synchronize(
                        self.spec.clone(),
                        self.runtime.clone(),
                        self.focus_change.clone(),
                        cx,
                    );
                });
                let mut element = state.view.clone().into_any_element();
                let layout = element.request_layout(window, cx);
                ((layout, VirtualListFrame { element }), state)
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

impl IntoElement for VirtualListEntityElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct VirtualListView {
    spec: VirtualListNodeSpec,
    state: VirtualListState,
    runtime: DropdownSlotRuntime,
    scroll: UniformListScrollHandle,
    focus_change: Option<VirtualFocusHandler>,
}

impl VirtualListView {
    fn new(
        spec: VirtualListNodeSpec,
        runtime: DropdownSlotRuntime,
        focus_change: Option<VirtualFocusHandler>,
    ) -> Self {
        let mut this = Self {
            spec,
            state: VirtualListState::default(),
            runtime,
            scroll: UniformListScrollHandle::new(),
            focus_change,
        };
        this.install_keys();
        this
    }

    fn synchronize(
        &mut self,
        spec: VirtualListNodeSpec,
        runtime: DropdownSlotRuntime,
        focus_change: Option<VirtualFocusHandler>,
        cx: &mut Context<Self>,
    ) {
        let changed = self.spec != spec;
        self.spec = spec;
        self.runtime = runtime;
        self.focus_change = focus_change;
        self.install_keys();
        if changed {
            cx.notify();
        }
    }

    fn install_keys(&mut self) {
        let _ = self.state.set_keys(
            self.spec
                .items
                .iter()
                .map(|item| item.key.clone())
                .collect(),
        );
        if self.state.focused().is_none() {
            self.state.focus_first();
        }
    }

    fn handle_key(&mut self, key: &str, cx: &mut Context<Self>) -> Option<String> {
        let previous = self.state.focused().map(ToOwned::to_owned);
        match key {
            "up" => self.state.focus_previous(),
            "down" => self.state.focus_next(),
            "home" => self.state.focus_first(),
            "end" => self.state.focus_last(),
            _ => return None,
        }
        let focused = self.state.focused().map(ToOwned::to_owned);
        if focused != previous {
            if let Some(index) = focused
                .as_ref()
                .and_then(|focused| self.spec.items.iter().position(|item| &item.key == focused))
            {
                self.scroll.scroll_to_item_with_offset(
                    index,
                    ScrollStrategy::Center,
                    self.spec.overscan,
                );
            }
            cx.notify();
        }
        focused
    }
}

impl Render for VirtualListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.spec.items.clone();
        let count = items.len();
        let row_height = finite_to_f32(self.spec.row_height);
        let height = finite_to_f32(self.spec.height);
        let runtime = self.runtime.clone();
        let focused = self.state.focused().map(ToOwned::to_owned);
        let focus_color = runtime
            .colors
            .resolve(&ColorValue::Token("surface_hover".to_owned()))
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0x0000_0000));
        let focus_ring = runtime
            .colors
            .resolve(&ColorValue::Token("focus_ring".to_owned()))
            .unwrap_or_else(|| Rgba8::from_rgb_hex(0x003b_82f6));
        let focus_surface = runtime
            .colors
            .resolve(&ColorValue::Token("surface".to_owned()))
            .unwrap_or_else(|| Rgba8::from_rgb_hex(0x0018_181b));
        let list = uniform_list(
            SharedString::from(format!("virtual-list-{}", self.spec.key)),
            count,
            move |range, _window, _cx| {
                range
                    .map(|index| {
                        let item = &items[index];
                        div()
                            .id(("virtual-list-row", index))
                            .h(px(row_height))
                            .when(focused.as_deref() == Some(item.key.as_str()), |row| {
                                row.bg(rgba(focus_color.as_rgba_hex()))
                            })
                            .child(runtime.render(&item.node, &format!("item:{}", item.key)))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(self.scroll.clone())
        .h(px(height))
        .w_full();
        let weak = cx.entity().downgrade();
        div()
            .id(SharedString::from(format!(
                "virtual-list-root-{}",
                self.spec.key
            )))
            .tab_index(0)
            .tab_stop(true)
            .focus(move |style| {
                style.shadow(vec![
                    BoxShadow {
                        color: rgba(focus_ring.as_rgba_hex()).into(),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(4.0),
                    },
                    BoxShadow {
                        color: rgba(focus_surface.as_rgba_hex()).into(),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(2.0),
                    },
                ])
            })
            .on_key_down(move |event: &KeyDownEvent, window, app| {
                if event.keystroke.key.as_str() == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    app.stop_propagation();
                    return;
                }
                if let Ok(Some((focused, handler))) = weak.update(app, |view, cx| {
                    view.handle_key(event.keystroke.key.as_str(), cx)
                        .map(|focused| (focused, view.focus_change.clone()))
                }) {
                    if let Some(handler) = handler {
                        handler(focused, window, app);
                    }
                    app.stop_propagation();
                }
            })
            .child(list)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn finite_to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX));
    value as f32
}
