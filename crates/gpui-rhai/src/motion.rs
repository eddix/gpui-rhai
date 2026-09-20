use std::collections::{BTreeMap, BTreeSet};
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

    fn sample(self, progress: f64) -> f64 {
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
        match self {
            Self::Transition(spec) if spec.iterations.is_none() => {
                f64::midpoint(spec.from, spec.to)
            }
            Self::Transition(spec) => spec.to,
            Self::Spring(spec) => spec.to,
            Self::Keyframes(spec) => spec.frames.last().map_or(0.0, |frame| frame.value),
            Self::Inertia(spec) => inertia_target(spec),
        }
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
    pub component: ComponentInstancePath,
    pub name: String,
}

impl MotionHandle {
    #[must_use]
    pub fn new(component: ComponentInstancePath, name: impl Into<String>) -> Self {
        Self {
            component,
            name: name.into(),
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
}

#[derive(Clone, Debug)]
struct ScheduledMotionTrack {
    target: String,
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
    source: MotionSource,
    state: MotionState,
}

#[derive(Clone, Debug)]
enum MotionState {
    Transition {
        from: f64,
        to: f64,
        started: Instant,
        delay: Duration,
        duration: Duration,
        easing: MotionEasing,
        iterations: Option<u32>,
        autoreverse: bool,
    },
    Spring {
        position: f64,
        velocity: f64,
        target: f64,
        stiffness: f64,
        damping: f64,
        mass: f64,
        last_tick: Instant,
    },
    Keyframes {
        frames: Vec<MotionKeyframe>,
        started: Instant,
        delay: Duration,
        duration: Duration,
        iterations: Option<u32>,
        autoreverse: bool,
    },
    Inertia {
        position: f64,
        velocity: f64,
        friction: f64,
        min: Option<f64>,
        max: Option<f64>,
        bounce: f64,
        snap_points: Vec<f64>,
        last_tick: Instant,
    },
}

#[derive(Clone, Debug, Default)]
pub struct MotionRuntime {
    active: BTreeMap<MotionKey, ActiveMotion>,
    settled: BTreeMap<MotionKey, f64>,
    settled_replay: BTreeMap<MotionKey, Option<String>>,
    timelines: BTreeMap<MotionHandle, ActiveTimeline>,
    timeline_events: Vec<MotionTimelineEvent>,
    host_preference: Option<MotionPreference>,
    preference: Option<MotionPreference>,
    quality: Option<MotionQuality>,
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
                self.settled_replay
                    .insert(key.clone(), animation.replay_key.clone());
                self.settled.insert(key, reduced_value(&animation));
            }
            let handles = self
                .timelines
                .iter()
                .filter(|(_, timeline)| timeline.state == MotionPlaybackState::Playing)
                .map(|(handle, _)| handle.clone())
                .collect::<Vec<_>>();
            for handle in handles {
                let (tracks, callback) = self.timelines.get_mut(&handle).map_or_else(
                    || (Vec::new(), None),
                    |timeline| {
                        timeline.state = MotionPlaybackState::Completed;
                        (timeline.tracks.clone(), timeline.spec.on_complete.clone())
                    },
                );
                for track in tracks {
                    let key = timeline_track_key(&handle, &track.target, track.source.property());
                    let value = if preference == MotionPreference::None {
                        track.source.target()
                    } else {
                        track.source.reduced_value()
                    };
                    self.settled.insert(key.clone(), value);
                    self.settled_replay.insert(key, None);
                }
                self.timeline_events.push(MotionTimelineEvent {
                    handle,
                    kind: MotionTimelineEventKind::Complete,
                    callback,
                });
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

    /// Start or retarget while including an explicit declarative replay key.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError`] for an invalid source.
    #[allow(clippy::too_many_lines)]
    pub fn start_with_replay(
        &mut self,
        component: ComponentInstancePath,
        source: MotionSource,
        replay_key: Option<String>,
        now: Instant,
    ) -> Result<MotionKey, MotionError> {
        validate_source(&source)?;
        let key = MotionKey {
            component,
            property: source.property(),
        };
        let previous_replay = self
            .active
            .get(&key)
            .map(|motion| &motion.replay_key)
            .or_else(|| self.settled_replay.get(&key));
        let replaying = replay_key.is_some() && previous_replay != Some(&replay_key);
        if self
            .active
            .get(&key)
            .is_some_and(|motion| motion.source == source && motion.replay_key == replay_key)
            || self.settled.get(&key).is_some_and(|value| {
                (*value - source.target()).abs() < f64::EPSILON
                    && self.settled_replay.get(&key) == Some(&replay_key)
            })
        {
            return Ok(key);
        }
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            self.active.remove(&key);
            self.settled_replay.insert(key.clone(), replay_key);
            self.settled.insert(
                key.clone(),
                if self.preference == Some(MotionPreference::None) {
                    source.target()
                } else {
                    source.reduced_value()
                },
            );
            return Ok(key);
        }
        let declared_from = match &source {
            MotionSource::Transition(spec) => spec.from,
            MotionSource::Spring(spec) => spec.from,
            MotionSource::Keyframes(spec) => spec.frames.first().map_or(0.0, |frame| frame.value),
            MotionSource::Inertia(spec) => spec.from,
        };
        let current = if replaying {
            declared_from
        } else {
            self.sample(&key, now).unwrap_or(declared_from)
        };
        let state = match &source {
            MotionSource::Transition(spec) => MotionState::Transition {
                from: current,
                to: spec.to,
                started: now,
                delay: Duration::from_millis(spec.delay_ms),
                duration: Duration::from_millis(spec.duration_ms),
                easing: spec.easing,
                iterations: spec.iterations,
                autoreverse: spec.autoreverse,
            },
            MotionSource::Spring(spec) => MotionState::Spring {
                position: current,
                velocity: spec.initial_velocity,
                target: spec.to,
                stiffness: spec.stiffness,
                damping: spec.damping,
                mass: spec.mass,
                last_tick: now,
            },
            MotionSource::Keyframes(spec) => {
                let mut frames = spec.frames.clone();
                if let Some(first) = frames.first_mut() {
                    first.value = current;
                }
                MotionState::Keyframes {
                    frames,
                    started: now,
                    delay: Duration::from_millis(spec.delay_ms),
                    duration: Duration::from_millis(spec.duration_ms),
                    iterations: spec.iterations,
                    autoreverse: spec.autoreverse,
                }
            }
            MotionSource::Inertia(spec) => MotionState::Inertia {
                position: current,
                velocity: spec.velocity,
                friction: spec.friction,
                min: spec.min,
                max: spec.max,
                bounce: spec.bounce,
                snap_points: spec.snap_points.clone(),
                last_tick: now,
            },
        };
        self.settled.remove(&key);
        self.settled_replay.remove(&key);
        self.active.insert(
            key.clone(),
            ActiveMotion {
                replay_key,
                source,
                state,
            },
        );
        Ok(key)
    }

    pub fn cancel(&mut self, key: &MotionKey) {
        self.active.remove(key);
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
        validate_timeline_name(&spec.name)?;
        if spec.iterations == Some(0) {
            return Err(MotionError::InvalidTimeline(
                "timeline iterations must be greater than zero".to_owned(),
            ));
        }
        let (tracks, duration_ms) = compile_timeline(&spec.root)?;
        let handle = MotionHandle::new(component, spec.name.clone());
        if let Some(active) = self.timelines.get_mut(&handle)
            && timeline_compatible(&active.spec, &spec)
        {
            active.spec.on_complete = spec.on_complete;
            active.spec.on_cancel = spec.on_cancel;
            return Ok(handle);
        }
        let state = if spec.autoplay {
            MotionPlaybackState::Playing
        } else {
            MotionPlaybackState::Idle
        };
        if matches!(
            self.preference,
            Some(MotionPreference::Reduced | MotionPreference::None)
        ) {
            for track in &tracks {
                let key = timeline_track_key(&handle, &track.target, track.source.property());
                let value = if self.preference == Some(MotionPreference::None) {
                    track.source.target()
                } else {
                    track.source.reduced_value()
                };
                self.settled.insert(key.clone(), value);
                self.settled_replay.insert(key, None);
            }
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
                },
            );
            self.timeline_events.push(MotionTimelineEvent {
                handle: handle.clone(),
                kind: MotionTimelineEventKind::Complete,
                callback,
            });
            return Ok(handle);
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
            },
        );
        Ok(handle)
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
        let timeline = self.timeline_mut(handle)?;
        if matches!(
            timeline.state,
            MotionPlaybackState::Completed | MotionPlaybackState::Cancelled
        ) {
            timeline.elapsed_before_play = Duration::ZERO;
        }
        timeline.started = now;
        timeline.state = MotionPlaybackState::Playing;
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
        let timeline = self.timeline_mut(handle)?;
        if timeline.state == MotionPlaybackState::Playing {
            timeline.elapsed_before_play = timeline
                .elapsed_before_play
                .saturating_add(now.saturating_duration_since(timeline.started));
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
        if timeline.state == MotionPlaybackState::Completed {
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
        let timeline = self.timeline_mut(handle)?;
        timeline.elapsed_before_play = Duration::ZERO;
        timeline.started = now;
        timeline.state = MotionPlaybackState::Playing;
        Ok(())
    }

    /// Cancel a timeline and enqueue one post-frame cancellation event.
    ///
    /// # Errors
    ///
    /// Returns [`MotionError::UnknownTimeline`] for a stale handle.
    pub fn cancel_timeline(&mut self, handle: &MotionHandle) -> Result<(), MotionError> {
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
        self.timelines.get(handle).map(|timeline| timeline.state)
    }

    /// Resolve one unique named timeline below a view root.
    ///
    /// # Errors
    ///
    /// Returns an explicit error for missing or duplicate names.
    pub fn timeline_handle_in_scope(
        &self,
        scope: &str,
        name: &str,
    ) -> Result<MotionHandle, MotionError> {
        let mut matches = self
            .timelines
            .keys()
            .filter(|handle| handle.name == name && handle.in_node_scope(scope));
        let handle = matches
            .next()
            .cloned()
            .ok_or_else(|| MotionError::UnknownTimeline(name.to_owned()))?;
        if matches.next().is_some() {
            return Err(MotionError::InvalidTimeline(format!(
                "timeline name `{name}` is duplicated inside `{scope}`"
            )));
        }
        Ok(handle)
    }

    pub fn drain_timeline_events(&mut self) -> Vec<MotionTimelineEvent> {
        std::mem::take(&mut self.timeline_events)
    }

    #[must_use]
    pub fn resource_usage(&self) -> MotionResourceUsage {
        MotionResourceUsage {
            particles: 0,
            shared_snapshots: 0,
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
            .ok_or_else(|| MotionError::UnknownTimeline(handle.name.clone()))
    }

    pub fn retain_keys(&mut self, keys: &BTreeSet<MotionKey>) {
        self.active.retain(|key, _| keys.contains(key));
        self.settled.retain(|key, _| keys.contains(key));
        self.settled_replay.retain(|key, _| keys.contains(key));
    }

    pub fn retain_node_scope(&mut self, path: &str, keys: &BTreeSet<MotionKey>) {
        self.active
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled_replay
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
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
                    handle,
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
        }
    }

    pub fn cancel_node_scope(&mut self, path: &str) {
        self.active.retain(|key, _| !key.in_node_scope(path));
        self.settled.retain(|key, _| !key.in_node_scope(path));
        self.settled_replay
            .retain(|key, _| !key.in_node_scope(path));
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
                    handle,
                    kind: MotionTimelineEventKind::Cancel,
                    callback: timeline.spec.on_cancel,
                });
            }
        }
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

    /// Shift one retained node scope forward so elapsed suspended time is not
    /// sampled as animation progress.
    pub fn delay_node_scope(&mut self, path: &str, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        for (key, motion) in &mut self.active {
            if !key.in_node_scope(path) {
                continue;
            }
            match &mut motion.state {
                MotionState::Transition { started, .. }
                | MotionState::Keyframes { started, .. } => {
                    *started = started.checked_add(delay).unwrap_or(*started);
                }
                MotionState::Spring { last_tick, .. } | MotionState::Inertia { last_tick, .. } => {
                    *last_tick = last_tick.checked_add(delay).unwrap_or(*last_tick);
                }
            }
        }
        for (handle, timeline) in &mut self.timelines {
            if handle.in_node_scope(path) && timeline.state == MotionPlaybackState::Playing {
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
            values.extend(advance_timeline(handle, timeline, now).0);
        }
        values
    }

    #[must_use]
    pub fn inspect(&self, now: Instant) -> Vec<MotionSnapshot> {
        let mut snapshots = self
            .active
            .iter()
            .map(|(key, motion)| match &motion.state {
                MotionState::Transition {
                    to,
                    started,
                    duration,
                    iterations,
                    ..
                } => MotionSnapshot {
                    key: key.clone(),
                    kind: "transition".to_owned(),
                    value: sample_motion(motion, now),
                    target: *to,
                    velocity: None,
                    elapsed_ms: duration_ms(now.saturating_duration_since(*started)),
                    duration_ms: Some(duration_ms(*duration)),
                    repeating: iterations.is_none() || iterations.is_some_and(|count| count > 1),
                    active: true,
                },
                MotionState::Spring {
                    velocity,
                    target,
                    last_tick,
                    ..
                } => MotionSnapshot {
                    key: key.clone(),
                    kind: "spring".to_owned(),
                    value: sample_motion(motion, now),
                    target: *target,
                    velocity: Some(*velocity),
                    elapsed_ms: duration_ms(now.saturating_duration_since(*last_tick)),
                    duration_ms: None,
                    repeating: false,
                    active: true,
                },
                MotionState::Keyframes {
                    frames,
                    started,
                    duration,
                    iterations,
                    ..
                } => MotionSnapshot {
                    key: key.clone(),
                    kind: "keyframes".to_owned(),
                    value: sample_motion(motion, now),
                    target: frames.last().map_or(0.0, |frame| frame.value),
                    velocity: None,
                    elapsed_ms: duration_ms(now.saturating_duration_since(*started)),
                    duration_ms: Some(duration_ms(*duration)),
                    repeating: iterations.is_none() || iterations.is_some_and(|count| count > 1),
                    active: true,
                },
                MotionState::Inertia {
                    position,
                    velocity,
                    last_tick,
                    ..
                } => MotionSnapshot {
                    key: key.clone(),
                    kind: "inertia".to_owned(),
                    value: *position,
                    target: motion.source.target(),
                    velocity: Some(*velocity),
                    elapsed_ms: duration_ms(now.saturating_duration_since(*last_tick)),
                    duration_ms: None,
                    repeating: false,
                    active: true,
                },
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
                let active_elapsed = if timeline.state == MotionPlaybackState::Playing {
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
        self.active
            .get(key)
            .map(|animation| sample_motion(animation, now))
            .or_else(|| self.settled.get(key).copied())
    }

    #[must_use]
    pub fn tick(&mut self, now: Instant) -> MotionFrame {
        let mut values = BTreeMap::new();
        let mut completed = BTreeSet::new();
        for (key, animation) in &mut self.active {
            let (value, done) = advance_motion(animation, now);
            values.insert(key.clone(), value);
            if done {
                completed.insert(key.clone());
            }
        }
        for key in &completed {
            if let Some(animation) = self.active.remove(key) {
                self.settled_replay
                    .insert(key.clone(), animation.replay_key.clone());
                self.settled.insert(key.clone(), target(&animation));
            }
        }
        let mut completed_timelines = Vec::new();
        for (handle, timeline) in &mut self.timelines {
            let (timeline_values, done) = advance_timeline(handle, timeline, now);
            for (key, value) in timeline_values {
                values.insert(key.clone(), value);
                self.settled.insert(key, value);
            }
            if done {
                completed_timelines.push(handle.clone());
            }
        }
        for handle in completed_timelines {
            let callback = self.timelines.get_mut(&handle).and_then(|timeline| {
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
            needs_frame: !self.active.is_empty()
                || self
                    .timelines
                    .values()
                    .any(|timeline| timeline.state == MotionPlaybackState::Playing),
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

fn advance_timeline(
    handle: &MotionHandle,
    timeline: &ActiveTimeline,
    now: Instant,
) -> (BTreeMap<MotionKey, f64>, bool) {
    if timeline.state != MotionPlaybackState::Playing {
        return (BTreeMap::new(), false);
    }
    let elapsed = timeline
        .elapsed_before_play
        .saturating_add(now.saturating_duration_since(timeline.started));
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
        if local_ms < track.start_ms {
            continue;
        }
        let track_elapsed = local_ms.saturating_sub(track.start_ms);
        let value = sample_source_at(&track.source, track_elapsed.min(track.duration_ms));
        let key = timeline_track_key(handle, &track.target, track.source.property());
        values.insert(key, value);
    }
    (values, done)
}

fn timeline_track_key(handle: &MotionHandle, target: &str, property: MotionProperty) -> MotionKey {
    let root = handle
        .component
        .single_root_key("UiNode")
        .unwrap_or(handle.name.as_str());
    let path = match target {
        "." | "" => root.to_owned(),
        target if target.starts_with('/') => target.trim_start_matches('/').to_owned(),
        target => format!("{root}/{target}"),
    };
    MotionKey::for_node(&path, property)
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

fn sample_source_at(source: &MotionSource, elapsed_ms: u64) -> f64 {
    match source {
        MotionSource::Transition(spec) => {
            if elapsed_ms < spec.delay_ms {
                return spec.from;
            }
            let elapsed = elapsed_ms.saturating_sub(spec.delay_ms);
            let duration = spec.duration_ms.max(1);
            let iterations = spec.iterations.unwrap_or(1);
            let done = elapsed >= duration.saturating_mul(u64::from(iterations));
            let cycle = if done {
                u64::from(iterations.saturating_sub(1))
            } else {
                elapsed / duration
            };
            let mut progress = if done {
                1.0
            } else {
                Duration::from_millis(elapsed % duration).as_secs_f64()
                    / Duration::from_millis(duration).as_secs_f64()
            };
            if spec.autoreverse && cycle % 2 == 1 {
                progress = 1.0 - progress;
            }
            spec.from + (spec.to - spec.from) * spec.easing.sample(progress)
        }
        MotionSource::Keyframes(spec) => {
            if elapsed_ms < spec.delay_ms {
                return spec.frames.first().map_or(0.0, |frame| frame.value);
            }
            let elapsed = elapsed_ms.saturating_sub(spec.delay_ms);
            let duration = spec.duration_ms.max(1);
            let iterations = spec.iterations.unwrap_or(1);
            let done = elapsed >= duration.saturating_mul(u64::from(iterations));
            let cycle = if done {
                u64::from(iterations.saturating_sub(1))
            } else {
                elapsed / duration
            };
            let mut progress = if done {
                1.0
            } else {
                Duration::from_millis(elapsed % duration).as_secs_f64()
                    / Duration::from_millis(duration).as_secs_f64()
            };
            if spec.autoreverse && cycle % 2 == 1 {
                progress = 1.0 - progress;
            }
            sample_keyframes(&spec.frames, progress)
        }
        MotionSource::Spring(spec) => {
            sample_spring_at(spec, Duration::from_millis(elapsed_ms).as_secs_f64()).0
        }
        MotionSource::Inertia(spec) => {
            sample_inertia_at(spec, Duration::from_millis(elapsed_ms).as_secs_f64())
        }
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

fn sample_inertia_at(spec: &MotionInertia, seconds: f64) -> f64 {
    let projected =
        spec.from + spec.velocity / spec.friction * (1.0 - (-spec.friction * seconds).exp());
    let bounded = projected.clamp(
        spec.min.unwrap_or(f64::NEG_INFINITY),
        spec.max.unwrap_or(f64::INFINITY),
    );
    if Duration::from_secs_f64(seconds.max(0.0)) >= Duration::from_millis(inertia_settle_ms(spec)) {
        nearest_snap(bounded, &spec.snap_points).unwrap_or(bounded)
    } else {
        bounded
    }
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
    validate_shared_layout_ids(root)?;
    let mut declarations = Vec::<(String, MotionSource, Option<String>)>::new();
    let mut timelines = Vec::<(String, MotionTimeline)>::new();
    collect_node_motion(root, root_path, &mut declarations, &mut timelines)?;
    for (_, spec, _) in &declarations {
        validate_source(spec)?;
    }
    let mut keys = BTreeSet::new();
    for (path, spec, replay_key) in declarations {
        keys.insert(runtime.start_with_replay(
            ComponentInstancePath::root("UiNode", path),
            spec,
            replay_key,
            now,
        )?);
    }
    let mut handles = BTreeSet::new();
    for (path, timeline) in timelines {
        handles.insert(runtime.start_timeline(
            ComponentInstancePath::root("UiNode", path),
            timeline,
            now,
        )?);
    }
    runtime.retain_node_scope(root_path, &keys);
    runtime.retain_timeline_scope(root_path, &handles);
    Ok(runtime.snapshot(now))
}

fn validate_shared_layout_ids(root: &UiNode) -> Result<(), MotionError> {
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
    visit(root, &mut BTreeSet::new())
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
    output: &mut Vec<(String, MotionSource, Option<String>)>,
    timelines: &mut Vec<(String, MotionTimeline)>,
) -> Result<(), MotionError> {
    if (!node.motions().is_empty()
        || !node.exit_motions().is_empty()
        || !node.progress_motions().is_empty()
        || !node.timelines().is_empty())
        && node.key().is_none()
    {
        return Err(MotionError::MissingKey(path.to_owned()));
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
            .map(|timeline| (path.to_owned(), timeline)),
    );
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for (index, child) in children.iter().enumerate() {
                collect_node_motion(child, &child_path(path, index, child), output, timelines)?;
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            collect_node_motion(trigger, &format!("{path}/trigger"), output, timelines)?;
            collect_node_motion(content, &format!("{path}/content"), output, timelines)?;
        }
        UiNodeKind::Layer { content, .. } => {
            collect_node_motion(content, &format!("{path}/content"), output, timelines)?;
        }
        UiNodeKind::ErrorBoundary { child, fallback } => {
            collect_node_motion(child, &format!("{path}/boundary"), output, timelines)?;
            collect_node_motion(fallback, &format!("{path}/fallback"), output, timelines)?;
        }
        UiNodeKind::VirtualCollection { spec } => {
            for (index, item) in &spec.realized {
                collect_node_motion(item, &format!("{path}/item:{index}"), output, timelines)?;
            }
        }
        UiNodeKind::RichText { spans, .. } => {
            for (index, span) in spans.iter().enumerate() {
                if span.motions().is_empty() {
                    continue;
                }
                let key = span
                    .key()
                    .ok_or_else(|| MotionError::MissingKey(format!("{path}/span:{index}")))?;
                let span_path = format!("{path}/span:{key}");
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
    } else {
        Ok(())
    }
}

fn child_path(path: &str, index: usize, child: &UiNode) -> String {
    child.key().map_or_else(
        || format!("{path}/{index}"),
        |key| format!("{path}/{}", key.as_str()),
    )
}

fn sample_motion(animation: &ActiveMotion, now: Instant) -> f64 {
    match &animation.state {
        MotionState::Transition {
            from,
            to,
            started,
            delay,
            duration,
            easing,
            iterations,
            autoreverse,
        } => {
            let (progress, _) =
                cycle_progress(now, *started, *delay, *duration, *iterations, *autoreverse);
            from + (to - from) * easing.sample(progress)
        }
        MotionState::Spring { position, .. } | MotionState::Inertia { position, .. } => *position,
        MotionState::Keyframes {
            frames,
            started,
            delay,
            duration,
            iterations,
            autoreverse,
        } => {
            let (progress, _) =
                cycle_progress(now, *started, *delay, *duration, *iterations, *autoreverse);
            sample_keyframes(frames, progress)
        }
    }
}

fn advance_motion(animation: &mut ActiveMotion, now: Instant) -> (f64, bool) {
    match &mut animation.state {
        MotionState::Transition {
            from,
            to,
            started,
            delay,
            duration,
            easing,
            iterations,
            autoreverse,
        } => {
            let (progress, done) =
                cycle_progress(now, *started, *delay, *duration, *iterations, *autoreverse);
            (*from + (*to - *from) * easing.sample(progress), done)
        }
        MotionState::Spring {
            position,
            velocity,
            target,
            stiffness,
            damping,
            mass,
            last_tick,
        } => {
            let elapsed = now.duration_since(*last_tick).as_secs_f64().min(0.05);
            *last_tick = now;
            let displacement = *position - *target;
            let acceleration = (-*stiffness * displacement - *damping * *velocity) / *mass;
            *velocity += acceleration * elapsed;
            *position += *velocity * elapsed;
            let done = (*position - *target).abs() < 0.001 && velocity.abs() < 0.001;
            if done {
                *position = *target;
                *velocity = 0.0;
            }
            (*position, done)
        }
        MotionState::Keyframes {
            frames,
            started,
            delay,
            duration,
            iterations,
            autoreverse,
        } => {
            let (progress, done) =
                cycle_progress(now, *started, *delay, *duration, *iterations, *autoreverse);
            (sample_keyframes(frames, progress), done)
        }
        MotionState::Inertia {
            position,
            velocity,
            friction,
            min,
            max,
            bounce,
            snap_points,
            last_tick,
        } => {
            let elapsed = now.duration_since(*last_tick).as_secs_f64().min(0.05);
            *last_tick = now;
            *velocity *= (-*friction * elapsed).exp();
            *position += *velocity * elapsed;
            if let Some(minimum) = min
                && *position < *minimum
            {
                *position = *minimum;
                *velocity = velocity.abs() * *bounce;
            }
            if let Some(maximum) = max
                && *position > *maximum
            {
                *position = *maximum;
                *velocity = -velocity.abs() * *bounce;
            }
            let done = velocity.abs() < 0.01;
            if done && let Some(nearest) = nearest_snap(*position, snap_points) {
                *position = nearest;
                *velocity = 0.0;
            }
            (*position, done)
        }
    }
}

fn target(animation: &ActiveMotion) -> f64 {
    animation.source.target()
}

fn reduced_value(animation: &ActiveMotion) -> f64 {
    animation.source.reduced_value()
}

fn validate_source(spec: &MotionSource) -> Result<(), MotionError> {
    let finite = match spec {
        MotionSource::Transition(spec) => {
            spec.from.is_finite()
                && spec.to.is_finite()
                && spec.duration_ms > 0
                && spec.iterations != Some(0)
        }
        MotionSource::Spring(spec) => {
            spec.from.is_finite()
                && spec.to.is_finite()
                && spec.initial_velocity.is_finite()
                && spec.stiffness.is_finite()
                && spec.stiffness > 0.0
                && spec.damping.is_finite()
                && spec.damping >= 0.0
                && spec.mass.is_finite()
                && spec.mass > 0.0
        }
        MotionSource::Keyframes(spec) => {
            spec.duration_ms > 0 && spec.iterations != Some(0) && valid_keyframes(&spec.frames)
        }
        MotionSource::Inertia(spec) => {
            spec.from.is_finite()
                && spec.velocity.is_finite()
                && spec.friction.is_finite()
                && spec.friction > 0.0
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

fn cycle_progress(
    now: Instant,
    started: Instant,
    delay: Duration,
    duration: Duration,
    iterations: Option<u32>,
    autoreverse: bool,
) -> (f64, bool) {
    let elapsed = now.saturating_duration_since(started);
    if elapsed < delay {
        return (0.0, false);
    }
    if duration.is_zero() {
        return (1.0, true);
    }
    let raw = elapsed.saturating_sub(delay).as_secs_f64() / duration.as_secs_f64();
    let done = iterations.is_some_and(|count| raw >= f64::from(count));
    let cycle = if done {
        iterations.unwrap_or(1).saturating_sub(1)
    } else {
        raw.floor().to_string().parse::<u32>().unwrap_or(u32::MAX)
    };
    let local = if done { 1.0 } else { raw.fract() };
    (
        if autoreverse && cycle % 2 == 1 {
            1.0 - local
        } else {
            local
        },
        done,
    )
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
        assert!(!runtime.tick(start + Duration::from_millis(500)).needs_frame);
        runtime
            .seek_timeline(&handle, 200, start + Duration::from_millis(500))
            .unwrap();
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
        assert!((sample_source_at(&source, 100) - 1.0).abs() < f64::EPSILON);
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
}
