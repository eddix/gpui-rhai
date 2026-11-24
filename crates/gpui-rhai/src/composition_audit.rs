//! Composition audit: checks a mounted view's committed geometry and resolved
//! styles against the rules an application profile enables.
//!
//! The audit is design-language neutral: it compares relationships (heights
//! in a row, nested gaps, painted contrast) rather than fixed values. With no
//! rules enabled it reports nothing.

mod background;

use background::ContrastSurface;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Align, ColorResolver, DisplayMode, Environment, FlexDirection, FlexWrapMode, GeometryBounds,
    GeometryRegistry, InteractionState, Justify, Length, NodeId, OverflowMode, PositionMode,
    RetainedUiTree, Rgba8, StyleProperties, Symbol, UiNode, UiNodeKind, UiValue,
};

/// One composition rule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AuditRule {
    /// Peer controls of the same family in one row with different heights.
    RowHeightMismatch,
    /// Stacked text starts that nearly, but not exactly, align.
    TextEdgeMisaligned,
    /// An unheaded group's inner gap exceeds its enclosing content separation.
    SpacingNotNested,
    /// More than one solid action in one action group.
    MultipleSolidActions,
    /// More than one font size among the text of one row.
    MixedTypeInRow,
    /// Text below its contrast minimum against the painted background.
    LowContrastText,
    /// A font family that does not resolve on this platform.
    UnresolvedFont,
}

impl AuditRule {
    pub const ALL: [Self; 7] = [
        Self::RowHeightMismatch,
        Self::TextEdgeMisaligned,
        Self::SpacingNotNested,
        Self::MultipleSolidActions,
        Self::MixedTypeInRow,
        Self::LowContrastText,
        Self::UnresolvedFont,
    ];

    /// The profile identifier, such as `row-height-mismatch`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::RowHeightMismatch => "row-height-mismatch",
            Self::TextEdgeMisaligned => "text-edge-misaligned",
            Self::SpacingNotNested => "spacing-not-nested",
            Self::MultipleSolidActions => "multiple-solid-actions",
            Self::MixedTypeInRow => "mixed-type-in-row",
            Self::LowContrastText => "low-contrast-text",
            Self::UnresolvedFont => "unresolved-font",
        }
    }

    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.id() == id)
    }
}

/// The rules a profile enables. The default enables none.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditRules {
    enabled: BTreeSet<AuditRule>,
}

impl AuditRules {
    /// Every rule.
    #[must_use]
    pub fn all() -> Self {
        Self {
            enabled: AuditRule::ALL.into_iter().collect(),
        }
    }

    /// Enable rules by profile identifier.
    ///
    /// # Errors
    ///
    /// Returns the first unknown identifier.
    pub fn from_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<Self, String> {
        let enabled = ids
            .into_iter()
            .map(|id| AuditRule::parse(id).ok_or_else(|| id.to_owned()))
            .collect::<Result<_, _>>()?;
        Ok(Self { enabled })
    }

    #[must_use]
    pub fn contains(&self, rule: AuditRule) -> bool {
        self.enabled.contains(&rule)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
}

/// One audit finding.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditFinding {
    pub rule: AuditRule,
    /// The retained node the finding is anchored to.
    pub node: Option<NodeId>,
    /// A readable path of keys and indices from the root.
    pub path: String,
    pub message: String,
    /// Numeric evidence for a text contrast finding, without copying its text.
    pub contrast: Option<AuditContrastEvidence>,
}

/// Evidence from the audit's declarative paint model, not a GPU pixel capture.
///
/// The ratio keeps unquantized sRGB compositing. A non-solid backing has no
/// single reported RGB value; a conservative bound is never an exact witness.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditContrastEvidence {
    pub foreground: Rgba8,
    /// Declared opacity product; with isolated paint this is not a separate
    /// alpha applied to the final background a second time.
    pub element_opacity: f64,
    pub background_srgb: Option<[f64; 3]>,
    pub ratio: f64,
    pub minimum: f64,
    pub conservative: bool,
}

impl std::fmt::Display for AuditFinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} at {}: {}",
            self.rule.id(),
            self.path,
            self.message
        )
    }
}

const CONTROL_ROLES: &[&str] = &[
    "button",
    "tab",
    "text_field",
    "textbox",
    "searchbox",
    "combobox",
    "spinbutton",
    "checkbox",
    "radio",
    "switch",
    "slider",
];
const MARKER_ROLES: &[&str] = &["status"];
const NEAR_MISS_MAX: f64 = 8.0;
const EPSILON: f64 = 0.5;

/// Inputs shared by every node of one audit.
pub(crate) struct AuditInputs<'a, C: ColorResolver> {
    pub tree: &'a RetainedUiTree,
    pub geometry: &'a GeometryRegistry,
    pub theme: &'a C,
    pub rules: &'a AuditRules,
    /// Font families known to the text system, when the caller can provide
    /// them; `None` skips [`AuditRule::UnresolvedFont`].
    pub available_fonts: Option<&'a BTreeSet<String>>,
    /// Text starts are right edges in right-to-left views.
    pub direction: crate::TextDirection,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayoutOwner {
    Composition,
    Control,
    TreeItemChrome,
}

#[derive(Clone, Copy)]
struct Scope {
    environment: Environment,
    disabled: bool,
    text_color: Option<Rgba8>,
    background: ContrastSurface,
    /// Native GPUI applies this product to individual paint primitives.
    /// Isolation resets it for the local render surface.
    opacity: f64,
    /// Cumulative opacity before reset, for visibility and diagnostic context.
    total_opacity: f64,
    font_size: Option<f64>,
    font_weight: u16,
    type_voice: TypeVoice,
    /// Gaps of the nearest enclosing physical rows and columns with peers.
    /// Proximity only competes along one axis.
    enclosing_row_gap: Option<f64>,
    enclosing_column_gap: Option<f64>,
    layout_owner: LayoutOwner,
}

/// What the audit learned about one rendered node, for its parent's checks.
#[derive(Clone, Debug, Default)]
struct NodeSummary {
    role: Option<String>,
    bounds: Option<GeometryBounds>,
    /// Geometry of the control this subtree stands for: a control, or a wrapper
    /// that holds just one.
    control_geometry: Option<NodeId>,
    /// Only comparable control families share a height contract. A preset may
    /// intentionally use a 40px button beside a 36px input, for example.
    control_family: Option<String>,
    /// Whether the subtree holds a control anywhere; such a child carries the
    /// control type scale and stays out of a row's type mix.
    contains_control: bool,
    /// First comparable label's size and actual text node, not its wrapper's
    /// alignment edge. A tall card can contain text on another visual row.
    text_sample: Option<TextSample>,
    /// The first comparable start exported by this subtree, including
    /// whether it is an ink start or an independently laid-out boundary.
    text_edge: Option<StackEdge>,
    /// Actual text geometry before a surface, icon or control substitutes
    /// its edge. Tree row chrome exports this anchor to text-start checks.
    text_ink_edge: Option<StackEdge>,
    /// Whether the first control in this subtree is a solid action.
    solid_action: bool,
}

#[derive(Clone, Copy, Debug)]
struct TextSample {
    size: f64,
    geometry: Option<NodeId>,
    voice: TypeVoice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypeVoice {
    Inline,
    /// Primary display text is compared with its peers, not its units.
    Display,
    /// Timestamp, qualifier or ledger amount, distinct from its item label.
    Metadata,
    /// Initials are glyph marks, not the name written beside them.
    Monogram,
    /// Key legends and accessory facet labels sit beside body text.
    Annotation,
}

#[derive(Clone, Copy, Debug)]
struct StackEdge {
    node: NodeId,
    kind: StackEdgeKind,
    /// Local insets to the text's content box, not the painted surface edge.
    inline_insets: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StackEdgeKind {
    Text,
    Label,
    ColumnHeader,
    Surface,
    Centered,
    Control(&'static str),
    Marker,
}

/// Audit a committed tree.
pub(crate) fn audit<C: ColorResolver>(inputs: &AuditInputs<'_, C>) -> Vec<AuditFinding> {
    let mut findings = Vec::new();
    if inputs.rules.is_empty() {
        return findings;
    }
    let (Some(root), Some(root_id)) = (inputs.tree.root(), root_node_id(inputs.tree)) else {
        return findings;
    };
    let background = inputs
        .theme
        .resolve_token("surface")
        .unwrap_or(Rgba8::from_rgba_hex(0xffff_ffff));
    let scope = Scope {
        environment: Environment::EMPTY,
        disabled: false,
        text_color: inputs.theme.resolve_token("text_primary"),
        background: ContrastSurface::solid(background),
        opacity: 1.0,
        total_opacity: 1.0,
        font_size: None,
        font_weight: 400,
        type_voice: TypeVoice::Inline,
        enclosing_row_gap: None,
        enclosing_column_gap: None,
        layout_owner: LayoutOwner::Composition,
    };
    let solid = solid_colors(inputs.theme);
    let mut reported_fonts = BTreeSet::new();
    let mut walker = Walker {
        inputs,
        solid,
        allowed: BTreeSet::new(),
        findings: &mut findings,
        reported_fonts: &mut reported_fonts,
        paint_path: Vec::new(),
        paint_path_base: None,
    };
    walker.visit(root, Some(root_id), scope, "root");
    findings
}

/// Controls a reader cannot reach or tell apart, which the reading scale can
/// produce where literal widths meet scaled text: a control cut off by an
/// ancestor that clips without scrolling, and controls laid over one another.
/// A scroll container's own clip is no cut (scrolling reaches what it hides),
/// though a clip inside its content is; a control wholly outside the clip is
/// hidden on purpose (a carousel's other slides). Controls inside another
/// control or a hidden page are left alone, and overlay content, which floats
/// above the page, is checked for cuts only. Not yet an [`AuditRule`], so it
/// is not part of a profile.
pub(crate) fn unreachable_controls(
    tree: &RetainedUiTree,
    geometry: &GeometryRegistry,
) -> Vec<String> {
    let mut reach = Reach {
        tree,
        geometry,
        controls: Vec::new(),
        findings: Vec::new(),
    };
    if let (Some(root), Some(root_id)) = (tree.root(), root_node_id(tree)) {
        reach.visit(root, Some(root_id), Clips::default(), Placement::InFlow);
    }
    let Reach {
        controls,
        mut findings,
        ..
    } = reach;
    for (index, (name, a)) in controls.iter().enumerate() {
        for (other, b) in &controls[index + 1..] {
            let width = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
            let height = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
            if width > REACH_SLACK && height > REACH_SLACK {
                findings.push(format!(
                    "controls {name} and {other} overlap by {width:.1}x{height:.1}px"
                ));
            }
        }
    }
    findings
}

/// Pixels a control may pass its clip or overlap another by: borders that
/// collapse into a neighbour's.
const REACH_SLACK: f64 = 1.0;

struct Reach<'a> {
    tree: &'a RetainedUiTree,
    geometry: &'a GeometryRegistry,
    controls: Vec<(String, GeometryBounds)>,
    findings: Vec<String>,
}

/// The clips above a node for [`unreachable_controls`], in window coordinates.
#[derive(Clone, Copy, Default)]
struct Clips {
    /// Clips that do not scroll, since the nearest scroll container: what
    /// they hide cannot be reached.
    cut: Option<GeometryBounds>,
    /// Every clip, scroll viewports included: what can be seen.
    view: Option<GeometryBounds>,
}

impl Clips {
    /// `bounds` within `clip`, empty where they do not meet.
    fn within(clip: Option<GeometryBounds>, bounds: GeometryBounds) -> GeometryBounds {
        clip.map_or(Some(bounds), |clip| intersection(clip, bounds))
            .unwrap_or(GeometryBounds {
                x: bounds.x,
                y: bounds.y,
                width: 0.0,
                height: 0.0,
            })
    }
}

/// Where a node sits for [`unreachable_controls`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    InFlow,
    /// Overlay or layer content: checked for cuts, not for overlap with the
    /// page it floats above.
    Floating,
    /// Inside a control or a hidden page: not checked.
    Exempt,
}

impl Reach<'_> {
    fn visit(&mut self, node: &UiNode, id: Option<NodeId>, clips: Clips, placement: Placement) {
        let style = node.style().resolve(&InteractionState::default());
        // A node the last frame did not draw (a virtual row released from the
        // window) may keep the geometry of an earlier frame.
        let bounds = id
            .filter(|id| self.geometry.is_presented(*id))
            .and_then(|id| self.geometry.get(id))
            .map(|geometry| geometry.visual);
        let attribute = |name: &str| {
            id.and_then(|id| self.tree.node(id)).and_then(|node| {
                match node.attributes().get(name) {
                    Some(UiValue::String(value)) => Some(value.clone()),
                    _ => None,
                }
            })
        };
        let is_control =
            attribute("role").is_some_and(|role| CONTROL_ROLES.contains(&role.as_str()));
        if is_control
            && placement != Placement::Exempt
            && let Some(visual) = bounds
        {
            let name = attribute("label").unwrap_or_else(|| format!("{id:?}"));
            self.check_cut(&name, visual, clips.cut);
            let seen = clips
                .view
                .map_or(Some(visual), |view| intersection(visual, view));
            if placement == Placement::InFlow
                && let Some(seen) = seen
            {
                self.controls.push((name, seen));
            }
        }
        let hidden_page = node.attributes().get(crate::retained_page::PAGE_VISIBLE)
            == Some(&UiValue::Bool(false));
        let placement = if is_control || hidden_page {
            Placement::Exempt
        } else {
            placement
        };
        let clips = match bounds {
            // Scrolling reaches what its viewport hides.
            Some(bounds)
                if crate::renderer::node_scrollable(node)
                    || matches!(node.kind(), UiNodeKind::VirtualCollection { .. }) =>
            {
                Clips {
                    cut: None,
                    view: Some(Clips::within(clips.view, bounds)),
                }
            }
            Some(bounds) if clips_children(&style) => Clips {
                cut: Some(Clips::within(clips.cut, bounds)),
                view: Some(Clips::within(clips.view, bounds)),
            },
            _ => clips,
        };
        let floating = if placement == Placement::Exempt {
            Placement::Exempt
        } else {
            Placement::Floating
        };
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                for (index, child) in children.iter().enumerate() {
                    let child_id = child_link(self.tree, id, "children", index);
                    self.visit(child, child_id, clips, placement);
                }
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                let trigger_id = child_link(self.tree, id, "trigger", 0);
                self.visit(trigger, trigger_id, clips, placement);
                let content_id = child_link(self.tree, id, "content", 0);
                self.visit(content, content_id, Clips::default(), floating);
            }
            UiNodeKind::Layer { content, .. } => {
                let content_id = child_link(self.tree, id, "content", 0);
                self.visit(content, content_id, Clips::default(), floating);
            }
            UiNodeKind::ErrorBoundary { child, fallback } => {
                let child_id = child_link(self.tree, id, "child", 0);
                self.visit(child, child_id, clips, placement);
                let fallback_id = child_link(self.tree, id, "fallback", 0);
                self.visit(fallback, fallback_id, clips, placement);
            }
            UiNodeKind::VirtualCollection { spec } => {
                for (index, item) in spec.realized.values().enumerate() {
                    let item_id = child_link(self.tree, id, "items", index);
                    self.visit(item, item_id, clips, placement);
                }
            }
            _ => {}
        }
    }

    /// A control that meets `clip` but passes it: part of it cannot be seen
    /// or pressed. Wholly outside, it is hidden on purpose.
    fn check_cut(&mut self, name: &str, visual: GeometryBounds, clip: Option<GeometryBounds>) {
        let Some(clip) = clip else {
            return;
        };
        let outside = (clip.x - visual.x)
            .max(visual.x + visual.width - clip.x - clip.width)
            .max(clip.y - visual.y)
            .max(visual.y + visual.height - clip.y - clip.height);
        if intersection(visual, clip).is_some() && outside > REACH_SLACK {
            self.findings.push(format!(
                "control {name} is cut off by {outside:.1}px where nothing scrolls"
            ));
        }
    }
}

/// Whether a style clips its children.
fn clips_children(style: &StyleProperties) -> bool {
    style.clip == Some(true)
        || matches!(
            style.overflow_x,
            Some(OverflowMode::Hidden | OverflowMode::Scroll)
        )
        || matches!(
            style.overflow_y,
            Some(OverflowMode::Hidden | OverflowMode::Scroll)
        )
}

fn intersection(a: GeometryBounds, b: GeometryBounds) -> Option<GeometryBounds> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then_some(GeometryBounds {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

fn root_node_id(tree: &RetainedUiTree) -> Option<NodeId> {
    tree.nodes()
        .find(|node| node.parent().is_none())
        .map(crate::RetainedNode::id)
}

fn solid_colors(theme: &impl ColorResolver) -> BTreeSet<u32> {
    [
        "accent",
        "danger",
        "warning",
        "success",
        "button.primary_bg",
        "button.primary_bottom",
    ]
    .into_iter()
    .filter_map(|token| theme.resolve_token(token))
    .map(Rgba8::as_rgba_hex)
    .collect()
}

struct Walker<'a, 'b, C: ColorResolver> {
    inputs: &'a AuditInputs<'b, C>,
    solid: BTreeSet<u32>,
    /// Rules a node deliberately opts out of (`audit_allow`), by node path.
    allowed: BTreeSet<(String, AuditRule)>,
    findings: &'a mut Vec<AuditFinding>,
    reported_fonts: &'a mut BTreeSet<String>,
    /// Refine scope at a text node crossing a clipped fill or isolation
    /// boundary. Sibling traversal always restores the shared stack.
    paint_path: Vec<PaintStep>,
    /// The unrefined backing immediately before the first relevant step.
    paint_path_base: Option<ContrastSurface>,
}

enum PaintStep {
    Fill(SurfacePaint, f64),
    Isolate { opacity: f64, grayscale: f64 },
    GridBacking(GridBacking),
}

struct GridBacking {
    child: GeometryBounds,
    plates: Vec<GridPlate>,
    intervening: Vec<(usize, Option<GeometryBounds>)>,
}

struct GridPlate {
    position: usize,
    ink: Rgba8,
    footprint: GeometryBounds,
    interior: Option<PaddingCoverage>,
}

impl GridBacking {
    fn apply(&self, mut paper: ContrastSurface, text: crate::PresentedGeometry) -> ContrastSurface {
        let Some(visible) = clipped_visual(text) else {
            return paper;
        };
        for plate in &self.plates {
            if intersection(plate.footprint, visible).is_none() {
                continue;
            }
            let unobstructed = contains_rect(self.child, visible)
                && !self.intervening.iter().any(|(index, bounds)| {
                    *index > plate.position
                        && bounds.is_none_or(|bounds| intersection(bounds, visible).is_some())
                });
            let covered = unobstructed
                && contains_rect(plate.footprint, visible)
                && plate
                    .interior
                    .as_ref()
                    .is_some_and(|inner| inner.covers(text));
            if covered {
                paper = paper.over_with_opacity(plate.ink, 1.0);
            } else {
                // The plate may meet only part of a line, or an earlier
                // sibling may intervene. Keep both possible backings.
                paper = paper.layered(
                    std::iter::once(([gpui::rgba(plate.ink.as_rgba_hex())], true)),
                    1.0,
                );
            }
        }
        paper
    }
}

fn contains_rect(outer: GeometryBounds, inner: GeometryBounds) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.width >= inner.x + inner.width
        && outer.y + outer.height >= inner.y + inner.height
}

fn clipped_visual(presented: crate::PresentedGeometry) -> Option<GeometryBounds> {
    match presented.clip {
        Some(clip) => intersection(presented.visual, clip),
        None if presented.visual.width > 0.0 && presented.visual.height > 0.0 => {
            Some(presented.visual)
        }
        None => None,
    }
}

enum SurfacePaint {
    Solid(Rgba8),
    Linear(Rgba8, Rgba8),
    Layered(Vec<(Vec<gpui::Rgba>, bool)>, Option<PaddingCoverage>),
}

impl SurfacePaint {
    fn apply(
        &self,
        background: ContrastSurface,
        opacity: f64,
        text: Option<crate::PresentedGeometry>,
    ) -> ContrastSurface {
        match self {
            Self::Solid(color) => background.over_with_opacity(*color, opacity),
            Self::Linear(from, to) => background.gradient_with_opacity(*from, *to, opacity),
            Self::Layered(layers, padding) => {
                let covered = padding
                    .as_ref()
                    .zip(text)
                    .is_some_and(|(padding, text)| padding.covers(text));
                background.layered(
                    layers
                        .iter()
                        .map(|(stops, padded)| (stops.as_slice(), *padded && !covered)),
                    opacity,
                )
            }
        }
    }
}

struct PaddingCoverage {
    window_to_owner: crate::Affine2D,
    /// An inner approximation of the painted padding box. It leaves space
    /// for pixel snapping/antialiasing and uses an upper bound on both
    /// axes of every native corner radius.
    inner: GeometryBounds,
    radius: f64,
}

impl PaddingCoverage {
    fn covers(&self, text: crate::PresentedGeometry) -> bool {
        let Ok(map) = text.local_to_window.then(self.window_to_owner) else {
            return false;
        };
        let rect = map.transform_bounds(text.local);
        let distances_x = [
            rect.x - self.inner.x,
            self.inner.x + self.inner.width - rect.x - rect.width,
        ];
        let distances_y = [
            rect.y - self.inner.y,
            self.inner.y + self.inner.height - rect.y - rect.height,
        ];
        if rect.width <= 0.
            || rect.height <= 0.
            || distances_x
                .iter()
                .chain(&distances_y)
                .any(|d| !d.is_finite() || *d < 0.)
        {
            return false;
        }
        if self.radius == 0. {
            return true;
        }
        // Each pair is the most exposed point of this text footprint for
        // one corner. Admit the arc as well as the two rectangular bands,
        // without admitting any point outside the rounded padding box.
        let deficit = |d: f64| (1. - d / self.radius).max(0.);
        distances_x.into_iter().all(|x| {
            distances_y
                .into_iter()
                .all(|y| deficit(x).hypot(deficit(y)) <= 1.)
        })
    }
}

/// A semantic button role does not make a square icon action and a text action
/// the same visual family. Preserve the innermost control identity when formal
/// components directly forward another component's root; keys never define a
/// family, so two instances of the same control remain comparable.
fn button_family(node: &UiNode) -> String {
    node.forwarded_components()
        .and_then(|components| components.first())
        .or_else(|| node.component_root())
        .and_then(|component| component.segments().last())
        .map_or_else(
            || "button".to_owned(),
            |(component, _)| format!("button:{component}"),
        )
}

impl<C: ColorResolver> Walker<'_, '_, C> {
    fn report(&mut self, rule: AuditRule, node: Option<NodeId>, path: &str, message: String) {
        self.report_with_contrast(rule, node, path, message, None);
    }

    fn report_with_contrast(
        &mut self,
        rule: AuditRule,
        node: Option<NodeId>,
        path: &str,
        message: String,
        contrast: Option<AuditContrastEvidence>,
    ) {
        if self.inputs.rules.contains(rule) && !self.allowed.contains(&(path.to_owned(), rule)) {
            self.findings.push(AuditFinding {
                rule,
                node,
                path: path.to_owned(),
                message,
                contrast,
            });
        }
    }

    /// The edge text starts from: left in left-to-right views, right otherwise.
    fn start_edge(&self, bounds: GeometryBounds) -> f64 {
        match self.inputs.direction {
            crate::TextDirection::LeftToRight => bounds.x,
            crate::TextDirection::RightToLeft => bounds.x + bounds.width,
        }
    }

    fn pixels(&self, length: Option<Length>, environment: &Environment) -> Option<f64> {
        match self.inputs.theme.resolve_length_in(length?, environment)? {
            Length::Pixels(value) => Some(value),
            Length::Rems(value) => Some(value * 16.0),
            Length::Relative(_) | Length::Token(_) => None,
        }
    }

    fn visit(
        &mut self,
        node: &UiNode,
        id: Option<NodeId>,
        parent: Scope,
        path: &str,
    ) -> NodeSummary {
        let depth = self.paint_path.len();
        let summary = self.visit_node(node, id, parent, path);
        self.paint_path.truncate(depth);
        if depth == 0 {
            self.paint_path_base = None;
        }
        summary
    }

    #[allow(clippy::too_many_lines)] // One ordered pass per node keeps scope inheritance readable.
    fn visit_node(
        &mut self,
        node: &UiNode,
        id: Option<NodeId>,
        parent: Scope,
        path: &str,
    ) -> NodeSummary {
        let retained = id.and_then(|id| self.inputs.tree.node(id));
        let style = node.style().resolve(&InteractionState::default());
        let mut scope = parent;
        let opacity = style.opacity.unwrap_or(1.0).clamp(0.0, 1.0);
        scope.opacity *= opacity;
        scope.total_opacity *= opacity;
        if scope.total_opacity <= 0.0
            || style.visible == Some(false)
            // Elided fragments delegate frame membership to their children.
            || id.is_some_and(|id| !self.inputs.geometry.is_presented(id))
                && !matches!(node.kind(), UiNodeKind::Fragment { .. })
        {
            return NodeSummary::default();
        }
        let grayscale = if cfg!(target_os = "macos") {
            style.foreground_grayscale.unwrap_or_default()
        } else {
            0.
        };
        if self.inputs.rules.contains(AuditRule::LowContrastText)
            && (style.isolated_opacity == Some(true) || grayscale > 0.)
        {
            self.paint_path_base.get_or_insert(scope.background);
            self.paint_path.push(PaintStep::Isolate {
                opacity: scope.opacity,
                grayscale,
            });
            // Match the native paint boundary: the foreground and fills
            // composite into clear local RGBA, then fade together.
            scope.background = ContrastSurface::transparent();
            scope.opacity = 1.0;
        }
        if let Some(UiValue::Map(values)) = node.attributes().get("environment") {
            for (name, value) in values {
                if let UiValue::String(value) = value {
                    scope.environment = scope
                        .environment
                        .with(Symbol::intern(name), Symbol::intern(value))
                        .unwrap_or(scope.environment);
                }
            }
        }
        // Compare content proximity, not just two declarations separated by
        // padded wrappers. Tray/surface insets contribute to the separation
        // between groups even when an intermediate wrapper has only one child.
        let padding = &style.padding;
        let horizontal = [
            padding.start.or(padding.left),
            padding.end.or(padding.right),
        ]
        .into_iter()
        .filter_map(|value| self.pixels(value, &scope.environment))
        .sum::<f64>();
        let vertical = [padding.top, padding.bottom]
            .into_iter()
            .filter_map(|value| self.pixels(value, &scope.environment))
            .sum::<f64>();
        scope.enclosing_row_gap = scope.enclosing_row_gap.map(|gap| gap + horizontal);
        scope.enclosing_column_gap = scope.enclosing_column_gap.map(|gap| gap + vertical);
        scope.disabled |= node.attributes().get("disabled") == Some(&UiValue::Bool(true));
        if let Some(UiValue::Array(rules)) = node.attributes().get("audit_allow") {
            for rule in rules {
                if let UiValue::String(id) = rule
                    && let Some(rule) = AuditRule::parse(id)
                {
                    self.allowed.insert((path.to_owned(), rule));
                }
            }
        }
        self.apply_paint(&style, &mut scope, id);
        self.apply_text(&style, &mut scope, id, path);

        let role = retained
            .and_then(|node| node.attributes().get("role"))
            .and_then(|role| match role {
                UiValue::String(role) => Some(role.clone()),
                _ => None,
            });
        let bounds = id
            .filter(|id| self.inputs.geometry.is_presented(*id))
            .and_then(|id| self.inputs.geometry.get(id))
            .map(|geometry| geometry.visual);
        let mut summary = NodeSummary {
            role: role.clone(),
            bounds,
            ..NodeSummary::default()
        };
        let is_control = role
            .as_deref()
            .is_some_and(|role| CONTROL_ROLES.contains(&role));
        // Ownership is a layout fact, independent of which diagnostics the
        // caller enables. A one-rule audit must see the same anchors.
        let owns_tree_chrome = role.as_deref() == Some("treeitem")
            && (node.handler("click").is_some()
                || node
                    .handlers()
                    .keys()
                    .any(|name| name.starts_with("key:") && node.handler(name).is_some()))
            && retained.is_some_and(|row| !contains_nested_collection(self.inputs.tree, row));
        let tree_chrome = owns_tree_chrome || scope.layout_owner == LayoutOwner::TreeItemChrome;
        if owns_tree_chrome {
            // An actionable row's typography is not peer body text. It
            // still has no fixed-height control family of its own.
            summary.contains_control = true;
        }
        // A marker (Badge, Tag) lays out its own lamp and text, like a control.
        let is_marker = role
            .as_deref()
            .is_some_and(|role| MARKER_ROLES.contains(&role));
        if let Some(role) = &role
            && CONTROL_ROLES.contains(&role.as_str())
        {
            summary.contains_control = true;
            summary.control_geometry = id.filter(|_| bounds.is_some());
            summary.control_family = Some(match role.as_str() {
                "button" => button_family(node),
                "text_field" | "textbox" | "searchbox" | "spinbutton" | "combobox" => {
                    "field".to_owned()
                }
                other => other.to_owned(),
            });
            summary.solid_action = !scope.disabled
                && role == "button"
                && self.fill_matches(&style, scope.text_color, |color| {
                    alpha_of(color) > 0.0 && self.solid.contains(&color.as_rgba_hex())
                });
        }

        match node.kind() {
            // An empty label can reserve space without painting any glyphs.
            UiNodeKind::Text { text } | UiNodeKind::RichText { text, .. } if !text.is_empty() => {
                if scope.type_voice != TypeVoice::Annotation {
                    summary.text_sample = scope.font_size.map(|size| TextSample {
                        size,
                        geometry: id.filter(|_| bounds.is_some()),
                        voice: scope.type_voice,
                    });
                }
                let (left, right) = match self.inputs.direction {
                    crate::TextDirection::LeftToRight => (padding.start, padding.end),
                    crate::TextDirection::RightToLeft => (padding.end, padding.start),
                };
                let inset = |padding, border| {
                    self.pixels(padding, &scope.environment).unwrap_or(0.0)
                        + self.pixels(border, &scope.environment).unwrap_or(0.0)
                };
                summary.text_ink_edge = id.filter(|_| bounds.is_some()).map(|node| StackEdge {
                    node,
                    kind: match role.as_deref() {
                        Some("columnheader") => StackEdgeKind::ColumnHeader,
                        Some("label") => StackEdgeKind::Label,
                        _ => StackEdgeKind::Text,
                    },
                    inline_insets: [
                        inset(left.or(padding.left), style.border_widths.left),
                        inset(right.or(padding.right), style.border_widths.right),
                    ],
                });
                summary.text_edge = summary.text_ink_edge;
                self.check_contrast(&scope, id, path);
                if node
                    .attributes()
                    .contains_key(crate::node::INLINE_PROSE_SOURCE)
                    && self.inputs.rules.contains(AuditRule::LowContrastText)
                    && let UiNodeKind::RichText { spans, .. } = node.kind()
                {
                    for (index, span) in spans.iter().enumerate() {
                        if span.text().trim().is_empty()
                            || (span.color_value().is_none() && span.background_value().is_none())
                        {
                            continue;
                        }
                        let mut run = scope;
                        let mut style = crate::Style::new();
                        if let Some(color) = span.color_value() {
                            style = style.text_color(color.clone());
                        }
                        if let Some(paper) = span.background_value() {
                            style = style.background(paper.clone());
                        }
                        self.apply_paint(&style.base, &mut run, id);
                        if span.is_bold() {
                            run.font_weight = 700;
                        }
                        self.check_contrast(&run, id, &format!("{path}/span:{index}"));
                    }
                }
            }
            // A native control renders its own text at the inherited size.
            UiNodeKind::Custom { .. } if style.typography.is_some() => {
                summary.text_sample = scope.font_size.map(|size| TextSample {
                    size,
                    geometry: id.filter(|_| bounds.is_some()),
                    voice: scope.type_voice,
                });
            }
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                let direction = style.direction.unwrap_or(FlexDirection::Column);
                let child_ids = retained
                    .into_iter()
                    .flat_map(crate::RetainedNode::children)
                    .filter(|child| child.group() == "children")
                    .map(|child| Some(child.node()))
                    .chain(std::iter::repeat(None))
                    .take(children.len())
                    .collect::<Vec<_>>();
                let spacing_peers = (self.inputs.rules.contains(AuditRule::SpacingNotNested)
                    && matches!(
                        style.flex_wrap,
                        Some(FlexWrapMode::Wrap | FlexWrapMode::WrapReverse)
                    )
                    && matches!(style.display, None | Some(DisplayMode::Flex))
                    && style.grid_columns.is_none()
                    && scope.layout_owner == LayoutOwner::Composition
                    && !owns_tree_chrome
                    && !is_control
                    && !is_marker)
                    .then(|| self.wrapped_spacing_peers(children, &child_ids, direction))
                    .flatten();
                let gap = self.pixels(
                    match direction {
                        FlexDirection::Row => style.gap_x(),
                        FlexDirection::Column => style.gap_y(),
                    },
                    &scope.environment,
                );
                let heading_led = leads_with_heading(children) || introduced_elsewhere(node);
                // Distributed regions treat gap as a floor, not their actual
                // separation. Zero-basis flex regions distribute free row
                // space like justify-between. A growing column body alone
                // does not turn its parent's content into distributed chrome.
                let distributed = matches!(style.justify, Some(Justify::Between | Justify::Around))
                    || direction == FlexDirection::Row
                        && children.iter().any(|child| {
                            let style = &child.style().base;
                            (style.flex_grow == Some(true)
                                || style.flex_grow_weight.is_some_and(|weight| weight > 0.0))
                                && style.flex_basis
                                    == Some(crate::LayoutLength::Definite(Length::Relative(0.0)))
                        });
                // A control's or marker's internal layout is the component's business.
                let counted = scope.layout_owner == LayoutOwner::Composition
                    && !owns_tree_chrome
                    && !is_control
                    && !is_marker
                    && !distributed
                    && children.len() >= 2
                    && spacing_peers
                        .as_ref()
                        .is_none_or(|peers| peers.contains(&true))
                    && gap.is_some_and(|gap| gap > 0.0);
                let enclosing = match direction {
                    FlexDirection::Row => scope.enclosing_row_gap,
                    FlexDirection::Column => scope.enclosing_column_gap,
                };
                if counted
                    && !heading_led
                    && let (Some(inner), Some(outer)) = (gap, enclosing)
                    && inner > outer + EPSILON
                {
                    self.report(
                        AuditRule::SpacingNotNested,
                        id,
                        path,
                        format!(
                            "inner gap {inner}px exceeds the enclosing content separation {outer}px"
                        ),
                    );
                }
                // A heading starts a new content hierarchy: its optical title gap
                // is not a bound on the section's inner grouping relationships.
                // A container that opts out of the nesting rule (a grid, such as a
                // label column) neither answers to it nor imposes its gap below.
                let opted_out = self
                    .allowed
                    .contains(&(path.to_owned(), AuditRule::SpacingNotNested));
                let sets_limit = counted && !opted_out && !heading_led;
                let child_scope = Scope {
                    layout_owner: if scope.layout_owner == LayoutOwner::Control
                        || is_control
                        || is_marker
                    {
                        LayoutOwner::Control
                    } else if tree_chrome {
                        LayoutOwner::TreeItemChrome
                    } else {
                        LayoutOwner::Composition
                    },
                    ..scope
                };
                let summaries = children
                    .iter()
                    .zip(child_ids.iter().copied())
                    .enumerate()
                    .map(|(index, (child, child_id))| {
                        let mut child_scope = child_scope;
                        let limit = match direction {
                            FlexDirection::Row => &mut child_scope.enclosing_row_gap,
                            FlexDirection::Column => &mut child_scope.enclosing_column_gap,
                        };
                        if heading_led || spacing_peers.as_ref().is_some_and(|peers| !peers[index])
                        {
                            // Another flex line is not a source of main-axis spacing.
                            *limit = None;
                        } else if sets_limit {
                            *limit = gap;
                        }
                        let child_path = child.key().map_or_else(
                            || format!("{path}/{index}"),
                            |key| format!("{path}/{}", key.as_str()),
                        );
                        if style.display == Some(DisplayMode::Grid)
                            && let Some(backing) =
                                self.grid_backing(children, &child_ids, index, child_scope)
                        {
                            let depth = self.paint_path.len();
                            let base = self.paint_path_base;
                            self.paint_path_base.get_or_insert(child_scope.background);
                            self.paint_path.push(PaintStep::GridBacking(backing));
                            let summary = self.visit(child, child_id, child_scope, &child_path);
                            self.paint_path.truncate(depth);
                            self.paint_path_base = base;
                            summary
                        } else {
                            self.visit(child, child_id, child_scope, &child_path)
                        }
                    })
                    .collect::<Vec<_>>();
                if scope.layout_owner != LayoutOwner::Control && !is_control && !is_marker {
                    let skip_mixed_type = heading_led || tree_chrome;
                    if matches!(node.kind(), UiNodeKind::Box { .. })
                        && (style.display == Some(DisplayMode::Grid)
                            || style.auto_grid_columns().is_some())
                    {
                        self.check_grid(children, &child_ids, &summaries, tree_chrome, id, path);
                    } else {
                        self.check_container(&style, &summaries, skip_mixed_type, id, path);
                    }
                }
                inherit_first(&mut summary, &summaries, role.as_deref());
                // Only a wrapper that hugs one control stands in for it; a container
                // of several (a pane with a toolbar and a body) is not a control in
                // its parent's row.
                if (children.len() > 1 || owns_tree_chrome) && !is_control {
                    summary.control_geometry = None;
                    summary.control_family = None;
                    summary.solid_action = false;
                }
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                let trigger_id = child_link(self.inputs.tree, id, "trigger", 0);
                let trigger = self.visit(trigger, trigger_id, scope, &format!("{path}/trigger"));
                // Overlay content floats in its own layer: no spacing relationship
                // with the place its trigger sits in.
                let content_id = child_link(self.inputs.tree, id, "content", 0);
                let layer = Scope {
                    enclosing_row_gap: None,
                    enclosing_column_gap: None,
                    layout_owner: outside_tree_chrome(scope.layout_owner),
                    ..scope
                };
                self.visit(content, content_id, layer, &format!("{path}/content"));
                inherit_first(&mut summary, &[trigger], role.as_deref());
            }
            UiNodeKind::Layer { content, .. } => {
                // A layer floats like overlay content.
                let content_id = child_link(self.inputs.tree, id, "content", 0);
                let layer = Scope {
                    enclosing_row_gap: None,
                    enclosing_column_gap: None,
                    layout_owner: outside_tree_chrome(scope.layout_owner),
                    ..scope
                };
                self.visit(content, content_id, layer, &format!("{path}/content"));
            }
            UiNodeKind::ErrorBoundary { child, fallback } => {
                // Audit what renders: the child, or the fallback once the child
                // failed (only the rendered one has geometry).
                let child_id = child_link(self.inputs.tree, id, "child", 0);
                let fallback_id = child_link(self.inputs.tree, id, "fallback", 0);
                let fallback_rendered = child_id
                    .is_none_or(|id| self.inputs.geometry.get(id).is_none())
                    && fallback_id.is_some_and(|id| self.inputs.geometry.get(id).is_some());
                let rendered = if fallback_rendered {
                    self.visit(fallback, fallback_id, scope, &format!("{path}/fallback"))
                } else {
                    self.visit(child, child_id, scope, &format!("{path}/child"))
                };
                inherit_first(&mut summary, &[rendered], role.as_deref());
            }
            UiNodeKind::VirtualCollection { spec } => {
                // The realized items: what a virtual list shows is audited like any
                // other content; unrealized items have no geometry yet.
                for (index, item) in spec.realized.values().enumerate() {
                    let item_id = child_link(self.inputs.tree, id, "items", index);
                    let item_path = item.key().map_or_else(
                        || format!("{path}/item/{index}"),
                        |key| format!("{path}/item/{}", key.as_str()),
                    );
                    self.visit(item, item_id, scope, &item_path);
                }
            }
            _ => {}
        }
        // Controls, markers and any filled or framed box align by their own edge;
        // their inner padding is the component's business. Do not compare a
        // boundary with actual text just because the two happen to be nearby.
        let painted = self.fill_matches(&style, scope.text_color, |color| alpha_of(color) > 0.0);
        // A section divider is not an enclosing surface: its border has no
        // horizontal visual start of its own.
        let framed = [
            style.border_widths.top,
            style.border_widths.right,
            style.border_widths.bottom,
            style.border_widths.left,
        ]
        .into_iter()
        .all(|width| {
            self.pixels(width, &scope.environment)
                .is_some_and(|width| width > 0.0)
        });
        // A cell that centers its content (weekday labels, day numbers) has no
        // text start of its own: the cell edge is what lines up.
        let centered = match style.direction {
            Some(FlexDirection::Row) => style.justify == Some(Justify::Center),
            Some(FlexDirection::Column) => style.align == Some(Align::Center),
            None => false,
        };
        if tree_chrome {
            // A fold icon, swatch, or interactive row frame can precede the
            // label. Mixing that edge with peer body-text starts is not a
            // text alignment comparison.
            summary.text_edge = summary.text_ink_edge;
        } else if let Some(node) = id.filter(|_| bounds.is_some()) {
            let kind = CONTROL_ROLES
                .iter()
                .copied()
                .find(|control| role.as_deref() == Some(*control))
                .map(StackEdgeKind::Control)
                .or(if is_marker {
                    Some(StackEdgeKind::Marker)
                } else if painted || framed {
                    Some(StackEdgeKind::Surface)
                } else if centered {
                    Some(StackEdgeKind::Centered)
                } else {
                    None
                });
            if let Some(kind) = kind {
                summary.text_edge = Some(StackEdge {
                    node,
                    kind,
                    inline_insets: [0.0; 2],
                });
            }
        }
        // Absolute children keep their own audit (including contrast and
        // action policy), but do not supply peers to an enclosing flow.
        if style.position == Some(PositionMode::Absolute) {
            summary.text_edge = None;
            summary.text_ink_edge = None;
            summary.text_sample = None;
            summary.control_geometry = None;
            summary.control_family = None;
            summary.contains_control = false;
        } else if node.attributes().get("accessibility_hidden") == Some(&UiValue::Bool(true))
            && summary.text_ink_edge.is_none()
            && !summary.contains_control
            && node.handlers().is_empty()
        {
            // A decorative lamp or motif can have real paint and geometry,
            // but must not lend its internal edge to the enclosing text.
            // Visible text still participates even if omitted from AT.
            summary.text_edge = None;
        }
        summary
    }

    fn paint_ink(&self, ink: &crate::PaintColor, text: Option<Rgba8>) -> Option<Rgba8> {
        Some(Rgba8::from_native_color(self.native_ink(ink, text)?.into()))
    }

    fn native_ink(&self, ink: &crate::PaintColor, text: Option<Rgba8>) -> Option<gpui::Rgba> {
        let ink = ink.native_ink(&mut |value| self.inputs.theme.resolve(value))?;
        Some(ink.resolve(text.map_or(gpui::Hsla::default(), |text| {
            gpui::rgba(text.as_rgba_hex()).into()
        })))
    }

    fn fill_matches(
        &self,
        style: &StyleProperties,
        text: Option<Rgba8>,
        accepts: impl Fn(Rgba8) -> bool,
    ) -> bool {
        use crate::BackgroundFill::{Layers, LegacyLinear, Solid};
        match style.background.as_deref() {
            Some(Solid(ink)) => self.paint_ink(ink, text).is_some_and(accepts),
            Some(LegacyLinear(gradient)) => [&gradient.from, &gradient.to]
                .into_iter()
                .all(|stop| self.inputs.theme.resolve(stop).is_some_and(&accepts)),
            Some(Layers(layers)) => layers.layers().first().is_some_and(|layer| {
                layer
                    .stops
                    .iter()
                    .all(|stop| self.paint_ink(&stop.color, text).is_some_and(&accepts))
            }),
            None => false,
        }
    }

    fn apply_paint(&mut self, style: &StyleProperties, scope: &mut Scope, node: Option<NodeId>) {
        use crate::BackgroundFill::{Layers, LegacyLinear, Solid};
        if let Some(color) = style
            .text_color
            .as_ref()
            .and_then(|color| self.inputs.theme.resolve(color))
        {
            scope.text_color = Some(color);
        }
        if !self.inputs.rules.contains(AuditRule::LowContrastText) {
            return;
        }
        let paint = match style.background.as_deref() {
            Some(Solid(ink)) => self
                .paint_ink(ink, scope.text_color)
                .map(SurfacePaint::Solid),
            Some(LegacyLinear(gradient)) => self
                .inputs
                .theme
                .resolve(&gradient.from)
                .zip(self.inputs.theme.resolve(&gradient.to))
                .map(|(from, to)| SurfacePaint::Linear(from, to)),
            Some(Layers(layers)) => {
                let resolved = layers
                    .layers()
                    .iter()
                    .rev()
                    .map(|layer| {
                        let colors = layer
                            .stops
                            .iter()
                            .map(|stop| self.native_ink(&stop.color, scope.text_color))
                            .collect::<Option<Vec<_>>>()?;
                        Some((colors, layer.clip == crate::FillBox::Padding))
                    })
                    .collect::<Option<Vec<_>>>();
                resolved.map(|resolved| {
                    let coverage = resolved
                        .iter()
                        .any(|(_, padded)| *padded)
                        .then(|| self.padding_coverage(style, &scope.environment, node))
                        .flatten();
                    SurfacePaint::Layered(resolved, coverage)
                })
            }
            None => None,
        };
        if let Some(paint) = paint {
            if self.paint_path_base.is_none() && matches!(paint, SurfacePaint::Layered(_, Some(_)))
            {
                self.paint_path_base = Some(scope.background);
            }
            if self.paint_path_base.is_some() {
                self.paint_path.push(PaintStep::Fill(paint, scope.opacity));
            } else {
                scope.background = paint.apply(scope.background, scope.opacity, None);
            }
        }
    }

    /// A bounded grid can draw simple empty decorative paper before its
    /// content without nesting the content. Use only committed, visible
    /// geometry; arbitrary overlapping widgets are not a paint oracle.
    fn grid_backing(
        &self,
        children: &[UiNode],
        ids: &[Option<NodeId>],
        index: usize,
        scope: Scope,
    ) -> Option<GridBacking> {
        if index == 0
            || children.len() > 32
            || scope.opacity.to_bits() != 1.0_f64.to_bits()
            || !self.inputs.rules.contains(AuditRule::LowContrastText)
        {
            return None;
        }
        let child = clipped_visual(self.inputs.geometry.presented(ids[index]?).ok()?)?;
        let mut plates = Vec::new();
        let mut intervening = Vec::new();
        for (position, (node, id)) in children[..index].iter().zip(&ids[..index]).enumerate() {
            let Some(id) = *id else {
                intervening.push((position, None));
                continue;
            };
            if !self.inputs.geometry.is_presented(id) {
                continue;
            }
            let Ok(presented) = self.inputs.geometry.presented(id) else {
                intervening.push((position, None));
                continue;
            };
            let Some(footprint) = clipped_visual(presented) else {
                continue;
            };
            if let Some(ink) = self.grid_plate_ink(node, scope.text_color) {
                plates.push(GridPlate {
                    position,
                    ink,
                    footprint,
                    interior: self.padding_coverage(
                        &node.style().base,
                        &scope.environment,
                        Some(id),
                    ),
                });
            } else {
                let style = &node.style().base;
                let bounded = (matches!(node.kind(), UiNodeKind::Box { children } if children.is_empty())
                    || style.clip == Some(true))
                    && style.foreground_blur.unwrap_or_default() == 0.
                    && style.shadows.as_ref().is_none_or(Vec::is_empty);
                // Layout bounds alone do not bound overflowing descendants,
                // foreign elements, blur, or shadows.
                intervening.push((position, bounded.then_some(footprint)));
            }
        }
        (!plates.is_empty()).then_some(GridBacking {
            child,
            plates,
            intervening,
        })
    }

    fn grid_plate_ink(&self, node: &UiNode, ambient: Option<Rgba8>) -> Option<Rgba8> {
        let declaration = node.style();
        let style = &declaration.base;
        if !matches!(node.kind(), UiNodeKind::Box { children } if children.is_empty())
            || node.attributes().get("accessibility_hidden") != Some(&UiValue::Bool(true))
            || node.attributes().contains_key("environment")
            || !node.handlers().is_empty()
            || !node.motions().is_empty()
            || node.signal_bindings().next().is_some()
            || crate::paint_motion::transition(node).is_some()
            || declaration.hover.is_some()
            || declaration.active.is_some()
            || declaration.focus.is_some()
            || declaration.group_focus.is_some()
            || declaration.group_hover.is_some()
            || declaration.focus_within.is_some()
            || declaration.disabled.is_some()
            || style.opacity.unwrap_or(1.).to_bits() != 1.0_f64.to_bits()
            || style.visible == Some(false)
            || style.bottom_fade.is_some()
            || style.paint_reveal_x.is_some()
            || style.foreground_blur.unwrap_or_default() > 0.
            || style.foreground_grayscale.unwrap_or_default() > 0.
            || style.backdrop_blur.unwrap_or_default() > 0.
            || style
                .shadows
                .as_ref()
                .is_some_and(|shadows| shadows.iter().any(|shadow| shadow.inset))
        {
            return None;
        }
        let crate::BackgroundFill::Solid(fill) = style.background.as_deref()? else {
            return None;
        };
        let own_text = style
            .text_color
            .as_ref()
            .and_then(|ink| self.inputs.theme.resolve(ink));
        self.paint_ink(fill, own_text.or(ambient))
            .filter(|color| color.as_rgba_hex() & 0xff == 0xff)
    }

    fn padding_coverage(
        &self,
        style: &StyleProperties,
        environment: &Environment,
        id: Option<NodeId>,
    ) -> Option<PaddingCoverage> {
        let geometry = self.inputs.geometry.presented(id?).ok()?;
        let window_to_owner = geometry.window_to_local?;
        let pixels = |length: Option<Length>| match length {
            None => Some(0.),
            Some(length) => match self.inputs.theme.resolve_length_in(length, environment)? {
                Length::Pixels(value) if value.is_finite() && value >= 0. => Some(value),
                // No proof when geometry depends on a Window-specific rem.
                _ => None,
            },
        };
        let border = &style.border_widths;
        let start = pixels(border.start)?;
        let end = pixels(border.end)?;
        // Upper-bound both possible physical edges for logical refinements;
        // a spacer may choose its own direction independent of the view.
        let left = pixels(border.left)?.max(start).max(end);
        let right = pixels(border.right)?.max(start).max(end);
        let top = pixels(border.top)?;
        let bottom = pixels(border.bottom)?;
        let radii = &style.radii;
        let radius = [
            radii.top_left,
            radii.top_right,
            radii.bottom_left,
            radii.bottom_right,
            radii.start,
            radii.end,
        ]
        .into_iter()
        .try_fold(0_f64, |largest, length| Some(largest.max(pixels(length)?)))?;
        // Native resolution clamps radii and subtracts the border; using the
        // authored maximum is a conservative bound. Leave extra room for
        // edge antialiasing and logical->device snapping.
        let edge = 1.5;
        let inner = GeometryBounds::new(
            left + edge,
            top + edge,
            geometry.local.width - left - right - 2. * edge,
            geometry.local.height - top - bottom - 2. * edge,
        )
        .ok()?;
        Some(PaddingCoverage {
            window_to_owner,
            inner,
            radius,
        })
    }

    fn apply_text(
        &mut self,
        style: &StyleProperties,
        scope: &mut Scope,
        id: Option<NodeId>,
        path: &str,
    ) {
        let role = style.typography.as_deref().and_then(|role| {
            self.inputs
                .theme
                .resolve_typography_in(role, &scope.environment)
        });
        if let Some(name) = style.typography.as_deref() {
            scope.type_voice = match name {
                "display" | "display_large" if role.is_some() => TypeVoice::Display,
                "metadata" if role.is_some() => TypeVoice::Metadata,
                "initials" if role.is_some() => TypeVoice::Monogram,
                "label" | "key_legend" | "key_legend_cap" | "key_legend_tooltip"
                | "key_legend_menu" => TypeVoice::Annotation,
                _ => TypeVoice::Inline,
            };
        }
        if let Some(role) = &role {
            scope.font_weight = role.weight;
            if let Some(size) = self.pixels(Some(role.size), &scope.environment) {
                scope.font_size = Some(size);
            }
            if let Some(family) = &role.family {
                self.check_font(family, &role.fallbacks, id, path);
            }
        }
        if let Some(size) = self.pixels(style.font_size, &scope.environment) {
            scope.font_size = Some(size);
        }
        if let Some(weight) = style.font_weight {
            scope.font_weight = weight;
        }
        if let Some(family) = &style.font_family {
            let fallbacks = style.font_fallbacks.clone().unwrap_or_default();
            self.check_font(family, &fallbacks, id, path);
        }
    }

    fn check_font(&mut self, family: &str, fallbacks: &[String], id: Option<NodeId>, path: &str) {
        let Some(available) = self.inputs.available_fonts else {
            return;
        };
        // Declared fallbacks are a deliberate chain; only a chain that resolves
        // nowhere falls back silently.
        let resolves = available.contains(family)
            || fallbacks
                .iter()
                .any(|fallback| available.contains(fallback));
        if !resolves && self.reported_fonts.insert(family.to_owned()) {
            self.report(
                AuditRule::UnresolvedFont,
                id,
                path,
                format!("font family `{family}` is not available; text falls back silently"),
            );
        }
    }

    fn check_contrast(&mut self, scope: &Scope, id: Option<NodeId>, path: &str) {
        if scope.disabled || !self.inputs.rules.contains(AuditRule::LowContrastText) {
            return;
        }
        let Some(text) = scope.text_color else {
            return;
        };
        let size = scope.font_size.unwrap_or(16.0);
        let large = size >= 24.0 || (size >= 18.66 && scope.font_weight >= 700);
        let minimum = if large { 3.0 } else { 4.5 };
        let (background, evidence) = self.contrast_at(scope, id, text);
        let ratio = evidence.ratio;
        if ratio + 0.005 < minimum {
            self.report_with_contrast(
                AuditRule::LowContrastText,
                id,
                path,
                format!(
                    "text contrast{} {ratio:.2}:1 is below {minimum}:1",
                    if evidence.conservative {
                        " lower bound"
                    } else {
                        ""
                    }
                ),
                Some(AuditContrastEvidence {
                    foreground: text,
                    element_opacity: scope.total_opacity,
                    background_srgb: background.solid_srgb(),
                    ratio,
                    minimum,
                    conservative: evidence.conservative,
                }),
            );
        }
    }

    fn contrast_at(
        &self,
        scope: &Scope,
        id: Option<NodeId>,
        text: Rgba8,
    ) -> (ContrastSurface, background::Evidence) {
        let Some(mut paper) = self.paint_path_base else {
            return (
                scope.background,
                scope.background.contrast_with_opacity(text, scope.opacity),
            );
        };
        let position = id.and_then(|id| self.inputs.geometry.presented(id).ok());
        let mut groups = Vec::new();
        for step in &self.paint_path {
            match step {
                PaintStep::Fill(paint, opacity) => paper = paint.apply(paper, *opacity, position),
                PaintStep::GridBacking(backing) => {
                    if let Some(position) = position {
                        paper = backing.apply(paper, position);
                    }
                }
                PaintStep::Isolate { opacity, grayscale } => {
                    groups.push((paper, *opacity, *grayscale));
                    paper = ContrastSurface::transparent();
                }
            }
        }
        if groups.is_empty() {
            return (paper, paper.contrast_with_opacity(text, scope.opacity));
        }
        let mut ink = paper.over_with_opacity(text, scope.opacity);
        for (backing, opacity, grayscale) in groups.into_iter().rev() {
            paper = paper.grayscale(grayscale).composite_over(backing, opacity);
            ink = ink.grayscale(grayscale).composite_over(backing, opacity);
        }
        (paper, paper.contrast_against(ink))
    }

    fn geometry_in_flow(
        &self,
        id: Option<NodeId>,
        window_to_flow: Option<crate::Affine2D>,
    ) -> Option<GeometryBounds> {
        let id = id?;
        if let Some(window_to_flow) = window_to_flow {
            let presented = self.inputs.geometry.presented(id).ok()?;
            Some(
                presented
                    .local_to_window
                    .then(window_to_flow)
                    .ok()?
                    .transform_bounds(presented.local),
            )
        } else {
            self.inputs.geometry.get(id).map(|geometry| geometry.visual)
        }
    }

    fn stack_bounds(
        &self,
        anchor: StackEdge,
        window_to_flow: Option<crate::Affine2D>,
    ) -> Option<GeometryBounds> {
        let [left, right] = anchor.inline_insets;
        if left == 0.0 && right == 0.0 {
            return self.geometry_in_flow(Some(anchor.node), window_to_flow);
        }
        let presented = self.inputs.geometry.presented(anchor.node).ok()?;
        let mut content = presented.local;
        content.x += left;
        content.width -= left + right;
        // A collapsed text box does not establish a peer text start.
        if content.width <= 0.0 {
            return None;
        }
        let transform = if let Some(window_to_flow) = window_to_flow {
            presented.local_to_window.then(window_to_flow).ok()?
        } else {
            presented.local_to_window
        };
        Some(transform.transform_bounds(content))
    }

    /// Determine which children have peers on their own flex line. Only use
    /// measured layout positions, not painted positions moved by a transform.
    /// Incomplete or ambiguous geometry retains the declaration-based check.
    fn wrapped_spacing_peers(
        &self,
        children: &[UiNode],
        child_ids: &[Option<NodeId>],
        direction: FlexDirection,
    ) -> Option<Vec<bool>> {
        let mut positions = Vec::new();
        for (index, (child, &id)) in children.iter().zip(child_ids).enumerate() {
            let style = &child.style().base;
            if style.position == Some(PositionMode::Absolute)
                || style.display == Some(DisplayMode::None)
            {
                continue;
            }
            let id = id?;
            if !self.inputs.geometry.is_presented(id) {
                return None;
            }
            let bounds = self.inputs.geometry.get(id)?.layout;
            if bounds.width <= 0.0 || bounds.height <= 0.0 {
                return None;
            }
            positions.push((Some(bounds), (index, bounds)));
        }
        let bands = flow_bands(positions, cross_axis(direction));
        if bands.len() < 2 {
            return None;
        }
        // Cross-axis alignment alone can make boxes in the same line
        // disjoint. Consecutive bands must also reuse main-axis space.
        let extents = |band: &[(usize, GeometryBounds)]| {
            band.iter()
                .map(|&(_, bounds)| axis_extent(bounds, direction))
                .reduce(|(a, b), (c, d)| (a.min(c), b.max(d)))
                .expect("a layout band contains at least one child")
        };
        if bands.windows(2).any(|pair| {
            let (a, b) = extents(&pair[0]);
            let (c, d) = extents(&pair[1]);
            b.min(d) <= a.max(c)
        }) {
            return None;
        }
        let mut peers = vec![false; children.len()];
        for band in bands {
            let has_peer = band.len() > 1;
            for (index, _) in band {
                peers[index] = has_peer;
            }
        }
        Some(peers)
    }

    fn check_container(
        &mut self,
        style: &StyleProperties,
        children: &[NodeSummary],
        skip_mixed_type: bool,
        id: Option<NodeId>,
        path: &str,
    ) {
        let window_to_flow = self.window_to_flow(id);
        match style.direction.unwrap_or(FlexDirection::Column) {
            FlexDirection::Row => {
                self.check_inline(children.iter(), skip_mixed_type, window_to_flow, id, path);
            }
            FlexDirection::Column => {
                self.check_stack(
                    children.iter(),
                    style.align == Some(Align::End),
                    window_to_flow,
                    id,
                    path,
                );
            }
        }
        self.check_action_group(children, id, path);
    }

    fn window_to_flow(&self, id: Option<NodeId>) -> Option<crate::Affine2D> {
        // Compare relative to the container, not the enclosing screen AABB.
        // A shared rotation preserves real alignment while changing each text
        // line's screen-space left edge and apparent control height.
        id.and_then(|id| self.inputs.geometry.exact_placement(id))
            .and_then(crate::Affine2D::inverse)
    }

    fn check_grid(
        &mut self,
        children: &[UiNode],
        child_ids: &[Option<NodeId>],
        summaries: &[NodeSummary],
        tree_chrome: bool,
        id: Option<NodeId>,
        path: &str,
    ) {
        let window_to_flow = self.window_to_flow(id);
        if self.inputs.rules.contains(AuditRule::RowHeightMismatch)
            || !tree_chrome && self.inputs.rules.contains(AuditRule::MixedTypeInRow)
        {
            for row in self.grid_tracks(children, child_ids, FlexDirection::Column) {
                // A title across the grid doesn't own the type of subsequent
                // rows. Apply the heading rule only to its own inline peers.
                let heading_led = row
                    .iter()
                    .filter(|&&index| !inert_pictogram(&children[index]))
                    .filter_map(|&index| {
                        let bounds = self.inputs.geometry.get(child_ids[index]?)?.layout;
                        let start = self.start_edge(bounds);
                        let start = if self.inputs.direction == crate::TextDirection::RightToLeft {
                            -start
                        } else {
                            start
                        };
                        Some((start, &children[index]))
                    })
                    .min_by(|(a, _), (b, _)| a.total_cmp(b))
                    .is_some_and(|(_, child)| leads_with_heading(std::slice::from_ref(child)));
                self.check_inline(
                    row.iter().map(|&index| &summaries[index]),
                    tree_chrome || heading_led,
                    window_to_flow,
                    id,
                    path,
                );
            }
        }
        if self.inputs.rules.contains(AuditRule::TextEdgeMisaligned) {
            for column in self.grid_tracks(children, child_ids, FlexDirection::Row) {
                self.check_stack(
                    column.iter().map(|&index| &summaries[index]),
                    false,
                    window_to_flow,
                    id,
                    path,
                );
            }
        }
        self.check_action_group(summaries, id, path);
    }

    fn grid_tracks(
        &self,
        children: &[UiNode],
        child_ids: &[Option<NodeId>],
        axis: FlexDirection,
    ) -> Vec<Vec<usize>> {
        // Occupation is determined before visual translations/animations.
        // A spanning tile isn't a single-track peer and must not bridge
        // its neighbouring tracks. Auto-fit uses the same laid-out facts.
        let positions = children
            .iter()
            .zip(child_ids.iter().copied())
            .enumerate()
            .filter_map(|(index, (child, id))| {
                let id = id.filter(|id| self.inputs.geometry.is_presented(*id))?;
                let style = &child.style().base;
                let span = match axis {
                    FlexDirection::Row => style.column_span,
                    FlexDirection::Column => style.row_span,
                };
                if style.position == Some(PositionMode::Absolute) || span.unwrap_or(1) > 1 {
                    return None;
                }
                let bounds = self.inputs.geometry.get(id)?.layout;
                (bounds.width > 0.0 && bounds.height > 0.0).then_some((Some(bounds), index))
            })
            .collect();
        flow_bands(positions, axis)
            .into_iter()
            .filter(|band| band.len() > 1)
            .collect()
    }

    fn check_inline<'a>(
        &mut self,
        children: impl Iterator<Item = &'a NodeSummary> + Clone,
        skip_mixed_type: bool,
        window_to_flow: Option<crate::Affine2D>,
        id: Option<NodeId>,
        path: &str,
    ) {
        if self.inputs.rules.contains(AuditRule::RowHeightMismatch) {
            let mut families: BTreeMap<&str, Vec<(Option<GeometryBounds>, f64)>> = BTreeMap::new();
            for child in children.clone() {
                if let (Some(bounds), Some(family)) = (
                    self.geometry_in_flow(child.control_geometry, window_to_flow),
                    child.control_family.as_deref(),
                ) {
                    families
                        .entry(family)
                        .or_default()
                        .push((Some(bounds), bounds.height));
                }
            }
            for (family, controls) in families {
                for heights in flow_bands(controls, FlexDirection::Column) {
                    if let (Some(min), Some(max)) = (
                        heights.iter().copied().reduce(f64::min),
                        heights.iter().copied().reduce(f64::max),
                    ) && max - min > EPSILON
                    {
                        self.report(
                            AuditRule::RowHeightMismatch,
                            id,
                            path,
                            format!(
                                "{family} controls in one row range from {min}px to {max}px high"
                            ),
                        );
                    }
                }
            }
        }
        // Headers and owned tree row chrome deliberately combine type scales.
        if !skip_mixed_type && self.inputs.rules.contains(AuditRule::MixedTypeInRow) {
            self.check_inline_type(children, window_to_flow, id, path);
        }
    }

    fn check_stack<'a>(
        &mut self,
        children: impl Iterator<Item = &'a NodeSummary>,
        trailing: bool,
        window_to_flow: Option<crate::Affine2D>,
        id: Option<NodeId>,
        path: &str,
    ) {
        if !self.inputs.rules.contains(AuditRule::TextEdgeMisaligned) {
            return;
        }
        let starts = children
            .filter_map(|child| {
                let anchor = child.text_edge?;
                let bounds = self.stack_bounds(anchor, window_to_flow)?;
                let edge = if trailing {
                    match self.inputs.direction {
                        crate::TextDirection::LeftToRight => bounds.x + bounds.width,
                        crate::TextDirection::RightToLeft => bounds.x,
                    }
                } else {
                    self.start_edge(bounds)
                };
                Some((anchor, edge))
            })
            .collect::<Vec<_>>();
        for (index, &(left_anchor, left)) in starts.iter().enumerate() {
            for &(right_anchor, right) in &starts[index + 1..] {
                let delta = (left - right).abs();
                if left_anchor.kind == right_anchor.kind && delta > EPSILON && delta < NEAR_MISS_MAX
                {
                    let left_name = alignment_anchor_name(self.inputs.tree, left_anchor.node);
                    let right_name = alignment_anchor_name(self.inputs.tree, right_anchor.node);
                    self.report(
                        AuditRule::TextEdgeMisaligned,
                        id,
                        path,
                        format!("stacked text {} differ by {delta:.1}px: {left_name} at {left:.1}px and {right_name} at {right:.1}px",
                            if trailing {"ends"} else {"starts"}),
                    );
                    return;
                }
            }
        }
    }

    fn check_action_group(&mut self, children: &[NodeSummary], id: Option<NodeId>, path: &str) {
        let solid = children.iter().filter(|child| child.solid_action).count();
        if solid > 1 {
            self.report(
                AuditRule::MultipleSolidActions,
                id,
                path,
                format!("{solid} solid actions share one group"),
            );
        }
    }

    fn check_inline_type<'a>(
        &mut self,
        children: impl Iterator<Item = &'a NodeSummary>,
        window_to_flow: Option<crate::Affine2D>,
        id: Option<NodeId>,
        path: &str,
    ) {
        // Markers and controls carry their own type scale. Their text is
        // not a peer label of the enclosing row.
        let samples = children
            .filter(|child| {
                !child.contains_control
                    && !child
                        .role
                        .as_deref()
                        .is_some_and(|role| MARKER_ROLES.contains(&role))
            })
            .filter_map(|child| child.text_sample);
        let mut lanes: [Vec<_>; 4] = std::array::from_fn(|_| Vec::new());
        for sample in samples {
            let lane = match sample.voice {
                TypeVoice::Inline => 0,
                TypeVoice::Display => 1,
                TypeVoice::Metadata => 2,
                TypeVoice::Monogram => 3,
                TypeVoice::Annotation => continue,
            };
            lanes[lane].push((
                self.geometry_in_flow(sample.geometry, window_to_flow),
                centi_pixels(sample.size),
            ));
        }
        for positions in lanes {
            for band in flow_bands(positions, FlexDirection::Column) {
                let sizes = band.into_iter().collect::<BTreeSet<_>>();
                if sizes.len() > 1 {
                    let sizes = sizes
                        .iter()
                        .map(|size| format!("{}px", f64::from(*size) / 100.0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.report(
                        AuditRule::MixedTypeInRow,
                        id,
                        path,
                        format!("one row mixes font sizes {sizes}"),
                    );
                }
            }
        }
    }
}

/// Font sizes in hundredths of a pixel, so sizes compare exactly.
#[allow(clippy::cast_possible_truncation)]
fn centi_pixels(size: f64) -> i32 {
    (size * 100.0)
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

fn alpha_of(color: Rgba8) -> f64 {
    f64::from(u8::try_from(color.as_rgba_hex() & 0xff).unwrap_or(0)) / 255.0
}

/// Whether content starts with a heading or a view switcher that plays the
/// heading's part. A decorative icon can prefix that heading; preceding
/// content and controls still prevent the heading from leading the group.
fn leads_with_heading(mut children: &[UiNode]) -> bool {
    while let Some(node) = children.iter().find(|node| !inert_pictogram(node)) {
        if node.attributes().get("accessibility_hidden") == Some(&UiValue::Bool(true)) {
            return false;
        }
        if matches!(
            node.attributes().get("role"),
            Some(UiValue::String(role)) if role == "heading" || role == "tablist"
        ) {
            return true;
        }
        match node.kind() {
            UiNodeKind::Box { children: nested } | UiNodeKind::Fragment { children: nested } => {
                children = nested;
            }
            _ => break,
        }
    }
    false
}

fn inert_pictogram(node: &UiNode) -> bool {
    matches!(
        node.kind(),
        UiNodeKind::Svg { .. } | UiNodeKind::Image { .. } | UiNodeKind::DirectionalImage { .. }
    ) && node.attributes().get("accessibility_hidden") == Some(&UiValue::Bool(true))
        && node.attributes().get("tab_stop") != Some(&UiValue::Bool(true))
        && node.handlers().is_empty()
}

fn outside_tree_chrome(owner: LayoutOwner) -> LayoutOwner {
    match owner {
        LayoutOwner::TreeItemChrome => LayoutOwner::Composition,
        other => other,
    }
}

fn alignment_anchor_name(tree: &RetainedUiTree, id: NodeId) -> String {
    let mut current = Some(id);
    while let Some(node) = current.and_then(|node| tree.node(node)) {
        if let Some(key) = node.key() {
            return format!("node {id} ({key})");
        }
        current = node.parent();
    }
    format!("node {id}")
}

/// A structural branch (even when clickable) still contains independently
/// composed rows. Never treat its entire subtree as the parent's chrome.
fn contains_nested_collection(tree: &RetainedUiTree, row: &crate::RetainedNode) -> bool {
    fn in_flow_children(node: &crate::RetainedNode) -> impl Iterator<Item = NodeId> + '_ {
        use crate::UiNodeKindTag;
        node.children()
            .filter(|child| {
                child.group() != "content"
                    || !matches!(node.kind(), UiNodeKindTag::Layer | UiNodeKindTag::Overlay)
            })
            .map(crate::RetainedChildLink::node)
    }
    let mut pending = in_flow_children(row).collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        let Some(node) = tree.node(node) else {
            continue;
        };
        if matches!(
            node.attributes().get("role"),
            Some(UiValue::String(role))
                if ["treeitem", "tree", "list", "listbox", "menu", "menubar", "tablist", "grid", "table"]
                    .contains(&role.as_str())
        ) || node.kind() == crate::UiNodeKindTag::VirtualCollection
        {
            return true;
        }
        pending.extend(in_flow_children(node));
    }
    false
}

/// A container whose heading the application draws elsewhere (a Region with
/// `external_title`, set through `.heading_elsewhere()`).
fn introduced_elsewhere(node: &UiNode) -> bool {
    node.attributes().get("heading_elsewhere") == Some(&UiValue::Bool(true))
}

fn axis_extent(bounds: GeometryBounds, axis: FlexDirection) -> (f64, f64) {
    match axis {
        FlexDirection::Row => (bounds.x, bounds.x + bounds.width),
        FlexDirection::Column => (bounds.y, bounds.y + bounds.height),
    }
}

fn cross_axis(direction: FlexDirection) -> FlexDirection {
    match direction {
        FlexDirection::Row => FlexDirection::Column,
        FlexDirection::Column => FlexDirection::Row,
    }
}

/// Sort physical bands along the cross-axis, joining overlapping footprints.
/// If a footprint has not been presented, keep the conservative old
/// single-band check rather than inventing a separate line.
fn flow_bands<T>(mut values: Vec<(Option<GeometryBounds>, T)>, axis: FlexDirection) -> Vec<Vec<T>> {
    if values.is_empty() {
        return Vec::new();
    }
    if values.iter().any(|(bounds, _)| bounds.is_none()) {
        return vec![values.into_iter().map(|(_, value)| value).collect()];
    }
    values.sort_by(|(a, _), (b, _)| {
        axis_extent(a.expect("presented footprint"), axis)
            .0
            .total_cmp(&axis_extent(b.expect("presented footprint"), axis).0)
    });
    let mut bands: Vec<Vec<T>> = Vec::new();
    let mut end = f64::NEG_INFINITY;
    for (bounds, value) in values {
        let (start, stop) = axis_extent(bounds.expect("presented footprint"), axis);
        if start >= end {
            bands.push(Vec::new());
        }
        end = end.max(stop);
        bands.last_mut().expect("active inline band").push(value);
    }
    bands
}

/// Carry the first control and text of a subtree up to the parent.
fn inherit_first(summary: &mut NodeSummary, children: &[NodeSummary], role: Option<&str>) {
    let inside_marker = role.is_some_and(|role| MARKER_ROLES.contains(&role));
    for child in children {
        summary.contains_control |= child.contains_control;
        if summary.control_geometry.is_none() && child.control_geometry.is_some() {
            summary.control_geometry = child.control_geometry;
            summary.control_family.clone_from(&child.control_family);
            summary.solid_action = child.solid_action;
        }
        if summary.text_sample.is_none() && !inside_marker {
            summary.text_sample = child.text_sample;
        }
        if summary.text_edge.is_none() {
            summary.text_edge = child.text_edge;
        }
        if summary.text_ink_edge.is_none() {
            summary.text_ink_edge = child.text_ink_edge;
        }
    }
    if summary.bounds.is_none() {
        summary.bounds = children.iter().find_map(|child| child.bounds);
    }
    // One wrapped action still owns a policy slot even if it contributed no
    // layout peer (for example when the action floats inside its wrapper).
    if let [child] = children
        && !role.is_some_and(|role| CONTROL_ROLES.contains(&role))
    {
        summary.solid_action |= child.solid_action;
    }
}

fn child_link(
    tree: &RetainedUiTree,
    parent: Option<NodeId>,
    group: &str,
    index: usize,
) -> Option<NodeId> {
    tree.node(parent?)?
        .children()
        .filter(|child| child.group() == group)
        .nth(index)
        .map(crate::RetainedChildLink::node)
}

/// An application profile: the composition rules it enables at runtime and
/// the static rules `gpui-rhai check` applies to application source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub audit: AuditRules,
    pub static_rules: BTreeSet<String>,
}

/// Static rule identifiers understood by `gpui-rhai check`.
pub const STATIC_RULES: &[&str] = &["literal-geometry"];

/// Load `profile() -> #{ name, audit: [...], check: [...] }`.
///
/// # Errors
///
/// Returns a message for script errors, unknown fields or rule identifiers.
pub fn load_profile_source(
    engine: &rhai::Engine,
    source_name: &str,
    source: &str,
) -> Result<Profile, String> {
    let ast = engine
        .compile(source)
        .map_err(|error| format!("{source_name}: {error}"))?;
    let value: rhai::Dynamic = engine
        .call_fn(&mut rhai::Scope::new(), &ast, "profile", ())
        .map_err(|error| format!("{source_name}: {error}"))?;
    let mut map = value
        .try_cast::<rhai::Map>()
        .ok_or_else(|| format!("{source_name}: profile() must return a map"))?;
    let strings = |value: Option<rhai::Dynamic>, field: &str| -> Result<Vec<String>, String> {
        value
            .map(|value| {
                value
                    .into_typed_array::<rhai::ImmutableString>()
                    .map(|items| items.into_iter().map(|item| item.to_string()).collect())
                    .map_err(|_| format!("{source_name}: `{field}` must be an array of strings"))
            })
            .transpose()
            .map(Option::unwrap_or_default)
    };
    let name = map.remove("name").map_or_else(Default::default, |name| {
        name.into_string().unwrap_or_default()
    });
    let audit = strings(map.remove("audit"), "audit")?;
    let audit = AuditRules::from_ids(audit.iter().map(String::as_str))
        .map_err(|id| format!("{source_name}: unknown audit rule `{id}`"))?;
    let static_rules = strings(map.remove("check"), "check")?
        .into_iter()
        .map(|rule| {
            if STATIC_RULES.contains(&rule.as_str()) {
                Ok(rule)
            } else {
                Err(format!("{source_name}: unknown check rule `{rule}`"))
            }
        })
        .collect::<Result<_, _>>()?;
    if let Some(field) = map.keys().next() {
        return Err(format!("{source_name}: unknown profile field `{field}`"));
    }
    Ok(Profile {
        name,
        audit,
        static_rules,
    })
}

/// Group findings by rule for reports.
#[must_use]
pub fn summarize(findings: &[AuditFinding]) -> BTreeMap<&'static str, usize> {
    let mut summary = BTreeMap::new();
    for finding in findings {
        *summary.entry(finding.rule.id()).or_default() += 1;
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_load_rule_sets_and_reject_unknown_rules() {
        let engine = rhai::Engine::new();
        let profile = load_profile_source(
            &engine,
            "profile.rhai",
            r#"fn profile() { #{ name: "productivity",
                audit: ["row-height-mismatch", "low-contrast-text"],
                check: ["literal-geometry"] } }"#,
        )
        .unwrap();
        assert_eq!(profile.name, "productivity");
        assert!(profile.audit.contains(AuditRule::LowContrastText));
        assert!(!profile.audit.contains(AuditRule::MixedTypeInRow));
        assert!(profile.static_rules.contains("literal-geometry"));
        assert!(
            load_profile_source(&engine, "p.rhai", r#"fn profile() { #{ audit: ["x"] } }"#)
                .is_err()
        );
        assert!(load_profile_source(&engine, "p.rhai", "fn profile() { #{ extra: 1 } }").is_err());
    }

    #[test]
    fn rule_ids_round_trip_and_unknown_ids_are_rejected() {
        for rule in AuditRule::ALL {
            assert_eq!(AuditRule::parse(rule.id()), Some(rule));
        }
        assert_eq!(
            AuditRules::from_ids(["row-height-mismatch", "nope"]),
            Err("nope".to_owned())
        );
        assert!(AuditRules::default().is_empty());
        assert!(AuditRules::all().contains(AuditRule::LowContrastText));
    }
    #[test]
    fn directly_forwarded_controls_keep_their_visual_family_not_their_wrapper_or_key() {
        let control = crate::UiNode::text("Tool")
            .with_component_root(crate::ComponentInstancePath::root("IconAction", "first"));
        let same_kind = crate::UiNode::text("Other")
            .with_component_root(crate::ComponentInstancePath::root("IconAction", "second"));
        let wrapped = control
            .clone()
            .with_component_root(crate::ComponentInstancePath::root("Outer", "a"))
            .with_component_root(crate::ComponentInstancePath::root("Another", "b"));
        assert_eq!(button_family(&control), button_family(&same_kind));
        assert_eq!(button_family(&control), button_family(&wrapped));
        assert_ne!(
            button_family(&control),
            button_family(&crate::UiNode::text("Raw"))
        );
    }
}
