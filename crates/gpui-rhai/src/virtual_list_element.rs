use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

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
    view: Entity<VirtualListView>,
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
                let mut element = div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(state.view.clone())
                    .into_any_element();
                let layout = element.request_layout(window, cx);
                (
                    (
                        layout,
                        VirtualListFrame {
                            element,
                            view: state.view.clone(),
                        },
                    ),
                    state,
                )
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
        let required = frame.view.read(cx).frame_indices.borrow().clone();
        let realized = self
            .content
            .realized
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let halo = overdraw_item_count(&self.content);
        let target = retained_frame_target(&required, &realized, self.content.data.len(), halo);
        if target.is_empty() || target == realized {
            self.runtime.virtual_requests.clear_target(&self.content.id);
        } else {
            self.runtime
                .virtual_requests
                .request_target(self.content.id.clone(), target);
        }
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

fn overdraw_item_count(content: &VirtualCollectionNodeSpec) -> usize {
    (content.overdraw_pixels / content.estimated_height)
        .ceil()
        .to_string()
        .parse()
        .unwrap_or(usize::MAX)
}

fn retained_frame_target(
    required: &BTreeSet<usize>,
    realized: &BTreeSet<usize>,
    item_count: usize,
    halo: usize,
) -> BTreeSet<usize> {
    if required.is_empty() {
        return BTreeSet::new();
    }
    let mut target = required.clone();
    for index in required {
        let retain_start = index.saturating_sub(halo);
        let retain_end = index.saturating_add(halo).saturating_add(1).min(item_count);
        target.extend(realized.range(retain_start..retain_end).copied());
    }
    target
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
    frame_indices: Rc<RefCell<BTreeSet<usize>>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StickyHeaderFrame {
    index: usize,
    height: Pixels,
    offset: Pixels,
}

impl VirtualListView {
    fn new(content: VirtualCollectionNodeSpec, runtime: NodeSlotRuntime) -> Self {
        let scroll = list_state(&content);
        let mut this = Self {
            content,
            state: VirtualListState::default(),
            runtime,
            scroll,
            frame_indices: Rc::new(RefCell::new(BTreeSet::new())),
        };
        this.install_keys();
        this.reveal_controlled_target();
        this.install_metrics_handler();
        this
    }

    fn synchronize(
        &mut self,
        content: VirtualCollectionNodeSpec,
        runtime: NodeSlotRuntime,
        cx: &mut Context<Self>,
    ) {
        let changed = self.content != content;
        let recreate = collection_requires_recreation(&self.content, &content);
        let reset = collection_requires_reset(&self.content, &content);
        let reveal_changed = self.content.reveal_key != content.reveal_key;
        self.content = content;
        self.runtime = runtime;
        if recreate {
            self.scroll = list_state(&self.content);
        } else if reset {
            self.scroll.reset(self.content.data.len());
        }
        self.install_keys();
        self.install_metrics_handler();
        if changed {
            if reveal_changed || reset || recreate {
                self.reveal_controlled_target();
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

    fn reveal_controlled_target(&mut self) {
        let Some(index) = self.content.reveal_key.as_deref().and_then(|target| {
            (0..self.content.data.len())
                .find(|index| collection_item_key(&self.content, *index) == Some(target))
        }) else {
            return;
        };
        let viewport = self.scroll.viewport_bounds();
        if let Some(bounds) = self.scroll.bounds_for_item(index)
            && viewport.size.height > px(0.0)
        {
            if bounds.top() < viewport.top() || bounds.bottom() > viewport.bottom() {
                self.scroll.scroll_to_reveal_item(index);
            }
        } else {
            // GPUI's variable list cannot infer the height of an unmeasured
            // offscreen item. Top-aligning its logical index gives the next
            // frame a deterministic place to realize and measure it.
            self.scroll.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: px(0.0),
            });
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

    fn install_metrics_handler(&self) {
        let metrics = self.runtime.virtual_requests.clone();
        let id = self.content.id.clone();
        self.scroll.set_scroll_handler(move |event, _, _| {
            metrics.report_scroll(&id, event.visible_range.clone(), event.is_scrolled);
        });
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
        let viewport = self.scroll.viewport_bounds();
        let scroll_top = self.scroll.logical_scroll_top();
        let measured_visible = measured_visible_range(&self.scroll, &self.content, viewport);
        self.runtime.virtual_requests.report_frame(
            &self.content,
            f64::from(viewport.size.height),
            scroll_top.item_ix,
            f64::from(scroll_top.offset_in_item),
            measured_visible,
        );
        let content = self.content.clone();
        let runtime = self.runtime.clone();
        let frame_indices = Rc::new(RefCell::new(BTreeSet::new()));
        self.frame_indices = Rc::clone(&frame_indices);
        let sticky = sticky_header_frame(&self.scroll, &content, &runtime, viewport);
        if let Some(sticky) = sticky {
            frame_indices.borrow_mut().insert(sticky.index);
        }
        let focused = self.state.focused().map(ToOwned::to_owned);
        let (focus_color, focus_shadows) = virtual_focus_style(&runtime);
        let fixed_height = content.height.map(finite_to_f32);
        let list_content = content.clone();
        let list_runtime = runtime.clone();
        let list_frame_indices = Rc::clone(&frame_indices);
        let list = list(self.scroll.clone(), move |index, _window, _cx| {
            list_frame_indices.borrow_mut().insert(index);
            render_virtual_item(
                &list_content,
                &list_runtime,
                focused.as_deref(),
                focus_color,
                sticky,
                index,
            )
        })
        .w_full()
        .when_some(fixed_height, |list, height| list.h(px(height)))
        .when(fixed_height.is_none(), |list| list.flex_1().min_h(px(0.0)));
        let sticky = sticky.and_then(|sticky| {
            let key = collection_item_key(&content, sticky.index)?;
            let node = content.realized.get(&sticky.index)?;
            Some((sticky, runtime.render(node, &format!("item:{key}"))))
        });
        let root_selector = format!("virtual-list:{}", self.content.id.key);
        let sticky_selector = format!("virtual-list-sticky:{}", self.content.id.key);
        let weak = cx.entity().downgrade();
        div()
            .relative()
            .flex()
            .flex_col()
            .id(SharedString::from(format!(
                "virtual-list-root-{}",
                self.content.id.key
            )))
            .debug_selector(move || root_selector.clone())
            .tab_index(0)
            .tab_stop(true)
            .focus(move |style| style.shadow(focus_shadows.clone()))
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
                let _ = weak.update(app, |view, cx| {
                    view.handle_key(event.keystroke.key.as_str(), cx)
                });
            })
            .when(fixed_height.is_none(), |root| root.flex_1().min_h(px(0.0)))
            .child(list)
            .when_some(sticky, |root, (sticky, element)| {
                root.child(
                    div()
                        .id(("virtual-list-sticky-header", sticky.index))
                        .debug_selector(move || sticky_selector.clone())
                        .absolute()
                        .top(sticky.offset)
                        .left_0()
                        .right_0()
                        .h(sticky.height)
                        .child(element),
                )
            })
    }
}

fn virtual_focus_style(runtime: &NodeSlotRuntime) -> (Rgba8, Vec<BoxShadow>) {
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
    (
        focus_color,
        vec![
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
        ],
    )
}

fn render_virtual_item(
    content: &VirtualCollectionNodeSpec,
    runtime: &NodeSlotRuntime,
    focused: Option<&str>,
    focus_color: Rgba8,
    sticky: Option<StickyHeaderFrame>,
    index: usize,
) -> AnyElement {
    if sticky.is_some_and(|sticky| sticky.index == index) {
        return div()
            .id(("virtual-list-row", index))
            .h(sticky.map_or(px(0.0), |sticky| sticky.height))
            .into_any_element();
    }
    let key = collection_item_key(content, index)
        .map_or_else(|| format!("item-{index}"), ToOwned::to_owned);
    let child = content.realized.get(&index).map_or_else(
        || {
            div()
                .h(px(finite_to_f32(content.estimated_height)))
                .into_any_element()
        },
        |node| runtime.render(node, &format!("item:{key}")),
    );
    div()
        .id(("virtual-list-row", index))
        .when(focused == Some(key.as_str()), |row| {
            row.bg(rgba(focus_color.as_rgba_hex()))
        })
        .child(child)
        .into_any_element()
}

fn sticky_header_frame(
    state: &ListState,
    content: &VirtualCollectionNodeSpec,
    runtime: &NodeSlotRuntime,
    viewport: Bounds<Pixels>,
) -> Option<StickyHeaderFrame> {
    let scroll_top = state.logical_scroll_top();
    let index = active_sticky_header(&content.sticky_headers, scroll_top.item_ix)?;
    let height = sticky_header_height(content, runtime, index);
    let next = content
        .sticky_headers
        .range(index.saturating_add(1)..)
        .next()
        .and_then(|next| state.bounds_for_item(*next));
    let offset = sticky_push_offset(height, viewport.top(), next);
    Some(StickyHeaderFrame {
        index,
        height,
        offset,
    })
}

fn active_sticky_header(headers: &BTreeSet<usize>, item: usize) -> Option<usize> {
    headers.range(..=item).next_back().copied()
}

fn sticky_push_offset(
    height: Pixels,
    viewport_top: Pixels,
    next: Option<Bounds<Pixels>>,
) -> Pixels {
    next.map_or(px(0.0), |next| {
        (next.top() - viewport_top - height)
            .min(px(0.0))
            .max(-height)
    })
}

fn sticky_header_height(
    content: &VirtualCollectionNodeSpec,
    runtime: &NodeSlotRuntime,
    index: usize,
) -> Pixels {
    collection_item_key(content, index)
        .and_then(|key| runtime.retained_roots.get(&format!("item:{key}")))
        .and_then(|node| runtime.geometry.get(*node))
        .map_or_else(
            || px(finite_to_f32(content.estimated_height)),
            |geometry| px(finite_to_f32(geometry.visual.height)),
        )
}

fn measured_visible_range(
    state: &ListState,
    content: &VirtualCollectionNodeSpec,
    viewport: Bounds<Pixels>,
) -> Option<std::ops::Range<usize>> {
    if viewport.size.height <= px(0.0) {
        return None;
    }
    let mut visible = content.realized.keys().copied().filter(|index| {
        state.bounds_for_item(*index).is_some_and(|bounds| {
            bounds.bottom() > viewport.top() && bounds.top() < viewport.bottom()
        })
    });
    let first = visible.next()?;
    let (min, max) = visible.fold((first, first), |(min, max), index| {
        (min.min(index), max.max(index))
    });
    Some(min..max.saturating_add(1))
}

pub(crate) fn collection_item_key(spec: &VirtualCollectionNodeSpec, index: usize) -> Option<&str> {
    spec.data.key(index)
}

fn collection_requires_recreation(
    current: &VirtualCollectionNodeSpec,
    next: &VirtualCollectionNodeSpec,
) -> bool {
    current.id != next.id
        || current.overdraw_pixels.to_bits() != next.overdraw_pixels.to_bits()
        || current.bottom_align != next.bottom_align
}

fn collection_requires_reset(
    current: &VirtualCollectionNodeSpec,
    next: &VirtualCollectionNodeSpec,
) -> bool {
    current.estimated_height.to_bits() != next.estimated_height.to_bits()
        || !same_key_order(&current.data, &next.data)
}

fn same_key_order(
    current: &crate::VirtualCollectionData,
    next: &crate::VirtualCollectionData,
) -> bool {
    current.len() == next.len()
        && (0..current.len()).all(|index| current.key(index) == next.key(index))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn indices(range: std::ops::Range<usize>) -> BTreeSet<usize> {
        range.collect()
    }

    #[test]
    fn frame_target_retains_cached_overdraw_without_oscillation() {
        assert_eq!(
            retained_frame_target(&indices(0..22), &indices(0..26), 1_000, 4),
            indices(0..26)
        );
    }

    #[test]
    fn frame_target_prunes_items_outside_the_current_halo() {
        assert_eq!(
            retained_frame_target(&indices(0..10), &indices(0..26), 1_000, 4),
            indices(0..14)
        );
        assert_eq!(
            retained_frame_target(&indices(20..30), &indices(0..26), 1_000, 4),
            indices(16..30)
        );
    }

    #[test]
    fn frame_target_does_not_fill_the_gap_to_an_offscreen_focused_item() {
        let mut required = indices(20..30);
        required.insert(0);
        let mut expected = indices(0..5);
        expected.extend(indices(16..30));
        assert_eq!(
            retained_frame_target(&required, &indices(0..26), 1_000, 4),
            expected
        );
    }

    #[test]
    fn sticky_sections_select_the_latest_header_and_push_before_the_next() {
        let headers = BTreeSet::from([0, 8, 20]);
        assert_eq!(active_sticky_header(&headers, 0), Some(0));
        assert_eq!(active_sticky_header(&headers, 19), Some(8));
        assert_eq!(active_sticky_header(&headers, 20), Some(20));
        assert_eq!(
            sticky_push_offset(
                px(30.0),
                px(100.0),
                Some(Bounds::new(
                    point(px(0.0), px(110.0)),
                    gpui::size(px(200.0), px(30.0))
                ))
            ),
            px(-20.0)
        );
        assert_eq!(sticky_push_offset(px(30.0), px(100.0), None), px(0.0));
    }
}
