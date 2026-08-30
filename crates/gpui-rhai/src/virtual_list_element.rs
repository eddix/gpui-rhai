use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, BoxShadow, Context, Element, ElementId, Entity,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    ListAlignment, ListOffset, ListState, ParentElement, Pixels, Render, SharedString, Styled,
    Window, div, list, point, px, rgba,
};

use crate::slot_runtime::NodeSlotRuntime;
use crate::{ColorResolver, ColorValue, Rgba8, VirtualCollectionNodeSpec, VirtualListState};

pub(crate) struct VirtualListEntityElement {
    id: ElementId,
    content: VirtualCollectionNodeSpec,
    runtime: NodeSlotRuntime,
}

impl VirtualListEntityElement {
    pub(crate) fn new_collection(
        path: &str,
        spec: VirtualCollectionNodeSpec,
        runtime: NodeSlotRuntime,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/virtual-collection-entity")).into(),
            content: spec,
            runtime,
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
                    view: cx
                        .new(|_| VirtualListView::new(self.content.clone(), self.runtime.clone())),
                });
                state.view.update(cx, |view, cx| {
                    view.synchronize(self.content.clone(), self.runtime.clone(), cx);
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
    content: VirtualCollectionNodeSpec,
    state: VirtualListState,
    runtime: NodeSlotRuntime,
    scroll: ListState,
}

impl VirtualListView {
    fn new(content: VirtualCollectionNodeSpec, runtime: NodeSlotRuntime) -> Self {
        let scroll = list_state(&content);
        let mut this = Self {
            content,
            state: VirtualListState::default(),
            runtime,
            scroll,
        };
        this.install_keys();
        this
    }

    fn synchronize(
        &mut self,
        content: VirtualCollectionNodeSpec,
        runtime: NodeSlotRuntime,
        cx: &mut Context<Self>,
    ) {
        let changed = self.content != content;
        let reset = collection_requires_reset(&self.content, &content);
        let alignment_changed = self.content.bottom_align != content.bottom_align;
        self.content = content;
        self.runtime = runtime;
        self.install_keys();
        if changed {
            if reset {
                if alignment_changed {
                    self.scroll = list_state(&self.content);
                } else {
                    self.scroll.reset(self.content.data.len());
                }
            }
            if self.content.follow_tail && !self.content.data.is_empty() {
                self.scroll.scroll_to(ListOffset {
                    item_ix: self.content.data.len() - 1,
                    offset_in_item: px(0.0),
                });
            }
            cx.notify();
        }
    }

    fn install_keys(&mut self) {
        let _ = self.state.set_keys(
            (0..self.content.data.len())
                .filter_map(|index| {
                    collection_item_key(&self.content, index).map(ToOwned::to_owned)
                })
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
            if let Some(index) = focused.as_ref().and_then(|focused| {
                (0..self.content.data.len()).find(|index| {
                    collection_item_key(&self.content, *index) == Some(focused.as_str())
                })
            }) {
                self.scroll.scroll_to_reveal_item(index);
            }
            cx.notify();
        }
        focused
    }
}

impl Render for VirtualListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.content.clone();
        let height = finite_to_f32(content.height);
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
        let list = list(self.scroll.clone(), move |index, _window, _cx| {
            let key = collection_item_key(&content, index)
                .map_or_else(|| format!("item-{index}"), ToOwned::to_owned);
            runtime
                .virtual_requests
                .request(content.id.clone(), [index]);
            let node = content.realized.get(&index);
            let child = node.map_or_else(
                || {
                    div()
                        .h(px(finite_to_f32(content.estimated_height)))
                        .into_any_element()
                },
                |node| runtime.render(node, &format!("item:{key}")),
            );
            div()
                .id(("virtual-list-row", index))
                .when(focused.as_deref() == Some(key.as_str()), |row| {
                    row.bg(rgba(focus_color.as_rgba_hex()))
                })
                .child(child)
                .into_any_element()
        })
        .h(px(height))
        .w_full();
        let weak = cx.entity().downgrade();
        div()
            .id(SharedString::from(format!(
                "virtual-list-root-{}",
                self.content.id.key
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
                if let Ok(Some(_)) = weak.update(app, |view, cx| {
                    view.handle_key(event.keystroke.key.as_str(), cx)
                }) {
                    app.stop_propagation();
                }
            })
            .child(list)
    }
}

fn collection_item_key(spec: &VirtualCollectionNodeSpec, index: usize) -> Option<&str> {
    spec.data.get(index).and_then(|item| match item {
        crate::UiValue::Map(map) => match map.get("key") {
            Some(crate::UiValue::String(key)) => Some(key.as_str()),
            _ => None,
        },
        _ => None,
    })
}

fn collection_requires_reset(
    current: &VirtualCollectionNodeSpec,
    next: &VirtualCollectionNodeSpec,
) -> bool {
    current.id != next.id
        || current.data != next.data
        || current.estimated_height.to_bits() != next.estimated_height.to_bits()
        || current.height.to_bits() != next.height.to_bits()
        || current.overdraw_pixels.to_bits() != next.overdraw_pixels.to_bits()
        || current.bottom_align != next.bottom_align
}

fn list_state(spec: &VirtualCollectionNodeSpec) -> ListState {
    ListState::new(
        spec.data.len(),
        if spec.bottom_align {
            ListAlignment::Bottom
        } else {
            ListAlignment::Top
        },
        px(finite_to_f32(spec.overdraw_pixels)),
    )
}

#[allow(clippy::cast_possible_truncation)]
fn finite_to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX));
    value as f32
}
