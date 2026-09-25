use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, FLOAT, FnPtr, FuncRegistration, INT,
    ImmutableString, Map, NativeCallContext, Position, TypeBuilder,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    ComponentIncarnation, ComponentInstancePath, EventSchema, ScriptCallback, ScriptGeneration,
    UiNode, UiNodeKind,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionProperty {
    Opacity,
    TranslateX,
    TranslateY,
    Rotate,
    ScaleX,
    ScaleY,
    SkewX,
    SkewY,
    Width,
    Height,
    ClipHeight,
    PathProgress,
}

impl MotionProperty {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opacity => "opacity",
            Self::TranslateX => "translate_x",
            Self::TranslateY => "translate_y",
            Self::Rotate => "rotate",
            Self::ScaleX => "scale_x",
            Self::ScaleY => "scale_y",
            Self::SkewX => "skew_x",
            Self::SkewY => "skew_y",
            Self::Width => "width",
            Self::Height => "height",
            Self::ClipHeight => "clip_height",
            Self::PathProgress => "path_progress",
        }
    }

    fn parse(value: &str) -> Result<Self, MotionError> {
        match value {
            "opacity" => Ok(Self::Opacity),
            "translate_x" => Ok(Self::TranslateX),
            "translate_y" => Ok(Self::TranslateY),
            "rotate" => Ok(Self::Rotate),
            "scale_x" => Ok(Self::ScaleX),
            "scale_y" => Ok(Self::ScaleY),
            "skew_x" => Ok(Self::SkewX),
            "skew_y" => Ok(Self::SkewY),
            "width" => Ok(Self::Width),
            "height" => Ok(Self::Height),
            "clip_height" => Ok(Self::ClipHeight),
            "path_progress" => Ok(Self::PathProgress),
            _ => Err(MotionError::UnknownProperty(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionIntent {
    Decorative,
    Feedback,
    Essential,
}

impl MotionIntent {
    fn parse(value: &str) -> Result<Self, MotionError> {
        match value {
            "decorative" => Ok(Self::Decorative),
            "feedback" => Ok(Self::Feedback),
            "essential" => Ok(Self::Essential),
            _ => Err(MotionError::UnknownIntent(value.to_owned())),
        }
    }
}

impl MotionEasing {
    /// Parse the stable script spelling of an easing.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownEasing`] for an unsupported name.
    pub fn parse(value: &str) -> Result<Self, MotionError> {
        match value {
            "linear" => Ok(Self::Linear),
            "ease_in" => Ok(Self::EaseIn),
            "ease_out" => Ok(Self::EaseOut),
            "ease_in_out" => Ok(Self::EaseInOut),
            _ => Err(MotionError::UnknownEasing(value.to_owned())),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease_in",
            Self::EaseOut => "ease_out",
            Self::EaseInOut => "ease_in_out",
        }
    }

    /// Sample the stable easing curve at a clamped normalized progress value.
    #[must_use]
    pub fn sample(self, progress: f64) -> f64 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
            Self::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionTransition {
    pub property: MotionProperty,
    pub from: f64,
    pub to: f64,
    pub delay_ms: u64,
    pub duration_ms: u64,
    pub easing: MotionEasing,
    pub iterations: Option<u32>,
    pub autoreverse: bool,
    pub intent: MotionIntent,
}

impl MotionTransition {
    #[must_use]
    pub const fn new(property: MotionProperty, from: f64, to: f64, duration_ms: u64) -> Self {
        Self {
            property,
            from,
            to,
            delay_ms: 0,
            duration_ms,
            easing: MotionEasing::EaseOut,
            iterations: Some(1),
            autoreverse: false,
            intent: MotionIntent::Feedback,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionSpring {
    pub property: MotionProperty,
    pub from: f64,
    pub to: f64,
    pub initial_velocity: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
    pub intent: MotionIntent,
}

impl MotionSpring {
    #[must_use]
    pub const fn new(property: MotionProperty, from: f64, to: f64) -> Self {
        Self {
            property,
            from,
            to,
            initial_velocity: 0.0,
            stiffness: 180.0,
            damping: 24.0,
            mass: 1.0,
            intent: MotionIntent::Feedback,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionKeyframe {
    pub offset: f64,
    pub value: f64,
    pub easing: MotionEasing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionKeyframes {
    pub property: MotionProperty,
    pub frames: Vec<MotionKeyframe>,
    pub delay_ms: u64,
    pub duration_ms: u64,
    pub iterations: Option<u32>,
    pub autoreverse: bool,
    pub intent: MotionIntent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionInertia {
    pub property: MotionProperty,
    pub from: f64,
    pub velocity: f64,
    pub friction: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub bounce: f64,
    pub snap_points: Vec<f64>,
    pub intent: MotionIntent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MotionSource {
    Transition(MotionTransition),
    Spring(MotionSpring),
    Keyframes(MotionKeyframes),
    Inertia(MotionInertia),
}

impl MotionSource {
    #[must_use]
    pub const fn property(&self) -> MotionProperty {
        match self {
            Self::Transition(spec) => spec.property,
            Self::Spring(spec) => spec.property,
            Self::Keyframes(spec) => spec.property,
            Self::Inertia(spec) => spec.property,
        }
    }

    #[must_use]
    pub fn target(&self) -> f64 {
        source_terminal_value(self)
    }

    #[must_use]
    pub fn reduced_value(&self) -> f64 {
        match self {
            Self::Transition(spec) if spec.iterations.is_none() => {
                f64::midpoint(spec.from, spec.to)
            }
            Self::Transition(spec) => spec.to,
            Self::Spring(spec) => spec.to,
            Self::Keyframes(spec) if spec.iterations.is_none() => spec
                .frames
                .first()
                .zip(spec.frames.last())
                .map_or(0.0, |(first, last)| f64::midpoint(first.value, last.value)),
            Self::Keyframes(spec) => spec.frames.last().map_or(0.0, |frame| frame.value),
            Self::Inertia(spec) => inertia_target(spec),
        }
    }

    #[must_use]
    pub const fn intent(&self) -> MotionIntent {
        match self {
            Self::Transition(spec) => spec.intent,
            Self::Spring(spec) => spec.intent,
            Self::Keyframes(spec) => spec.intent,
            Self::Inertia(spec) => spec.intent,
        }
    }
}

impl CustomType for MotionSource {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionSource");
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionTrack {
    pub target: String,
    pub source: MotionSource,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MotionTimelineStep {
    Track(MotionTrack),
    Delay(u64),
    Sequence(Vec<Self>),
    Parallel(Vec<Self>),
    Stagger { interval_ms: u64, steps: Vec<Self> },
}

impl CustomType for MotionTimelineStep {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionTimelineStep");
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionTimeline {
    pub name: String,
    pub root: MotionTimelineStep,
    pub autoplay: bool,
    pub iterations: Option<u32>,
    pub autoreverse: bool,
    pub intent: MotionIntent,
    pub(crate) on_complete: Option<ScriptCallback>,
    pub(crate) on_cancel: Option<ScriptCallback>,
}

impl MotionTimeline {
    #[must_use]
    pub fn new(name: impl Into<String>, root: MotionTimelineStep) -> Self {
        Self {
            name: name.into(),
            root,
            autoplay: true,
            iterations: Some(1),
            autoreverse: false,
            intent: MotionIntent::Decorative,
            on_complete: None,
            on_cancel: None,
        }
    }

    pub(crate) fn bind_generation(&mut self, generation: ScriptGeneration) {
        for callback in [&mut self.on_complete, &mut self.on_cancel]
            .into_iter()
            .flatten()
        {
            callback.bind_generation(generation);
        }
    }

    pub(crate) fn bind_component_scope(
        &mut self,
        component: &ComponentInstancePath,
        incarnation: ComponentIncarnation,
        events: &BTreeMap<String, EventSchema>,
        native_context: Option<&crate::invocation::ScriptInvocationContext>,
    ) {
        for callback in [&mut self.on_complete, &mut self.on_cancel]
            .into_iter()
            .flatten()
        {
            callback.bind_component_scope_if_unset(component, incarnation, events.clone());
            if let Some(context) = native_context {
                callback.bind_native_context_if_unset(context.clone());
            }
        }
    }
}

impl CustomType for MotionTimeline {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionTimeline");
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MotionHandle {
    runtime_id: u64,
    instance: u64,
    domain: String,
    owner: ComponentInstancePath,
    incarnation: ComponentIncarnation,
    generation: ScriptGeneration,
    node_path: String,
    name: String,
}

impl MotionHandle {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn owner(&self) -> &ComponentInstancePath {
        &self.owner
    }

    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    fn in_node_scope(&self, path: &str) -> bool {
        self.node_path == path
            || self
                .node_path
                .strip_prefix(path)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }
}

impl CustomType for MotionHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionHandle");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionPlaybackState {
    Idle,
    Playing,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionTimelineEventKind {
    Complete,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionTimelineEvent {
    pub handle: MotionHandle,
    pub kind: MotionTimelineEventKind,
    pub callback: Option<ScriptCallback>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MotionGhost {
    pub id: String,
    pub node: UiNode,
    pub bounds: crate::GeometryBounds,
    pub path: String,
    pub domain: String,
}

#[derive(Clone, Debug)]
struct ScheduledMotionTrack {
    target: String,
    resolved_path: Option<String>,
    source: MotionSource,
    start_ms: u64,
    duration_ms: u64,
}

#[derive(Clone, Debug)]
struct ActiveTimeline {
    spec: MotionTimeline,
    tracks: Vec<ScheduledMotionTrack>,
    duration_ms: u64,
    state: MotionPlaybackState,
    started: Instant,
    elapsed_before_play: Duration,
    policy_settled: Option<MotionPreference>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TimelineLogicalKey {
    domain: String,
    node_path: String,
    name: String,
}

#[derive(Clone, Debug)]
struct TimelineOwner {
    domain: String,
    component: ComponentInstancePath,
    incarnation: ComponentIncarnation,
    generation: ScriptGeneration,
    node_path: String,
    target_root: String,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MotionKey {
    pub component: ComponentInstancePath,
    pub property: MotionProperty,
}

impl MotionKey {
    #[must_use]
    pub fn for_node(path: &str, property: MotionProperty) -> Self {
        Self {
            component: ComponentInstancePath::root("UiNode", path),
            property,
        }
    }

    fn in_node_scope(&self, path: &str) -> bool {
        self.component
            .single_root_key("UiNode")
            .is_some_and(|node_path| {
                node_path == path
                    || node_path
                        .strip_prefix(path)
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionPreference {
    Normal,
    Reduced,
    None,
}

const fn stricter_preference(left: MotionPreference, right: MotionPreference) -> MotionPreference {
    match (left, right) {
        (MotionPreference::None, _) | (_, MotionPreference::None) => MotionPreference::None,
        (MotionPreference::Reduced, _) | (_, MotionPreference::Reduced) => {
            MotionPreference::Reduced
        }
        (MotionPreference::Normal, MotionPreference::Normal) => MotionPreference::Normal,
    }
}

fn motion_domain_from_path(path: &str) -> String {
    path.find("/root")
        .map_or_else(|| "root".to_owned(), |end| path[..end + 5].to_owned())
}

fn encode_path_segment(kind: &str, value: &str) -> String {
    let mut encoded = String::with_capacity(kind.len() + 1 + value.len() * 2);
    encoded.push_str(kind);
    encoded.push(':');
    for byte in value.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

pub(crate) fn retained_node_path(path: &str, node: crate::NodeId) -> String {
    format!("{}/node:{}", motion_domain_from_path(path), node.get())
}

pub(crate) fn virtual_item_path(path: &str, key: &str) -> String {
    format!("{path}/{}", encode_path_segment("item", key))
}

pub(crate) fn span_motion_path(path: &str, key: &str) -> String {
    format!("{path}/{}", encode_path_segment("span", key))
}

fn path_contains_scope(path: &str, scope: &str) -> bool {
    scope == path
        || scope
            .strip_prefix(path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionQuality {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MotionProgressDriver {
    InView,
    Viewport,
    ScrollX,
    ScrollY,
    Hover,
    Press,
    Focus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionProgressBinding {
    pub driver: MotionProgressDriver,
    pub source: MotionSource,
}

impl MotionProgressBinding {
    /// Construct a geometry/scroll-driven property source.
    ///
    /// # Errors
    ///
    /// Only transition and keyframe mappings are valid progress sources.
    pub fn new(driver: MotionProgressDriver, source: MotionSource) -> Result<Self, MotionError> {
        validate_progress_source(&source)?;
        Ok(Self { driver, source })
    }

    #[must_use]
    pub const fn property(&self) -> MotionProperty {
        self.source.property()
    }
}

impl CustomType for MotionProgressBinding {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("MotionProgressBinding");
    }
}

#[derive(Clone, Debug)]
struct ActiveMotion {
    replay_key: Option<String>,
    declaration: MotionSource,
    source: MotionSource,
    finite_duration_ms: Option<u64>,
    started: Instant,
    elapsed_before_play: Duration,
    retention: MotionRetention,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MotionRetention {
    Host,
    Live(String),
    Exit,
}

#[derive(Debug)]
pub struct MotionRuntime {
    runtime_id: u64,
    next_timeline_instance: u64,
    active: BTreeMap<MotionKey, ActiveMotion>,
    settled: BTreeMap<MotionKey, f64>,
    settled_declaration: BTreeMap<MotionKey, (MotionSource, Option<String>)>,
    settled_retention: BTreeMap<MotionKey, MotionRetention>,
    timelines: BTreeMap<MotionHandle, ActiveTimeline>,
    timeline_index: BTreeMap<TimelineLogicalKey, MotionHandle>,
    timeline_events: Vec<MotionTimelineEvent>,
    suspended_scopes: BTreeSet<String>,
    active_reservations: BTreeMap<String, usize>,
    live_scene_signatures: BTreeMap<String, String>,
    active_limit: usize,
    host_preference: Option<MotionPreference>,
    preference: Option<MotionPreference>,
    quality: Option<MotionQuality>,
}

impl Default for MotionRuntime {
    fn default() -> Self {
        static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            runtime_id: NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed),
            next_timeline_instance: 1,
            active: BTreeMap::new(),
            settled: BTreeMap::new(),
            settled_declaration: BTreeMap::new(),
            settled_retention: BTreeMap::new(),
            timelines: BTreeMap::new(),
            timeline_index: BTreeMap::new(),
            timeline_events: Vec::new(),
            suspended_scopes: BTreeSet::new(),
            active_reservations: BTreeMap::new(),
            live_scene_signatures: BTreeMap::new(),
            active_limit: usize::MAX,
            host_preference: None,
            preference: None,
            quality: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionSnapshot {
    pub key: MotionKey,
    pub kind: String,
    pub value: f64,
    pub target: f64,
    pub velocity: Option<f64>,
    pub elapsed_ms: u64,
    pub duration_ms: Option<u64>,
    pub repeating: bool,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MotionTimelineSnapshot {
    pub handle: MotionHandle,
    pub state: MotionPlaybackState,
    pub elapsed_ms: u64,
    pub duration_ms: Option<u64>,
    pub iterations: Option<u32>,
    pub autoreverse: bool,
    pub track_count: usize,
}

impl MotionRuntime {
    pub(crate) fn transaction_snapshot(&self) -> Self {
        Self {
            runtime_id: self.runtime_id,
            next_timeline_instance: self.next_timeline_instance,
            active: self.active.clone(),
            settled: self.settled.clone(),
            settled_declaration: self.settled_declaration.clone(),
            settled_retention: self.settled_retention.clone(),
            timelines: self.timelines.clone(),
            timeline_index: self.timeline_index.clone(),
            timeline_events: self.timeline_events.clone(),
            suspended_scopes: self.suspended_scopes.clone(),
            active_reservations: self.active_reservations.clone(),
            live_scene_signatures: self.live_scene_signatures.clone(),
            active_limit: self.active_limit,
            host_preference: self.host_preference,
            preference: self.preference,
            quality: self.quality,
        }
    }

    #[must_use]
    pub fn new(preference: MotionPreference) -> Self {
        Self {
            host_preference: Some(preference),
            preference: Some(preference),
            quality: Some(MotionQuality::High),
            ..Self::default()
        }
    }

    pub fn set_preference(&mut self, preference: MotionPreference) {
        self.host_preference = Some(preference);
        self.apply_preference(preference);
    }

    pub fn request_preference(&mut self, preference: MotionPreference) {
        let host = self.host_preference.unwrap_or(MotionPreference::Normal);
        self.apply_preference(stricter_preference(host, preference));
    }

    fn apply_preference(&mut self, preference: MotionPreference) {
        self.preference = Some(preference);
        if matches!(
            preference,
            MotionPreference::Reduced | MotionPreference::None
        ) {
            let active = std::mem::take(&mut self.active);
            for (key, animation) in active {
                self.settled_declaration.insert(
                    key.clone(),
                    (animation.declaration.clone(), animation.replay_key.clone()),
                );
                self.settled.insert(
                    key.clone(),
                    if preference == MotionPreference::None {
                        source_terminal_value(&animation.declaration)
                    } else {
                        reduced_value(&animation)
                    },
                );
                self.settled_retention.insert(key, animation.retention);
            }
            for (key, (declaration, _)) in &self.settled_declaration {
                self.settled.insert(
                    key.clone(),
                    if preference == MotionPreference::None {
                        source_terminal_value(declaration)
                    } else {
                        declaration.reduced_value()
                    },
                );
            }
            let handles = self
                .timelines
                .iter()
                .filter(|(_, timeline)| {
                    !matches!(
                        timeline.state,
                        MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
                    )
                })
                .map(|(handle, _)| handle.clone())
                .collect::<Vec<_>>();
            for handle in handles {
                let callback = self.timelines.get_mut(&handle).map_or_else(
                    || None,
                    |timeline| {
                        timeline.state = MotionPlaybackState::Completed;
                        timeline.elapsed_before_play = Duration::from_millis(
                            timeline_total_duration(timeline).unwrap_or(timeline.duration_ms),
                        );
                        timeline.policy_settled = Some(preference);
                        timeline.spec.on_complete.clone()
                    },
                );
                self.timeline_events.push(MotionTimelineEvent {
                    handle,
                    kind: MotionTimelineEventKind::Complete,
                    callback,
                });
            }
            for timeline in self
                .timelines
                .values_mut()
                .filter(|timeline| timeline.state == MotionPlaybackState::Completed)
            {
                timeline.elapsed_before_play = Duration::from_millis(
                    timeline_total_duration(timeline).unwrap_or(timeline.duration_ms),
                );
                timeline.policy_settled = Some(preference);
            }
        }
    }

    #[must_use]
    pub const fn preference(&self) -> MotionPreference {
        match self.preference {
            Some(preference) => preference,
            None => MotionPreference::Normal,
        }
    }

    pub fn set_quality(&mut self, quality: MotionQuality) {
        self.quality = Some(quality);
    }

    #[must_use]
    pub const fn quality(&self) -> MotionQuality {
        match self.quality {
            Some(quality) => quality,
            None => MotionQuality::High,
        }
    }

    /// Start or retarget a keyed animation at the sampled current value.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError`] for non-finite values or invalid spring
    /// parameters.
    pub fn start(
        &mut self,
        component: ComponentInstancePath,
        source: MotionSource,
        now: Instant,
    ) -> Result<MotionKey, MotionError> {
        self.start_with_replay(component, source, None, now)
    }

    pub(crate) fn start_exit(
        &mut self,
        component: ComponentInstancePath,
        source: MotionSource,
        now: Instant,
    ) -> Result<MotionKey, MotionError> {
        self.start_with_replay_retained(component, source, None, now, MotionRetention::Exit)
    }

    /// Start or retarget while including an explicit declarative replay key.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError`] for an invalid source.
    #[allow(clippy::too_many_lines)]
    pub fn start_with_replay(
        &mut self,
        component: ComponentInstancePath,
        declaration: MotionSource,
        replay_key: Option<String>,
        now: Instant,
    ) -> Result<MotionKey, MotionError> {
        self.start_with_replay_retained(
            component,
            declaration,
            replay_key,
            now,
            MotionRetention::Host,
        )
    }

    fn start_with_replay_retained(
        &mut self,
        component: ComponentInstancePath,
        declaration: MotionSource,
        replay_key: Option<String>,
        now: Instant,
        retention: MotionRetention,
    ) -> Result<MotionKey, MotionError> {
        validate_source(&declaration)?;
        let key = MotionKey {
            component,
            property: declaration.property(),
        };
        let previous_replay = self
            .active
            .get(&key)
            .map(|motion| &motion.replay_key)
            .or_else(|| self.settled_declaration.get(&key).map(|(_, replay)| replay));
        let replaying = replay_key.is_some() && previous_replay != Some(&replay_key);
        if self.active.get(&key).is_some_and(|motion| {
            motion.declaration == declaration && motion.replay_key == replay_key
        }) || self
            .settled_declaration
            .get(&key)
            .is_some_and(|settled| settled == &(declaration.clone(), replay_key.clone()))
        {
            if let Some(motion) = self.active.get_mut(&key) {
                motion.retention = retention;
            } else {
                self.settled_retention.insert(key.clone(), retention);
            }
            return Ok(key);
        }
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            self.active.remove(&key);
            self.settled_declaration
                .insert(key.clone(), (declaration.clone(), replay_key));
            self.settled_retention.insert(key.clone(), retention);
            self.settled.insert(
                key.clone(),
                if self.preference == Some(MotionPreference::None) {
                    source_terminal_value(&declaration)
                } else {
                    declaration.reduced_value()
                },
            );
            return Ok(key);
        }
        let declared_from = source_initial_value(&declaration);
        let previous = if replaying {
            None
        } else {
            self.sample_with_velocity(&key, now)
        };
        let current = previous.map_or(declared_from, |sample| sample.value);
        let source = retarget_source(
            &declaration,
            current,
            previous.and_then(|sample| sample.velocity),
        );
        validate_source(&source)?;
        let finite_duration_ms = source_duration_ms(&source).ok();
        let active_increase = usize::from(!self.active.contains_key(&key));
        self.ensure_active_capacity(active_increase)?;
        self.settled.remove(&key);
        self.settled_declaration.remove(&key);
        self.settled_retention.remove(&key);
        self.active.insert(
            key.clone(),
            ActiveMotion {
                replay_key,
                declaration,
                source,
                finite_duration_ms,
                started: now,
                elapsed_before_play: Duration::ZERO,
                retention,
            },
        );
        Ok(key)
    }

    pub fn cancel(&mut self, key: &MotionKey) {
        self.active.remove(key);
        self.settled.remove(key);
        self.settled_declaration.remove(key);
        self.settled_retention.remove(key);
    }

    /// Register or replace a named timeline in one retained node scope.
    ///
    /// # Errors
    ///
    /// Returns an invalid timeline error before changing the active registry.
    pub fn start_timeline(
        &mut self,
        component: ComponentInstancePath,
        spec: MotionTimeline,
        now: Instant,
    ) -> Result<MotionHandle, MotionError> {
        let node_path = component
            .single_root_key("UiNode")
            .unwrap_or("root")
            .to_owned();
        let owner = TimelineOwner {
            domain: motion_domain_from_path(&node_path),
            component,
            incarnation: ComponentIncarnation::unscoped(),
            generation: ScriptGeneration::default(),
            target_root: node_path.clone(),
            node_path,
        };
        let mut candidate = self.transaction_snapshot();
        let handle = candidate.start_timeline_owned(&owner, spec, now, None)?;
        *self = candidate;
        Ok(handle)
    }

    fn start_timeline_owned(
        &mut self,
        owner: &TimelineOwner,
        spec: MotionTimeline,
        now: Instant,
        identities: Option<&BTreeMap<String, String>>,
    ) -> Result<MotionHandle, MotionError> {
        validate_timeline_name(&spec.name)?;
        if spec.iterations == Some(0) {
            return Err(MotionError::InvalidTimeline(
                "timeline iterations must be greater than zero".to_owned(),
            ));
        }
        let (mut tracks, duration_ms) = compile_timeline(&spec.root)?;
        for track in &mut tracks {
            let target = resolve_timeline_target(&owner.target_root, &track.target);
            track.resolved_path = Some(
                identities
                    .and_then(|identities| identities.get(&target))
                    .cloned()
                    .unwrap_or(target),
            );
        }
        let logical = TimelineLogicalKey {
            domain: owner.domain.clone(),
            node_path: owner.node_path.clone(),
            name: spec.name.clone(),
        };
        if let Some(previous_handle) = self.timeline_index.get(&logical).cloned()
            && let Some(mut active) = self.timelines.remove(&previous_handle)
        {
            if timeline_compatible(&active.spec, &spec)
                && previous_handle.owner == owner.component
                && previous_handle.incarnation == owner.incarnation
            {
                active.tracks = tracks;
                active.duration_ms = duration_ms;
                active.spec.on_complete = spec.on_complete;
                active.spec.on_cancel = spec.on_cancel;
                if previous_handle.generation == owner.generation {
                    self.timelines.insert(previous_handle.clone(), active);
                    return Ok(previous_handle);
                }
                let handle = self.allocate_timeline_handle(owner, spec.name.clone());
                self.timelines.insert(handle.clone(), active);
                self.timeline_index.insert(logical, handle.clone());
                return Ok(handle);
            }
            if !matches!(
                active.state,
                MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
            ) {
                self.timeline_events.push(MotionTimelineEvent {
                    handle: previous_handle,
                    kind: MotionTimelineEventKind::Cancel,
                    callback: active.spec.on_cancel,
                });
            }
        }
        let handle = self.allocate_timeline_handle(owner, spec.name.clone());
        let state = if spec.autoplay {
            MotionPlaybackState::Playing
        } else {
            MotionPlaybackState::Idle
        };
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            let callback = spec.on_complete.clone();
            self.timelines.insert(
                handle.clone(),
                ActiveTimeline {
                    spec,
                    tracks,
                    duration_ms,
                    state: MotionPlaybackState::Completed,
                    started: now,
                    elapsed_before_play: Duration::from_millis(duration_ms),
                    policy_settled: self.preference,
                },
            );
            self.timeline_events.push(MotionTimelineEvent {
                handle: handle.clone(),
                kind: MotionTimelineEventKind::Complete,
                callback,
            });
            self.timeline_index.insert(logical, handle.clone());
            return Ok(handle);
        }
        if state == MotionPlaybackState::Playing {
            self.ensure_active_capacity(tracks.len())?;
        }
        self.timelines.insert(
            handle.clone(),
            ActiveTimeline {
                spec,
                tracks,
                duration_ms,
                state,
                started: now,
                elapsed_before_play: Duration::ZERO,
                policy_settled: None,
            },
        );
        self.timeline_index.insert(logical, handle.clone());
        Ok(handle)
    }

    fn allocate_timeline_handle(&mut self, owner: &TimelineOwner, name: String) -> MotionHandle {
        let instance = self.next_timeline_instance.max(1);
        self.next_timeline_instance = instance.saturating_add(1);
        MotionHandle {
            runtime_id: self.runtime_id,
            instance,
            domain: owner.domain.clone(),
            owner: owner.component.clone(),
            incarnation: owner.incarnation,
            generation: owner.generation,
            node_path: owner.node_path.clone(),
            name,
        }
    }

    /// Resume or begin a timeline.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownTimeline`] for a stale handle.
    pub fn play_timeline(
        &mut self,
        handle: &MotionHandle,
        now: Instant,
    ) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        let state = self
            .timelines
            .get(handle)
            .map(|timeline| timeline.state)
            .ok_or_else(|| MotionError::StaleTimelineHandle(handle.name.clone()))?;
        match state {
            MotionPlaybackState::Playing | MotionPlaybackState::Completed => return Ok(()),
            MotionPlaybackState::Cancelled => {
                return Err(MotionError::StaleTimelineHandle(handle.name.clone()));
            }
            MotionPlaybackState::Idle | MotionPlaybackState::Paused => {}
        }
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            let preference = self.preference;
            let timeline = self.timeline_mut(handle)?;
            timeline.state = MotionPlaybackState::Completed;
            timeline.elapsed_before_play = Duration::from_millis(
                timeline_total_duration(timeline).unwrap_or(timeline.duration_ms),
            );
            timeline.policy_settled = preference;
            return Ok(());
        }
        let tracks = self
            .timelines
            .get(handle)
            .map_or(0, |timeline| timeline.tracks.len());
        self.ensure_active_capacity(tracks)?;
        let timeline = self.timeline_mut(handle)?;
        timeline.started = now;
        timeline.state = MotionPlaybackState::Playing;
        timeline.policy_settled = None;
        Ok(())
    }

    /// Pause a timeline at its current deterministic clock position.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownTimeline`] for a stale handle.
    pub fn pause_timeline(
        &mut self,
        handle: &MotionHandle,
        now: Instant,
    ) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        let suspended = self.handle_suspended(handle);
        let timeline = self.timeline_mut(handle)?;
        if timeline.state == MotionPlaybackState::Playing {
            if !suspended {
                timeline.elapsed_before_play = timeline_position(timeline, now);
            }
            timeline.state = MotionPlaybackState::Paused;
        }
        Ok(())
    }

    /// Seek to an absolute timeline position in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns a stale-handle or invalid-position error.
    pub fn seek_timeline(
        &mut self,
        handle: &MotionHandle,
        position_ms: u64,
        now: Instant,
    ) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        let timeline = self.timeline_mut(handle)?;
        let total = timeline_total_duration(timeline);
        if total.is_some_and(|total| position_ms > total) {
            return Err(MotionError::InvalidTimeline(format!(
                "seek position {position_ms}ms exceeds timeline duration {}ms",
                total.unwrap_or_default()
            )));
        }
        timeline.elapsed_before_play = Duration::from_millis(position_ms);
        timeline.started = now;
        if matches!(
            timeline.state,
            MotionPlaybackState::Completed | MotionPlaybackState::Idle
        ) {
            timeline.state = MotionPlaybackState::Paused;
        }
        Ok(())
    }

    /// Restart a timeline from zero and enter the playing state.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownTimeline`] for a stale handle.
    pub fn restart_timeline(
        &mut self,
        handle: &MotionHandle,
        now: Instant,
    ) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            let preference = self.preference;
            let timeline = self.timeline_mut(handle)?;
            timeline.elapsed_before_play = Duration::from_millis(
                timeline_total_duration(timeline).unwrap_or(timeline.duration_ms),
            );
            timeline.state = MotionPlaybackState::Completed;
            timeline.policy_settled = preference;
            return Ok(());
        }
        let tracks = self
            .timelines
            .get(handle)
            .map_or(0, |timeline| timeline.tracks.len());
        let already_active = self
            .timelines
            .get(handle)
            .is_some_and(|timeline| timeline.state == MotionPlaybackState::Playing);
        if !already_active {
            self.ensure_active_capacity(tracks)?;
        }
        let timeline = self.timeline_mut(handle)?;
        timeline.elapsed_before_play = Duration::ZERO;
        timeline.started = now;
        timeline.state = MotionPlaybackState::Playing;
        timeline.policy_settled = None;
        Ok(())
    }

    /// Cancel a timeline and enqueue one post-frame cancellation event.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownTimeline`] for a stale handle.
    pub fn cancel_timeline(&mut self, handle: &MotionHandle) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        let callback = {
            let timeline = self.timeline_mut(handle)?;
            if matches!(
                timeline.state,
                MotionPlaybackState::Cancelled | MotionPlaybackState::Completed
            ) {
                return Ok(());
            }
            timeline.state = MotionPlaybackState::Cancelled;
            timeline.spec.on_cancel.clone()
        };
        self.timeline_events.push(MotionTimelineEvent {
            handle: handle.clone(),
            kind: MotionTimelineEventKind::Cancel,
            callback,
        });
        Ok(())
    }

    #[must_use]
    pub fn timeline_state(&self, handle: &MotionHandle) -> Option<MotionPlaybackState> {
        (handle.runtime_id == self.runtime_id)
            .then(|| self.timelines.get(handle).map(|timeline| timeline.state))
            .flatten()
    }

    /// Resolve one unique named timeline below a view root.
    ///
    /// # Errors
    ///
    /// Returns an explicit error for missing or duplicate names.
    pub fn timeline_handle_for_owner(
        &self,
        domain: &str,
        owner: &ComponentInstancePath,
        incarnation: ComponentIncarnation,
        generation: ScriptGeneration,
        name: &str,
    ) -> Result<MotionHandle, MotionError> {
        let mut matches = self.timelines.keys().filter(|handle| {
            handle.name == name
                && handle.domain == domain
                && &handle.owner == owner
                && handle.incarnation == incarnation
                && handle.generation == generation
        });
        let handle = matches
            .next()
            .cloned()
            .ok_or_else(|| MotionError::UnknownTimeline(name.to_owned()))?;
        if matches.next().is_some() {
            return Err(MotionError::InvalidTimeline(format!(
                "timeline name `{name}` is duplicated for component `{owner}`"
            )));
        }
        Ok(handle)
    }

    pub fn drain_timeline_events(&mut self) -> Vec<MotionTimelineEvent> {
        std::mem::take(&mut self.timeline_events)
    }

    pub fn drain_timeline_events_for_domain(&mut self, domain: &str) -> Vec<MotionTimelineEvent> {
        let events = std::mem::take(&mut self.timeline_events);
        let (matching, retained) = events
            .into_iter()
            .partition(|event| event.handle.domain == domain);
        self.timeline_events = retained;
        matching
    }

    pub(crate) fn discard_timeline_events_in_scope(&mut self, path: &str) {
        self.timeline_events
            .retain(|event| !event.handle.in_node_scope(path));
    }

    pub(crate) fn prepend_timeline_events(&mut self, mut events: Vec<MotionTimelineEvent>) {
        events.append(&mut self.timeline_events);
        self.timeline_events = events;
    }

    #[must_use]
    pub fn resource_usage(&self) -> MotionResourceUsage {
        let geometry_slots = self.active_reservations.values().copied().sum::<usize>();
        MotionResourceUsage {
            particles: 0,
            shared_snapshots: 0,
            geometry_slots,
            active: self.active.len().saturating_add(
                self.timelines
                    .values()
                    .filter(|timeline| timeline.state == MotionPlaybackState::Playing)
                    .map(|timeline| timeline.tracks.len())
                    .sum::<usize>(),
            ),
            timelines: self.timelines.len(),
            timeline_steps: self
                .timelines
                .values()
                .map(|timeline| timeline_step_count(&timeline.spec.root))
                .sum(),
            keyframes: self
                .active
                .values()
                .map(|motion| source_keyframe_count(&motion.source))
                .chain(self.timelines.values().flat_map(|timeline| {
                    timeline
                        .tracks
                        .iter()
                        .map(|track| source_keyframe_count(&track.source))
                }))
                .sum(),
            declarations: self.active.len(),
        }
    }

    fn timeline_mut(&mut self, handle: &MotionHandle) -> Result<&mut ActiveTimeline, MotionError> {
        self.timelines
            .get_mut(handle)
            .ok_or_else(|| MotionError::StaleTimelineHandle(handle.name.clone()))
    }

    fn validate_handle(&self, handle: &MotionHandle) -> Result<(), MotionError> {
        if handle.runtime_id == self.runtime_id && self.timelines.contains_key(handle) {
            Ok(())
        } else {
            Err(MotionError::StaleTimelineHandle(handle.name.clone()))
        }
    }

    pub(crate) fn validate_handle_owner(
        &self,
        handle: &MotionHandle,
        domain: &str,
        owner: &ComponentInstancePath,
        incarnation: ComponentIncarnation,
        generation: ScriptGeneration,
    ) -> Result<(), MotionError> {
        self.validate_handle(handle)?;
        if handle.domain == domain
            && &handle.owner == owner
            && handle.incarnation == incarnation
            && handle.generation == generation
        {
            Ok(())
        } else {
            Err(MotionError::ForeignTimelineHandle(handle.name.clone()))
        }
    }

    fn ensure_active_capacity(&self, additional: usize) -> Result<(), MotionError> {
        let usage = self.resource_usage();
        let actual = usage
            .active
            .saturating_add(usage.geometry_slots)
            .saturating_add(additional);
        if actual <= self.active_limit {
            Ok(())
        } else {
            Err(MotionError::ActiveBudget {
                actual,
                limit: self.active_limit,
            })
        }
    }

    pub(crate) fn set_active_limit(&mut self, limit: usize) {
        self.active_limit = limit;
    }

    pub(crate) fn set_active_reservation(&mut self, path: &str, slots: usize) {
        if slots == 0 {
            self.active_reservations.remove(path);
        } else {
            self.active_reservations.insert(path.to_owned(), slots);
        }
    }

    pub fn retain_keys(&mut self, keys: &BTreeSet<MotionKey>) {
        self.active.retain(|key, _| keys.contains(key));
        self.settled.retain(|key, _| keys.contains(key));
        self.settled_declaration.retain(|key, _| keys.contains(key));
        self.settled_retention.retain(|key, _| keys.contains(key));
    }

    pub fn retain_node_scope(&mut self, path: &str, keys: &BTreeSet<MotionKey>) {
        self.active
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled_declaration
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled_retention
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
    }

    fn retain_live_scope(&mut self, domain: &str, keys: &BTreeSet<MotionKey>) {
        self.active.retain(|key, motion| {
            !matches!(&motion.retention, MotionRetention::Live(owner) if owner == domain)
                || keys.contains(key)
        });
        let removed = self
            .settled_retention
            .iter()
            .filter(|(_, retention)| {
                matches!(retention, MotionRetention::Live(owner) if owner == domain)
            })
            .map(|(key, _)| key.clone())
            .filter(|key| !keys.contains(key))
            .collect::<Vec<_>>();
        for key in removed {
            self.settled.remove(&key);
            self.settled_declaration.remove(&key);
            self.settled_retention.remove(&key);
        }
    }

    fn reset_live_scope(&mut self, domain: &str) {
        self.active.retain(
            |_, motion| !matches!(&motion.retention, MotionRetention::Live(owner) if owner == domain),
        );
        let keys = self
            .settled_retention
            .iter()
            .filter(|(_, retention)| {
                matches!(retention, MotionRetention::Live(owner) if owner == domain)
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            self.settled.remove(&key);
            self.settled_declaration.remove(&key);
            self.settled_retention.remove(&key);
        }
        let handles = self
            .timelines
            .keys()
            .filter(|handle| handle.domain == domain)
            .cloned()
            .collect::<Vec<_>>();
        for handle in handles {
            if let Some(timeline) = self.timelines.remove(&handle)
                && !matches!(
                    timeline.state,
                    MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
                )
            {
                self.timeline_events.push(MotionTimelineEvent {
                    handle: handle.clone(),
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
            self.timeline_index.retain(|_, active| active != &handle);
        }
    }

    pub fn retain_timeline_scope(&mut self, path: &str, handles: &BTreeSet<MotionHandle>) {
        let removed = self
            .timelines
            .keys()
            .filter(|handle| handle.in_node_scope(path) && !handles.contains(*handle))
            .cloned()
            .collect::<Vec<_>>();
        for handle in removed {
            if let Some(timeline) = self.timelines.remove(&handle)
                && !matches!(
                    timeline.state,
                    MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
                )
            {
                self.timeline_events.push(MotionTimelineEvent {
                    handle: handle.clone(),
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
            self.timeline_index.retain(|_, active| active != &handle);
        }
    }

    fn retain_timeline_plan_scope(&mut self, path: &str, logical: &BTreeSet<(String, String)>) {
        let removed = self
            .timelines
            .keys()
            .filter(|handle| {
                handle.in_node_scope(path)
                    && !logical.contains(&(handle.node_path.clone(), handle.name.clone()))
            })
            .cloned()
            .collect::<Vec<_>>();
        for handle in removed {
            if let Some(timeline) = self.timelines.remove(&handle)
                && !matches!(
                    timeline.state,
                    MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
                )
            {
                self.timeline_events.push(MotionTimelineEvent {
                    handle: handle.clone(),
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
            self.timeline_index.retain(|_, active| active != &handle);
        }
    }

    pub fn cancel_node_scope(&mut self, path: &str) {
        self.active.retain(|key, _| !key.in_node_scope(path));
        self.settled.retain(|key, _| !key.in_node_scope(path));
        self.settled_declaration
            .retain(|key, _| !key.in_node_scope(path));
        self.settled_retention
            .retain(|key, _| !key.in_node_scope(path));
        self.active_reservations
            .retain(|scope, _| !path_contains_scope(path, scope));
        self.live_scene_signatures
            .retain(|scope, _| !path_contains_scope(path, scope));
        let removed = self
            .timelines
            .keys()
            .filter(|handle| handle.in_node_scope(path))
            .cloned()
            .collect::<Vec<_>>();
        for handle in removed {
            if let Some(timeline) = self.timelines.remove(&handle)
                && !matches!(
                    timeline.state,
                    MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
                )
            {
                self.timeline_events.push(MotionTimelineEvent {
                    handle: handle.clone(),
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
            self.timeline_index.retain(|_, active| active != &handle);
        }
    }

    pub(crate) fn release_node_scope(&mut self, path: &str) {
        self.cancel_node_scope(path);
        self.suspended_scopes
            .retain(|scope| !path_contains_scope(path, scope));
        self.discard_timeline_events_in_scope(path);
    }

    #[must_use]
    pub fn is_node_scope_active(&self, path: &str) -> bool {
        self.active.keys().any(|key| key.in_node_scope(path))
            || self.timelines.keys().any(|handle| {
                handle.in_node_scope(path)
                    && self
                        .timelines
                        .get(handle)
                        .is_some_and(|timeline| timeline.state == MotionPlaybackState::Playing)
            })
    }

    fn key_suspended(&self, key: &MotionKey) -> bool {
        self.suspended_scopes
            .iter()
            .any(|scope| key.in_node_scope(scope))
    }

    fn handle_suspended(&self, handle: &MotionHandle) -> bool {
        self.suspended_scopes
            .iter()
            .any(|scope| handle.in_node_scope(scope))
    }

    pub fn suspend_node_scope(&mut self, path: &str, now: Instant) {
        if !self.suspended_scopes.insert(path.to_owned()) {
            return;
        }
        for (key, motion) in &mut self.active {
            if !key.in_node_scope(path) {
                continue;
            }
            motion.elapsed_before_play = motion_elapsed(motion, now);
        }
        for (handle, timeline) in &mut self.timelines {
            if handle.in_node_scope(path) && timeline.state == MotionPlaybackState::Playing {
                timeline.elapsed_before_play = timeline_position(timeline, now);
            }
        }
    }

    pub fn resume_node_scope(&mut self, path: &str, now: Instant) {
        if !self.suspended_scopes.remove(path) {
            return;
        }
        for (key, motion) in &mut self.active {
            if key.in_node_scope(path) {
                motion.started = now;
            }
        }
        for (handle, timeline) in &mut self.timelines {
            if handle.in_node_scope(path) && timeline.state == MotionPlaybackState::Playing {
                timeline.started = now;
            }
        }
    }

    /// Compatibility for internal callers that already measured a pause.
    pub fn delay_node_scope(&mut self, path: &str, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        for (key, motion) in &mut self.active {
            if key.in_node_scope(path) {
                motion.started = motion.started.checked_add(delay).unwrap_or(motion.started);
            }
        }
        for (handle, timeline) in &mut self.timelines {
            if handle.in_node_scope(path) {
                timeline.started = timeline
                    .started
                    .checked_add(delay)
                    .unwrap_or(timeline.started);
            }
        }
    }

    #[must_use]
    pub fn snapshot(&self, now: Instant) -> BTreeMap<MotionKey, f64> {
        let mut values = self
            .active
            .keys()
            .chain(self.settled.keys())
            .filter_map(|key| self.sample(key, now).map(|value| (key.clone(), value)))
            .collect::<BTreeMap<_, _>>();
        for (handle, timeline) in &self.timelines {
            values.extend(
                sample_timeline(handle, timeline, now, self.handle_suspended(handle)).values,
            );
        }
        values
    }

    #[must_use]
    pub fn inspect(&self, now: Instant) -> Vec<MotionSnapshot> {
        let mut snapshots = self
            .active
            .iter()
            .map(|(key, motion)| {
                let sample = sample_active_motion(motion, now, self.key_suspended(key));
                let (kind, duration, repeating) = match &motion.source {
                    MotionSource::Transition(spec) => (
                        "transition",
                        Some(spec.duration_ms),
                        spec.iterations.is_none_or(|count| count > 1),
                    ),
                    MotionSource::Spring(_) => ("spring", None, false),
                    MotionSource::Keyframes(spec) => (
                        "keyframes",
                        Some(spec.duration_ms),
                        spec.iterations.is_none_or(|count| count > 1),
                    ),
                    MotionSource::Inertia(_) => ("inertia", None, false),
                };
                MotionSnapshot {
                    key: key.clone(),
                    kind: kind.to_owned(),
                    value: sample.value,
                    target: source_terminal_value(&motion.source),
                    velocity: sample.velocity,
                    elapsed_ms: duration_ms(if self.key_suspended(key) {
                        motion.elapsed_before_play
                    } else {
                        motion_elapsed(motion, now)
                    }),
                    duration_ms: duration,
                    repeating,
                    active: true,
                }
            })
            .collect::<Vec<_>>();
        snapshots.extend(self.settled.iter().map(|(key, value)| MotionSnapshot {
            key: key.clone(),
            kind: "settled".to_owned(),
            value: *value,
            target: *value,
            velocity: None,
            elapsed_ms: 0,
            duration_ms: None,
            repeating: false,
            active: false,
        }));
        snapshots.sort_by(|left, right| left.key.cmp(&right.key));
        snapshots
    }

    #[must_use]
    pub fn inspect_timelines(&self, now: Instant) -> Vec<MotionTimelineSnapshot> {
        self.timelines
            .iter()
            .map(|(handle, timeline)| {
                let active_elapsed = if timeline.state == MotionPlaybackState::Playing
                    && !self.handle_suspended(handle)
                {
                    now.saturating_duration_since(timeline.started)
                } else {
                    Duration::ZERO
                };
                let elapsed = timeline.elapsed_before_play.saturating_add(active_elapsed);
                MotionTimelineSnapshot {
                    handle: handle.clone(),
                    state: timeline.state,
                    elapsed_ms: duration_ms(elapsed),
                    duration_ms: timeline_total_duration(timeline),
                    iterations: timeline.spec.iterations,
                    autoreverse: timeline.spec.autoreverse,
                    track_count: timeline.tracks.len(),
                }
            })
            .collect()
    }

    #[must_use]
    pub fn sample(&self, key: &MotionKey, now: Instant) -> Option<f64> {
        self.sample_with_velocity(key, now)
            .map(|sample| sample.value)
    }

    fn sample_with_velocity(&self, key: &MotionKey, now: Instant) -> Option<SourceSample> {
        self.active
            .get(key)
            .map(|motion| sample_active_motion(motion, now, self.key_suspended(key)))
            .or_else(|| {
                self.settled.get(key).copied().map(|value| SourceSample {
                    value,
                    velocity: None,
                    done: true,
                })
            })
    }

    #[must_use]
    pub fn tick(&mut self, now: Instant) -> MotionFrame {
        self.tick_filtered(now, None)
    }

    #[must_use]
    pub fn tick_scope(&mut self, now: Instant, path: &str) -> MotionFrame {
        self.tick_filtered(now, Some(path))
    }

    fn tick_filtered(&mut self, now: Instant, scope: Option<&str>) -> MotionFrame {
        let mut values = BTreeMap::new();
        let mut completed = BTreeSet::new();
        for (key, animation) in &mut self.active {
            if scope.is_some_and(|scope| !key.in_node_scope(scope)) {
                continue;
            }
            if self
                .suspended_scopes
                .iter()
                .any(|scope| key.in_node_scope(scope))
            {
                values.insert(
                    key.clone(),
                    sample_active_motion(animation, now, true).value,
                );
                continue;
            }
            let sample = sample_active_motion(animation, now, false);
            values.insert(key.clone(), sample.value);
            if sample.done {
                completed.insert(key.clone());
            }
        }
        for key in &completed {
            if let Some(animation) = self.active.remove(key) {
                let sample = sample_active_motion(&animation, now, false);
                self.settled_declaration
                    .insert(key.clone(), (animation.declaration, animation.replay_key));
                self.settled_retention
                    .insert(key.clone(), animation.retention);
                self.settled.insert(key.clone(), sample.value);
            }
        }
        let mut completed_timelines = Vec::new();
        let suspended_scopes = &self.suspended_scopes;
        for (handle, timeline) in &mut self.timelines {
            if scope.is_some_and(|scope| !handle.in_node_scope(scope)) {
                continue;
            }
            let suspended = suspended_scopes
                .iter()
                .any(|scope| handle.in_node_scope(scope));
            let sample = sample_timeline(handle, timeline, now, suspended);
            values.extend(sample.values);
            if sample.done && !suspended && timeline.state == MotionPlaybackState::Playing {
                completed_timelines.push(handle.clone());
            }
        }
        for handle in completed_timelines {
            let callback = self.timelines.get_mut(&handle).and_then(|timeline| {
                timeline.elapsed_before_play = Duration::from_millis(
                    timeline_total_duration(timeline).unwrap_or(timeline.duration_ms),
                );
                timeline.state = MotionPlaybackState::Completed;
                timeline.spec.on_complete.clone()
            });
            self.timeline_events.push(MotionTimelineEvent {
                handle,
                kind: MotionTimelineEventKind::Complete,
                callback,
            });
        }
        MotionFrame {
            values,
            completed,
            needs_frame: self.active.keys().any(|key| {
                scope.is_none_or(|scope| key.in_node_scope(scope))
                    && !self
                        .suspended_scopes
                        .iter()
                        .any(|suspended| key.in_node_scope(suspended))
            }) || self.timelines.iter().any(|(handle, timeline)| {
                timeline.state == MotionPlaybackState::Playing
                    && scope.is_none_or(|scope| handle.in_node_scope(scope))
                    && !self
                        .suspended_scopes
                        .iter()
                        .any(|suspended| handle.in_node_scope(suspended))
            }),
        }
    }
}

fn timeline_compatible(previous: &MotionTimeline, next: &MotionTimeline) -> bool {
    previous.name == next.name
        && previous.root == next.root
        && previous.autoplay == next.autoplay
        && previous.iterations == next.iterations
        && previous.autoreverse == next.autoreverse
        && previous.intent == next.intent
}

struct TimelineSample {
    values: BTreeMap<MotionKey, f64>,
    done: bool,
}

fn sample_timeline(
    handle: &MotionHandle,
    timeline: &ActiveTimeline,
    now: Instant,
    suspended: bool,
) -> TimelineSample {
    if timeline.state == MotionPlaybackState::Cancelled {
        return TimelineSample {
            values: BTreeMap::new(),
            done: false,
        };
    }
    if let Some(preference) = timeline.policy_settled {
        if preference == MotionPreference::None
            && let Some(total) = timeline_total_duration(timeline)
        {
            return sample_timeline_position(handle, timeline, Duration::from_millis(total));
        }
        let values = timeline
            .tracks
            .iter()
            .map(|track| {
                let key = timeline_track_key(handle, track);
                let value = track.source.reduced_value();
                (key, value)
            })
            .collect();
        return TimelineSample { values, done: true };
    }
    let elapsed = if suspended {
        timeline.elapsed_before_play
    } else {
        timeline_position(timeline, now)
    };
    sample_timeline_position(handle, timeline, elapsed)
}

fn sample_timeline_position(
    handle: &MotionHandle,
    timeline: &ActiveTimeline,
    elapsed: Duration,
) -> TimelineSample {
    let elapsed_ms = duration_ms(elapsed);
    let iteration_duration = timeline.duration_ms.max(1);
    let raw_iteration = elapsed_ms / iteration_duration;
    let done = timeline
        .spec
        .iterations
        .is_some_and(|iterations| raw_iteration >= u64::from(iterations));
    let iteration = if done {
        u64::from(timeline.spec.iterations.unwrap_or(1).saturating_sub(1))
    } else {
        raw_iteration
    };
    let mut local_ms = if done {
        iteration_duration
    } else {
        elapsed_ms % iteration_duration
    };
    if timeline.spec.autoreverse && iteration % 2 == 1 {
        local_ms = iteration_duration.saturating_sub(local_ms);
    }
    let mut values = BTreeMap::new();
    for track in &timeline.tracks {
        let key = timeline_track_key(handle, track);
        if local_ms < track.start_ms {
            values
                .entry(key)
                .or_insert_with(|| source_initial_value(&track.source));
        } else {
            let track_elapsed = local_ms.saturating_sub(track.start_ms);
            values.insert(
                key,
                sample_source_with_duration(
                    &track.source,
                    Duration::from_millis(track_elapsed),
                    Some(track.duration_ms),
                )
                .value,
            );
        }
    }
    TimelineSample { values, done }
}

fn timeline_position(timeline: &ActiveTimeline, now: Instant) -> Duration {
    if timeline.state == MotionPlaybackState::Playing {
        timeline
            .elapsed_before_play
            .saturating_add(now.saturating_duration_since(timeline.started))
    } else {
        timeline.elapsed_before_play
    }
}

fn timeline_track_key(handle: &MotionHandle, track: &ScheduledMotionTrack) -> MotionKey {
    let root = handle.node_path.as_str();
    let path = track
        .resolved_path
        .clone()
        .unwrap_or_else(|| match track.target.as_str() {
            "." | "" => root.to_owned(),
            target if target.starts_with('/') => target.trim_start_matches('/').to_owned(),
            target => format!("{root}/{target}"),
        });
    MotionKey::for_node(&path, track.source.property())
}

fn timeline_total_duration(timeline: &ActiveTimeline) -> Option<u64> {
    timeline
        .spec
        .iterations
        .map(|iterations| timeline.duration_ms.saturating_mul(u64::from(iterations)))
}

fn compile_timeline(
    root: &MotionTimelineStep,
) -> Result<(Vec<ScheduledMotionTrack>, u64), MotionError> {
    let mut tracks = Vec::new();
    let duration_ms = flatten_timeline(root, 0, &mut tracks)?;
    if tracks.is_empty() {
        return Err(MotionError::InvalidTimeline(
            "timeline must contain at least one motion track".to_owned(),
        ));
    }
    let mut occupied = BTreeMap::<(String, MotionProperty), Vec<(u64, u64)>>::new();
    for track in &tracks {
        let key = (track.target.clone(), track.source.property());
        let end = track.start_ms.saturating_add(track.duration_ms);
        let intervals = occupied.entry(key.clone()).or_default();
        if intervals
            .iter()
            .any(|(start, existing_end)| track.start_ms < *existing_end && *start < end)
        {
            return Err(MotionError::InvalidTimeline(format!(
                "overlapping tracks target `{}` property `{}`",
                key.0,
                key.1.as_str()
            )));
        }
        intervals.push((track.start_ms, end));
    }
    Ok((tracks, duration_ms.max(1)))
}

fn flatten_timeline(
    step: &MotionTimelineStep,
    start_ms: u64,
    output: &mut Vec<ScheduledMotionTrack>,
) -> Result<u64, MotionError> {
    match step {
        MotionTimelineStep::Track(track) => {
            validate_source(&track.source)?;
            validate_timeline_target(&track.target)?;
            let duration_ms = source_duration_ms(&track.source)?;
            output.push(ScheduledMotionTrack {
                target: track.target.clone(),
                resolved_path: None,
                source: track.source.clone(),
                start_ms,
                duration_ms,
            });
            Ok(start_ms.saturating_add(duration_ms))
        }
        MotionTimelineStep::Delay(duration_ms) => Ok(start_ms.saturating_add(*duration_ms)),
        MotionTimelineStep::Sequence(steps) => {
            let mut cursor = start_ms;
            for step in steps {
                cursor = flatten_timeline(step, cursor, output)?;
            }
            Ok(cursor)
        }
        MotionTimelineStep::Parallel(steps) => {
            let mut end = start_ms;
            for step in steps {
                end = end.max(flatten_timeline(step, start_ms, output)?);
            }
            Ok(end)
        }
        MotionTimelineStep::Stagger { interval_ms, steps } => {
            let mut end = start_ms;
            for (index, step) in steps.iter().enumerate() {
                let offset = interval_ms.saturating_mul(u64::try_from(index).unwrap_or(u64::MAX));
                end = end.max(flatten_timeline(
                    step,
                    start_ms.saturating_add(offset),
                    output,
                )?);
            }
            Ok(end)
        }
    }
}

fn source_duration_ms(source: &MotionSource) -> Result<u64, MotionError> {
    match source {
        MotionSource::Transition(spec) => spec
            .iterations
            .map(|iterations| {
                spec.delay_ms
                    .saturating_add(spec.duration_ms.saturating_mul(u64::from(iterations)))
            })
            .ok_or_else(|| {
                MotionError::InvalidTimeline(
                    "an infinite transition cannot be nested in a timeline".to_owned(),
                )
            }),
        MotionSource::Keyframes(spec) => spec
            .iterations
            .map(|iterations| {
                spec.delay_ms
                    .saturating_add(spec.duration_ms.saturating_mul(u64::from(iterations)))
            })
            .ok_or_else(|| {
                MotionError::InvalidTimeline(
                    "infinite keyframes cannot be nested in a timeline".to_owned(),
                )
            }),
        MotionSource::Spring(spec) => Ok(spring_settle_ms(spec)),
        MotionSource::Inertia(spec) => Ok(inertia_settle_ms(spec)),
    }
}

pub(crate) fn sample_progress_source(source: &MotionSource, progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    match source {
        MotionSource::Transition(spec) => {
            spec.from + (spec.to - spec.from) * spec.easing.sample(progress)
        }
        MotionSource::Keyframes(spec) => sample_keyframes(&spec.frames, progress),
        MotionSource::Spring(spec) => spec.from + (spec.to - spec.from) * progress,
        MotionSource::Inertia(spec) => spec.from + (inertia_target(spec) - spec.from) * progress,
    }
}

pub(crate) fn progress_source_duration(source: &MotionSource) -> Duration {
    Duration::from_millis(match source {
        MotionSource::Transition(spec) => spec.duration_ms,
        MotionSource::Keyframes(spec) => spec.duration_ms,
        MotionSource::Spring(_) | MotionSource::Inertia(_) => 1,
    })
}

fn validate_progress_source(source: &MotionSource) -> Result<(), MotionError> {
    validate_source(source)?;
    match source {
        MotionSource::Transition(spec)
            if spec.delay_ms == 0 && spec.iterations == Some(1) && !spec.autoreverse =>
        {
            Ok(())
        }
        MotionSource::Keyframes(spec)
            if spec.delay_ms == 0 && spec.iterations == Some(1) && !spec.autoreverse =>
        {
            Ok(())
        }
        MotionSource::Transition(_) | MotionSource::Keyframes(_) => {
            Err(MotionError::InvalidProgressSource(
                "progress mappings cannot use delay, repeat, or autoreverse".to_owned(),
            ))
        }
        MotionSource::Spring(_) | MotionSource::Inertia(_) => {
            Err(MotionError::InvalidProgressSource(
                "progress mappings must use transition or keyframes".to_owned(),
            ))
        }
    }
}

fn sample_spring_at(spec: &MotionSpring, seconds: f64) -> (f64, f64) {
    let omega = (spec.stiffness / spec.mass).sqrt();
    let zeta = spec.damping / (2.0 * (spec.stiffness * spec.mass).sqrt());
    let displacement = spec.from - spec.to;
    if zeta < 1.0 - 1.0e-6 {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let coefficient = (spec.initial_velocity + zeta * omega * displacement) / damped;
        let decay = (-zeta * omega * seconds).exp();
        let cos = (damped * seconds).cos();
        let sin = (damped * seconds).sin();
        let y = decay * (displacement * cos + coefficient * sin);
        let velocity = decay
            * (-zeta * omega * (displacement * cos + coefficient * sin)
                + (-displacement * damped * sin + coefficient * damped * cos));
        (spec.to + y, velocity)
    } else if (zeta - 1.0).abs() <= 1.0e-6 {
        let coefficient = spec.initial_velocity + omega * displacement;
        let decay = (-omega * seconds).exp();
        let y = decay * (displacement + coefficient * seconds);
        let velocity = decay * (coefficient - omega * (displacement + coefficient * seconds));
        (spec.to + y, velocity)
    } else {
        let root = (zeta * zeta - 1.0).sqrt();
        let first = -omega * (zeta - root);
        let second = -omega * (zeta + root);
        let a = (spec.initial_velocity - second * displacement) / (first - second);
        let b = displacement - a;
        let a_term = a * (first * seconds).exp();
        let b_term = b * (second * seconds).exp();
        (spec.to + a_term + b_term, first * a_term + second * b_term)
    }
}

fn spring_settle_ms(spec: &MotionSpring) -> u64 {
    (1..=1_200)
        .map(|step| step * 8)
        .find(|milliseconds| {
            let (position, velocity) =
                sample_spring_at(spec, Duration::from_millis(*milliseconds).as_secs_f64());
            (position - spec.to).abs() < 0.001 && velocity.abs() < 0.001
        })
        .unwrap_or(9_600)
}

fn sample_inertia_at(spec: &MotionInertia, elapsed: Duration) -> (f64, f64) {
    if spec.min.is_none() && spec.max.is_none() {
        let seconds = elapsed.as_secs_f64().min(10.0);
        let decay = (-spec.friction * seconds).exp();
        let mut position = spec.from + spec.velocity * (1.0 - decay) / spec.friction;
        let mut velocity = spec.velocity * decay;
        if elapsed >= Duration::from_millis(inertia_settle_ms(spec)) {
            position = nearest_snap(position, &spec.snap_points).unwrap_or(position);
            velocity = 0.0;
        }
        return (position, velocity);
    }
    if spec.bounce <= f64::EPSILON {
        let seconds = elapsed.as_secs_f64().min(10.0);
        let decay = (-spec.friction * seconds).exp();
        let projected = spec.from + spec.velocity * (1.0 - decay) / spec.friction;
        let mut position = projected.clamp(
            spec.min.unwrap_or(f64::NEG_INFINITY),
            spec.max.unwrap_or(f64::INFINITY),
        );
        let mut velocity = if (position - projected).abs() <= f64::EPSILON {
            spec.velocity * decay
        } else {
            0.0
        };
        if elapsed >= Duration::from_millis(inertia_settle_ms(spec)) {
            position = nearest_snap(position, &spec.snap_points).unwrap_or(position);
            velocity = 0.0;
        }
        return (position, velocity);
    }
    let mut remaining = elapsed.as_secs_f64().min(10.0);
    let mut position = spec.from.clamp(
        spec.min.unwrap_or(f64::NEG_INFINITY),
        spec.max.unwrap_or(f64::INFINITY),
    );
    let mut velocity = spec.velocity;
    if spec.min.is_some_and(|minimum| spec.from < minimum) {
        velocity = velocity.abs() * spec.bounce;
    } else if spec.max.is_some_and(|maximum| spec.from > maximum) {
        velocity = -velocity.abs() * spec.bounce;
    }
    for _ in 0..4_096 {
        if remaining <= f64::EPSILON || velocity.abs() <= 0.01 {
            break;
        }
        let boundary = if velocity > 0.0 { spec.max } else { spec.min };
        let Some(boundary) = boundary else {
            let decay = (-spec.friction * remaining).exp();
            position += velocity * (1.0 - decay) / spec.friction;
            velocity *= decay;
            remaining = 0.0;
            break;
        };
        if (position - boundary).abs() <= f64::EPSILON {
            velocity = -velocity * spec.bounce;
            continue;
        }
        let distance = boundary - position;
        let ratio = distance * spec.friction / velocity;
        if !(0.0..1.0).contains(&ratio) {
            let decay = (-spec.friction * remaining).exp();
            position += velocity * (1.0 - decay) / spec.friction;
            velocity *= decay;
            remaining = 0.0;
            break;
        }
        let collision_time = -(1.0 - ratio).ln() / spec.friction;
        if collision_time >= remaining {
            let decay = (-spec.friction * remaining).exp();
            position += velocity * (1.0 - decay) / spec.friction;
            velocity *= decay;
            remaining = 0.0;
            break;
        }
        velocity *= (-spec.friction * collision_time).exp();
        position = boundary;
        velocity = -velocity * spec.bounce;
        remaining -= collision_time;
    }
    if remaining > f64::EPSILON {
        velocity = 0.0;
    }
    if elapsed >= Duration::from_millis(inertia_settle_ms(spec)) {
        position = nearest_snap(position, &spec.snap_points).unwrap_or(position);
        velocity = 0.0;
    }
    (position, velocity)
}

fn inertia_settle_ms(spec: &MotionInertia) -> u64 {
    if spec.velocity.abs() <= 0.01 {
        return 1;
    }
    let seconds = (spec.velocity.abs() / 0.01).ln() / spec.friction;
    u64::try_from(Duration::from_secs_f64(seconds.clamp(0.001, 10.0)).as_millis()).unwrap_or(10_000)
}

fn validate_timeline_name(name: &str) -> Result<(), MotionError> {
    if !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        Ok(())
    } else {
        Err(MotionError::InvalidTimeline(format!(
            "timeline name `{name}` must use ASCII letters, digits, `_`, or `-`"
        )))
    }
}

fn validate_timeline_target(target: &str) -> Result<(), MotionError> {
    if target == "."
        || (!target.is_empty()
            && target.split('/').all(|segment| {
                !segment.is_empty()
                    && segment.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
                    })
            }))
    {
        Ok(())
    } else {
        Err(MotionError::InvalidTimeline(format!(
            "timeline target `{target}` is not a stable relative node path"
        )))
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Reconcile animation declarations from one successfully rendered node tree.
///
/// # Errors
///
/// Returns invalid-spec or missing-key errors before mutating the runtime.
pub fn reconcile_node_motion(
    root: &UiNode,
    runtime: &mut MotionRuntime,
    now: Instant,
) -> Result<BTreeMap<MotionKey, f64>, MotionError> {
    reconcile_node_motion_scoped(root, runtime, now, "root")
}

#[derive(Clone, Copy)]
pub(crate) struct MotionReconcileContext<'a> {
    pub domain: &'a str,
    pub root_component: &'a ComponentInstancePath,
    pub root_incarnation: ComponentIncarnation,
    pub generation: ScriptGeneration,
    pub incarnations: &'a BTreeMap<ComponentInstancePath, ComponentIncarnation>,
    pub identities: Option<&'a BTreeMap<String, String>>,
}

/// Reconcile one window's animation declarations without touching other windows.
///
/// # Errors
///
/// Returns invalid-spec or missing-key errors before mutating the runtime.
pub fn reconcile_node_motion_scoped(
    root: &UiNode,
    runtime: &mut MotionRuntime,
    now: Instant,
    root_path: &str,
) -> Result<BTreeMap<MotionKey, f64>, MotionError> {
    let root_component = ComponentInstancePath::root("UiNode", root_path);
    let incarnations = BTreeMap::new();
    reconcile_node_motion_scoped_owned(
        root,
        runtime,
        now,
        root_path,
        MotionReconcileContext {
            domain: root_path,
            root_component: &root_component,
            root_incarnation: ComponentIncarnation::unscoped(),
            generation: ScriptGeneration::default(),
            incarnations: &incarnations,
            identities: None,
        },
    )
}

#[allow(clippy::too_many_lines)]
pub(crate) fn reconcile_node_motion_scoped_owned(
    root: &UiNode,
    runtime: &mut MotionRuntime,
    now: Instant,
    root_path: &str,
    context: MotionReconcileContext<'_>,
) -> Result<BTreeMap<MotionKey, f64>, MotionError> {
    validate_shared_layout_ids(root)?;
    let mut declarations = Vec::<(String, MotionSource, Option<String>)>::new();
    let mut timelines = Vec::<(String, ComponentInstancePath, MotionTimeline)>::new();
    collect_node_motion(
        root,
        root_path,
        context.root_component,
        &mut declarations,
        &mut timelines,
    )?;
    validate_motion_plan(root, root_path, &declarations, &timelines)?;
    for (_, spec, _) in &declarations {
        validate_source(spec)?;
    }
    let mut candidate = runtime.transaction_snapshot();
    let active_limit = candidate.active_limit;
    candidate.active_limit = usize::MAX;
    if context.identities.is_none() {
        let signature = format!(
            "{:?}:{}",
            root.kind_tag(),
            root.key().map_or("<unkeyed>", crate::NodeKey::as_str)
        );
        if candidate
            .live_scene_signatures
            .get(context.domain)
            .is_some_and(|previous| previous != &signature)
        {
            candidate.reset_live_scope(context.domain);
        }
        candidate
            .live_scene_signatures
            .insert(context.domain.to_owned(), signature);
    }
    let keys = declarations
        .iter()
        .map(|(path, source, _)| {
            MotionKey::for_node(
                context
                    .identities
                    .and_then(|identities| identities.get(path))
                    .map_or(path.as_str(), String::as_str),
                source.property(),
            )
        })
        .collect::<BTreeSet<_>>();
    candidate.retain_live_scope(context.domain, &keys);
    let logical_timelines = timelines
        .iter()
        .map(|(path, _, timeline)| {
            (
                context
                    .identities
                    .and_then(|identities| identities.get(path))
                    .cloned()
                    .unwrap_or_else(|| path.clone()),
                timeline.name.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    candidate.retain_timeline_plan_scope(root_path, &logical_timelines);
    let mut handles = BTreeSet::new();
    for (path, owner, timeline) in timelines {
        let incarnation = context
            .incarnations
            .get(&owner)
            .copied()
            .unwrap_or(context.root_incarnation);
        handles.insert(
            candidate.start_timeline_owned(
                &TimelineOwner {
                    domain: context.domain.to_owned(),
                    component: owner,
                    incarnation,
                    generation: context.generation,
                    node_path: context
                        .identities
                        .and_then(|identities| identities.get(&path))
                        .cloned()
                        .unwrap_or_else(|| path.clone()),
                    target_root: path,
                },
                timeline,
                now,
                context.identities,
            )?,
        );
    }
    for (path, spec, replay_key) in declarations {
        let identity = context
            .identities
            .and_then(|identities| identities.get(&path))
            .cloned()
            .unwrap_or(path);
        candidate.start_with_replay_retained(
            ComponentInstancePath::root("UiNode", identity),
            spec,
            replay_key,
            now,
            MotionRetention::Live(context.domain.to_owned()),
        )?;
    }
    candidate.retain_timeline_scope(root_path, &handles);
    let usage = candidate.resource_usage();
    let active = usage.active.saturating_add(usage.geometry_slots);
    if active > active_limit {
        return Err(MotionError::ActiveBudget {
            actual: active,
            limit: active_limit,
        });
    }
    candidate.active_limit = active_limit;
    let values = candidate.snapshot(now);
    *runtime = candidate;
    Ok(values)
}

fn validate_shared_layout_ids(root: &UiNode) -> Result<(), MotionError> {
    shared_layout_ids(root).map(|_| ())
}

pub(crate) fn shared_layout_ids(root: &UiNode) -> Result<BTreeSet<(String, String)>, MotionError> {
    fn visit(node: &UiNode, seen: &mut BTreeSet<(String, String)>) -> Result<(), MotionError> {
        let group = node.attributes().get("shared_layout_group");
        let id = node.attributes().get("shared_layout_id");
        match (group, id) {
            (Some(crate::UiValue::String(group)), Some(crate::UiValue::String(id))) => {
                if group.is_empty() || id.is_empty() {
                    return Err(MotionError::InvalidSharedLayout(
                        "shared layout group and id must be non-empty".to_owned(),
                    ));
                }
                if !seen.insert((group.clone(), id.clone())) {
                    return Err(MotionError::DuplicateSharedLayout {
                        group: group.clone(),
                        id: id.clone(),
                    });
                }
            }
            (None, None) => {}
            _ => {
                return Err(MotionError::InvalidSharedLayout(
                    "shared layout group and id must be declared together".to_owned(),
                ));
            }
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                for child in children {
                    visit(child, seen)?;
                }
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                visit(trigger, seen)?;
                visit(content, seen)?;
            }
            UiNodeKind::Layer { content, .. } => visit(content, seen)?,
            UiNodeKind::ErrorBoundary { child, fallback } => {
                visit(child, seen)?;
                visit(fallback, seen)?;
            }
            UiNodeKind::VirtualCollection { spec } => {
                for child in spec.realized.values() {
                    visit(child, seen)?;
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Svg { .. }
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. } => {}
        }
        Ok(())
    }
    let mut seen = BTreeSet::new();
    visit(root, &mut seen)?;
    Ok(seen)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn retained_motion_identities(
    root: &UiNode,
    retained: &crate::RetainedUiTree,
    domain: &str,
) -> Result<BTreeMap<String, String>, MotionError> {
    fn group_ids(
        retained: &crate::RetainedUiTree,
        parent: crate::NodeId,
        group: &str,
    ) -> Result<Vec<crate::NodeId>, MotionError> {
        let node = retained
            .node(parent)
            .ok_or_else(|| MotionError::MissingRetainedIdentity(parent.get().to_string()))?;
        Ok(node
            .children()
            .filter(|child| child.group() == group)
            .map(crate::RetainedChildLink::node)
            .collect())
    }

    #[allow(clippy::too_many_lines)]
    fn visit(
        node: &UiNode,
        retained: &crate::RetainedUiTree,
        id: crate::NodeId,
        path: &str,
        domain: &str,
        identities: &mut BTreeMap<String, String>,
    ) -> Result<(), MotionError> {
        let identity = retained_node_path(domain, id);
        if identities
            .insert(path.to_owned(), identity.clone())
            .is_some()
        {
            return Err(MotionError::DuplicateScenePath(path.to_owned()));
        }
        if let UiNodeKind::RichText { spans, .. } = node.kind() {
            for span in spans {
                if let Some(key) = span.key() {
                    identities.insert(
                        span_motion_path(path, key),
                        span_motion_path(&identity, key),
                    );
                }
            }
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                let ids = group_ids(retained, id, "children")?;
                if ids.len() != children.len() {
                    return Err(MotionError::MissingRetainedIdentity(path.to_owned()));
                }
                for (index, (child, child_id)) in children.iter().zip(ids).enumerate() {
                    visit(
                        child,
                        retained,
                        child_id,
                        &child_path(path, index, child),
                        domain,
                        identities,
                    )?;
                }
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                let trigger_id = group_ids(retained, id, "trigger")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        MotionError::MissingRetainedIdentity(format!("{path}/trigger"))
                    })?;
                let content_id = group_ids(retained, id, "content")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        MotionError::MissingRetainedIdentity(format!("{path}/content"))
                    })?;
                visit(
                    trigger,
                    retained,
                    trigger_id,
                    &format!("{path}/trigger"),
                    domain,
                    identities,
                )?;
                visit(
                    content,
                    retained,
                    content_id,
                    &format!("{path}/content"),
                    domain,
                    identities,
                )?;
            }
            UiNodeKind::Layer { content, .. } => {
                let child_id = group_ids(retained, id, "content")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        MotionError::MissingRetainedIdentity(format!("{path}/content"))
                    })?;
                visit(
                    content,
                    retained,
                    child_id,
                    &format!("{path}/content"),
                    domain,
                    identities,
                )?;
            }
            UiNodeKind::ErrorBoundary { child, fallback } => {
                let child_id = group_ids(retained, id, "child")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        MotionError::MissingRetainedIdentity(format!("{path}/boundary"))
                    })?;
                let fallback_id = group_ids(retained, id, "fallback")?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        MotionError::MissingRetainedIdentity(format!("{path}/fallback"))
                    })?;
                visit(
                    child,
                    retained,
                    child_id,
                    &format!("{path}/boundary"),
                    domain,
                    identities,
                )?;
                visit(
                    fallback,
                    retained,
                    fallback_id,
                    &format!("{path}/fallback"),
                    domain,
                    identities,
                )?;
            }
            UiNodeKind::VirtualCollection { spec } => {
                let ids = group_ids(retained, id, "items")?;
                if ids.len() != spec.realized.len() {
                    return Err(MotionError::MissingRetainedIdentity(path.to_owned()));
                }
                for ((index, item), item_id) in spec.realized.iter().zip(ids) {
                    let key = crate::virtual_list_element::collection_item_key(spec, *index)
                        .ok_or_else(|| MotionError::MissingVirtualItemKey {
                            path: path.to_owned(),
                            index: *index,
                        })?;
                    visit(
                        item,
                        retained,
                        item_id,
                        &virtual_item_path(path, key),
                        domain,
                        identities,
                    )?;
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::RichText { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Svg { .. }
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. } => {}
        }
        Ok(())
    }

    let root_id = retained
        .root_id()
        .ok_or_else(|| MotionError::MissingRetainedIdentity(domain.to_owned()))?;
    let mut identities = BTreeMap::new();
    visit(root, retained, root_id, domain, domain, &mut identities)?;
    Ok(identities)
}

#[must_use]
pub fn node_motion_resource_usage(root: &UiNode) -> MotionResourceUsage {
    fn visit(node: &UiNode, usage: &mut MotionResourceUsage) {
        if let Some(crate::UiValue::Integer(count)) = node.attributes().get("motion_particle_count")
        {
            usage.particles = usage
                .particles
                .saturating_add(usize::try_from(*count).unwrap_or(usize::MAX));
        }
        if node.attributes().contains_key("shared_layout_id") {
            usage.shared_snapshots = usage.shared_snapshots.saturating_add(1);
        }
        if node.attributes().contains_key("layout_motion_duration_ms") {
            usage.geometry_slots = usage.geometry_slots.saturating_add(1);
            usage.declarations = usage.declarations.saturating_add(1);
        }
        usage.geometry_slots = usage
            .geometry_slots
            .saturating_add(node.progress_motions().len());
        usage.declarations = usage.declarations.saturating_add(node.motions().len());
        usage.declarations = usage.declarations.saturating_add(node.exit_motions().len());
        usage.declarations = usage
            .declarations
            .saturating_add(node.progress_motions().len());
        usage.keyframes = usage.keyframes.saturating_add(
            node.motions()
                .iter()
                .map(source_keyframe_count)
                .sum::<usize>(),
        );
        usage.keyframes = usage.keyframes.saturating_add(
            node.exit_motions()
                .iter()
                .map(source_keyframe_count)
                .sum::<usize>(),
        );
        usage.keyframes = usage.keyframes.saturating_add(
            node.progress_motions()
                .iter()
                .map(|binding| source_keyframe_count(&binding.source))
                .sum::<usize>(),
        );
        usage.timelines = usage.timelines.saturating_add(node.timelines().len());
        for timeline in node.timelines() {
            usage.timeline_steps = usage
                .timeline_steps
                .saturating_add(timeline_step_count(&timeline.root));
            visit_timeline_sources(&timeline.root, &mut |source| {
                usage.declarations = usage.declarations.saturating_add(1);
                usage.keyframes = usage
                    .keyframes
                    .saturating_add(source_keyframe_count(source));
            });
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                for child in children {
                    visit(child, usage);
                }
            }
            UiNodeKind::Overlay {
                trigger, content, ..
            } => {
                visit(trigger, usage);
                visit(content, usage);
            }
            UiNodeKind::Layer { content, .. } => visit(content, usage),
            UiNodeKind::ErrorBoundary { child, fallback } => {
                visit(child, usage);
                visit(fallback, usage);
            }
            UiNodeKind::VirtualCollection { spec } => {
                for child in spec.realized.values() {
                    visit(child, usage);
                }
            }
            UiNodeKind::RichText { spans, .. } => {
                for span in spans {
                    usage.declarations = usage.declarations.saturating_add(span.motions().len());
                    usage.keyframes = usage.keyframes.saturating_add(
                        span.motions()
                            .iter()
                            .map(source_keyframe_count)
                            .sum::<usize>(),
                    );
                }
            }
            UiNodeKind::Text { .. }
            | UiNodeKind::Canvas { .. }
            | UiNodeKind::Svg { .. }
            | UiNodeKind::Custom { .. }
            | UiNodeKind::Image { .. }
            | UiNodeKind::DirectionalImage { .. } => {}
        }
    }

    let mut usage = MotionResourceUsage::default();
    visit(root, &mut usage);
    usage
}

fn source_keyframe_count(source: &MotionSource) -> usize {
    match source {
        MotionSource::Keyframes(spec) => spec.frames.len(),
        MotionSource::Transition(_) | MotionSource::Spring(_) | MotionSource::Inertia(_) => 0,
    }
}

fn timeline_step_count(step: &MotionTimelineStep) -> usize {
    1usize.saturating_add(match step {
        MotionTimelineStep::Track(_) | MotionTimelineStep::Delay(_) => 0,
        MotionTimelineStep::Sequence(steps)
        | MotionTimelineStep::Parallel(steps)
        | MotionTimelineStep::Stagger { steps, .. } => steps.iter().map(timeline_step_count).sum(),
    })
}

fn visit_timeline_sources(step: &MotionTimelineStep, visit: &mut impl FnMut(&MotionSource)) {
    match step {
        MotionTimelineStep::Track(track) => visit(&track.source),
        MotionTimelineStep::Delay(_) => {}
        MotionTimelineStep::Sequence(steps)
        | MotionTimelineStep::Parallel(steps)
        | MotionTimelineStep::Stagger { steps, .. } => {
            for step in steps {
                visit_timeline_sources(step, visit);
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn collect_node_motion(
    node: &UiNode,
    path: &str,
    inherited_owner: &ComponentInstancePath,
    output: &mut Vec<(String, MotionSource, Option<String>)>,
    timelines: &mut Vec<(String, ComponentInstancePath, MotionTimeline)>,
) -> Result<(), MotionError> {
    let owner = node.component_root().unwrap_or(inherited_owner);
    if (!node.motions().is_empty()
        || !node.exit_motions().is_empty()
        || !node.progress_motions().is_empty()
        || !node.timelines().is_empty())
        && node.key().is_none()
    {
        return Err(MotionError::MissingKey(path.to_owned()));
    }
    if !node.exit_motions().is_empty() && node.motion_ghost().is_none() {
        return Err(MotionError::UnsupportedExitGhost(format!(
            "{:?}",
            node.kind_tag()
        )));
    }
    for source in node.exit_motions() {
        validate_source(source)?;
        validate_node_property(node, source.property(), path)?;
    }
    for binding in node.progress_motions() {
        validate_progress_source(&binding.source)?;
        validate_node_property(node, binding.property(), path)?;
        if node
            .motions()
            .iter()
            .any(|source| source.property() == binding.property())
        {
            return Err(MotionError::DuplicatePropertySource {
                path: path.to_owned(),
                property: binding.property(),
            });
        }
    }
    for source in node.motions() {
        validate_node_property(node, source.property(), path)?;
    }
    let replay_key = match node.attributes().get("motion_replay_key") {
        Some(crate::UiValue::String(key)) => Some(key.clone()),
        _ => None,
    };
    output.extend(
        node.motions()
            .iter()
            .cloned()
            .map(|animation| (path.to_owned(), animation, replay_key.clone())),
    );
    timelines.extend(
        node.timelines()
            .iter()
            .cloned()
            .map(|timeline| (path.to_owned(), owner.clone(), timeline)),
    );
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for (index, child) in children.iter().enumerate() {
                collect_node_motion(
                    child,
                    &child_path(path, index, child),
                    owner,
                    output,
                    timelines,
                )?;
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            collect_node_motion(
                trigger,
                &format!("{path}/trigger"),
                owner,
                output,
                timelines,
            )?;
            collect_node_motion(
                content,
                &format!("{path}/content"),
                owner,
                output,
                timelines,
            )?;
        }
        UiNodeKind::Layer { content, .. } => {
            collect_node_motion(
                content,
                &format!("{path}/content"),
                owner,
                output,
                timelines,
            )?;
        }
        UiNodeKind::ErrorBoundary { child, fallback } => {
            collect_node_motion(child, &format!("{path}/boundary"), owner, output, timelines)?;
            collect_node_motion(
                fallback,
                &format!("{path}/fallback"),
                owner,
                output,
                timelines,
            )?;
        }
        UiNodeKind::VirtualCollection { spec } => {
            for (index, item) in &spec.realized {
                let key = crate::virtual_list_element::collection_item_key(spec, *index)
                    .ok_or_else(|| MotionError::MissingVirtualItemKey {
                        path: path.to_owned(),
                        index: *index,
                    })?;
                collect_node_motion(
                    item,
                    &virtual_item_path(path, key),
                    owner,
                    output,
                    timelines,
                )?;
            }
        }
        UiNodeKind::RichText { spans, .. } => {
            for (index, span) in spans.iter().enumerate() {
                if span.motions().is_empty() {
                    continue;
                }
                let key = span
                    .key()
                    .ok_or_else(|| MotionError::MissingKey(format!("{path}/span-index:{index}")))?;
                let span_path = span_motion_path(path, key);
                if span
                    .motions()
                    .iter()
                    .any(|source| source.property() != MotionProperty::Opacity)
                {
                    return Err(MotionError::UnsupportedProperty {
                        path: span_path,
                        property: span
                            .motions()
                            .iter()
                            .find(|source| source.property() != MotionProperty::Opacity)
                            .map_or(MotionProperty::Opacity, MotionSource::property),
                        node: "rich_text_span",
                    });
                }
                output.extend(
                    span.motions()
                        .iter()
                        .cloned()
                        .map(|source| (span_path.clone(), source, replay_key.clone())),
                );
            }
        }
        UiNodeKind::Text { .. }
        | UiNodeKind::Canvas { .. }
        | UiNodeKind::Svg { .. }
        | UiNodeKind::Custom { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. } => {}
    }
    Ok(())
}

enum MotionSceneTarget<'a> {
    Node(&'a UiNode),
    Span,
}

fn validate_motion_plan(
    root: &UiNode,
    root_path: &str,
    declarations: &[(String, MotionSource, Option<String>)],
    timelines: &[(String, ComponentInstancePath, MotionTimeline)],
) -> Result<(), MotionError> {
    let mut scene = BTreeMap::new();
    collect_motion_scene(root, root_path, &mut scene)?;
    let mut owners = BTreeMap::<(String, MotionProperty), String>::new();
    for (path, source, _) in declarations {
        insert_property_owner(&mut owners, path, source.property(), "motion declaration")?;
    }
    collect_non_timeline_owners(root, root_path, &mut owners)?;
    for (base, _, timeline) in timelines {
        let (tracks, _) = compile_timeline(&timeline.root)?;
        let mut timeline_properties = BTreeSet::new();
        for track in tracks {
            let target_path = resolve_timeline_target(base, &track.target);
            let target =
                scene
                    .get(&target_path)
                    .ok_or_else(|| MotionError::UnknownTimelineTarget {
                        timeline: timeline.name.clone(),
                        target: target_path.clone(),
                    })?;
            validate_scene_target_property(target, track.source.property(), &target_path)?;
            let key = (target_path.clone(), track.source.property());
            if timeline_properties.insert(key.clone()) {
                insert_property_owner(
                    &mut owners,
                    &target_path,
                    track.source.property(),
                    &format!("timeline `{}`", timeline.name),
                )?;
            }
        }
    }
    Ok(())
}

fn insert_property_owner(
    owners: &mut BTreeMap<(String, MotionProperty), String>,
    path: &str,
    property: MotionProperty,
    owner: &str,
) -> Result<(), MotionError> {
    let key = (path.to_owned(), property);
    if let Some(previous) = owners.insert(key, owner.to_owned()) {
        Err(MotionError::PropertyOwnerConflict {
            path: path.to_owned(),
            property,
            first: previous,
            second: owner.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn resolve_timeline_target(base: &str, target: &str) -> String {
    match target {
        "." | "" => base.to_owned(),
        target => target.split('/').fold(base.to_owned(), |path, segment| {
            format!("{path}/{}", encode_path_segment("key", segment))
        }),
    }
}

fn validate_scene_target_property(
    target: &MotionSceneTarget<'_>,
    property: MotionProperty,
    path: &str,
) -> Result<(), MotionError> {
    match target {
        MotionSceneTarget::Node(node) => validate_node_property(node, property, path),
        MotionSceneTarget::Span if property == MotionProperty::Opacity => Ok(()),
        MotionSceneTarget::Span => Err(MotionError::UnsupportedProperty {
            path: path.to_owned(),
            property,
            node: "rich_text_span",
        }),
    }
}

fn collect_non_timeline_owners(
    node: &UiNode,
    path: &str,
    owners: &mut BTreeMap<(String, MotionProperty), String>,
) -> Result<(), MotionError> {
    for binding in node.progress_motions() {
        insert_property_owner(owners, path, binding.property(), "progress binding")?;
    }
    for (property, _) in node.signal_bindings() {
        let property = match property {
            crate::SignalProperty::Opacity => Some(MotionProperty::Opacity),
            crate::SignalProperty::TranslateX => Some(MotionProperty::TranslateX),
            crate::SignalProperty::TranslateY => Some(MotionProperty::TranslateY),
            crate::SignalProperty::Width | crate::SignalProperty::WidthOverride => {
                Some(MotionProperty::Width)
            }
            crate::SignalProperty::Height => Some(MotionProperty::Height),
            crate::SignalProperty::Background
            | crate::SignalProperty::TextColor
            | crate::SignalProperty::BorderColor => None,
        };
        if let Some(property) = property {
            insert_property_owner(owners, path, property, "native signal")?;
        }
    }
    visit_motion_children(node, path, |child, child_path| {
        collect_non_timeline_owners(child, child_path, owners)
    })
}

fn collect_motion_scene<'a>(
    node: &'a UiNode,
    path: &str,
    scene: &mut BTreeMap<String, MotionSceneTarget<'a>>,
) -> Result<(), MotionError> {
    if scene
        .insert(path.to_owned(), MotionSceneTarget::Node(node))
        .is_some()
    {
        return Err(MotionError::DuplicateScenePath(path.to_owned()));
    }
    if let UiNodeKind::RichText { spans, .. } = node.kind() {
        for span in spans {
            if let Some(key) = span.key() {
                let span_path = span_motion_path(path, key);
                if scene
                    .insert(span_path.clone(), MotionSceneTarget::Span)
                    .is_some()
                {
                    return Err(MotionError::DuplicateScenePath(span_path));
                }
            }
        }
    }
    visit_motion_children(node, path, |child, child_path| {
        collect_motion_scene(child, child_path, scene)
    })
}

fn visit_motion_children<'a>(
    node: &'a UiNode,
    path: &str,
    mut visit: impl FnMut(&'a UiNode, &str) -> Result<(), MotionError>,
) -> Result<(), MotionError> {
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for (index, child) in children.iter().enumerate() {
                visit(child, &child_path(path, index, child))?;
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            visit(trigger, &format!("{path}/trigger"))?;
            visit(content, &format!("{path}/content"))?;
        }
        UiNodeKind::Layer { content, .. } => visit(content, &format!("{path}/content"))?,
        UiNodeKind::ErrorBoundary { child, fallback } => {
            visit(child, &format!("{path}/boundary"))?;
            visit(fallback, &format!("{path}/fallback"))?;
        }
        UiNodeKind::VirtualCollection { spec } => {
            for (index, item) in &spec.realized {
                let key = crate::virtual_list_element::collection_item_key(spec, *index)
                    .ok_or_else(|| MotionError::MissingVirtualItemKey {
                        path: path.to_owned(),
                        index: *index,
                    })?;
                visit(item, &virtual_item_path(path, key))?;
            }
        }
        UiNodeKind::Text { .. }
        | UiNodeKind::RichText { .. }
        | UiNodeKind::Canvas { .. }
        | UiNodeKind::Svg { .. }
        | UiNodeKind::Custom { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. } => {}
    }
    Ok(())
}

fn validate_node_property(
    node: &UiNode,
    property: MotionProperty,
    path: &str,
) -> Result<(), MotionError> {
    if matches!(
        property,
        MotionProperty::Rotate
            | MotionProperty::ScaleX
            | MotionProperty::ScaleY
            | MotionProperty::SkewX
            | MotionProperty::SkewY
            | MotionProperty::PathProgress
    ) && !matches!(node.kind(), UiNodeKind::Canvas { .. })
    {
        Err(MotionError::UnsupportedProperty {
            path: path.to_owned(),
            property,
            node: "non_canvas",
        })
    } else if matches!(
        property,
        MotionProperty::Rotate
            | MotionProperty::ScaleX
            | MotionProperty::ScaleY
            | MotionProperty::SkewX
            | MotionProperty::SkewY
    ) && matches!(node.kind(), UiNodeKind::Canvas { scene } if scene.commands().iter().any(|command| {
        matches!(
            command,
            crate::CanvasCommand::Path { clip: Some(_), .. }
                | crate::CanvasCommand::MorphPath { clip: Some(_), .. }
        )
    })) {
        Err(MotionError::UnsupportedProperty {
            path: path.to_owned(),
            property,
            node: "canvas_with_clipped_path",
        })
    } else {
        Ok(())
    }
}

pub(crate) fn child_path(path: &str, index: usize, child: &UiNode) -> String {
    child.key().map_or_else(
        || format!("{path}/index:{index}"),
        |key| format!("{path}/{}", encode_path_segment("key", key.as_str())),
    )
}

#[derive(Clone, Copy, Debug)]
struct SourceSample {
    value: f64,
    velocity: Option<f64>,
    done: bool,
}

fn motion_elapsed(motion: &ActiveMotion, now: Instant) -> Duration {
    motion
        .elapsed_before_play
        .saturating_add(now.saturating_duration_since(motion.started))
}

fn sample_active_motion(motion: &ActiveMotion, now: Instant, suspended: bool) -> SourceSample {
    let elapsed = if suspended {
        motion.elapsed_before_play
    } else {
        motion_elapsed(motion, now)
    };
    sample_source_with_duration(&motion.source, elapsed, motion.finite_duration_ms)
}

fn source_initial_value(source: &MotionSource) -> f64 {
    match source {
        MotionSource::Transition(spec) => spec.from,
        MotionSource::Spring(spec) => spec.from,
        MotionSource::Keyframes(spec) => spec.frames.first().map_or(0.0, |frame| frame.value),
        MotionSource::Inertia(spec) => spec.from,
    }
}

fn source_terminal_value(source: &MotionSource) -> f64 {
    source_duration_ms(source).map_or_else(
        |_| source.reduced_value(),
        |duration| {
            sample_source_with_duration(source, Duration::from_millis(duration), Some(duration))
                .value
        },
    )
}

fn retarget_source(
    declaration: &MotionSource,
    current: f64,
    inherited_velocity: Option<f64>,
) -> MotionSource {
    match declaration {
        MotionSource::Transition(spec) => {
            let mut spec = spec.clone();
            spec.from = current;
            MotionSource::Transition(spec)
        }
        MotionSource::Spring(spec) => {
            let mut spec = spec.clone();
            spec.from = current;
            spec.initial_velocity = inherited_velocity.unwrap_or(spec.initial_velocity);
            MotionSource::Spring(spec)
        }
        MotionSource::Keyframes(spec) => {
            let mut spec = spec.clone();
            if let Some(first) = spec.frames.first_mut() {
                first.value = current;
            }
            MotionSource::Keyframes(spec)
        }
        MotionSource::Inertia(spec) => {
            let mut spec = spec.clone();
            spec.from = current;
            MotionSource::Inertia(spec)
        }
    }
}

#[cfg(test)]
fn sample_source(source: &MotionSource, elapsed: Duration) -> SourceSample {
    sample_source_with_duration(source, elapsed, source_duration_ms(source).ok())
}

fn sample_source_with_duration(
    source: &MotionSource,
    elapsed: Duration,
    finite_duration_ms: Option<u64>,
) -> SourceSample {
    let elapsed_ms = duration_ms(elapsed);
    let sample = match source {
        MotionSource::Transition(spec) => {
            let (progress, done) = timed_progress(
                elapsed_ms,
                spec.delay_ms,
                spec.duration_ms,
                spec.iterations,
                spec.autoreverse,
            );
            SourceSample {
                value: spec.from + (spec.to - spec.from) * spec.easing.sample(progress),
                velocity: None,
                done,
            }
        }
        MotionSource::Keyframes(spec) => {
            let (progress, done) = timed_progress(
                elapsed_ms,
                spec.delay_ms,
                spec.duration_ms,
                spec.iterations,
                spec.autoreverse,
            );
            SourceSample {
                value: sample_keyframes(&spec.frames, progress),
                velocity: None,
                done,
            }
        }
        MotionSource::Spring(spec) => {
            let settle_ms = finite_duration_ms.unwrap_or_else(|| spring_settle_ms(spec));
            let done = elapsed_ms >= settle_ms;
            let (value, velocity) = if done {
                (spec.to, 0.0)
            } else {
                sample_spring_at(spec, elapsed.as_secs_f64())
            };
            SourceSample {
                value,
                velocity: Some(velocity),
                done,
            }
        }
        MotionSource::Inertia(spec) => {
            let settle_ms = finite_duration_ms.unwrap_or_else(|| inertia_settle_ms(spec));
            let done = elapsed_ms >= settle_ms;
            let (value, velocity) =
                sample_inertia_at(spec, elapsed.min(Duration::from_millis(settle_ms)));
            SourceSample {
                value,
                velocity: Some(if done { 0.0 } else { velocity }),
                done,
            }
        }
    };
    if sample.value.is_finite() && sample.velocity.is_none_or(f64::is_finite) {
        sample
    } else {
        SourceSample {
            value: source_initial_value(source),
            velocity: Some(0.0),
            done: true,
        }
    }
}

fn timed_progress(
    elapsed_ms: u64,
    delay_ms: u64,
    duration_ms: u64,
    iterations: Option<u32>,
    autoreverse: bool,
) -> (f64, bool) {
    if elapsed_ms < delay_ms {
        return (0.0, false);
    }
    let elapsed = elapsed_ms.saturating_sub(delay_ms);
    let duration = duration_ms.max(1);
    let raw_iteration = elapsed / duration;
    let done = iterations.is_some_and(|count| raw_iteration >= u64::from(count));
    let cycle = if done {
        u64::from(iterations.unwrap_or(1).saturating_sub(1))
    } else {
        raw_iteration
    };
    let mut progress = if done {
        1.0
    } else {
        Duration::from_millis(elapsed % duration).as_secs_f64()
            / Duration::from_millis(duration).as_secs_f64()
    };
    if autoreverse && cycle % 2 == 1 {
        progress = 1.0 - progress;
    }
    (progress, done)
}

fn reduced_value(animation: &ActiveMotion) -> f64 {
    animation.declaration.reduced_value()
}

fn validate_source(spec: &MotionSource) -> Result<(), MotionError> {
    let finite = match spec {
        MotionSource::Transition(spec) => {
            spec.from.is_finite()
                && spec.to.is_finite()
                && spec.duration_ms > 0
                && spec.iterations != Some(0)
        }
        MotionSource::Spring(spec) => valid_spring(spec),
        MotionSource::Keyframes(spec) => {
            spec.duration_ms > 0 && spec.iterations != Some(0) && valid_keyframes(&spec.frames)
        }
        MotionSource::Inertia(spec) => {
            spec.from.is_finite()
                && spec.velocity.is_finite()
                && spec.friction.is_finite()
                && (1.0e-9..=1.0e9).contains(&spec.friction)
                && spec.from.abs() <= 1.0e12
                && spec.velocity.abs() <= 1.0e12
                && spec.min.is_none_or(f64::is_finite)
                && spec.max.is_none_or(f64::is_finite)
                && spec.min.zip(spec.max).is_none_or(|(min, max)| min <= max)
                && spec.bounce.is_finite()
                && (0.0..=1.0).contains(&spec.bounce)
                && spec.snap_points.len() <= 1_024
                && spec.snap_points.iter().all(|point| point.is_finite())
        }
    };
    if finite {
        Ok(())
    } else {
        Err(MotionError::InvalidSpec)
    }
}

fn valid_spring(spec: &MotionSpring) -> bool {
    if !spec.from.is_finite()
        || !spec.to.is_finite()
        || !spec.initial_velocity.is_finite()
        || spec.from.abs() > 1.0e12
        || spec.to.abs() > 1.0e12
        || spec.initial_velocity.abs() > 1.0e12
        || !(1.0e-9..=1.0e12).contains(&spec.stiffness)
        || !(0.0..=1.0e12).contains(&spec.damping)
        || !(1.0e-9..=1.0e12).contains(&spec.mass)
    {
        return false;
    }
    let ratio = spec.stiffness / spec.mass;
    let product = spec.stiffness * spec.mass;
    ratio.is_finite()
        && product.is_finite()
        && ratio.sqrt().is_finite()
        && product.sqrt().is_finite()
}

fn sample_keyframes(frames: &[MotionKeyframe], progress: f64) -> f64 {
    let Some(first) = frames.first() else {
        return 0.0;
    };
    if progress <= first.offset {
        return first.value;
    }
    for pair in frames.windows(2) {
        let [left, right] = pair else { continue };
        if progress <= right.offset {
            let width = right.offset - left.offset;
            let local = if width <= f64::EPSILON {
                1.0
            } else {
                (progress - left.offset) / width
            };
            return left.value + (right.value - left.value) * right.easing.sample(local);
        }
    }
    frames.last().map_or(first.value, |frame| frame.value)
}

fn valid_keyframes(frames: &[MotionKeyframe]) -> bool {
    frames.len() >= 2
        && frames.len() <= 4_096
        && frames
            .first()
            .is_some_and(|frame| frame.offset.abs() < f64::EPSILON)
        && frames
            .last()
            .is_some_and(|frame| (frame.offset - 1.0).abs() < f64::EPSILON)
        && frames.iter().all(|frame| {
            frame.offset.is_finite()
                && (0.0..=1.0).contains(&frame.offset)
                && frame.value.is_finite()
        })
        && frames
            .windows(2)
            .all(|pair| pair[0].offset < pair[1].offset)
}

fn inertia_target(spec: &MotionInertia) -> f64 {
    let projected = spec.from + spec.velocity / spec.friction.max(f64::EPSILON);
    let clamped = projected.clamp(
        spec.min.unwrap_or(f64::NEG_INFINITY),
        spec.max.unwrap_or(f64::INFINITY),
    );
    nearest_snap(clamped, &spec.snap_points).unwrap_or(clamped)
}

fn nearest_snap(value: f64, points: &[f64]) -> Option<f64> {
    points
        .iter()
        .copied()
        .min_by(|left, right| (value - *left).abs().total_cmp(&(value - *right).abs()))
}

pub(crate) fn register_motion_api(engine: &mut Engine) {
    engine.build_type::<MotionSource>();
    engine.build_type::<MotionTimelineStep>();
    engine.build_type::<MotionTimeline>();
    engine.build_type::<MotionHandle>();
    engine.build_type::<MotionProgressBinding>();
    FuncRegistration::new("motion_transition")
        .in_global_namespace()
        .register_into_engine(engine, motion_transition_from_script);
    FuncRegistration::new("motion_spring")
        .in_global_namespace()
        .register_into_engine(engine, motion_spring_from_script);
    FuncRegistration::new("motion_keyframes")
        .in_global_namespace()
        .register_into_engine(engine, motion_keyframes_from_script);
    FuncRegistration::new("motion_inertia")
        .in_global_namespace()
        .register_into_engine(engine, motion_inertia_from_script);
    FuncRegistration::new("motion_path_follow")
        .in_global_namespace()
        .register_into_engine(engine, motion_path_follow_from_script);
    FuncRegistration::new("motion_in_view")
        .in_global_namespace()
        .register_into_engine(engine, |source: MotionSource| {
            MotionProgressBinding::new(MotionProgressDriver::InView, source)
                .map_err(script_boxed_error)
        });
    FuncRegistration::new("motion_viewport")
        .in_global_namespace()
        .register_into_engine(engine, |source: MotionSource| {
            MotionProgressBinding::new(MotionProgressDriver::Viewport, source)
                .map_err(script_boxed_error)
        });
    FuncRegistration::new("motion_scroll")
        .in_global_namespace()
        .register_into_engine(engine, motion_scroll_from_script);
    for (name, driver) in [
        ("motion_hover", MotionProgressDriver::Hover),
        ("motion_press", MotionProgressDriver::Press),
        ("motion_focus", MotionProgressDriver::Focus),
    ] {
        FuncRegistration::new(name)
            .in_global_namespace()
            .register_into_engine(engine, move |source: MotionSource| {
                MotionProgressBinding::new(driver, source).map_err(script_boxed_error)
            });
    }
    FuncRegistration::new("motion_text_spans")
        .in_global_namespace()
        .register_into_engine(engine, motion_text_spans_from_script);
    FuncRegistration::new("motion_track")
        .in_global_namespace()
        .register_into_engine(engine, |target: &str, source: MotionSource| {
            MotionTimelineStep::Track(MotionTrack {
                target: target.to_owned(),
                source,
            })
        });
    FuncRegistration::new("motion_delay")
        .in_global_namespace()
        .register_into_engine(engine, motion_delay_from_script);
    FuncRegistration::new("motion_sequence")
        .in_global_namespace()
        .register_into_engine(engine, motion_sequence_from_script);
    FuncRegistration::new("motion_parallel")
        .in_global_namespace()
        .register_into_engine(engine, motion_parallel_from_script);
    FuncRegistration::new("motion_stagger")
        .in_global_namespace()
        .register_into_engine(engine, motion_stagger_from_script);
    FuncRegistration::new("motion_timeline")
        .in_global_namespace()
        .register_into_engine(engine, motion_timeline_from_script);
}

fn motion_text_spans_from_script(text: &str, mut config: Map) -> Result<Array, Box<EvalAltResult>> {
    let duration_ms = take_u64(&mut config, "duration_ms")?.unwrap_or(180);
    let stagger_ms = take_u64(&mut config, "stagger_ms")?.unwrap_or(24);
    let easing = take_string(&mut config, "easing")?
        .map_or(Ok(MotionEasing::EaseOut), |value| {
            MotionEasing::parse(&value)
        })
        .map_err(script_boxed_error)?;
    let intent = take_intent(&mut config)?.unwrap_or(MotionIntent::Decorative);
    reject_unknown_config(&config)?;
    Ok(text
        .graphemes(true)
        .enumerate()
        .map(|(index, grapheme)| {
            let delay_ms = stagger_ms.saturating_mul(u64::try_from(index).unwrap_or(u64::MAX));
            let mut transition =
                MotionTransition::new(MotionProperty::Opacity, 0.0, 1.0, duration_ms);
            transition.delay_ms = delay_ms;
            transition.easing = easing;
            transition.intent = intent;
            Dynamic::from(
                crate::Span::new(grapheme)
                    .with_key(format!("grapheme-{index}"))
                    .motion(MotionSource::Transition(transition)),
            )
        })
        .collect::<Array>())
}

fn motion_scroll_from_script(
    axis: &str,
    source: MotionSource,
) -> Result<MotionProgressBinding, Box<EvalAltResult>> {
    let driver = match axis {
        "x" | "horizontal" => MotionProgressDriver::ScrollX,
        "y" | "vertical" => MotionProgressDriver::ScrollY,
        _ => return Err(script_boxed_error("motion_scroll axis must be `x` or `y`")),
    };
    MotionProgressBinding::new(driver, source).map_err(script_boxed_error)
}

#[allow(clippy::needless_pass_by_value)]
fn motion_path_follow_from_script(
    scene: crate::CanvasScene,
    key: &str,
    property: &str,
    mut config: Map,
) -> Result<MotionSource, Box<EvalAltResult>> {
    let property = MotionProperty::parse(property).map_err(script_boxed_error)?;
    if !matches!(
        property,
        MotionProperty::TranslateX | MotionProperty::TranslateY | MotionProperty::Rotate
    ) {
        return Err(script_boxed_error(
            "motion_path_follow property must be `translate_x`, `translate_y`, or `rotate`",
        ));
    }
    let samples = take_u64(&mut config, "samples")?.unwrap_or(128);
    if !(16..=512).contains(&samples) {
        return Err(script_boxed_error(
            "motion_path_follow samples must be between 16 and 512",
        ));
    }
    let mut frames = Vec::with_capacity(usize::try_from(samples + 1).unwrap_or(513));
    let samples_u32 = u32::try_from(samples).unwrap_or(512);
    for index in 0..=samples_u32 {
        let offset = f64::from(index) / f64::from(samples_u32);
        let sample = scene.sample_path(key, offset).map_err(script_boxed_error)?;
        let value = match property {
            MotionProperty::TranslateX => sample.x,
            MotionProperty::TranslateY => sample.y,
            MotionProperty::Rotate => sample.tangent_degrees,
            _ => unreachable!(),
        };
        frames.push(MotionKeyframe {
            offset,
            value,
            easing: MotionEasing::Linear,
        });
    }
    let source = MotionSource::Keyframes(MotionKeyframes {
        property,
        frames,
        delay_ms: take_u64(&mut config, "delay_ms")?.unwrap_or(0),
        duration_ms: take_u64(&mut config, "duration_ms")?.unwrap_or(1_000),
        iterations: take_iterations(&mut config)?.unwrap_or(Some(1)),
        autoreverse: take_bool(&mut config, "autoreverse")?.unwrap_or(false),
        intent: take_intent(&mut config)?.unwrap_or(MotionIntent::Decorative),
    });
    reject_unknown_config(&config)?;
    checked_source(source)
}

fn motion_delay_from_script(duration_ms: INT) -> Result<MotionTimelineStep, Box<EvalAltResult>> {
    let duration_ms = u64::try_from(duration_ms)
        .map_err(|_| script_boxed_error("motion delay must be non-negative"))?;
    Ok(MotionTimelineStep::Delay(duration_ms))
}

fn timeline_steps(
    values: Array,
    kind: &str,
) -> Result<Vec<MotionTimelineStep>, Box<EvalAltResult>> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            value.try_cast::<MotionTimelineStep>().ok_or_else(|| {
                script_boxed_error(format!(
                    "{kind} item {index} must be a motion timeline step"
                ))
            })
        })
        .collect()
}

fn motion_sequence_from_script(values: Array) -> Result<MotionTimelineStep, Box<EvalAltResult>> {
    Ok(MotionTimelineStep::Sequence(timeline_steps(
        values,
        "motion_sequence",
    )?))
}

fn motion_parallel_from_script(values: Array) -> Result<MotionTimelineStep, Box<EvalAltResult>> {
    Ok(MotionTimelineStep::Parallel(timeline_steps(
        values,
        "motion_parallel",
    )?))
}

fn motion_stagger_from_script(
    values: Array,
    interval_ms: INT,
) -> Result<MotionTimelineStep, Box<EvalAltResult>> {
    let interval_ms = u64::try_from(interval_ms)
        .map_err(|_| script_boxed_error("motion stagger interval must be non-negative"))?;
    Ok(MotionTimelineStep::Stagger {
        interval_ms,
        steps: timeline_steps(values, "motion_stagger")?,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn motion_timeline_from_script(
    call: NativeCallContext<'_>,
    name: &str,
    root: MotionTimelineStep,
    mut config: Map,
) -> Result<MotionTimeline, Box<EvalAltResult>> {
    validate_timeline_name(name).map_err(script_boxed_error)?;
    let timeline = MotionTimeline {
        name: name.to_owned(),
        root,
        autoplay: take_bool(&mut config, "autoplay")?.unwrap_or(true),
        iterations: take_iterations(&mut config)?.unwrap_or(Some(1)),
        autoreverse: take_bool(&mut config, "autoreverse")?.unwrap_or(false),
        intent: take_intent(&mut config)?.unwrap_or(MotionIntent::Decorative),
        on_complete: take_callback(&call, &mut config, "on_complete")?,
        on_cancel: take_callback(&call, &mut config, "on_cancel")?,
    };
    reject_unknown_config(&config)?;
    compile_timeline(&timeline.root).map_err(script_boxed_error)?;
    Ok(timeline)
}

fn take_callback(
    call: &NativeCallContext<'_>,
    config: &mut Map,
    key: &str,
) -> Result<Option<ScriptCallback>, Box<EvalAltResult>> {
    let Some(value) = take_dynamic(config, key) else {
        return Ok(None);
    };
    let function = value.try_cast::<FnPtr>().ok_or_else(|| {
        script_boxed_error(format!("motion option `{key}` must be a function pointer"))
    })?;
    let mut callback = ScriptCallback::try_from_fn_ptr(function, ScriptGeneration::default())
        .map_err(script_boxed_error)?;
    callback
        .bind_native_context_if_unset(crate::invocation::ScriptInvocationContext::capture(call));
    Ok(Some(callback))
}

fn motion_transition_from_script(
    property: &str,
    from: FLOAT,
    to: FLOAT,
    mut config: Map,
) -> Result<MotionSource, Box<EvalAltResult>> {
    let property = script_property(property)?;
    let mut transition = MotionTransition::new(property, from, to, 180);
    transition.delay_ms = take_u64(&mut config, "delay_ms")?.unwrap_or(0);
    transition.duration_ms = take_u64(&mut config, "duration_ms")?.unwrap_or(180);
    transition.easing = take_string(&mut config, "easing")?
        .map_or(Ok(MotionEasing::EaseOut), |value| {
            MotionEasing::parse(&value)
        })
        .map_err(script_boxed_error)?;
    transition.iterations = take_iterations(&mut config)?.unwrap_or(Some(1));
    transition.autoreverse = take_bool(&mut config, "autoreverse")?.unwrap_or(false);
    transition.intent = take_intent(&mut config)?.unwrap_or(MotionIntent::Feedback);
    reject_unknown_config(&config)?;
    checked_source(MotionSource::Transition(transition))
}

fn motion_spring_from_script(
    property: &str,
    from: FLOAT,
    to: FLOAT,
    mut config: Map,
) -> Result<MotionSource, Box<EvalAltResult>> {
    let property = script_property(property)?;
    let mut spring = MotionSpring::new(property, from, to);
    spring.initial_velocity = take_float(&mut config, "initial_velocity")?.unwrap_or(0.0);
    spring.stiffness = take_float(&mut config, "stiffness")?.unwrap_or(180.0);
    spring.damping = take_float(&mut config, "damping")?.unwrap_or(24.0);
    spring.mass = take_float(&mut config, "mass")?.unwrap_or(1.0);
    spring.intent = take_intent(&mut config)?.unwrap_or(MotionIntent::Feedback);
    reject_unknown_config(&config)?;
    checked_source(MotionSource::Spring(spring))
}

fn motion_keyframes_from_script(
    property: &str,
    frames: Array,
    mut config: Map,
) -> Result<MotionSource, Box<EvalAltResult>> {
    let frames = frames
        .into_iter()
        .enumerate()
        .map(|(index, value)| parse_keyframe(index, value))
        .collect::<Result<Vec<_>, _>>()?;
    let spec = MotionKeyframes {
        property: script_property(property)?,
        frames,
        delay_ms: take_u64(&mut config, "delay_ms")?.unwrap_or(0),
        duration_ms: take_u64(&mut config, "duration_ms")?.unwrap_or(240),
        iterations: take_iterations(&mut config)?.unwrap_or(Some(1)),
        autoreverse: take_bool(&mut config, "autoreverse")?.unwrap_or(false),
        intent: take_intent(&mut config)?.unwrap_or(MotionIntent::Decorative),
    };
    reject_unknown_config(&config)?;
    checked_source(MotionSource::Keyframes(spec))
}

fn motion_inertia_from_script(
    property: &str,
    from: FLOAT,
    velocity: FLOAT,
    mut config: Map,
) -> Result<MotionSource, Box<EvalAltResult>> {
    let spec = MotionInertia {
        property: script_property(property)?,
        from,
        velocity,
        friction: take_float(&mut config, "friction")?.unwrap_or(8.0),
        min: take_float(&mut config, "min")?,
        max: take_float(&mut config, "max")?,
        bounce: take_float(&mut config, "bounce")?.unwrap_or(0.0),
        snap_points: take_float_array(&mut config, "snap_points")?.unwrap_or_default(),
        intent: take_intent(&mut config)?.unwrap_or(MotionIntent::Feedback),
    };
    reject_unknown_config(&config)?;
    checked_source(MotionSource::Inertia(spec))
}

fn parse_keyframe(index: usize, value: Dynamic) -> Result<MotionKeyframe, Box<EvalAltResult>> {
    let mut frame = value
        .try_cast::<Map>()
        .ok_or_else(|| script_boxed_error(format!("motion keyframe {index} must be a map")))?;
    let offset = take_required_float(&mut frame, "offset")?;
    let value = take_required_float(&mut frame, "value")?;
    let easing = take_string(&mut frame, "easing")?
        .map_or(Ok(MotionEasing::Linear), |value| {
            MotionEasing::parse(&value)
        })
        .map_err(script_boxed_error)?;
    reject_unknown_config(&frame)?;
    Ok(MotionKeyframe {
        offset,
        value,
        easing,
    })
}

fn script_property(value: &str) -> Result<MotionProperty, Box<EvalAltResult>> {
    MotionProperty::parse(value).map_err(script_boxed_error)
}

fn checked_source(source: MotionSource) -> Result<MotionSource, Box<EvalAltResult>> {
    validate_source(&source).map_err(script_boxed_error)?;
    Ok(source)
}

fn take_dynamic(config: &mut Map, key: &str) -> Option<Dynamic> {
    config.remove(key)
}

fn take_string(config: &mut Map, key: &str) -> Result<Option<ImmutableString>, Box<EvalAltResult>> {
    take_dynamic(config, key)
        .map(|value| {
            value.try_cast::<ImmutableString>().ok_or_else(|| {
                script_boxed_error(format!("motion option `{key}` must be a string"))
            })
        })
        .transpose()
}

fn take_bool(config: &mut Map, key: &str) -> Result<Option<bool>, Box<EvalAltResult>> {
    take_dynamic(config, key)
        .map(|value| {
            value.try_cast::<bool>().ok_or_else(|| {
                script_boxed_error(format!("motion option `{key}` must be a boolean"))
            })
        })
        .transpose()
}

fn dynamic_float(value: Dynamic, key: &str) -> Result<f64, Box<EvalAltResult>> {
    if value.is::<FLOAT>() {
        return Ok(value.cast::<FLOAT>());
    }
    value
        .try_cast::<INT>()
        .and_then(|value| value.to_string().parse::<f64>().ok())
        .ok_or_else(|| script_boxed_error(format!("motion option `{key}` must be numeric")))
}

fn take_float(config: &mut Map, key: &str) -> Result<Option<f64>, Box<EvalAltResult>> {
    take_dynamic(config, key)
        .map(|value| dynamic_float(value, key))
        .transpose()
}

fn take_required_float(config: &mut Map, key: &str) -> Result<f64, Box<EvalAltResult>> {
    take_float(config, key)?
        .ok_or_else(|| script_boxed_error(format!("motion option `{key}` is required")))
}

fn take_u64(config: &mut Map, key: &str) -> Result<Option<u64>, Box<EvalAltResult>> {
    take_dynamic(config, key)
        .map(|value| {
            let value = value.try_cast::<INT>().ok_or_else(|| {
                script_boxed_error(format!("motion option `{key}` must be an integer"))
            })?;
            u64::try_from(value).map_err(|_| {
                script_boxed_error(format!("motion option `{key}` must be non-negative"))
            })
        })
        .transpose()
}

#[allow(clippy::option_option)]
fn take_iterations(config: &mut Map) -> Result<Option<Option<u32>>, Box<EvalAltResult>> {
    let Some(value) = take_dynamic(config, "iterations") else {
        return Ok(None);
    };
    if let Some(value) = value.clone().try_cast::<ImmutableString>() {
        return (value.as_str() == "infinite")
            .then_some(Some(None))
            .ok_or_else(|| script_boxed_error("motion `iterations` string must be `infinite`"));
    }
    let value = value.try_cast::<INT>().ok_or_else(|| {
        script_boxed_error("motion option `iterations` must be a positive integer or `infinite`")
    })?;
    let value = u32::try_from(value)
        .map_err(|_| script_boxed_error("motion option `iterations` must be a positive integer"))?;
    if value == 0 {
        return Err(script_boxed_error(
            "motion option `iterations` must be greater than zero",
        ));
    }
    Ok(Some(Some(value)))
}

fn take_intent(config: &mut Map) -> Result<Option<MotionIntent>, Box<EvalAltResult>> {
    take_string(config, "intent")?
        .map(|value| MotionIntent::parse(&value).map_err(script_boxed_error))
        .transpose()
}

fn take_float_array(config: &mut Map, key: &str) -> Result<Option<Vec<f64>>, Box<EvalAltResult>> {
    take_dynamic(config, key)
        .map(|value| {
            let values = value.try_cast::<Array>().ok_or_else(|| {
                script_boxed_error(format!("motion option `{key}` must be an array"))
            })?;
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| dynamic_float(value, &format!("{key}[{index}]")))
                .collect()
        })
        .transpose()
}

fn reject_unknown_config(config: &Map) -> Result<(), Box<EvalAltResult>> {
    if let Some(key) = config.keys().next() {
        Err(script_boxed_error(format!("unknown motion option `{key}`")))
    } else {
        Ok(())
    }
}

#[allow(clippy::needless_pass_by_value)]
fn script_boxed_error(error: impl ToString) -> Box<EvalAltResult> {
    Box::new(motion_script_error(&error))
}

fn motion_script_error(error: &impl ToString) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE)
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum MotionError {
    #[error("motion property `{0}` is unknown")]
    UnknownProperty(String),
    #[error("motion easing `{0}` is unknown")]
    UnknownEasing(String),
    #[error("motion intent `{0}` is unknown")]
    UnknownIntent(String),
    #[error("motion parameters must be finite and physically valid")]
    InvalidSpec,
    #[error("motion node `{0}` requires a stable key")]
    MissingKey(String),
    #[error("invalid motion timeline: {0}")]
    InvalidTimeline(String),
    #[error("motion timeline `{0}` is not active in this scope")]
    UnknownTimeline(String),
    #[error("motion timeline handle `{0}` is stale")]
    StaleTimelineHandle(String),
    #[error("motion timeline handle `{0}` belongs to another runtime/view/component")]
    ForeignTimelineHandle(String),
    #[error("active motion budget exceeded: {actual} > {limit}")]
    ActiveBudget { actual: usize, limit: usize },
    #[error("shared layout identity `{group}/{id}` is duplicated in one presentation domain")]
    DuplicateSharedLayout { group: String, id: String },
    #[error("invalid shared layout declaration: {0}")]
    InvalidSharedLayout(String),
    #[error("invalid motion progress source: {0}")]
    InvalidProgressSource(String),
    #[error("node `{path}` declares more than one source for motion property `{property:?}`")]
    DuplicatePropertySource {
        path: String,
        property: MotionProperty,
    },
    #[error(
        "node `{path}` motion property `{property:?}` has conflicting owners `{first}` and `{second}`"
    )]
    PropertyOwnerConflict {
        path: String,
        property: MotionProperty,
        first: String,
        second: String,
    },
    #[error("timeline `{timeline}` targets missing node `{target}`")]
    UnknownTimelineTarget { timeline: String, target: String },
    #[error("virtual collection `{path}` item index {index} has no stable data key")]
    MissingVirtualItemKey { path: String, index: usize },
    #[error("motion scene is missing retained identity `{0}`")]
    MissingRetainedIdentity(String),
    #[error("motion scene path `{0}` is not unique")]
    DuplicateScenePath(String),
    #[error("exit motion does not support paint snapshots for node kind `{0}`")]
    UnsupportedExitGhost(String),
    #[error("motion property `{property:?}` is unsupported on {node} node `{path}`")]
    UnsupportedProperty {
        path: String,
        property: MotionProperty,
        node: &'static str,
    },
}

#[derive(Clone, Debug)]
pub struct MotionFrame {
    pub values: BTreeMap<MotionKey, f64>,
    pub completed: BTreeSet<MotionKey>,
    pub needs_frame: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MotionResourceUsage {
    pub declarations: usize,
    pub keyframes: usize,
    pub timelines: usize,
    pub timeline_steps: usize,
    pub active: usize,
    pub particles: usize,
    pub shared_snapshots: usize,
    pub geometry_slots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_path() -> ComponentInstancePath {
        ComponentInstancePath::root("Accordion", "settings")
    }

    fn transition(property: MotionProperty, from: f64, to: f64, duration_ms: u64) -> MotionSource {
        let mut spec = MotionTransition::new(property, from, to, duration_ms);
        spec.easing = MotionEasing::Linear;
        MotionSource::Transition(spec)
    }

    fn repeating_transition(
        property: MotionProperty,
        from: f64,
        to: f64,
        duration_ms: u64,
    ) -> MotionSource {
        let MotionSource::Transition(mut spec) = transition(property, from, to, duration_ms) else {
            unreachable!()
        };
        spec.iterations = None;
        MotionSource::Transition(spec)
    }

    #[test]
    fn transition_retargets_from_current_sample() {
        let start = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                transition(MotionProperty::Opacity, 0.0, 1.0, 100),
                start,
            )
            .unwrap();
        let halfway = start + Duration::from_millis(50);
        assert!((runtime.sample(&key, halfway).unwrap() - 0.5).abs() < 0.01);
        runtime
            .start(
                key_path(),
                transition(MotionProperty::Opacity, 1.0, 0.0, 100),
                halfway,
            )
            .unwrap();
        assert!((runtime.sample(&key, halfway).unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn reduced_motion_settles_without_frames() {
        let mut runtime = MotionRuntime::new(MotionPreference::Reduced);
        let key = runtime
            .start(
                key_path(),
                transition(MotionProperty::Height, 0.0, 200.0, 300),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(runtime.sample(&key, Instant::now()), Some(200.0));
        assert!(!runtime.tick(Instant::now()).needs_frame);
    }

    #[test]
    fn reduced_motion_keeps_looping_indicators_visible_at_midpoint() {
        let now = Instant::now();
        let spec = repeating_transition(MotionProperty::TranslateX, -72.0, 200.0, 900);
        let mut reduced = MotionRuntime::new(MotionPreference::Reduced);
        let reduced_key = reduced.start(key_path(), spec.clone(), now).unwrap();
        assert_eq!(reduced.sample(&reduced_key, now), Some(64.0));
        assert!(!reduced.tick(now).needs_frame);

        let mut switched = MotionRuntime::new(MotionPreference::Normal);
        let switched_key = switched.start(key_path(), spec, now).unwrap();
        switched.set_preference(MotionPreference::Reduced);
        assert_eq!(switched.sample(&switched_key, now), Some(64.0));
        assert!(!switched.tick(now).needs_frame);
    }

    #[test]
    fn spring_converges_without_rhai_frame_callbacks() {
        let start = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                MotionSource::Spring(MotionSpring::new(MotionProperty::TranslateY, 0.0, 10.0)),
                start,
            )
            .unwrap();
        let mut now = start;
        for _ in 0..600 {
            now += Duration::from_millis(16);
            if !runtime.tick(now).needs_frame {
                break;
            }
        }
        assert!((runtime.sample(&key, now).unwrap() - 10.0).abs() < 0.001);
    }

    #[test]
    fn node_declarations_reconcile_without_restarting_same_target() {
        let start = Instant::now();
        let node = UiNode::text("animated")
            .with_key("status")
            .with_motion(transition(MotionProperty::Opacity, 0.0, 1.0, 100));
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        reconcile_node_motion(&node, &mut runtime, start).unwrap();
        let halfway = start + Duration::from_millis(50);
        let values = reconcile_node_motion(&node, &mut runtime, halfway).unwrap();
        let key = MotionKey::for_node("root", MotionProperty::Opacity);
        assert!((values[&key] - 0.5).abs() < 0.01);
    }

    #[test]
    fn animated_nodes_require_keys_and_removed_declarations_cancel() {
        let start = Instant::now();
        let animation = transition(MotionProperty::Height, 0.0, 40.0, 100);
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        assert!(matches!(
            reconcile_node_motion(
                &UiNode::text("missing key").with_motion(animation.clone()),
                &mut runtime,
                start
            ),
            Err(MotionError::MissingKey(_))
        ));
        reconcile_node_motion(
            &UiNode::text("keyed").with_key("row").with_motion(animation),
            &mut runtime,
            start,
        )
        .unwrap();
        assert!(!runtime.snapshot(start).is_empty());
        reconcile_node_motion(&UiNode::text("plain"), &mut runtime, start).unwrap();
        assert!(runtime.snapshot(start).is_empty());
    }

    #[test]
    fn scoped_reconciliation_does_not_remove_other_window_motion() {
        let now = Instant::now();
        let node = UiNode::text("loading")
            .with_key("progress")
            .with_motion(repeating_transition(MotionProperty::Opacity, 0.0, 1.0, 100));
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        reconcile_node_motion_scoped(&node, &mut runtime, now, "window:main/root").unwrap();
        reconcile_node_motion_scoped(&node, &mut runtime, now, "window:settings/root").unwrap();
        assert_eq!(runtime.snapshot(now).len(), 2);
        runtime.cancel_node_scope("window:settings/root");
        assert_eq!(runtime.snapshot(now).len(), 1);
    }

    #[test]
    fn looping_transition_repeats_without_completing() {
        let start = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                repeating_transition(MotionProperty::Opacity, 0.2, 0.8, 100),
                start,
            )
            .unwrap();
        let first = runtime.tick(start + Duration::from_millis(50));
        assert!(first.needs_frame);
        assert!((first.values[&key] - 0.5).abs() < 0.01);
        let repeated = runtime.tick(start + Duration::from_millis(150));
        assert!(repeated.needs_frame);
        assert!((repeated.values[&key] - 0.5).abs() < 0.01);
    }

    #[test]
    fn timeline_sequence_seek_pause_and_complete_are_deterministic() {
        let start = Instant::now();
        let root = MotionTimelineStep::Sequence(vec![
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
            }),
            MotionTimelineStep::Delay(50),
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::TranslateX, 0.0, 40.0, 100),
            }),
        ]);
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let handle = runtime
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/card"),
                MotionTimeline::new("intro", root),
                start,
            )
            .unwrap();
        let first = runtime.tick(start + Duration::from_millis(50));
        assert!(
            (first.values[&MotionKey::for_node("root/card", MotionProperty::Opacity)] - 0.5).abs()
                < 0.01
        );
        runtime
            .pause_timeline(&handle, start + Duration::from_millis(60))
            .unwrap();
        assert!(
            (runtime.snapshot(start + Duration::from_millis(500))
                [&MotionKey::for_node("root/card", MotionProperty::Opacity)]
                - 0.6)
                .abs()
                < 0.01
        );
        assert!(!runtime.tick(start + Duration::from_millis(500)).needs_frame);
        runtime
            .seek_timeline(&handle, 200, start + Duration::from_millis(500))
            .unwrap();
        assert!(
            (runtime.snapshot(start + Duration::from_millis(500))
                [&MotionKey::for_node("root/card", MotionProperty::TranslateX)]
                - 20.0)
                .abs()
                < 0.01
        );
        runtime
            .play_timeline(&handle, start + Duration::from_millis(500))
            .unwrap();
        let final_frame = runtime.tick(start + Duration::from_millis(550));
        assert!(
            (final_frame.values[&MotionKey::for_node("root/card", MotionProperty::TranslateX)]
                - 40.0)
                .abs()
                < 0.01
        );
        assert_eq!(
            runtime.timeline_state(&handle),
            Some(MotionPlaybackState::Completed)
        );
    }

    #[test]
    fn timeline_rejects_overlapping_property_ownership() {
        let root = MotionTimelineStep::Parallel(vec![
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
            }),
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::Opacity, 1.0, 0.0, 100),
            }),
        ]);
        assert!(matches!(
            compile_timeline(&root),
            Err(MotionError::InvalidTimeline(_))
        ));
    }

    #[test]
    fn script_motion_configs_are_strict_and_keyframes_sample_natively() {
        let mut engine = Engine::new();
        register_motion_api(&mut engine);
        let source = engine
            .eval::<MotionSource>(
                r#"motion_keyframes("opacity", [
                    #{ offset: 0.0, value: 0.0 },
                    #{ offset: 0.5, value: 1.0, easing: "ease_out" },
                    #{ offset: 1.0, value: 0.25 }
                ], #{ duration_ms: 200, intent: "decorative" })"#,
            )
            .unwrap();
        assert!(
            (sample_source(&source, Duration::from_millis(100)).value - 1.0).abs() < f64::EPSILON
        );
        assert!(
            engine
                .eval::<MotionSource>(r#"motion_transition("opacity", 0.0, 1.0, #{ typo: 100 })"#,)
                .unwrap_err()
                .to_string()
                .contains("unknown motion option `typo`")
        );
    }

    #[test]
    fn replay_key_restarts_a_settled_declaration_without_restarting_equal_renders() {
        let start = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let source = transition(MotionProperty::Opacity, 0.0, 1.0, 100);
        let component = key_path();
        let key = runtime
            .start_with_replay(
                component.clone(),
                source.clone(),
                Some("first".to_owned()),
                start,
            )
            .unwrap();
        let _ = runtime.tick(start + Duration::from_millis(100));
        runtime
            .start_with_replay(
                component.clone(),
                source.clone(),
                Some("first".to_owned()),
                start + Duration::from_millis(120),
            )
            .unwrap();
        assert_eq!(
            runtime.sample(&key, start + Duration::from_millis(120)),
            Some(1.0)
        );
        runtime
            .start_with_replay(
                component,
                source,
                Some("second".to_owned()),
                start + Duration::from_millis(120),
            )
            .unwrap();
        let frame = runtime.tick(start + Duration::from_millis(170));
        assert!(frame.needs_frame);
        assert!((frame.values[&key] - 0.5).abs() < 0.01);
    }

    #[test]
    fn script_preference_cannot_relax_a_stricter_host_policy() {
        let mut runtime = MotionRuntime::new(MotionPreference::Reduced);
        runtime.request_preference(MotionPreference::Normal);
        assert_eq!(runtime.preference(), MotionPreference::Reduced);
        runtime.set_preference(MotionPreference::None);
        runtime.request_preference(MotionPreference::Normal);
        assert_eq!(runtime.preference(), MotionPreference::None);
    }

    #[test]
    fn timeline_handles_are_instance_bound_and_play_is_idempotent() {
        let now = Instant::now();
        let scope = ComponentInstancePath::root("UiNode", "root/card");
        let timeline = || {
            MotionTimeline::new(
                "intro",
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 1_000),
                }),
            )
        };
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let old = runtime
            .start_timeline(scope.clone(), timeline(), now)
            .unwrap();
        let _ = runtime.tick(now + Duration::from_millis(400));
        runtime
            .play_timeline(&old, now + Duration::from_millis(400))
            .unwrap();
        let key = MotionKey::for_node("root/card", MotionProperty::Opacity);
        assert!((runtime.snapshot(now + Duration::from_millis(400))[&key] - 0.4).abs() < 0.01);
        runtime.cancel_node_scope("root/card");
        let new = runtime.start_timeline(scope, timeline(), now).unwrap();
        assert_ne!(old, new);
        assert!(matches!(
            runtime.cancel_timeline(&old),
            Err(MotionError::StaleTimelineHandle(_))
        ));
        assert_eq!(
            runtime.timeline_state(&new),
            Some(MotionPlaybackState::Playing)
        );
        assert!(matches!(
            MotionRuntime::new(MotionPreference::Normal).cancel_timeline(&new),
            Err(MotionError::StaleTimelineHandle(_))
        ));
    }

    #[test]
    fn compatible_generation_migration_preserves_progress_but_rekeys_authority() {
        let now = Instant::now();
        let owner = ComponentInstancePath::root("View", "root");
        let node = UiNode::text("animated")
            .with_key("card")
            .with_timeline(MotionTimeline::new(
                "intro",
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 1_000),
                }),
            ));
        let incarnations = BTreeMap::new();
        let reconcile = |runtime: &mut MotionRuntime, generation| {
            reconcile_node_motion_scoped_owned(
                &node,
                runtime,
                now,
                "root",
                MotionReconcileContext {
                    domain: "root",
                    root_component: &owner,
                    root_incarnation: ComponentIncarnation::unscoped(),
                    generation,
                    incarnations: &incarnations,
                    identities: None,
                },
            )
            .unwrap();
        };
        let first_generation = ScriptGeneration::initial();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        reconcile(&mut runtime, first_generation);
        let first = runtime
            .timeline_handle_for_owner(
                "root",
                &owner,
                ComponentIncarnation::unscoped(),
                first_generation,
                "intro",
            )
            .unwrap();
        let _ = runtime.tick(now + Duration::from_millis(400));
        reconcile(&mut runtime, first_generation);
        assert_eq!(
            runtime
                .timeline_handle_for_owner(
                    "root",
                    &owner,
                    ComponentIncarnation::unscoped(),
                    first_generation,
                    "intro",
                )
                .unwrap(),
            first
        );

        let next_generation = first_generation.next();
        reconcile(&mut runtime, next_generation);
        let migrated = runtime
            .timeline_handle_for_owner(
                "root",
                &owner,
                ComponentIncarnation::unscoped(),
                next_generation,
                "intro",
            )
            .unwrap();
        assert_ne!(first, migrated);
        assert!(matches!(
            runtime.cancel_timeline(&first),
            Err(MotionError::StaleTimelineHandle(_))
        ));
        assert!(
            (runtime.snapshot(now + Duration::from_millis(400))
                [&MotionKey::for_node("root", MotionProperty::Opacity)]
                - 0.4)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn none_policy_cannot_be_bypassed_by_restart() {
        let now = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::None);
        let handle = runtime
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/card"),
                MotionTimeline::new(
                    "intro",
                    MotionTimelineStep::Track(MotionTrack {
                        target: ".".to_owned(),
                        source: transition(MotionProperty::Opacity, 0.0, 1.0, 1_000),
                    }),
                ),
                now,
            )
            .unwrap();
        runtime.restart_timeline(&handle, now).unwrap();
        let frame = runtime.tick(now + Duration::from_millis(250));
        assert!(!frame.needs_frame);
        assert_eq!(
            runtime.timeline_state(&handle),
            Some(MotionPlaybackState::Completed)
        );
        assert!(
            (runtime.snapshot(now)[&MotionKey::for_node("root/card", MotionProperty::Opacity)]
                - 1.0)
                .abs()
                < f64::EPSILON
        );

        let mut deferred = MotionTimeline::new(
            "deferred",
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::TranslateX, 0.0, 10.0, 1_000),
            }),
        );
        deferred.autoplay = false;
        let mut switched = MotionRuntime::new(MotionPreference::Normal);
        let handle = switched
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/deferred"),
                deferred,
                now,
            )
            .unwrap();
        switched.set_preference(MotionPreference::None);
        assert_eq!(
            switched.timeline_state(&handle),
            Some(MotionPlaybackState::Completed)
        );
        assert!(
            (switched.snapshot(now)
                [&MotionKey::for_node("root/deferred", MotionProperty::TranslateX,)]
                - 10.0)
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn none_policy_reprojects_settled_and_outer_autoreverse_terminals() {
        let now = Instant::now();
        let mut reversed = MotionTransition::new(MotionProperty::Opacity, 0.0, 1.0, 100);
        reversed.iterations = Some(2);
        reversed.autoreverse = true;
        let mut reprojected = MotionRuntime::new(MotionPreference::Reduced);
        let key = reprojected
            .start(
                ComponentInstancePath::root("UiNode", "root/reprojected"),
                MotionSource::Transition(reversed),
                now,
            )
            .unwrap();
        assert!((reprojected.snapshot(now)[&key] - 1.0).abs() < f64::EPSILON);
        reprojected.set_preference(MotionPreference::None);
        assert!(reprojected.snapshot(now)[&key].abs() < f64::EPSILON);

        let mut outer = MotionTimeline::new(
            "outer-reverse",
            MotionTimelineStep::Track(MotionTrack {
                target: ".".to_owned(),
                source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
            }),
        );
        outer.iterations = Some(2);
        outer.autoreverse = true;
        let mut reduced_timeline = MotionRuntime::new(MotionPreference::Reduced);
        reduced_timeline
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/outer"),
                outer.clone(),
                now,
            )
            .unwrap();
        let initial_events = reduced_timeline.drain_timeline_events().len();
        reduced_timeline.set_preference(MotionPreference::None);
        assert!(
            reduced_timeline.snapshot(now)
                [&MotionKey::for_node("root/outer", MotionProperty::Opacity)]
                .abs()
                < f64::EPSILON
        );
        assert_eq!(reduced_timeline.drain_timeline_events().len(), 0);
        assert_eq!(initial_events, 1);
        let mut static_timeline = MotionRuntime::new(MotionPreference::None);
        static_timeline
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/outer"),
                outer,
                now,
            )
            .unwrap();
        assert!(
            static_timeline.snapshot(now)
                [&MotionKey::for_node("root/outer", MotionProperty::Opacity)]
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn direct_and_timeline_physics_share_one_sampler_and_extremes_fail() {
        let now = Instant::now();
        let scope = ComponentInstancePath::root("UiNode", "root/card");
        let spring = MotionSpring::new(MotionProperty::TranslateX, 0.0, 1.0);
        let mut direct = MotionRuntime::new(MotionPreference::Normal);
        let key = direct
            .start(scope.clone(), MotionSource::Spring(spring.clone()), now)
            .unwrap();
        let direct_value = direct.tick(now + Duration::from_secs(1)).values[&key];
        let mut timeline = MotionRuntime::new(MotionPreference::Normal);
        timeline
            .start_timeline(
                scope.clone(),
                MotionTimeline::new(
                    "spring",
                    MotionTimelineStep::Track(MotionTrack {
                        target: ".".to_owned(),
                        source: MotionSource::Spring(spring.clone()),
                    }),
                ),
                now,
            )
            .unwrap();
        let timeline_value = timeline.tick(now + Duration::from_secs(1)).values[&key];
        assert!((direct_value - timeline_value).abs() < 1.0e-9);

        let mut retargeted = MotionRuntime::new(MotionPreference::Normal);
        retargeted
            .start(scope.clone(), MotionSource::Spring(spring.clone()), now)
            .unwrap();
        let retarget_at = now + Duration::from_millis(16);
        let before = retargeted.inspect(retarget_at)[0].velocity.unwrap();
        let mut next = spring.clone();
        next.to = 2.0;
        retargeted
            .start(scope.clone(), MotionSource::Spring(next), retarget_at)
            .unwrap();
        let inherited = retargeted.inspect(retarget_at)[0].velocity.unwrap();
        assert!((before - inherited).abs() < 1.0e-9);

        let mut extreme = spring;
        extreme.stiffness = 1.0e308;
        extreme.mass = 1.0e-308;
        assert_eq!(
            direct.start(scope, MotionSource::Spring(extreme), now),
            Err(MotionError::InvalidSpec)
        );
    }

    #[test]
    fn unconstrained_inertia_uses_the_closed_form_at_any_animation_age() {
        let spec = MotionInertia {
            property: MotionProperty::TranslateX,
            from: 10.0,
            velocity: 1_000.0,
            friction: 0.5,
            min: None,
            max: None,
            bounce: 0.0,
            snap_points: Vec::new(),
            intent: MotionIntent::Decorative,
        };
        for seconds in [1.0, 5.0, 8.0] {
            let elapsed = Duration::from_secs_f64(seconds);
            let (position, velocity) = sample_inertia_at(&spec, elapsed);
            let decay = (-spec.friction * seconds).exp();
            let expected_position = spec.from + spec.velocity * (1.0 - decay) / spec.friction;
            let expected_velocity = spec.velocity * decay;
            assert!((position - expected_position).abs() < 1.0e-9);
            assert!((velocity - expected_velocity).abs() < 1.0e-9);
        }
        let before_cap = sample_inertia_at(&spec, Duration::from_millis(9_999)).0;
        let at_cap = sample_inertia_at(&spec, Duration::from_secs(10)).0;
        let after_cap = sample_inertia_at(&spec, Duration::from_secs(11)).0;
        assert!((at_cap - before_cap).abs() < 0.1);
        assert!((after_cap - at_cap).abs() < f64::EPSILON);
    }

    #[test]
    fn bouncing_inertia_is_bounded_and_matches_timeline_sampling() {
        let now = Instant::now();
        let scope = ComponentInstancePath::root("UiNode", "root/inertia");
        let spec = MotionInertia {
            property: MotionProperty::TranslateX,
            from: 50.0,
            velocity: 500.0,
            friction: 1.0,
            min: Some(0.0),
            max: Some(100.0),
            bounce: 0.5,
            snap_points: vec![0.0, 100.0],
            intent: MotionIntent::Feedback,
        };
        let mut direct = MotionRuntime::new(MotionPreference::Normal);
        let key = direct
            .start(scope.clone(), MotionSource::Inertia(spec.clone()), now)
            .unwrap();
        let mut timeline = MotionRuntime::new(MotionPreference::Normal);
        timeline
            .start_timeline(
                scope,
                MotionTimeline::new(
                    "inertia",
                    MotionTimelineStep::Track(MotionTrack {
                        target: ".".to_owned(),
                        source: MotionSource::Inertia(spec),
                    }),
                ),
                now,
            )
            .unwrap();
        for elapsed in [100, 500, 1_000, 5_000] {
            let at = now + Duration::from_millis(elapsed);
            let direct_value = direct.snapshot(at)[&key];
            let timeline_value = timeline.snapshot(at)[&key];
            assert!((0.0..=100.0).contains(&direct_value));
            assert!((direct_value - timeline_value).abs() < 1.0e-9);
        }
    }

    #[test]
    fn timeline_snapshot_resets_future_sequence_tracks_when_seeking_back() {
        let now = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let handle = runtime
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/card"),
                MotionTimeline::new(
                    "sequence",
                    MotionTimelineStep::Sequence(vec![
                        MotionTimelineStep::Track(MotionTrack {
                            target: ".".to_owned(),
                            source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
                        }),
                        MotionTimelineStep::Track(MotionTrack {
                            target: ".".to_owned(),
                            source: transition(MotionProperty::TranslateX, 0.0, 100.0, 100),
                        }),
                    ]),
                ),
                now,
            )
            .unwrap();
        let _ = runtime.tick(now + Duration::from_millis(150));
        runtime
            .seek_timeline(&handle, 0, now + Duration::from_millis(150))
            .unwrap();
        assert!(
            runtime.snapshot(now)[&MotionKey::for_node("root/card", MotionProperty::TranslateX)]
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn terminal_events_are_partitioned_and_discarded_by_presentation_scope() {
        let now = Instant::now();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        for domain in ["window:a/view:a/root", "window:b/view:b/root"] {
            runtime
                .start_timeline(
                    ComponentInstancePath::root("UiNode", domain),
                    MotionTimeline::new(
                        "done",
                        MotionTimelineStep::Track(MotionTrack {
                            target: ".".to_owned(),
                            source: transition(MotionProperty::Opacity, 0.0, 1.0, 1),
                        }),
                    ),
                    now,
                )
                .unwrap();
        }
        let _ = runtime.tick(now + Duration::from_millis(1));
        assert_eq!(
            runtime
                .drain_timeline_events_for_domain("window:a/view:a/root")
                .len(),
            1
        );
        runtime.discard_timeline_events_in_scope("window:b");
        assert!(runtime.drain_timeline_events().is_empty());
    }

    #[test]
    fn autoreverse_completion_keeps_the_sampled_terminal_value() {
        let now = Instant::now();
        let mut spec = MotionTransition::new(MotionProperty::Opacity, 0.0, 1.0, 100);
        spec.easing = MotionEasing::Linear;
        spec.iterations = Some(2);
        spec.autoreverse = true;
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(key_path(), MotionSource::Transition(spec), now)
            .unwrap();
        let frame = runtime.tick(now + Duration::from_millis(200));
        assert!(frame.values[&key].abs() < f64::EPSILON);
        assert!(runtime.snapshot(now)[&key].abs() < f64::EPSILON);
    }

    #[test]
    fn reconciliation_validates_targets_ownership_and_is_atomic() {
        let now = Instant::now();
        let conflict = UiNode::text("conflict")
            .with_key("conflict")
            .with_motion(transition(MotionProperty::Opacity, 0.0, 1.0, 100))
            .with_timeline(MotionTimeline::new(
                "conflict",
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::Opacity, 1.0, 0.0, 100),
                }),
            ));
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        assert!(matches!(
            reconcile_node_motion(&conflict, &mut runtime, now),
            Err(MotionError::PropertyOwnerConflict { .. })
        ));
        assert_eq!(runtime.resource_usage().active, 0);

        let missing = UiNode::text("missing")
            .with_key("root")
            .with_timeline(MotionTimeline::new(
                "missing",
                MotionTimelineStep::Track(MotionTrack {
                    target: "child".to_owned(),
                    source: transition(MotionProperty::Rotate, 0.0, 90.0, 100),
                }),
            ));
        assert!(matches!(
            reconcile_node_motion(&missing, &mut runtime, now),
            Err(MotionError::UnknownTimelineTarget { .. })
        ));
        assert_eq!(runtime.resource_usage().active, 0);

        runtime.set_active_limit(1);
        reconcile_node_motion(
            &UiNode::text("old").with_key("node").with_motion(transition(
                MotionProperty::Opacity,
                0.0,
                1.0,
                100,
            )),
            &mut runtime,
            now,
        )
        .unwrap();
        reconcile_node_motion(
            &UiNode::text("new").with_key("node").with_motion(transition(
                MotionProperty::TranslateX,
                0.0,
                1.0,
                100,
            )),
            &mut runtime,
            now,
        )
        .unwrap();
        assert_eq!(runtime.resource_usage().active, 1);
    }

    #[test]
    fn encoded_paths_do_not_alias_legal_slash_keys_and_root_remounts_restart() {
        let now = Instant::now();
        let nested = UiNode::column(vec![
            UiNode::text("flat").with_key("a/b").with_motion(transition(
                MotionProperty::Opacity,
                0.0,
                1.0,
                100,
            )),
            UiNode::column(vec![
                UiNode::text("nested").with_key("b").with_motion(transition(
                    MotionProperty::Opacity,
                    0.0,
                    1.0,
                    100,
                )),
            ])
            .with_key("a"),
            UiNode::text("numeric")
                .with_key("0")
                .with_motion(transition(MotionProperty::Opacity, 0.0, 1.0, 100)),
            UiNode::text("reserved-looking")
                .with_key("item:alpha")
                .with_motion(transition(MotionProperty::Opacity, 0.0, 1.0, 100)),
        ]);
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let values = reconcile_node_motion(&nested, &mut runtime, now).unwrap();
        assert_eq!(values.len(), 4);

        let direct = UiNode::column(vec![
            UiNode::text("Title")
                .with_key("title")
                .with_motion(transition(MotionProperty::Opacity, 0.0, 1.0, 100)),
        ])
        .with_key("panel");
        let timeline_target = UiNode::column(vec![UiNode::text("Title").with_key("title")])
            .with_key("panel")
            .with_timeline(MotionTimeline::new(
                "child",
                MotionTimelineStep::Track(MotionTrack {
                    target: "title".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
                }),
            ));
        let direct_keys = reconcile_node_motion(
            &direct,
            &mut MotionRuntime::new(MotionPreference::Normal),
            now,
        )
        .unwrap()
        .into_keys()
        .collect::<Vec<_>>();
        let timeline_keys = reconcile_node_motion(
            &timeline_target,
            &mut MotionRuntime::new(MotionPreference::Normal),
            now,
        )
        .unwrap()
        .into_keys()
        .collect::<Vec<_>>();
        assert_eq!(direct_keys, timeline_keys);

        let timeline = || {
            MotionTimeline::new(
                "intro",
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 1_000),
                }),
            )
        };
        let first = UiNode::text("first")
            .with_key("one")
            .with_timeline(timeline());
        reconcile_node_motion(&first, &mut runtime, now).unwrap();
        let old = runtime.inspect_timelines(now)[0].handle.clone();
        let second = UiNode::text("second")
            .with_key("two")
            .with_timeline(timeline());
        reconcile_node_motion(&second, &mut runtime, now + Duration::from_millis(500)).unwrap();
        let current = runtime.inspect_timelines(now)[0].handle.clone();
        assert_ne!(old, current);
        assert!(matches!(
            runtime.cancel_timeline(&old),
            Err(MotionError::StaleTimelineHandle(_))
        ));
        assert!(
            runtime
                .snapshot(now + Duration::from_millis(500))
                .values()
                .next()
                .copied()
                .unwrap()
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn retained_timeline_targets_resolve_to_the_target_node_identity() {
        let now = Instant::now();
        let timeline = || {
            MotionTimeline::new(
                "intro",
                MotionTimelineStep::Track(MotionTrack {
                    target: "title".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
                }),
            )
        };
        let root = UiNode::column(vec![UiNode::text("Title").with_key("title")])
            .with_key("panel")
            .with_timeline(timeline());
        let mut retained = crate::RetainedUiTree::new();
        retained.reconcile(root.clone()).unwrap();
        let title = retained
            .nodes()
            .find(|node| node.key() == Some("title"))
            .unwrap()
            .id();
        let identities = retained_motion_identities(&root, &retained, "root").unwrap();
        let component = ComponentInstancePath::root("View", "retained-target");
        let incarnations = BTreeMap::new();
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        let values = reconcile_node_motion_scoped_owned(
            &root,
            &mut runtime,
            now,
            "root",
            MotionReconcileContext {
                domain: "root",
                root_component: &component,
                root_incarnation: ComponentIncarnation::unscoped(),
                generation: ScriptGeneration::initial(),
                incarnations: &incarnations,
                identities: Some(&identities),
            },
        )
        .unwrap();
        assert!(values.contains_key(&MotionKey::for_node(
            &retained_node_path("root", title),
            MotionProperty::Opacity,
        )));

        let replacement = UiNode::column(vec![UiNode::box_node(Vec::new()).with_key("title")])
            .with_key("panel")
            .with_timeline(timeline());
        retained.reconcile(replacement.clone()).unwrap();
        let replacement_title = retained
            .nodes()
            .find(|node| node.key() == Some("title"))
            .unwrap()
            .id();
        assert_ne!(title, replacement_title);
        let replacement_identities =
            retained_motion_identities(&replacement, &retained, "root").unwrap();
        let values = reconcile_node_motion_scoped_owned(
            &replacement,
            &mut runtime,
            now + Duration::from_millis(50),
            "root",
            MotionReconcileContext {
                domain: "root",
                root_component: &component,
                root_incarnation: ComponentIncarnation::unscoped(),
                generation: ScriptGeneration::initial(),
                incarnations: &incarnations,
                identities: Some(&replacement_identities),
            },
        )
        .unwrap();
        assert!(!values.contains_key(&MotionKey::for_node(
            &retained_node_path("root", title),
            MotionProperty::Opacity,
        )));
        assert!(
            (values[&MotionKey::for_node(
                &retained_node_path("root", replacement_title),
                MotionProperty::Opacity,
            )] - 0.5)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn exit_ghost_rejects_an_unsupported_nested_subtree_atomically() {
        let now = Instant::now();
        let root = UiNode::box_node(vec![UiNode::error_boundary(
            UiNode::text("content"),
            UiNode::text("fallback"),
        )])
        .with_key("panel")
        .with_exit_motion(transition(MotionProperty::Opacity, 1.0, 0.0, 100));
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        assert!(matches!(
            reconcile_node_motion(&root, &mut runtime, now),
            Err(MotionError::UnsupportedExitGhost(_))
        ));
        assert_eq!(runtime.resource_usage().active, 0);
    }

    #[test]
    fn affine_canvas_motion_rejects_axis_aligned_path_clips() {
        let scene = crate::CanvasScene::new(vec![crate::CanvasCommand::Path {
            key: "clipped".to_owned(),
            segments: vec![
                crate::CanvasPathSegment::Move { x: 0.0, y: 0.0 },
                crate::CanvasPathSegment::Line { x: 20.0, y: 20.0 },
            ],
            fill: None,
            stroke: Some((
                crate::ColorValue::Literal(crate::Rgba8::from_rgb_hex(0x00ff_ffff)),
                1.0,
            )),
            transform: crate::CanvasTransform::default(),
            clip: Some(crate::CanvasClipRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
        }])
        .unwrap();
        let node = UiNode::canvas(scene)
            .with_key("canvas")
            .with_motion(transition(MotionProperty::Rotate, 0.0, 45.0, 100));
        assert!(matches!(
            reconcile_node_motion(
                &node,
                &mut MotionRuntime::new(MotionPreference::Normal),
                Instant::now(),
            ),
            Err(MotionError::UnsupportedProperty {
                node: "canvas_with_clipped_path",
                ..
            })
        ));
    }

    #[test]
    fn play_and_restart_recheck_active_budget() {
        let now = Instant::now();
        let mut timeline = MotionTimeline::new(
            "budget",
            MotionTimelineStep::Parallel(vec![
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::Opacity, 0.0, 1.0, 100),
                }),
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(MotionProperty::TranslateX, 0.0, 1.0, 100),
                }),
            ]),
        );
        timeline.autoplay = false;
        let mut runtime = MotionRuntime::new(MotionPreference::Normal);
        runtime.set_active_limit(1);
        let handle = runtime
            .start_timeline(
                ComponentInstancePath::root("UiNode", "root/card"),
                timeline,
                now,
            )
            .unwrap();
        assert!(matches!(
            runtime.play_timeline(&handle, now),
            Err(MotionError::ActiveBudget {
                actual: 2,
                limit: 1
            })
        ));
        assert_eq!(
            runtime.timeline_state(&handle),
            Some(MotionPlaybackState::Idle)
        );
    }

    #[test]
    fn timeline_budget_is_independent_of_declaration_order() {
        let now = Instant::now();
        let timeline = |name: &str, property: MotionProperty, autoplay: bool| {
            let mut timeline = MotionTimeline::new(
                name,
                MotionTimelineStep::Track(MotionTrack {
                    target: ".".to_owned(),
                    source: transition(property, 0.0, 1.0, 100),
                }),
            );
            timeline.autoplay = autoplay;
            timeline
        };
        for reversed in [false, true] {
            let mut runtime = MotionRuntime::new(MotionPreference::Normal);
            runtime.set_active_limit(1);
            let initial = UiNode::text("x")
                .with_key("x")
                .with_timeline(timeline("a", MotionProperty::Opacity, true))
                .with_timeline(timeline("b", MotionProperty::Width, false));
            reconcile_node_motion(&initial, &mut runtime, now).unwrap();
            let first = timeline("a", MotionProperty::Opacity, false);
            let second = timeline("b", MotionProperty::Width, true);
            let replacement = if reversed {
                UiNode::text("x")
                    .with_key("x")
                    .with_timeline(second)
                    .with_timeline(first)
            } else {
                UiNode::text("x")
                    .with_key("x")
                    .with_timeline(first)
                    .with_timeline(second)
            };
            reconcile_node_motion(&replacement, &mut runtime, now).unwrap();
            assert_eq!(runtime.resource_usage().active, 1);
        }
    }
}
