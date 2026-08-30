use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use rhai::{
    CustomType, Engine, EvalAltResult, FLOAT, FuncRegistration, INT, Position, TypeBuilder,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ComponentInstancePath, UiNode, UiNodeKind};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationProperty {
    Opacity,
    TranslateX,
    TranslateY,
    Width,
    Height,
    ClipHeight,
}

impl AnimationProperty {
    fn parse(value: &str) -> Result<Self, AnimationError> {
        match value {
            "opacity" => Ok(Self::Opacity),
            "translate_x" => Ok(Self::TranslateX),
            "translate_y" => Ok(Self::TranslateY),
            "width" => Ok(Self::Width),
            "height" => Ok(Self::Height),
            "clip_height" => Ok(Self::ClipHeight),
            _ => Err(AnimationError::UnknownProperty(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    fn parse(value: &str) -> Result<Self, AnimationError> {
        match value {
            "linear" => Ok(Self::Linear),
            "ease_in" => Ok(Self::EaseIn),
            "ease_out" => Ok(Self::EaseOut),
            "ease_in_out" => Ok(Self::EaseInOut),
            _ => Err(AnimationError::UnknownEasing(value.to_owned())),
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransitionSpec {
    pub property: AnimationProperty,
    pub from: f64,
    pub to: f64,
    pub duration_ms: u64,
    pub easing: Easing,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringSpec {
    pub property: AnimationProperty,
    pub from: f64,
    pub to: f64,
    pub initial_velocity: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AnimationSpec {
    Transition(TransitionSpec),
    LoopingTransition(TransitionSpec),
    Spring(SpringSpec),
}

impl AnimationSpec {
    #[must_use]
    pub const fn property(self) -> AnimationProperty {
        match self {
            Self::Transition(spec) | Self::LoopingTransition(spec) => spec.property,
            Self::Spring(spec) => spec.property,
        }
    }

    #[must_use]
    pub const fn target(self) -> f64 {
        match self {
            Self::Transition(spec) | Self::LoopingTransition(spec) => spec.to,
            Self::Spring(spec) => spec.to,
        }
    }

    #[must_use]
    pub const fn reduced_value(self) -> f64 {
        match self {
            Self::LoopingTransition(spec) => f64::midpoint(spec.from, spec.to),
            Self::Transition(spec) => spec.to,
            Self::Spring(spec) => spec.to,
        }
    }
}

impl CustomType for AnimationSpec {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("AnimationSpec");
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AnimationKey {
    pub component: ComponentInstancePath,
    pub property: AnimationProperty,
}

impl AnimationKey {
    #[must_use]
    pub fn for_node(path: &str, property: AnimationProperty) -> Self {
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
}

#[derive(Clone, Debug)]
enum ActiveAnimation {
    Transition {
        from: f64,
        to: f64,
        started: Instant,
        duration: Duration,
        easing: Easing,
        repeat: bool,
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
}

#[derive(Clone, Debug, Default)]
pub struct AnimationRuntime {
    active: BTreeMap<AnimationKey, ActiveAnimation>,
    settled: BTreeMap<AnimationKey, f64>,
    preference: Option<MotionPreference>,
}

impl AnimationRuntime {
    #[must_use]
    pub fn new(preference: MotionPreference) -> Self {
        Self {
            preference: Some(preference),
            ..Self::default()
        }
    }

    pub fn set_preference(&mut self, preference: MotionPreference) {
        self.preference = Some(preference);
        if preference == MotionPreference::Reduced {
            let active = std::mem::take(&mut self.active);
            for (key, animation) in active {
                self.settled.insert(key, reduced_value(&animation));
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

    /// Start or retarget a keyed animation at the sampled current value.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError`] for non-finite values or invalid spring
    /// parameters.
    pub fn start(
        &mut self,
        component: ComponentInstancePath,
        spec: AnimationSpec,
        now: Instant,
    ) -> Result<AnimationKey, AnimationError> {
        validate_spec(spec)?;
        let key = AnimationKey {
            component,
            property: spec.property(),
        };
        if self
            .active
            .get(&key)
            .is_some_and(|animation| (target(animation) - spec.target()).abs() < f64::EPSILON)
            || self
                .settled
                .get(&key)
                .is_some_and(|value| (*value - spec.target()).abs() < f64::EPSILON)
        {
            return Ok(key);
        }
        if self.preference == Some(MotionPreference::Reduced) {
            self.active.remove(&key);
            self.settled.insert(key.clone(), spec.reduced_value());
            return Ok(key);
        }
        let current = self.sample(&key, now).unwrap_or(match spec {
            AnimationSpec::Transition(spec) | AnimationSpec::LoopingTransition(spec) => spec.from,
            AnimationSpec::Spring(spec) => spec.from,
        });
        let animation = match spec {
            AnimationSpec::Transition(spec) => ActiveAnimation::Transition {
                from: current,
                to: spec.to,
                started: now,
                duration: Duration::from_millis(spec.duration_ms),
                easing: spec.easing,
                repeat: false,
            },
            AnimationSpec::LoopingTransition(spec) => ActiveAnimation::Transition {
                from: current,
                to: spec.to,
                started: now,
                duration: Duration::from_millis(spec.duration_ms),
                easing: spec.easing,
                repeat: true,
            },
            AnimationSpec::Spring(spec) => ActiveAnimation::Spring {
                position: current,
                velocity: spec.initial_velocity,
                target: spec.to,
                stiffness: spec.stiffness,
                damping: spec.damping,
                mass: spec.mass,
                last_tick: now,
            },
        };
        self.settled.remove(&key);
        self.active.insert(key.clone(), animation);
        Ok(key)
    }

    pub fn cancel(&mut self, key: &AnimationKey) {
        self.active.remove(key);
    }

    pub fn retain_keys(&mut self, keys: &BTreeSet<AnimationKey>) {
        self.active.retain(|key, _| keys.contains(key));
        self.settled.retain(|key, _| keys.contains(key));
    }

    pub fn retain_node_scope(&mut self, path: &str, keys: &BTreeSet<AnimationKey>) {
        self.active
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
        self.settled
            .retain(|key, _| !key.in_node_scope(path) || keys.contains(key));
    }

    pub fn cancel_node_scope(&mut self, path: &str) {
        self.active.retain(|key, _| !key.in_node_scope(path));
        self.settled.retain(|key, _| !key.in_node_scope(path));
    }

    #[must_use]
    pub fn snapshot(&self, now: Instant) -> BTreeMap<AnimationKey, f64> {
        self.active
            .keys()
            .chain(self.settled.keys())
            .filter_map(|key| self.sample(key, now).map(|value| (key.clone(), value)))
            .collect()
    }

    #[must_use]
    pub fn sample(&self, key: &AnimationKey, now: Instant) -> Option<f64> {
        self.active
            .get(key)
            .map(|animation| sample_animation(animation, now))
            .or_else(|| self.settled.get(key).copied())
    }

    #[must_use]
    pub fn tick(&mut self, now: Instant) -> AnimationFrame {
        let mut values = BTreeMap::new();
        let mut completed = BTreeSet::new();
        for (key, animation) in &mut self.active {
            let (value, done) = advance_animation(animation, now);
            values.insert(key.clone(), value);
            if done {
                completed.insert(key.clone());
            }
        }
        for key in &completed {
            if let Some(animation) = self.active.remove(key) {
                self.settled.insert(key.clone(), target(&animation));
            }
        }
        AnimationFrame {
            values,
            completed,
            needs_frame: !self.active.is_empty(),
        }
    }
}

/// Reconcile animation declarations from one successfully rendered node tree.
///
/// # Errors
///
/// Returns invalid-spec or missing-key errors before mutating the runtime.
pub fn reconcile_node_animations(
    root: &UiNode,
    runtime: &mut AnimationRuntime,
    now: Instant,
) -> Result<BTreeMap<AnimationKey, f64>, AnimationError> {
    reconcile_node_animations_scoped(root, runtime, now, "root")
}

/// Reconcile one window's animation declarations without touching other windows.
///
/// # Errors
///
/// Returns invalid-spec or missing-key errors before mutating the runtime.
pub fn reconcile_node_animations_scoped(
    root: &UiNode,
    runtime: &mut AnimationRuntime,
    now: Instant,
    root_path: &str,
) -> Result<BTreeMap<AnimationKey, f64>, AnimationError> {
    let mut declarations = Vec::<(String, AnimationSpec)>::new();
    collect_node_animations(root, root_path, &mut declarations)?;
    for (_, spec) in &declarations {
        validate_spec(*spec)?;
    }
    let mut keys = BTreeSet::new();
    for (path, spec) in declarations {
        keys.insert(runtime.start(ComponentInstancePath::root("UiNode", path), spec, now)?);
    }
    runtime.retain_node_scope(root_path, &keys);
    Ok(runtime.snapshot(now))
}

fn collect_node_animations(
    node: &UiNode,
    path: &str,
    output: &mut Vec<(String, AnimationSpec)>,
) -> Result<(), AnimationError> {
    if !node.animations().is_empty() && node.key().is_none() {
        return Err(AnimationError::MissingKey(path.to_owned()));
    }
    output.extend(
        node.animations()
            .iter()
            .copied()
            .map(|animation| (path.to_owned(), animation)),
    );
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for (index, child) in children.iter().enumerate() {
                collect_node_animations(child, &child_path(path, index, child), output)?;
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            collect_node_animations(trigger, &format!("{path}/trigger"), output)?;
            collect_node_animations(content, &format!("{path}/content"), output)?;
        }
        UiNodeKind::ErrorBoundary { child, fallback } => {
            collect_node_animations(child, &format!("{path}/boundary"), output)?;
            collect_node_animations(fallback, &format!("{path}/fallback"), output)?;
        }
        UiNodeKind::Dropdown { spec } => {
            for (name, slot) in [
                ("trigger_slot", spec.trigger_slot.as_deref()),
                ("header_slot", spec.header_slot.as_deref()),
                ("footer_slot", spec.footer_slot.as_deref()),
                ("empty_slot", spec.empty_slot.as_deref()),
            ] {
                if let Some(slot) = slot {
                    collect_node_animations(slot, &format!("{path}/{name}"), output)?;
                }
            }
        }
        UiNodeKind::VirtualList { spec } => {
            for item in &spec.items {
                collect_node_animations(&item.node, &format!("{path}/item:{}", item.key), output)?;
            }
        }
        UiNodeKind::Table { spec } => {
            for column in &spec.columns {
                for (index, cell) in column.custom_cells.iter().flatten().enumerate() {
                    collect_node_animations(
                        cell,
                        &format!("{path}/column:{}/cell:{index}", column.key),
                        output,
                    )?;
                }
            }
            for (name, slot) in [
                ("loading_slot", spec.loading_slot.as_deref()),
                ("empty_slot", spec.empty_slot.as_deref()),
            ] {
                if let Some(slot) = slot {
                    collect_node_animations(slot, &format!("{path}/{name}"), output)?;
                }
            }
            for (index, row) in spec.loading_rows.iter().enumerate() {
                collect_node_animations(row, &format!("{path}/loading_row:{index}"), output)?;
            }
        }
        UiNodeKind::Text { .. }
        | UiNodeKind::Custom { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. }
        | UiNodeKind::Select { .. }
        | UiNodeKind::DatePicker { .. }
        | UiNodeKind::ToastHost { .. } => {}
    }
    Ok(())
}

fn child_path(path: &str, index: usize, child: &UiNode) -> String {
    child.key().map_or_else(
        || format!("{path}/{index}"),
        |key| format!("{path}/{}", key.as_str()),
    )
}

fn sample_animation(animation: &ActiveAnimation, now: Instant) -> f64 {
    match animation {
        ActiveAnimation::Transition {
            from,
            to,
            started,
            duration,
            easing,
            repeat,
        } => {
            if duration.is_zero() {
                return *to;
            }
            let raw_progress = now.duration_since(*started).as_secs_f64() / duration.as_secs_f64();
            let progress = if *repeat {
                raw_progress % 1.0
            } else {
                raw_progress.clamp(0.0, 1.0)
            };
            from + (to - from) * easing.sample(progress)
        }
        ActiveAnimation::Spring { position, .. } => *position,
    }
}

fn advance_animation(animation: &mut ActiveAnimation, now: Instant) -> (f64, bool) {
    match animation {
        ActiveAnimation::Transition {
            from,
            to,
            started,
            duration,
            easing,
            repeat,
        } => {
            let progress = if duration.is_zero() {
                1.0
            } else {
                now.duration_since(*started).as_secs_f64() / duration.as_secs_f64()
            };
            let sampled_progress = if *repeat {
                progress % 1.0
            } else {
                progress.clamp(0.0, 1.0)
            };
            let value = *from + (*to - *from) * easing.sample(sampled_progress);
            (
                value,
                !*repeat && (duration.is_zero() || now.duration_since(*started) >= *duration),
            )
        }
        ActiveAnimation::Spring {
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
    }
}

fn target(animation: &ActiveAnimation) -> f64 {
    match animation {
        ActiveAnimation::Transition { to, .. } | ActiveAnimation::Spring { target: to, .. } => *to,
    }
}

fn reduced_value(animation: &ActiveAnimation) -> f64 {
    match animation {
        ActiveAnimation::Transition {
            from,
            to,
            repeat: true,
            ..
        } => f64::midpoint(*from, *to),
        _ => target(animation),
    }
}

fn validate_spec(spec: AnimationSpec) -> Result<(), AnimationError> {
    let finite = match spec {
        AnimationSpec::Transition(spec) | AnimationSpec::LoopingTransition(spec) => {
            spec.from.is_finite() && spec.to.is_finite() && spec.duration_ms > 0
        }
        AnimationSpec::Spring(spec) => {
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
    };
    if finite {
        Ok(())
    } else {
        Err(AnimationError::InvalidSpec)
    }
}

pub(crate) fn register_animation_api(engine: &mut Engine) {
    engine.build_type::<AnimationSpec>();
    FuncRegistration::new("transition")
        .in_global_namespace()
        .register_into_engine(engine, transition_from_script);
    FuncRegistration::new("spring")
        .in_global_namespace()
        .register_into_engine(engine, spring_from_script);
    FuncRegistration::new("loop_transition")
        .in_global_namespace()
        .register_into_engine(engine, loop_transition_from_script);
}

fn transition_from_script(
    property: &str,
    from: FLOAT,
    to: FLOAT,
    duration_ms: INT,
    easing: &str,
) -> Result<AnimationSpec, Box<EvalAltResult>> {
    let duration_ms = u64::try_from(duration_ms)
        .map_err(|_| Box::new(animation_script_error(&"duration must be non-negative")))?;
    let spec = AnimationSpec::Transition(TransitionSpec {
        property: AnimationProperty::parse(property)
            .map_err(|error| Box::new(animation_script_error(&error)))?,
        from,
        to,
        duration_ms,
        easing: Easing::parse(easing).map_err(|error| Box::new(animation_script_error(&error)))?,
    });
    validate_spec(spec).map_err(|error| Box::new(animation_script_error(&error)))?;
    Ok(spec)
}

fn spring_from_script(
    property: &str,
    from: FLOAT,
    to: FLOAT,
    stiffness: FLOAT,
    damping: FLOAT,
) -> Result<AnimationSpec, Box<EvalAltResult>> {
    let spec = AnimationSpec::Spring(SpringSpec {
        property: AnimationProperty::parse(property)
            .map_err(|error| Box::new(animation_script_error(&error)))?,
        from,
        to,
        initial_velocity: 0.0,
        stiffness,
        damping,
        mass: 1.0,
    });
    validate_spec(spec).map_err(|error| Box::new(animation_script_error(&error)))?;
    Ok(spec)
}

fn loop_transition_from_script(
    property: &str,
    from: FLOAT,
    to: FLOAT,
    duration_ms: INT,
    easing: &str,
) -> Result<AnimationSpec, Box<EvalAltResult>> {
    let AnimationSpec::Transition(spec) =
        transition_from_script(property, from, to, duration_ms, easing)?
    else {
        unreachable!("transition constructor always returns a transition")
    };
    Ok(AnimationSpec::LoopingTransition(spec))
}

fn animation_script_error(error: &impl ToString) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE)
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum AnimationError {
    #[error("animation property `{0}` is unknown")]
    UnknownProperty(String),
    #[error("animation easing `{0}` is unknown")]
    UnknownEasing(String),
    #[error("animation parameters must be finite and physically valid")]
    InvalidSpec,
    #[error("animated node `{0}` requires a stable key")]
    MissingKey(String),
}

#[derive(Clone, Debug)]
pub struct AnimationFrame {
    pub values: BTreeMap<AnimationKey, f64>,
    pub completed: BTreeSet<AnimationKey>,
    pub needs_frame: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_path() -> ComponentInstancePath {
        ComponentInstancePath::root("Accordion", "settings")
    }

    #[test]
    fn transition_retargets_from_current_sample() {
        let start = Instant::now();
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                AnimationSpec::Transition(TransitionSpec {
                    property: AnimationProperty::Opacity,
                    from: 0.0,
                    to: 1.0,
                    duration_ms: 100,
                    easing: Easing::Linear,
                }),
                start,
            )
            .unwrap();
        let halfway = start + Duration::from_millis(50);
        assert!((runtime.sample(&key, halfway).unwrap() - 0.5).abs() < 0.01);
        runtime
            .start(
                key_path(),
                AnimationSpec::Transition(TransitionSpec {
                    property: AnimationProperty::Opacity,
                    from: 1.0,
                    to: 0.0,
                    duration_ms: 100,
                    easing: Easing::Linear,
                }),
                halfway,
            )
            .unwrap();
        assert!((runtime.sample(&key, halfway).unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn reduced_motion_settles_without_frames() {
        let mut runtime = AnimationRuntime::new(MotionPreference::Reduced);
        let key = runtime
            .start(
                key_path(),
                AnimationSpec::Transition(TransitionSpec {
                    property: AnimationProperty::Height,
                    from: 0.0,
                    to: 200.0,
                    duration_ms: 300,
                    easing: Easing::EaseOut,
                }),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(runtime.sample(&key, Instant::now()), Some(200.0));
        assert!(!runtime.tick(Instant::now()).needs_frame);
    }

    #[test]
    fn reduced_motion_keeps_looping_indicators_visible_at_midpoint() {
        let now = Instant::now();
        let spec = AnimationSpec::LoopingTransition(TransitionSpec {
            property: AnimationProperty::TranslateX,
            from: -72.0,
            to: 200.0,
            duration_ms: 900,
            easing: Easing::EaseInOut,
        });
        let mut reduced = AnimationRuntime::new(MotionPreference::Reduced);
        let reduced_key = reduced.start(key_path(), spec, now).unwrap();
        assert_eq!(reduced.sample(&reduced_key, now), Some(64.0));
        assert!(!reduced.tick(now).needs_frame);

        let mut switched = AnimationRuntime::new(MotionPreference::Normal);
        let switched_key = switched.start(key_path(), spec, now).unwrap();
        switched.set_preference(MotionPreference::Reduced);
        assert_eq!(switched.sample(&switched_key, now), Some(64.0));
        assert!(!switched.tick(now).needs_frame);
    }

    #[test]
    fn spring_converges_without_rhai_frame_callbacks() {
        let start = Instant::now();
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                AnimationSpec::Spring(SpringSpec {
                    property: AnimationProperty::TranslateY,
                    from: 0.0,
                    to: 10.0,
                    initial_velocity: 0.0,
                    stiffness: 180.0,
                    damping: 24.0,
                    mass: 1.0,
                }),
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
        let node =
            UiNode::text("animated")
                .with_key("status")
                .with_animation(AnimationSpec::Transition(TransitionSpec {
                    property: AnimationProperty::Opacity,
                    from: 0.0,
                    to: 1.0,
                    duration_ms: 100,
                    easing: Easing::Linear,
                }));
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        reconcile_node_animations(&node, &mut runtime, start).unwrap();
        let halfway = start + Duration::from_millis(50);
        let values = reconcile_node_animations(&node, &mut runtime, halfway).unwrap();
        let key = AnimationKey::for_node("root", AnimationProperty::Opacity);
        assert!((values[&key] - 0.5).abs() < 0.01);
    }

    #[test]
    fn animated_nodes_require_keys_and_removed_declarations_cancel() {
        let start = Instant::now();
        let animation = AnimationSpec::Transition(TransitionSpec {
            property: AnimationProperty::Height,
            from: 0.0,
            to: 40.0,
            duration_ms: 100,
            easing: Easing::EaseOut,
        });
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        assert!(matches!(
            reconcile_node_animations(
                &UiNode::text("missing key").with_animation(animation),
                &mut runtime,
                start
            ),
            Err(AnimationError::MissingKey(_))
        ));
        reconcile_node_animations(
            &UiNode::text("keyed")
                .with_key("row")
                .with_animation(animation),
            &mut runtime,
            start,
        )
        .unwrap();
        assert!(!runtime.snapshot(start).is_empty());
        reconcile_node_animations(&UiNode::text("plain"), &mut runtime, start).unwrap();
        assert!(runtime.snapshot(start).is_empty());
    }

    #[test]
    fn scoped_reconciliation_does_not_remove_other_window_animations() {
        let now = Instant::now();
        let node = UiNode::text("loading").with_key("progress").with_animation(
            AnimationSpec::LoopingTransition(TransitionSpec {
                property: AnimationProperty::Opacity,
                from: 0.0,
                to: 1.0,
                duration_ms: 100,
                easing: Easing::Linear,
            }),
        );
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        reconcile_node_animations_scoped(&node, &mut runtime, now, "window:main/root").unwrap();
        reconcile_node_animations_scoped(&node, &mut runtime, now, "window:settings/root").unwrap();
        assert_eq!(runtime.snapshot(now).len(), 2);
        runtime.cancel_node_scope("window:settings/root");
        assert_eq!(runtime.snapshot(now).len(), 1);
    }

    #[test]
    fn looping_transition_repeats_without_completing() {
        let start = Instant::now();
        let mut runtime = AnimationRuntime::new(MotionPreference::Normal);
        let key = runtime
            .start(
                key_path(),
                AnimationSpec::LoopingTransition(TransitionSpec {
                    property: AnimationProperty::Opacity,
                    from: 0.2,
                    to: 0.8,
                    duration_ms: 100,
                    easing: Easing::Linear,
                }),
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
}
