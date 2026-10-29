//! Cache islands: cold subtrees of a view's committed tree embedded as cached
//! GPUI views, so a frame that changed nothing in them replays what they drew
//! instead of building, laying out and painting them again.
//!
//! The spine (the view's own render) walks the tree as before. At a planned
//! root ([`plan`]) it embeds the island's entity instead of rendering the
//! subtree: cached when everything the subtree's render reads is unchanged
//! since it last rendered ([`IslandCapture`]), and uncached otherwise, which
//! renders the subtree in place exactly as without islands and measures it.
//! Validity is pulled: the spine compares the capture on every frame, so
//! nothing relies on GPUI's notify tracking.
//!
//! GPUI lays a cached view out at the style its embedder gives it, without
//! measuring its contents (gpui-pre 0.3.9, `view.rs`). An island's root
//! therefore has a height that is its content height at a width that does not
//! depend on its content (checked where the spine reaches it), and the cached
//! view is pinned to the height measured in place. A cached render whose root
//! lays out to another height demotes the island until its capture changes.
//!
//! GPUI replays only its own frame data on a hit. The one runtime side effect
//! of a cold subtree is the geometry registry's presented mark, which the host
//! element records on a render and replays on a hit.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::{
    AnyElement, AnyView, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Size, Window,
};

use crate::{NodeId, RetainedUiTree, UiNode, UiNodeKindTag};

/// The smallest subtree worth a cached view: below it, building the subtree
/// costs less than the view's bookkeeping.
pub(crate) const MIN_ISLAND_NODES: u32 = 16;

/// Whether a view embeds cache islands, from its config and the
/// `GPUI_RHAI_RENDER_ISLANDS` environment variable (`off`, `on`, `verify`,
/// `aggressive`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IslandPolicy {
    pub enabled: bool,
    /// Checks every cached render against its pinned height and panics on a
    /// difference, instead of demoting the island.
    pub verify: bool,
    pub min_nodes: u32,
    /// Takes the width a Host lays a view out at as independent of the
    /// view's content, as a window's is, so that every view can hold islands.
    pub anchor_hosted_root: bool,
}

impl IslandPolicy {
    pub(crate) fn resolve(config: Option<bool>) -> Self {
        let variable = std::env::var("GPUI_RHAI_RENDER_ISLANDS").ok();
        let mode = variable.as_deref().unwrap_or("on");
        let enabled = config.unwrap_or(true) && mode != "off";
        Self {
            enabled,
            verify: matches!(mode, "verify" | "aggressive"),
            min_nodes: if mode == "aggressive" {
                1
            } else {
                MIN_ISLAND_NODES
            },
            anchor_hosted_root: mode == "aggressive",
        }
    }
}

/// The roots of a committed tree's islands: maximal cold subtrees of at
/// least `min_nodes` nodes whose parent lays them out as a Box child. The walk
/// visits only the nodes above them, through Box and Fragment children of
/// subtrees that hold a hot node; it stops at scroll containers, Table layout
/// and hidden pages, and never makes the view root an island. A root of
/// `kept` stays one down to half the size, so a subtree near the limit does
/// not move in and out of its island from one commit to the next.
pub(crate) fn plan(
    tree: &RetainedUiTree,
    min_nodes: u32,
    kept: impl Fn(NodeId) -> bool,
) -> BTreeSet<NodeId> {
    let mut roots = BTreeSet::new();
    if let Some(root) = tree.root_id() {
        visit(tree, root, true, min_nodes, &kept, &mut roots);
    }
    roots
}

fn visit(
    tree: &RetainedUiTree,
    id: NodeId,
    parent_is_box: bool,
    min_nodes: u32,
    kept: &impl Fn(NodeId) -> bool,
    roots: &mut BTreeSet<NodeId>,
) {
    let Some(node) = tree.node(id) else {
        return;
    };
    let is_box = match node.kind() {
        UiNodeKindTag::Box => true,
        // A Fragment's children are its parent's.
        UiNodeKindTag::Fragment => parent_is_box,
        _ => return,
    };
    if node.summary().opaque
        || node.attributes().get(crate::retained_page::PAGE_VISIBLE)
            == Some(&crate::UiValue::Bool(false))
    {
        return;
    }
    for link in node.children().filter(|link| link.group() == "children") {
        let Some(child) = tree.node(link.node()) else {
            continue;
        };
        if child.attributes().get(crate::retained_page::PAGE_VISIBLE)
            == Some(&crate::UiValue::Bool(false))
        {
            // A hidden page draws nothing.
            continue;
        }
        let summary = child.summary();
        let threshold = if kept(link.node()) {
            min_nodes.div_ceil(2)
        } else {
            min_nodes
        };
        if summary.hot == 0
            && is_box
            && child.kind() != UiNodeKindTag::Fragment
            && summary.nodes >= threshold
        {
            roots.insert(link.node());
        } else if summary.hot > 0 {
            visit(tree, link.node(), is_box, min_nodes, kept, roots);
        }
    }
}

/// What the frame's render of every island depends on beyond its subtree and
/// the context it inherits.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FrameFingerprint {
    pub generation: crate::ScriptGeneration,
    pub theme: crate::ThemeVariant,
    pub locale: String,
    pub number: Option<crate::NumberMetadata>,
    pub motion_preference: crate::MotionPreference,
    pub motion_quality: crate::MotionQuality,
    pub primitives: usize,
    pub window_drag: bool,
    pub rem_bits: u32,
    pub scale_bits: u32,
}

/// What an island root inherits from its ancestors.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct IslandInherited {
    pub environment: crate::Environment,
    pub inherited_disabled: bool,
    /// Whether the focus owner above has focus, when a node of the island
    /// styles `group_focus` by it. (Whether focus is within a node follows
    /// from the focus path.)
    pub focus_owner: bool,
    pub ambient_text_color: Option<crate::Rgba8>,
    pub stretch_parent: bool,
    pub row_parent: bool,
    pub direction: crate::TextDirection,
    /// The product of the ancestors' opacity, which GPUI does not key a
    /// cached view by.
    pub opacity_bits: u32,
    pub width_anchor: u64,
    /// The nearest ancestor with a `hover` style, when a node of the island
    /// paints `group_hover` by it.
    pub hover_group: Option<NodeId>,
    /// The room a row leaves the root, when the root grows into it.
    pub room: Option<u64>,
}

/// Everything an island's render reads: when two frames capture the same,
/// the second may replay the first.
#[derive(Clone, Debug)]
pub(crate) struct IslandCapture {
    /// Held, so a pointer comparison never matches a recycled allocation.
    pub node: UiNode,
    pub root: NodeId,
    pub path: Rc<str>,
    pub inherited: IslandInherited,
    pub fingerprint: Rc<FrameFingerprint>,
    pub revision: u64,
    /// The focus path from the island root up, when focus is inside it.
    pub focus: Vec<NodeId>,
    /// The sampled motion values of the island's nodes, which an ancestor's
    /// timeline can animate.
    pub motion: Rc<[(crate::MotionKey, u64)]>,
}

impl IslandCapture {
    /// The same render inputs, the committed revision aside. A node equal to
    /// the one rendered, as a render of the whole view yields, renders the
    /// same.
    fn same_inputs(&self, other: &Self) -> bool {
        self.node == other.node
            && self.root == other.root
            && self.path == other.path
            && self.inherited == other.inherited
            && (Rc::ptr_eq(&self.fingerprint, &other.fingerprint)
                || self.fingerprint == other.fingerprint)
            && self.focus == other.focus
            && self.motion == other.motion
    }
}

/// An island's state across frames, shared by the spine, the island's view
/// and its host element.
#[derive(Default)]
pub(crate) struct IslandSlot {
    /// What the island renders this frame, set by the spine.
    pub(crate) capture: Option<IslandCapture>,
    /// The capture of its last render and the retained ids it covered.
    rendered: Option<(IslandCapture, Rc<[NodeId]>)>,
    /// The frame the island last rendered in.
    rendered_frame: u64,
    /// The root's size the last time it was laid out in place.
    measured: Option<Size<Pixels>>,
    /// The cached view's bounds while it renders: the width its root takes.
    pub(crate) render_bounds: Option<Bounds<Pixels>>,
    /// Whether this frame embeds the island cached.
    cached: bool,
    /// A cached render's root laid out to another height than the pin: the
    /// island stays in place until its capture changes.
    demoted: bool,
    /// The nodes the last render presented, sorted.
    presented: Rc<[NodeId]>,
}

impl IslandSlot {
    /// Whether `capture` renders what the island last rendered: the same
    /// inputs over the same retained ids. One reconcile of the root's own
    /// node keeps its subtree as it was (the revision after another is
    /// always its reconcile); across more, `ids` checks.
    fn renders_as_before(
        &self,
        capture: &IslandCapture,
        tree: &RetainedUiTree,
        verify: bool,
    ) -> bool {
        let Some((rendered, ids)) = &self.rendered else {
            return false;
        };
        if !rendered.same_inputs(capture) {
            return false;
        }
        let kept = rendered.revision == capture.revision
            || (rendered.revision.wrapping_add(1) == capture.revision
                && rendered.node.ptr_eq(&capture.node));
        if kept && !verify {
            return true;
        }
        let same_ids = **ids == *subtree_ids(tree, capture.root);
        assert!(
            !kept || same_ids,
            "island {:?} changed its ids in one reconcile of the same node",
            capture.root
        );
        same_ids
    }

    /// How this frame embeds the island: cached at the measured height when
    /// its capture renders as before, in place otherwise.
    pub(crate) fn decide(
        &mut self,
        capture: IslandCapture,
        eligible: bool,
        tree: &RetainedUiTree,
        verify: bool,
    ) -> Option<Pixels> {
        let same = eligible && self.renders_as_before(&capture, tree, verify);
        if !same {
            self.demoted = false;
        }
        if same && let Some((rendered, _)) = &mut self.rendered {
            // Keep comparing against the newest revision and node the subtree
            // held, which the next frames' captures share.
            rendered.revision = capture.revision;
            rendered.node = capture.node.clone();
        }
        self.capture = Some(capture);
        self.cached = same && !self.demoted && self.measured.is_some();
        self.cached
            .then(|| self.measured.map(|size| size.height))
            .flatten()
    }

    /// The island rendered `capture` this frame.
    pub(crate) fn rendered(&mut self, frame: u64, tree: &RetainedUiTree) {
        if let Some(capture) = &self.capture {
            self.rendered = Some((capture.clone(), subtree_ids(tree, capture.root)));
        }
        self.rendered_frame = frame;
    }
}

/// The ids of `root`'s retained subtree, in pre-order.
fn subtree_ids(tree: &RetainedUiTree, root: NodeId) -> Rc<[NodeId]> {
    let mut ids = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        ids.push(id);
        if let Some(node) = tree.node(id) {
            let children = node.children().map(crate::RetainedChildLink::node);
            let start = stack.len();
            stack.extend(children);
            stack[start..].reverse();
        }
    }
    ids.into()
}

/// Island embeds of one frame, by outcome.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct IslandCounters {
    /// Replayed from the last frame.
    pub hits: u64,
    /// Embedded cached but rendered: the first cached frame after a change,
    /// or GPUI invalidated the view.
    pub misses: u64,
    /// Rendered in place: changed, measured again or not eligible.
    pub uncached: u64,
    /// A cached render whose root height differed from the pin.
    pub drift: u64,
    /// Time in island renders, which run after the view's own.
    pub render_time: std::time::Duration,
}

/// One planned island: its view and its slot.
#[derive(Clone)]
pub(crate) struct IslandEntry {
    pub view: AnyView,
    pub slot: Rc<RefCell<IslandSlot>>,
}

/// The islands of the frame being rendered.
pub(crate) struct IslandFrame {
    pub frame: u64,
    pub policy: IslandPolicy,
    pub fingerprint: Rc<FrameFingerprint>,
    pub revision: u64,
    pub entries: BTreeMap<NodeId, IslandEntry>,
    pub counters: Rc<RefCell<IslandCounters>>,
    /// The width anchor of the view root's parent: the viewport's width for a
    /// view that fills its window, `None` where a Host lays the view out.
    pub root_anchor: Option<u64>,
    /// The sampled motion values of each island's nodes.
    pub motions: BTreeMap<NodeId, Rc<[(crate::MotionKey, u64)]>>,
    /// A sampled value in the view's node namespace names no node, so it may
    /// reach any island.
    pub motion_unknown: bool,
}

impl IslandFrame {
    /// Assigns the view's sampled motion values to the islands holding the
    /// nodes they animate. `domain` is the view's motion path root.
    pub(crate) fn assign_motions(
        &mut self,
        tree: &RetainedUiTree,
        values: &BTreeMap<crate::MotionKey, f64>,
        domain: &str,
    ) {
        let prefix = format!("{domain}/node:");
        let mut motions = BTreeMap::<NodeId, Vec<(crate::MotionKey, u64)>>::new();
        for (key, value) in values {
            let Some(rest) = key
                .component
                .single_root_key("UiNode")
                .and_then(|path| path.strip_prefix(&prefix))
            else {
                continue;
            };
            let Some(id) = rest.split('/').next().and_then(|id| id.parse::<u64>().ok()) else {
                self.motion_unknown = true;
                continue;
            };
            let mut cursor = Some(NodeId::from_raw(id));
            while let Some(id) = cursor {
                if self.entries.contains_key(&id) {
                    motions
                        .entry(id)
                        .or_default()
                        .push((key.clone(), value.to_bits()));
                    break;
                }
                cursor = tree.node(id).and_then(crate::RetainedNode::parent);
            }
        }
        self.motions = motions
            .into_iter()
            .map(|(id, values)| (id, values.into()))
            .collect();
    }
}

/// Embeds an island's view and keeps its slot current: it records the
/// root's size when the island renders in place, sets the width a cached
/// render lays its root out at, records the nodes a render presents and
/// replays them on a hit, and checks a cached render against its pin.
pub(crate) struct IslandHostElement {
    pub child: AnyElement,
    pub slot: Rc<RefCell<IslandSlot>>,
    pub frame: u64,
    pub geometry: crate::GeometryRegistry,
    pub counters: Rc<RefCell<IslandCounters>>,
    pub verify: bool,
}

impl IntoElement for IslandHostElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for IslandHostElement {
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
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let cached = {
            let mut slot = self.slot.borrow_mut();
            slot.render_bounds = Some(bounds);
            slot.cached
        };
        self.geometry.begin_island_record();
        self.child.prepaint(window, cx);
        let recorded = self.geometry.end_island_record();
        let mut slot = self.slot.borrow_mut();
        slot.render_bounds = None;
        let rendered = slot.rendered_frame == self.frame;
        let mut counters = self.counters.borrow_mut();
        let root = slot.capture.as_ref().map(|capture| capture.root);
        // A root that drew nothing, a hidden page, has no layout of its own
        // this frame.
        let root_presented = root.is_some_and(|root| recorded.binary_search(&root).is_ok());
        if rendered {
            slot.presented = recorded;
        } else {
            self.geometry.replay_presented(Rc::clone(&slot.presented));
        }
        if !cached {
            counters.uncached += 1;
            slot.measured = Some(bounds.size);
            return;
        }
        if !rendered {
            counters.hits += 1;
            return;
        }
        counters.misses += 1;
        let height = root
            .filter(|_| root_presented)
            .and_then(|root| self.geometry.get(root))
            .map(|geometry| geometry.layout.height);
        let pin = f64::from(bounds.size.height);
        if height.is_some_and(|height| (height - pin).abs() > f64::EPSILON) {
            assert!(
                !self.verify,
                "island {root:?} at {:?} laid out to {height:?} in a cached view pinned to {pin}",
                slot.capture.as_ref().map(|capture| &capture.path)
            );
            counters.drift += 1;
            slot.demoted = true;
            // The next frame renders the island in place again.
            window.request_animation_frame();
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
