use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Element, ElementId, Entity, FollowMode,
    GlobalElementId, HitboxBehavior, InspectorElementId, InteractiveElement, IntoElement, LayoutId,
    ListAlignment, ListOffset, ListState, ParentElement, Pixels, Render, SharedString, Styled,
    Window, div, list, px,
};

use crate::VirtualCollectionNodeSpec;
use crate::slot_runtime::NodeSlotRuntime;

pub(crate) struct VirtualListEntityElement {
    id: ElementId,
    content: VirtualCollectionNodeSpec,
    runtime: NodeSlotRuntime,
    retained_node: Option<crate::NodeId>,
    scrollbars: crate::ScrollbarContext,
    scrollbar_spec: crate::ScrollbarSpec,
}

impl VirtualListEntityElement {
    pub(crate) fn new_collection(
        path: &str,
        spec: VirtualCollectionNodeSpec,
        runtime: NodeSlotRuntime,
        retained_node: Option<crate::NodeId>,
        scrollbars: crate::ScrollbarContext,
        scrollbar_spec: crate::ScrollbarSpec,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/virtual-collection-entity")).into(),
            content: spec,
            runtime,
            retained_node,
            scrollbars,
            scrollbar_spec,
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
                    view: cx.new(|cx| {
                        VirtualListView::new(
                            self.content.clone(),
                            self.runtime.clone(),
                            cx.entity_id(),
                            self.scrollbars.clone(),
                            self.scrollbar_spec,
                        )
                    }),
                });
                state.view.update(cx, |view, cx| {
                    view.synchronize(self.content.clone(), self.runtime.clone(), cx);
                    view.scrollbars = self.scrollbars.clone();
                    view.scrollbar_spec = self.scrollbar_spec;
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
        bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Register the container beneath its children using this layout's
        // bounds. Scroll physics still use ListState's measured viewport.
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        frame.element.prepaint(window, cx);
        frame.view.update(cx, VirtualListView::seed_viewport_hints);
        frame
            .view
            .update(cx, |view, cx| view.publish_follow_status(window, cx));
        let scroll = frame.view.read(cx).scroll.clone();
        self.runtime.interactions.register_virtual_scroll(
            format!("{}:{}", self.content.id.component, self.content.id.key),
            self.runtime.view_id.clone(),
            self.retained_node,
            scroll,
            frame.view.entity_id(),
            hitbox,
        );
        let mut required = frame.view.read(cx).frame_indices.borrow().clone();
        let collection = format!("{}:{}", self.content.id.component, self.content.id.key);
        if let Some((source, index)) = self.runtime.interactions.app_drag_pin(&collection)
            && index < self.content.data.len()
            && self.content.data.key(index) == Some(source.as_str())
            && let Ok(Some(item)) = self.content.data.item(index)
            && self
                .runtime
                .interactions
                .retain_virtual_drag_source(&collection, index, &item)
        {
            required.insert(index);
        }
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
    runtime: NodeSlotRuntime,
    scroll: ListState,
    frame_indices: Rc<RefCell<BTreeSet<usize>>>,
    /// A fill-height list cannot judge visibility before its viewport is
    /// measured; the reveal waits for the first measured frame.
    pending_reveal: bool,
    /// The item-bounds reader this list installed in the runtime registry.
    bounds_reader: Option<crate::virtual_list::ItemBoundsReader>,
    unseen: Rc<RefCell<BTreeSet<String>>>,
    notify: gpui::EntityId,
    last_follow_status: Option<crate::VirtualCollectionFollowStatus>,
    scrollbars: crate::ScrollbarContext,
    scrollbar_spec: crate::ScrollbarSpec,
    hinted_width: Option<Pixels>,
    scroll_source_epoch: u64,
}

impl Drop for VirtualListView {
    fn drop(&mut self) {
        self.runtime
            .virtual_requests
            .remove_scroll_handle(&self.content.id, &self.unseen);
        if let Some(reader) = self.bounds_reader.take() {
            self.runtime
                .virtual_requests
                .remove_item_bounds_reader(&self.content.id, &reader);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StickyHeaderFrame {
    index: usize,
    height: Pixels,
    offset: Pixels,
}

impl VirtualListView {
    fn new(
        content: VirtualCollectionNodeSpec,
        runtime: NodeSlotRuntime,
        notify: gpui::EntityId,
        scrollbars: crate::ScrollbarContext,
        scrollbar_spec: crate::ScrollbarSpec,
    ) -> Self {
        let scroll = list_state(&content);
        let mut this = Self {
            content,
            runtime,
            scroll,
            frame_indices: Rc::new(RefCell::new(BTreeSet::new())),
            pending_reveal: false,
            bounds_reader: None,
            unseen: Rc::default(),
            notify,
            last_follow_status: None,
            scrollbars,
            scrollbar_spec,
            hinted_width: None,
            scroll_source_epoch: 0,
        };
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
        // Row heights come from measuring rendered rows; a new environment or
        // theme can change every row's geometry, so cached heights are stale.
        let geometry_changed = self.runtime.environment != runtime.environment
            || self.runtime.inherited_disabled != runtime.inherited_disabled
            || !self.runtime.colors.same_tokens(&runtime.colors);
        let changed = self.content != content || geometry_changed;
        let recreate = collection_requires_recreation(&self.content, &content);
        let data_changed = self.content.data != content.data;
        let key_order_changed = data_changed && !same_key_order(&self.content.data, &content.data);
        let was_following = self.scroll.is_following_tail();
        let old_anchor = self.scroll.logical_scroll_top();
        let old_key = self
            .content
            .data
            .key(old_anchor.item_ix)
            .map(ToOwned::to_owned);
        let old_keys = if key_order_changed {
            collection_keys(&self.content.data)
        } else {
            BTreeSet::new()
        };
        let next_keys = if key_order_changed {
            collection_keys(&content.data)
        } else {
            BTreeSet::new()
        };
        let replaced = key_order_changed
            && !old_keys.is_empty()
            && !next_keys.is_empty()
            && old_keys.iter().all(|key| !next_keys.contains(key));
        let follow_changed = self.content.follow_tail != content.follow_tail;
        let preserve_reader =
            recreate && self.content.id == content.id && !replaced && !follow_changed;
        if key_order_changed && !recreate && !replaced {
            self.apply_keyed_edits(
                &content,
                old_anchor,
                old_key.as_deref(),
                was_following,
                (&old_keys, &next_keys),
            );
        }
        let reveal_changed = self.content.reveal_key != content.reveal_key;
        self.content = content;
        self.runtime = runtime;
        if recreate {
            self.scroll = list_state(&self.content);
            self.hinted_width = None;
            let unseen = if preserve_reader {
                self.unseen.borrow().clone()
            } else {
                BTreeSet::new()
            };
            self.unseen = Rc::new(RefCell::new(unseen));
            if preserve_reader && !was_following {
                self.scroll.pause_following_tail();
                if let Some(key) = old_key.as_deref() {
                    self.restore_reader(key, old_anchor.offset_in_item);
                }
            }
        } else if replaced {
            self.scroll.reset_with_uniform_height(
                self.content.data.len(),
                px(finite_to_f32(self.content.estimated_height)),
            );
            self.unseen = Rc::default();
            if self.content.follow_tail {
                self.scroll.set_follow_mode(FollowMode::Tail);
            }
        } else if geometry_changed {
            // GPUI owns remeasurement and anchor correction. Invalidate heights
            // without resetting identity, position or native follow state.
            self.scroll.remeasure_items(0..self.content.data.len());
        }
        if recreate || replaced {
            // Source replacement must retire a captured thumb gesture even
            // when the retained collection key itself survives.
            self.scroll_source_epoch = self.scroll_source_epoch.wrapping_add(1);
            self.last_follow_status = None;
        }
        if follow_changed && !recreate {
            self.scroll.set_follow_mode(if self.content.follow_tail {
                FollowMode::Tail
            } else {
                FollowMode::Normal
            });
            self.unseen.borrow_mut().clear();
        }
        self.install_metrics_handler();
        if changed {
            if reveal_changed || replaced || recreate {
                self.reveal_controlled_target();
            }
            cx.notify();
        }
        // Explicit commands have priority over keyed anchor preservation,
        // controlled reveal and follow synchronization in this same commit.
        self.apply_scroll_commands(cx);
    }

    fn apply_keyed_edits(
        &mut self,
        content: &VirtualCollectionNodeSpec,
        old_anchor: ListOffset,
        old_key: Option<&str>,
        was_following: bool,
        keys: (&BTreeSet<String>, &BTreeSet<String>),
    ) {
        let splices = keyed_splices(&self.content.data, &content.data);
        let has_insertions = splices.iter().any(|(_, count)| *count > 0);
        for (range, count) in splices {
            self.scroll.splice(range, count);
        }
        if has_insertions {
            // GPUI cannot hint just a splice. This preserves cached native
            // dimensions as hints and fills unknown dimensions with the estimate.
            self.scroll = self
                .scroll
                .clone()
                .with_uniform_item_height(px(finite_to_f32(content.estimated_height)));
        }
        if !was_following
            && let Some(key) = old_key
            && let Some(index) =
                (0..content.data.len()).find(|index| content.data.key(*index) == Some(key))
        {
            self.scroll.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: old_anchor.offset_in_item,
            });
        }
        let mut unseen = self.unseen.borrow_mut();
        unseen.retain(|key| keys.1.contains(key));
        if content.follow_tail && !was_following {
            unseen.extend(keys.1.difference(keys.0).cloned());
        }
    }

    fn restore_reader(&self, key: &str, offset: Pixels) {
        if let Some(index) =
            (0..self.content.data.len()).find(|index| self.content.data.key(*index) == Some(key))
        {
            self.scroll.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: offset,
            });
        }
    }

    fn apply_scroll_commands(&self, cx: &mut Context<Self>) {
        let handle = self.scroll_handle();
        for command in self
            .runtime
            .virtual_requests
            .take_scroll_commands(&self.content.id)
        {
            match command {
                crate::virtual_list::CollectionScrollCommand::Start => handle.scroll_to_start(cx),
                crate::virtual_list::CollectionScrollCommand::End => handle.scroll_to_end(cx),
                crate::virtual_list::CollectionScrollCommand::By(distance) => {
                    handle.scroll_by(px(crate::renderer::f64_to_f32(distance)), cx);
                }
            }
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
        } else if viewport.size.height <= px(0.0)
            && estimated_target_is_initially_visible(&self.content, index)
        {
            // Before GPUI's first measurement there are no item bounds. Keep
            // the natural top when the configured viewport already contains
            // the target instead of top-aligning (and hiding) its predecessors.
        } else if viewport.size.height <= px(0.0) && self.content.height.is_none() {
            // A fill-height viewport has no configured size to estimate with.
            self.pending_reveal = true;
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

    fn install_metrics_handler(&mut self) {
        let metrics = self.runtime.virtual_requests.clone();
        let id = self.content.id.clone();
        let scroll = self.scroll.clone();
        metrics.set_scroll_handle(id.clone(), self.scroll_handle());
        self.bounds_reader = Some(metrics.set_item_bounds_reader(id.clone(), move |index| {
            scroll
                .bounds_for_item(index)
                .map(|bounds| crate::GeometryBounds {
                    x: f64::from(bounds.origin.x),
                    y: f64::from(bounds.origin.y),
                    width: f64::from(bounds.size.width),
                    height: f64::from(bounds.size.height),
                })
        }));
        self.scroll.set_scroll_handler(move |event, _, _| {
            metrics.report_scroll(&id, event.visible_range.clone(), event.is_scrolled);
        });
    }

    fn scroll_handle(&self) -> crate::VirtualCollectionScrollHandle {
        crate::VirtualCollectionScrollHandle {
            state: self.scroll.clone(),
            unseen: Rc::clone(&self.unseen),
            follow_tail: self.content.follow_tail,
            notify: self.notify,
        }
    }

    fn seed_viewport_hints(&mut self, cx: &mut Context<Self>) {
        let width = self.scroll.viewport_bounds().size.width;
        if width > px(0.0) && self.hinted_width != Some(width) {
            // List::prepaint clears all hints on its first/changed-width pass.
            // Seed once after that pass, never on stable frames or realization.
            self.hinted_width = Some(width);
            self.scroll = self
                .scroll
                .clone()
                .with_uniform_item_height(px(finite_to_f32(self.content.estimated_height)));
            cx.notify();
        }
    }

    fn publish_follow_status(&mut self, window: &mut Window, cx: &mut App) {
        let status = self.scroll_handle().status();
        if status.following {
            self.unseen.borrow_mut().clear();
        }
        self.runtime
            .virtual_requests
            .report_follow(&self.content.id, status);
        if self.last_follow_status == Some(status) {
            return;
        }
        self.last_follow_status = Some(status);
        if let Some(callback) = self.content.on_follow_change.clone() {
            let dispatcher = self.runtime.dispatcher.clone();
            let registry = self.runtime.virtual_requests.clone();
            let id = self.content.id.clone();
            let lease = Rc::clone(&self.unseen);
            window.defer(cx, move |window, cx| {
                if !registry
                    .scroll_handle(&id)
                    .is_some_and(|handle| Rc::ptr_eq(&handle.unseen, &lease))
                {
                    return;
                }
                dispatcher.dispatch(
                    *callback,
                    "follow_change",
                    status.to_ui_value(),
                    None,
                    window,
                    cx,
                );
            });
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn estimated_target_is_initially_visible(
    content: &VirtualCollectionNodeSpec,
    index: usize,
) -> bool {
    content
        .height
        .is_some_and(|height| (index.saturating_add(1) as f64) * content.estimated_height <= height)
}

impl Render for VirtualListView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_reveal && self.scroll.viewport_bounds().size.height > px(0.0) {
            self.pending_reveal = false;
            self.reveal_controlled_target();
        }
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
        let mut runtime = self.runtime.clone();
        runtime.wheel_parents.insert(
            0,
            crate::scrollbar::ScrollWheelSource::new(self.scroll.clone(), (false, true), false),
        );
        let frame_indices = Rc::new(RefCell::new(BTreeSet::new()));
        self.frame_indices = Rc::clone(&frame_indices);
        let sticky = sticky_header_frame(&self.scroll, &content, &runtime, viewport);
        if let Some(sticky) = sticky {
            frame_indices.borrow_mut().insert(sticky.index);
        }
        let fixed_height = content.height.map(finite_to_f32);
        let list_content = content.clone();
        let list_runtime = runtime.clone();
        let list_frame_indices = Rc::clone(&frame_indices);
        let list = list(self.scroll.clone(), move |index, _window, _cx| {
            list_frame_indices.borrow_mut().insert(index);
            render_virtual_item(&list_content, &list_runtime, sticky, index)
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
        // Clipped to the list: a sticky header the next one pushes out slides
        // above the top edge and must not paint over (or take clicks from) what
        // sits above the list, such as a table's column header.
        div()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .id(SharedString::from(format!(
                "virtual-list-root-{}",
                self.content.id.key
            )))
            .debug_selector(move || root_selector.clone())
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
            .child(self.scrollbars.element(
                SharedString::from(format!("scrollbar-{}", self.scroll_source_epoch)),
                self.scroll.clone(),
                self.scrollbar_spec,
            ))
    }
}

fn render_virtual_item(
    content: &VirtualCollectionNodeSpec,
    runtime: &NodeSlotRuntime,
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
    let natural_header_has_crossed_top = scroll_top.item_ix > index
        || state
            .bounds_for_item(index)
            .is_some_and(|bounds| bounds.top() < viewport.top());
    if !natural_header_has_crossed_top {
        return None;
    }
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

fn collection_keys(data: &crate::VirtualCollectionData) -> BTreeSet<String> {
    (0..data.len())
        .filter_map(|index| data.key(index).map(ToOwned::to_owned))
        .collect()
}

fn same_key_order(
    current: &crate::VirtualCollectionData,
    next: &crate::VirtualCollectionData,
) -> bool {
    current.len() == next.len()
        && (0..current.len()).all(|index| current.key(index) == next.key(index))
}

/// Preserve a maximal ordered subsequence of native rows. Only gaps are
/// spliced, from the end, retaining GPUI's measured items and anchor logic for
/// ordinary insert/delete/append operations. Moved rows are remeasured.
fn keyed_splices(
    current: &crate::VirtualCollectionData,
    next: &crate::VirtualCollectionData,
) -> Vec<(std::ops::Range<usize>, usize)> {
    // Common growth/edit paths do not need an index map or LIS allocation.
    let prefix = (0..current.len().min(next.len()))
        .take_while(|index| current.key(*index) == next.key(*index))
        .count();
    let suffix = (0..(current.len() - prefix).min(next.len() - prefix))
        .take_while(|index| {
            current.key(current.len() - index - 1) == next.key(next.len() - index - 1)
        })
        .count();
    let old_end = current.len() - suffix;
    let new_end = next.len() - suffix;
    if prefix == old_end || prefix == new_end {
        return if old_end == new_end {
            Vec::new()
        } else {
            vec![(prefix..old_end, new_end - prefix)]
        };
    }
    let old = (0..current.len())
        .filter_map(|index| current.key(index).map(|key| (key, index)))
        .collect::<BTreeMap<_, _>>();
    let pairs = (0..next.len())
        .filter_map(|index| old.get(next.key(index)?).map(|old| (*old, index)))
        .collect::<Vec<_>>();
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![None; pairs.len()];
    for (index, (old, _)) in pairs.iter().enumerate() {
        let slot = tails.partition_point(|tail| pairs[*tail].0 < *old);
        if slot > 0 {
            previous[index] = Some(tails[slot - 1]);
        }
        if slot == tails.len() {
            tails.push(index);
        } else {
            tails[slot] = index;
        }
    }
    let mut kept = Vec::with_capacity(tails.len() + 1);
    let mut cursor = tails.last().copied();
    while let Some(index) = cursor {
        kept.push(pairs[index]);
        cursor = previous[index];
    }
    kept.reverse();
    kept.push((current.len(), next.len()));
    let mut from = (0, 0);
    let mut splices = Vec::new();
    for to in kept {
        if from != to {
            splices.push((from.0..to.0, to.1 - from.1));
        }
        from = (to.0.saturating_add(1), to.1.saturating_add(1));
    }
    splices.reverse();
    splices
}

fn list_state(spec: &VirtualCollectionNodeSpec) -> ListState {
    let state = ListState::new(
        spec.data.len(),
        if spec.bottom_align {
            ListAlignment::Bottom
        } else {
            ListAlignment::Top
        },
        // A one-pixel native measurement sentinel prevents zero-height unknown
        // rows beyond the viewport from looking like the true tail when the
        // caller requests zero overdraw. It does not create follow state.
        px(finite_to_f32(spec.overdraw_pixels)).max(px(1.0)),
    )
    .with_uniform_item_height(px(finite_to_f32(spec.estimated_height)));
    if spec.follow_tail {
        state.set_follow_mode(FollowMode::Tail);
    }
    state
}

#[allow(clippy::cast_possible_truncation)]
fn finite_to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX));
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    fn indices(range: std::ops::Range<usize>) -> BTreeSet<usize> {
        range.collect()
    }

    fn data(keys: impl IntoIterator<Item = String>) -> crate::VirtualCollectionData {
        keys.into_iter()
            .map(|key| {
                crate::UiValue::Map(BTreeMap::from([(
                    "key".to_owned(),
                    crate::UiValue::String(key),
                )]))
            })
            .collect()
    }

    #[test]
    fn keyed_diff_keeps_native_rows_for_insert_delete_and_large_append() {
        let current = data((0..30_000).map(|index| format!("row-{index}")));
        let appended = data((0..30_001).map(|index| format!("row-{index}")));
        assert_eq!(keyed_splices(&current, &current), Vec::new());
        assert_eq!(
            keyed_splices(&current, &appended),
            vec![(30_000..30_000, 1)]
        );
        assert_eq!(
            keyed_splices(&appended, &current),
            vec![(30_000..30_001, 0)]
        );
        let mut keys = (0..30_000)
            .map(|index| format!("row-{index}"))
            .collect::<Vec<_>>();
        keys.insert(12_000, "inserted".to_owned());
        assert_eq!(
            keyed_splices(&current, &data(keys)),
            vec![(12_000..12_000, 1)]
        );
    }

    #[test]
    fn keyed_diff_reorders_by_splicing_only_gaps_and_never_resets_surviving_rows() {
        let original = ["a", "b", "c", "d", "e"];
        let reordered = ["c", "a", "inserted", "b", "e"];
        let current = data(original.map(ToOwned::to_owned));
        let next = data(reordered.map(ToOwned::to_owned));
        let splices = keyed_splices(&current, &next);
        let mut rows = original.map(ToOwned::to_owned).to_vec();
        // Inserted payload comes from the corresponding final index, computed
        // from the edit's running length delta when applying in ascending order.
        let mut shift: isize = 0;
        for (range, count) in splices.iter().rev() {
            let target = range.start.checked_add_signed(shift).unwrap();
            shift += isize::try_from(*count).unwrap() - isize::try_from(range.len()).unwrap();
            rows.splice(
                target..target + range.len(),
                reordered[target..target + count]
                    .iter()
                    .map(|key| (*key).to_owned()),
            );
        }
        assert_eq!(rows, reordered);
        assert!(
            splices
                .iter()
                .all(|(range, _)| range.len() < original.len())
        );
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

    #[test]
    fn initial_reveal_keeps_estimated_visible_predecessors() {
        let mut content = crate::VirtualCollectionNodeSpec {
            id: crate::VirtualCollectionId {
                component: crate::ComponentInstancePath::root("Command", "palette"),
                key: "results".to_owned(),
            },
            label: "Results".to_owned(),
            data: crate::VirtualCollectionData::Values(vec![crate::UiValue::Null; 20]),
            realized: std::collections::BTreeMap::new(),
            estimated_height: 32.0,
            height: Some(384.0),
            overdraw_pixels: 64.0,
            bottom_align: false,
            follow_tail: false,
            on_follow_change: None,
            reveal_key: None,
            sticky_headers: std::sync::Arc::new(BTreeSet::new()),
            inherited_motion_group: None,
            inherited_motion_scope: None,
        };
        assert!(estimated_target_is_initially_visible(&content, 1));
        assert!(estimated_target_is_initially_visible(&content, 11));
        assert!(!estimated_target_is_initially_visible(&content, 12));
        content.height = None;
        assert!(!estimated_target_is_initially_visible(&content, 1));
    }
}
