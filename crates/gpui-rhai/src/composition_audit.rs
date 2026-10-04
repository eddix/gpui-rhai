//! Composition audit: checks a mounted view's committed geometry and resolved
//! styles against the rules an application profile enables.
//!
//! The audit is design-language neutral: it compares relationships (heights
//! in a row, nested gaps, painted contrast) rather than fixed values. With no
//! rules enabled it reports nothing.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ColorResolver, Environment, FlexDirection, GeometryBounds, GeometryRegistry, InteractionState,
    Justify, Length, NodeId, RetainedUiTree, Rgba8, StyleProperties, Symbol, UiNode, UiNodeKind,
    UiValue,
};

/// One composition rule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AuditRule {
    /// Sibling controls in one row with different heights.
    RowHeightMismatch,
    /// Stacked text starts that nearly, but not exactly, align.
    TextEdgeMisaligned,
    /// An inner gap greater than or equal to its enclosing gap.
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
}

#[derive(Clone, Copy)]
struct Scope {
    environment: Environment,
    disabled: bool,
    text_color: Option<Rgba8>,
    background: Rgba8,
    font_size: Option<f64>,
    font_weight: u16,
    /// Gaps of the nearest enclosing rows and columns with at least two
    /// children. Proximity only competes along one axis.
    enclosing_row_gap: Option<f64>,
    enclosing_column_gap: Option<f64>,
    /// Inside a control: its internal layout is the component's own
    /// responsibility, so composition rules other than contrast stop here.
    inside_control: bool,
}

/// What the audit learned about one rendered node, for its parent's checks.
#[derive(Clone, Debug, Default)]
struct NodeSummary {
    role: Option<String>,
    bounds: Option<GeometryBounds>,
    /// Height of the first control in this subtree.
    control_height: Option<f64>,
    /// Font size of the first text in this subtree, unless inside a marker.
    text_size: Option<f64>,
    /// Start x of the first text in this subtree.
    text_start: Option<f64>,
    /// Whether the first control in this subtree is a solid action.
    solid_action: bool,
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
        background,
        font_size: None,
        font_weight: 400,
        enclosing_row_gap: None,
        enclosing_column_gap: None,
        inside_control: false,
    };
    let solid = solid_colors(inputs.theme);
    let mut reported_fonts = BTreeSet::new();
    let mut walker = Walker {
        inputs,
        solid,
        allowed: BTreeSet::new(),
        findings: &mut findings,
        reported_fonts: &mut reported_fonts,
    };
    walker.visit(root, Some(root_id), scope, "root");
    findings
}

fn root_node_id(tree: &RetainedUiTree) -> Option<NodeId> {
    tree.nodes()
        .find(|node| node.parent().is_none())
        .map(crate::RetainedNode::id)
}

fn solid_colors(theme: &impl ColorResolver) -> BTreeSet<u32> {
    ["accent", "danger", "warning", "success"]
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
}

impl<C: ColorResolver> Walker<'_, '_, C> {
    fn report(&mut self, rule: AuditRule, node: Option<NodeId>, path: &str, message: String) {
        if self.inputs.rules.contains(rule) && !self.allowed.contains(&(path.to_owned(), rule)) {
            self.findings.push(AuditFinding {
                rule,
                node,
                path: path.to_owned(),
                message,
            });
        }
    }

    fn pixels(&self, length: Option<Length>, environment: &Environment) -> Option<f64> {
        match self.inputs.theme.resolve_length_in(length?, environment)? {
            Length::Pixels(value) => Some(value),
            Length::Rems(value) => Some(value * 16.0),
            Length::Relative(_) | Length::Token(_) => None,
        }
    }

    #[allow(clippy::too_many_lines)] // One ordered pass per node keeps scope inheritance readable.
    fn visit(
        &mut self,
        node: &UiNode,
        id: Option<NodeId>,
        parent: Scope,
        path: &str,
    ) -> NodeSummary {
        let retained = id.and_then(|id| self.inputs.tree.node(id));
        let style = node.style().resolve(&InteractionState::default());
        let mut scope = parent;
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
        self.apply_paint(&style, &mut scope);
        self.apply_text(&style, &mut scope, id, path);

        let role = retained
            .and_then(|node| node.attributes().get("role"))
            .and_then(|role| match role {
                UiValue::String(role) => Some(role.clone()),
                _ => None,
            });
        let bounds = id
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
        if let Some(role) = &role
            && CONTROL_ROLES.contains(&role.as_str())
        {
            summary.control_height = bounds.map(|bounds| bounds.height);
            summary.solid_action = role == "button"
                && style
                    .background
                    .as_ref()
                    .and_then(|color| self.inputs.theme.resolve(color))
                    .is_some_and(|color| {
                        color.as_rgba_hex() & 0xff == 0xff
                            && self.solid.contains(&color.as_rgba_hex())
                    });
        }

        match node.kind() {
            UiNodeKind::Text { .. } | UiNodeKind::RichText { .. } => {
                summary.text_size = scope.font_size;
                summary.text_start = bounds.map(|bounds| bounds.x);
                self.check_contrast(&scope, id, path);
            }
            // A native control renders its own text at the inherited size.
            UiNodeKind::Custom { .. } if style.typography.is_some() => {
                summary.text_size = scope.font_size;
            }
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                let direction = style.direction.unwrap_or(FlexDirection::Column);
                let gap = self.pixels(style.gap, &scope.environment);
                // Distributed rows treat the gap as a floor, not a relationship.
                let distributed = matches!(style.justify, Some(Justify::Between | Justify::Around));
                let counted = !scope.inside_control
                    && !distributed
                    && children.len() >= 2
                    && gap.is_some_and(|gap| gap > 0.0);
                let enclosing = match direction {
                    FlexDirection::Row => parent.enclosing_row_gap,
                    FlexDirection::Column => parent.enclosing_column_gap,
                };
                if counted
                    && let (Some(inner), Some(outer)) = (gap, enclosing)
                    && inner + EPSILON >= outer
                {
                    self.report(
                        AuditRule::SpacingNotNested,
                        id,
                        path,
                        format!(
                            "inner gap {inner}px is not smaller than the enclosing gap {outer}px"
                        ),
                    );
                }
                // A heading leads its content: the heading gap is a typographic
                // relationship, so the content keeps the outer gap as its limit.
                let sets_limit = counted && !leads_with_heading(children);
                let mut child_scope = Scope {
                    inside_control: scope.inside_control || is_control,
                    ..scope
                };
                if sets_limit {
                    match direction {
                        FlexDirection::Row => child_scope.enclosing_row_gap = gap,
                        FlexDirection::Column => child_scope.enclosing_column_gap = gap,
                    }
                }
                let summaries = children
                    .iter()
                    .enumerate()
                    .map(|(index, child)| {
                        let child_id = child_link(self.inputs.tree, id, "children", index);
                        let child_path = child.key().map_or_else(
                            || format!("{path}/{index}"),
                            |key| format!("{path}/{}", key.as_str()),
                        );
                        self.visit(child, child_id, child_scope, &child_path)
                    })
                    .collect::<Vec<_>>();
                if !scope.inside_control && !is_control {
                    self.check_container(direction, &summaries, id, path);
                }
                inherit_first(&mut summary, &summaries, role.as_deref());
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                let trigger_id = child_link(self.inputs.tree, id, "trigger", 0);
                let trigger = self.visit(trigger, trigger_id, scope, &format!("{path}/trigger"));
                let content_id = child_link(self.inputs.tree, id, "content", 0);
                self.visit(content, content_id, scope, &format!("{path}/content"));
                inherit_first(&mut summary, &[trigger], role.as_deref());
            }
            _ => {}
        }
        // Controls and markers align by their own edge; their inner padding is
        // the component's business.
        if role
            .as_deref()
            .is_some_and(|role| CONTROL_ROLES.contains(&role) || MARKER_ROLES.contains(&role))
        {
            summary.text_start = bounds.map(|bounds| bounds.x).or(summary.text_start);
        }
        summary
    }

    fn apply_paint(&self, style: &StyleProperties, scope: &mut Scope) {
        if let Some(color) = style
            .background
            .as_ref()
            .and_then(|color| self.inputs.theme.resolve(color))
        {
            let alpha = alpha_of(color);
            if alpha > 0.0 {
                scope.background = scope.background.mix(opaque(color), alpha);
            }
        }
        if let Some(color) = style
            .text_color
            .as_ref()
            .and_then(|color| self.inputs.theme.resolve(color))
        {
            scope.text_color = Some(color);
        }
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
        if scope.disabled {
            return;
        }
        let Some(text) = scope.text_color else {
            return;
        };
        let size = scope.font_size.unwrap_or(16.0);
        let large = size >= 24.0 || (size >= 18.66 && scope.font_weight >= 700);
        let minimum = if large { 3.0 } else { 4.5 };
        let painted = scope.background.mix(opaque(text), alpha_of(text));
        let ratio = painted.contrast_ratio(scope.background);
        if ratio + 0.005 < minimum {
            self.report(
                AuditRule::LowContrastText,
                id,
                path,
                format!("text contrast {ratio:.2}:1 is below {minimum}:1"),
            );
        }
    }

    fn check_container(
        &mut self,
        direction: FlexDirection,
        children: &[NodeSummary],
        id: Option<NodeId>,
        path: &str,
    ) {
        match direction {
            FlexDirection::Row => {
                let heights = children
                    .iter()
                    .filter_map(|child| child.control_height)
                    .collect::<Vec<_>>();
                if let (Some(min), Some(max)) = (
                    heights.iter().copied().reduce(f64::min),
                    heights.iter().copied().reduce(f64::max),
                ) && max - min > EPSILON
                {
                    self.report(
                        AuditRule::RowHeightMismatch,
                        id,
                        path,
                        format!("controls in one row range from {min}px to {max}px high"),
                    );
                }
                let sizes = children
                    .iter()
                    .filter(|child| {
                        !child
                            .role
                            .as_deref()
                            .is_some_and(|role| MARKER_ROLES.contains(&role))
                    })
                    .filter_map(|child| child.text_size)
                    .map(centi_pixels)
                    .collect::<BTreeSet<_>>();
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
            FlexDirection::Column => {
                let starts = children
                    .iter()
                    .filter_map(|child| child.text_start)
                    .collect::<Vec<_>>();
                for (index, left) in starts.iter().enumerate() {
                    for right in &starts[index + 1..] {
                        let delta = (left - right).abs();
                        if delta > EPSILON && delta < NEAR_MISS_MAX {
                            self.report(
                                AuditRule::TextEdgeMisaligned,
                                id,
                                path,
                                format!("stacked text starts differ by {delta:.1}px"),
                            );
                            return;
                        }
                    }
                }
            }
        }
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

fn opaque(color: Rgba8) -> Rgba8 {
    Rgba8::from_rgba_hex(color.as_rgba_hex() | 0xff)
}

/// Whether the first child subtree starts with a heading.
fn leads_with_heading(children: &[UiNode]) -> bool {
    let mut cursor = children.first();
    while let Some(node) = cursor {
        if node.attributes().get("role") == Some(&UiValue::String("heading".to_owned())) {
            return true;
        }
        cursor = match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => children.first(),
            _ => None,
        };
    }
    false
}

/// Carry the first control and text of a subtree up to the parent.
fn inherit_first(summary: &mut NodeSummary, children: &[NodeSummary], role: Option<&str>) {
    let inside_marker = role.is_some_and(|role| MARKER_ROLES.contains(&role));
    for child in children {
        if summary.control_height.is_none() && child.control_height.is_some() {
            summary.control_height = child.control_height;
            summary.solid_action = child.solid_action;
        }
        if summary.text_size.is_none() && !inside_marker {
            summary.text_size = child.text_size;
        }
        if summary.text_start.is_none() {
            summary.text_start = child.text_start;
        }
    }
    if summary.bounds.is_none() {
        summary.bounds = children.iter().find_map(|child| child.bounds);
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
    let name = map
        .remove("name")
        .map(|name| name.into_string().unwrap_or_default())
        .unwrap_or_default();
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
}
