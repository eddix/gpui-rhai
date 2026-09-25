#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, AppContext, Bounds, ContentMask, Context, Element, ElementId, Entity,
    FocusHandle, FontFallbacks, FontWeight, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollWheelEvent, Styled,
    Task, WeakEntity, Window, canvas, div, fill, point, px, rgba, size,
};

use super::{
    ChartAxisDirection, ChartAxisDomain, ChartAxisScale, ChartBrushMode, ChartCoordinateKind,
    ChartDataLimits, ChartDataSnapshot, ChartDataset, ChartFormatterRegistry, ChartGeoRegistry,
    ChartMark, ChartMarkGeometry, ChartMarkRole, ChartPoint, ChartPreparedData, ChartRect,
    ChartSeriesRegistry, ChartSpec, ChartTheme, ChartTransformRegistry, ChartViewport,
    NativeChartData, PreparedChartScene, apply_chart_selection, chart_region_rect,
    layout_chart_scene_with_axis_windows, prepare_chart_data,
};
use crate::{
    ComponentStateSchema, EffectPrimitiveDescriptor, EventSchema, ObjectField, PrimitiveDescriptor,
    PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitivePlatform, PrimitiveProps, PrimitiveTheme, PrimitiveValue, UiValue, ValueSchema,
};

const WHEEL_COMMIT_DELAY: Duration = Duration::from_millis(80);

#[derive(Clone, Debug)]
enum ChartDataInput {
    Inline(ChartDataSnapshot),
    Native(NativeChartData),
}

impl ChartDataInput {
    fn snapshot(&self) -> ChartDataSnapshot {
        match self {
            Self::Inline(snapshot) => snapshot.clone(),
            Self::Native(data) => data.snapshot(),
        }
    }

    fn same_source(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Inline(left), Self::Inline(right)) => left.same_identity(right),
            (Self::Native(left), Self::Native(right)) => left == right,
            (Self::Inline(_), Self::Native(_)) | (Self::Native(_), Self::Inline(_)) => false,
        }
    }

    fn native(&self) -> Option<&NativeChartData> {
        match self {
            Self::Native(data) => Some(data),
            Self::Inline(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
struct ChartConfig {
    spec: ChartSpec,
    data: ChartDataInput,
    selected: BTreeSet<String>,
    zoom: f64,
    pan: ChartPoint,
    viewport: Option<ChartLinkedViewport>,
    viewport_revision: u64,
    theme: PrimitiveTheme,
}

#[derive(Clone, Debug)]
struct ChartSourceCache {
    spec_prop: PrimitiveValue,
    data_prop: PrimitiveValue,
    key_dimension: Option<String>,
    spec: ChartSpec,
    data: ChartDataInput,
}

impl ChartSourceCache {
    fn matches(&self, props: &PrimitiveProps) -> bool {
        props.get("spec") == Some(&self.spec_prop)
            && props.get("data") == Some(&self.data_prop)
            && data_string_prop(props, "key_dimension") == self.key_dimension
    }
}

#[derive(Clone, Default)]
struct ChartLinkRegistry {
    members: Rc<RefCell<ChartLinkMembers>>,
    selections: Rc<RefCell<ChartLinkSelections>>,
    viewports: Rc<RefCell<BTreeMap<ChartLinkKey, ChartLinkViewportState>>>,
    next_viewport_commit: Rc<Cell<u64>>,
}

type ChartLinkKey = (String, String);
type ChartLinkMembers = BTreeMap<ChartLinkKey, BTreeSet<WeakEntity<ChartEntity>>>;
type ChartLinkSelections =
    BTreeMap<ChartLinkKey, BTreeMap<WeakEntity<ChartEntity>, BTreeSet<String>>>;

#[derive(Clone)]
struct ChartLinkViewportState {
    source: WeakEntity<ChartEntity>,
    commit: u64,
    viewport: ChartLinkedViewport,
}

#[derive(Clone, Debug, PartialEq)]
struct ChartLinkedProjection {
    key: ChartLinkKey,
    commit: u64,
    viewport: ChartLinkedViewport,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum ChartLinkedViewport {
    Cartesian {
        region: String,
        x: Option<ChartLinkedAxis>,
        y: Option<ChartLinkedAxis>,
    },
    Geo {
        region: String,
        map: String,
        projection: String,
        zoom: f64,
        normalized_pan: ChartPoint,
    },
    #[default]
    Unsupported,
}

#[derive(Clone, Debug, PartialEq)]
struct ChartLinkedAxis {
    key: String,
    visible: (f64, f64),
}

#[derive(Clone, Debug)]
struct ChartViewportProposal {
    revision: u64,
    input_generation: u64,
    zoom: f64,
    pan: ChartPoint,
    projection: Option<ChartLinkedViewport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChartDataKey {
    source_epoch: u64,
    revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChartFrameKey {
    data: ChartDataKey,
    frame_epoch: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ChartWheelGesture {
    #[default]
    PhaseLess,
    Explicit,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ChartActivity {
    #[default]
    Active,
    PreparingResume,
    Suspended,
}

impl ChartLinkRegistry {
    fn register(&self, key: Option<(String, String)>, entity: &WeakEntity<ChartEntity>) {
        let mut members = self.members.borrow_mut();
        let departed = members
            .iter()
            .filter(|(group, entries)| entries.contains(entity) && Some(*group) != key.as_ref())
            .map(|(group, _)| group.clone())
            .collect::<BTreeSet<_>>();
        for group in members.values_mut() {
            group.remove(entity);
        }
        members.retain(|_, group| !group.is_empty());
        if let Some(key) = key {
            members.entry(key).or_default().insert((*entity).clone());
        }
        drop(members);
        self.viewports
            .borrow_mut()
            .retain(|group, state| !(departed.contains(group) && state.source == *entity));
    }

    fn unregister(&self, entity: &WeakEntity<ChartEntity>) {
        let mut members = self.members.borrow_mut();
        for group in members.values_mut() {
            group.remove(entity);
        }
        members.retain(|_, group| !group.is_empty());
        let mut selections = self.selections.borrow_mut();
        for sources in selections.values_mut() {
            sources.remove(entity);
        }
        selections.retain(|_, sources| !sources.is_empty());
        self.viewports
            .borrow_mut()
            .retain(|_, state| &state.source != entity);
    }

    fn members(&self, key: &(String, String)) -> BTreeSet<WeakEntity<ChartEntity>> {
        self.members.borrow().get(key).cloned().unwrap_or_default()
    }

    fn linked_selection(
        &self,
        key: Option<&ChartLinkKey>,
        target: &WeakEntity<ChartEntity>,
    ) -> BTreeSet<String> {
        let Some(key) = key else {
            return BTreeSet::new();
        };
        self.selections
            .borrow()
            .get(key)
            .into_iter()
            .flat_map(BTreeMap::iter)
            .filter(|(source, _)| *source != target)
            .flat_map(|(_, selected)| selected.iter().cloned())
            .collect()
    }

    fn linked_viewport(
        &self,
        key: Option<&ChartLinkKey>,
        target: &WeakEntity<ChartEntity>,
    ) -> Option<ChartLinkedProjection> {
        let key = key?;
        let state = self.viewports.borrow().get(key).cloned()?;
        (state.source != *target).then_some(ChartLinkedProjection {
            key: key.clone(),
            commit: state.commit,
            viewport: state.viewport,
        })
    }

    fn broadcast_zoom(
        &self,
        key: &(String, String),
        source: &WeakEntity<ChartEntity>,
        viewport: &ChartLinkedViewport,
        cx: &mut App,
    ) {
        let commit = {
            let commit = self.next_viewport_commit.get().saturating_add(1);
            self.next_viewport_commit.set(commit);
            let mut viewports = self.viewports.borrow_mut();
            viewports.insert(
                key.clone(),
                ChartLinkViewportState {
                    source: source.clone(),
                    commit,
                    viewport: viewport.clone(),
                },
            );
            commit
        };
        for member in self.members(key) {
            if &member != source {
                let projection = ChartLinkedProjection {
                    key: key.clone(),
                    commit,
                    viewport: (*viewport).clone(),
                };
                let _ = member.update(cx, |chart, cx| {
                    if chart.apply_linked_projection(Some(projection)) {
                        chart.invalidate_frame();
                        chart.rebuild_scene_with_motion(cx, false);
                    }
                });
            }
        }
    }

    fn broadcast_hover(
        &self,
        key: &(String, String),
        source: &WeakEntity<ChartEntity>,
        hovered: Option<&str>,
        cx: &mut App,
    ) {
        for member in self.members(key) {
            if &member != source {
                let hovered = hovered.map(ToOwned::to_owned);
                let _ = member.update(cx, |chart, cx| {
                    let next = hovered.as_deref().and_then(|datum| {
                        chart.scene.as_ref()?.marks.iter().find_map(|mark| {
                            mark.datum
                                .as_ref()
                                .is_some_and(|reference| reference.key == datum)
                                .then(|| mark.key.clone())
                        })
                    });
                    if chart.hovered != next {
                        chart.hovered = next;
                        cx.notify();
                    }
                });
            }
        }
    }

    fn broadcast_selection_set(
        &self,
        key: &(String, String),
        source: &WeakEntity<ChartEntity>,
        selected: &BTreeSet<String>,
        cx: &mut App,
    ) {
        {
            let mut selections = self.selections.borrow_mut();
            let sources = selections.entry(key.clone()).or_default();
            sources.retain(|candidate, _| candidate.upgrade().is_some());
            if selected.is_empty() {
                sources.remove(source);
            } else {
                sources.insert(source.clone(), selected.clone());
            }
            if sources.is_empty() {
                selections.remove(key);
            }
        }
        let sources = self
            .selections
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or_default();
        for member in self.members(key) {
            let linked = sources
                .iter()
                .filter(|(candidate, _)| *candidate != &member)
                .flat_map(|(_, selected)| selected.iter().cloned())
                .collect::<BTreeSet<_>>();
            let _ = member.update(cx, |chart, cx| {
                if chart.linked_selected != linked {
                    chart.linked_selected = linked;
                    chart.invalidate_frame();
                    chart.rebuild_scene(cx);
                }
            });
        }
    }
}

struct ChartEntity {
    focus: FocusHandle,
    config: ChartConfig,
    events: PrimitiveEventEmitter,
    transforms: ChartTransformRegistry,
    geo: ChartGeoRegistry,
    custom_series: ChartSeriesRegistry,
    formatters: ChartFormatterRegistry,
    links: ChartLinkRegistry,
    prepared: Option<ChartPreparedData>,
    prepared_key: Option<ChartDataKey>,
    scene: Option<PreparedChartScene>,
    presented_key: Option<ChartFrameKey>,
    previous_scene: Option<PreparedChartScene>,
    bounds: Option<Bounds<Pixels>>,
    error: Option<String>,
    hovered: Option<String>,
    focused: Option<String>,
    zoom: f64,
    pan: ChartPoint,
    dragging_pan: bool,
    pan_origin: Option<Point<Pixels>>,
    brush: Option<(ChartPoint, ChartPoint)>,
    job: u64,
    data_source_epoch: u64,
    requested_data_key: ChartDataKey,
    frame_epoch: u64,
    prepare_task: Option<Task<()>>,
    layout_job: u64,
    layout_task: Option<Task<()>>,
    data_task: Option<Task<()>>,
    wheel_commit_task: Option<Task<()>>,
    wheel_generation: u64,
    activity: ChartActivity,
    viewport_preview_dirty: bool,
    viewport_commit_in_flight: bool,
    viewport_input_generation: u64,
    wheel_gesture: ChartWheelGesture,
    next_viewport_revision: u64,
    pending_viewport: Option<ChartViewportProposal>,
    suspended_at: Option<Instant>,
    transition_started: Option<Instant>,
    linked_selected: BTreeSet<String>,
    linked_projection: Option<ChartLinkedProjection>,
    local_committed_projection: Option<ChartLinkedViewport>,
    effective_projection: Option<ChartLinkedViewport>,
    linked_axis_windows: BTreeMap<String, (f64, f64)>,
}

impl ChartEntity {
    fn new(
        config: ChartConfig,
        events: PrimitiveEventEmitter,
        transforms: ChartTransformRegistry,
        geo: ChartGeoRegistry,
        custom_series: ChartSeriesRegistry,
        formatters: ChartFormatterRegistry,
        links: ChartLinkRegistry,
        cx: &mut Context<Self>,
    ) -> Self {
        let zoom = config.zoom;
        let pan = config.pan;
        let effective_projection = config.viewport.clone();
        let viewport_revision = config.viewport_revision;
        let data_source_epoch = 1;
        let requested_data_key = ChartDataKey {
            source_epoch: data_source_epoch,
            revision: config.data.snapshot().revision(),
        };
        Self {
            focus: cx.focus_handle(),
            config,
            events,
            transforms,
            geo,
            custom_series,
            formatters,
            links,
            prepared: None,
            prepared_key: None,
            scene: None,
            presented_key: None,
            previous_scene: None,
            bounds: None,
            error: None,
            hovered: None,
            focused: None,
            zoom,
            pan,
            dragging_pan: false,
            pan_origin: None,
            brush: None,
            job: 0,
            data_source_epoch,
            requested_data_key,
            frame_epoch: 1,
            prepare_task: None,
            layout_job: 0,
            layout_task: None,
            data_task: None,
            wheel_commit_task: None,
            wheel_generation: 0,
            activity: ChartActivity::Active,
            viewport_preview_dirty: false,
            viewport_commit_in_flight: false,
            viewport_input_generation: 0,
            wheel_gesture: ChartWheelGesture::default(),
            next_viewport_revision: viewport_revision,
            pending_viewport: None,
            suspended_at: None,
            transition_started: None,
            linked_selected: BTreeSet::new(),
            linked_projection: None,
            local_committed_projection: effective_projection.clone(),
            effective_projection,
            linked_axis_windows: BTreeMap::new(),
        }
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        self.materialize_effective_projection();
        self.restart_data_listener(cx);
        self.start_prepare(cx);
    }

    fn current_frame_key(&self) -> ChartFrameKey {
        ChartFrameKey {
            data: self.requested_data_key,
            frame_epoch: self.frame_epoch,
        }
    }

    fn invalidate_frame(&mut self) {
        self.frame_epoch = self.frame_epoch.saturating_add(1);
    }

    fn replace_data_request(&mut self, revision: u64, new_source: bool) {
        if new_source {
            self.data_source_epoch = self.data_source_epoch.saturating_add(1);
        }
        let next = ChartDataKey {
            source_epoch: self.data_source_epoch,
            revision,
        };
        if self.requested_data_key != next {
            self.requested_data_key = next;
            self.invalidate_frame();
        }
    }

    fn suspend(&mut self, cx: &mut Context<Self>) {
        if self.activity == ChartActivity::Suspended {
            return;
        }
        let was_active = self.activity == ChartActivity::Active;
        self.activity = ChartActivity::Suspended;
        if was_active {
            self.suspended_at = Some(self.config.theme.now());
        }
        self.job = self.job.saturating_add(1);
        self.layout_job = self.layout_job.saturating_add(1);
        self.wheel_generation = self.wheel_generation.saturating_add(1);
        self.data_task = None;
        self.prepare_task = None;
        self.layout_task = None;
        self.wheel_commit_task = None;
        self.wheel_gesture = ChartWheelGesture::PhaseLess;
        self.dragging_pan = false;
        self.pan_origin = None;
        self.brush = None;
        self.hovered = None;
        self.viewport_preview_dirty = false;
        self.viewport_commit_in_flight = false;
        self.pending_viewport = None;
        let previous_viewport = (self.zoom, self.pan, self.linked_axis_windows.clone());
        self.restore_committed_viewport();
        let viewport_changed = (previous_viewport.0 - self.zoom).abs() > f64::EPSILON
            || previous_viewport.1 != self.pan
            || previous_viewport.2 != self.linked_axis_windows;
        if viewport_changed {
            self.invalidate_frame();
        }
        cx.notify();
    }

    fn resume(&mut self, cx: &mut Context<Self>) {
        if self.activity != ChartActivity::Suspended {
            return;
        }
        self.activity = ChartActivity::PreparingResume;
        cx.notify();
    }

    fn commit_resume(&mut self, cx: &mut Context<Self>) {
        if self.activity != ChartActivity::PreparingResume {
            return;
        }
        self.activity = ChartActivity::Active;
        if let Some(suspended_at) = self.suspended_at.take()
            && let Some(started) = self.transition_started.as_mut()
        {
            *started += self
                .config
                .theme
                .now()
                .saturating_duration_since(suspended_at);
        }
        self.restart_data_listener(cx);
        let source_revision = self.config.data.snapshot().revision();
        self.replace_data_request(source_revision, false);
        if self.prepared_key != Some(self.requested_data_key) {
            self.start_prepare(cx);
        } else if self.presented_key != Some(self.current_frame_key()) {
            self.rebuild_scene(cx);
        } else {
            cx.notify();
        }
    }

    fn update_config(
        &mut self,
        config: ChartConfig,
        events: PrimitiveEventEmitter,
        linked_selected: BTreeSet<String>,
        linked_projection: Option<ChartLinkedProjection>,
        cx: &mut Context<Self>,
    ) {
        let previous_link = link_key(&self.config.spec);
        let next_link = link_key(&config.spec);
        let data_changed = !self.config.data.same_source(&config.data);
        let spec_changed = self.config.spec != config.spec;
        let selection_changed = self.config.selected != config.selected;
        let linked_selection_changed = self.linked_selected != linked_selected;
        let external_viewport_changed = (self.config.zoom - config.zoom).abs() > f64::EPSILON
            || self.config.pan != config.pan
            || self.config.viewport != config.viewport;
        let pending_viewport = self.pending_viewport.clone();
        let had_pending_viewport = pending_viewport.is_some();
        let acknowledges_pending = pending_viewport
            .as_ref()
            .is_some_and(|proposal| config.viewport_revision >= proposal.revision);
        let programmatic_viewport_change = !had_pending_viewport && external_viewport_changed;
        let theme_changed = primitive_chart_theme(&self.config.theme)
            != primitive_chart_theme(&config.theme)
            || self.config.theme.motion_preference() != config.theme.motion_preference()
            || self.config.theme.motion_quality() != config.theme.motion_quality();
        let acknowledge_viewport = acknowledges_pending || programmatic_viewport_change;
        let previous_local_viewport = (self.zoom, self.pan, self.linked_axis_windows.clone());
        self.config = config;
        self.linked_selected = linked_selected;
        if previous_link != next_link
            && let Some(key) = previous_link.clone()
        {
            let links = self.links.clone();
            let source = cx.weak_entity();
            cx.defer(move |cx| {
                links.broadcast_selection_set(&key, &source, &BTreeSet::new(), cx);
            });
        }
        let mut committed_projection = None;
        let mut local_viewport_authority = false;
        if acknowledges_pending {
            let proposal = pending_viewport
                .as_ref()
                .expect("acknowledgement requires a pending viewport");
            let has_newer_preview = self.viewport_input_generation > proposal.input_generation;
            let accepted = viewport_matches_proposal(&self.config, proposal);
            self.linked_projection = None;
            local_viewport_authority = true;
            if accepted {
                self.local_committed_projection
                    .clone_from(&proposal.projection);
                committed_projection = Some(
                    proposal
                        .projection
                        .clone()
                        .unwrap_or_else(|| self.linked_viewport_for(proposal.zoom, proposal.pan)),
                );
                if has_newer_preview {
                    self.viewport_preview_dirty = true;
                } else {
                    self.effective_projection.clone_from(&proposal.projection);
                    self.zoom = proposal.zoom;
                    self.pan = proposal.pan;
                    self.materialize_effective_projection();
                    self.viewport_preview_dirty = false;
                    self.wheel_generation = self.wheel_generation.saturating_add(1);
                    self.wheel_commit_task = None;
                }
            } else {
                let current_projection = self.effective_projection.clone();
                let proposal_projection = proposal.projection.clone();
                let zoom_ratio = if proposal.zoom.abs() > f64::EPSILON {
                    self.zoom / proposal.zoom
                } else {
                    1.0
                };
                let pan_delta = ChartPoint {
                    x: self.pan.x - proposal.pan.x,
                    y: self.pan.y - proposal.pan.y,
                };
                self.local_committed_projection = self.config.viewport.clone();
                self.effective_projection = self.config.viewport.clone();
                self.zoom = self.config.zoom;
                self.pan = self.config.pan;
                self.linked_axis_windows.clear();
                if self.effective_projection.is_some() {
                    self.materialize_effective_projection();
                }
                let base_projection = self
                    .effective_projection
                    .clone()
                    .unwrap_or_else(|| self.linked_viewport_for(self.zoom, self.pan));
                committed_projection = Some(base_projection.clone());
                if has_newer_preview {
                    self.effective_projection = rebase_viewport_projection(
                        base_projection,
                        proposal_projection,
                        current_projection,
                    );
                    self.zoom = (self.config.zoom * zoom_ratio).clamp(0.5, 20.0);
                    self.pan = ChartPoint {
                        x: self.config.pan.x + pan_delta.x,
                        y: self.config.pan.y + pan_delta.y,
                    };
                    self.materialize_effective_projection();
                    self.viewport_preview_dirty = true;
                } else {
                    self.viewport_preview_dirty = false;
                    self.wheel_generation = self.wheel_generation.saturating_add(1);
                    self.wheel_commit_task = None;
                }
            }
            self.viewport_commit_in_flight = false;
            self.next_viewport_revision = self
                .next_viewport_revision
                .max(self.config.viewport_revision);
            self.pending_viewport = None;
        } else if programmatic_viewport_change {
            self.linked_projection = None;
            self.local_committed_projection = self.config.viewport.clone();
            self.effective_projection = self.config.viewport.clone();
            if self.effective_projection.is_some() {
                self.materialize_effective_projection();
            } else {
                self.zoom = self.config.zoom;
                self.pan = self.config.pan;
                self.linked_axis_windows.clear();
            }
            committed_projection = Some(
                self.effective_projection
                    .clone()
                    .unwrap_or_else(|| self.linked_viewport_for(self.zoom, self.pan)),
            );
            local_viewport_authority = true;
        }
        let linked_viewport_changed = if local_viewport_authority {
            false
        } else {
            self.apply_linked_projection(linked_projection)
        };
        let local_viewport_changed = acknowledge_viewport
            && ((previous_local_viewport.0 - self.zoom).abs() > f64::EPSILON
                || previous_local_viewport.1 != self.pan
                || previous_local_viewport.2 != self.linked_axis_windows);
        if data_changed || spec_changed {
            let revision = self.config.data.snapshot().revision();
            self.replace_data_request(revision, true);
        }
        self.events = events;
        if let (Some(viewport), Some(key)) = (committed_projection, next_link.clone()) {
            let links = self.links.clone();
            let source = cx.weak_entity();
            cx.defer(move |cx| links.broadcast_zoom(&key, &source, &viewport, cx));
        }
        if let Some(key) = next_link
            && (selection_changed || previous_link.as_ref() != Some(&key))
        {
            let links = self.links.clone();
            let source = cx.weak_entity();
            let selected = self.config.selected.clone();
            cx.defer(move |cx| {
                links.broadcast_selection_set(&key, &source, &selected, cx);
            });
        }
        if data_changed {
            self.restart_data_listener(cx);
        }
        if data_changed || spec_changed {
            self.start_prepare(cx);
        } else if selection_changed
            || linked_selection_changed
            || linked_viewport_changed
            || theme_changed
        {
            self.invalidate_frame();
            self.rebuild_scene_with_motion(cx, !linked_viewport_changed);
        } else if local_viewport_changed {
            self.invalidate_frame();
            self.rebuild_scene_with_motion(cx, false);
        }
    }

    fn restart_data_listener(&mut self, cx: &mut Context<Self>) {
        self.data_task = None;
        if self.activity != ChartActivity::Active {
            return;
        }
        let Some(data) = self.config.data.native().cloned() else {
            return;
        };
        let mut listener = data.listen();
        self.data_task = Some(cx.spawn(async move |this, cx| {
            loop {
                listener.await;
                listener = data.listen();
                if this.update(cx, ChartEntity::start_prepare).is_err() {
                    break;
                }
            }
        }));
    }

    fn start_prepare(&mut self, cx: &mut Context<Self>) {
        if self.activity != ChartActivity::Active {
            return;
        }
        let data = self.config.data.snapshot();
        self.replace_data_request(data.revision(), false);
        let data_key = self.requested_data_key;
        self.job = self.job.saturating_add(1);
        self.layout_job = self.layout_job.saturating_add(1);
        self.layout_task = None;
        let job = self.job;
        let spec = self.config.spec.clone();
        let transforms = self.transforms.clone();
        let custom_series = self.custom_series.clone();
        let formatters = self.formatters.clone();
        self.error = None;
        self.prepare_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    prepare_chart_data(spec, &data, &transforms, &custom_series, &formatters)
                })
                .await;
            let _ = this.update(cx, |chart, cx| {
                if chart.activity != ChartActivity::Active
                    || chart.job != job
                    || chart.requested_data_key != data_key
                {
                    return;
                }
                chart.prepare_task = None;
                match result {
                    Ok(prepared) => {
                        chart.prepared = Some(prepared);
                        chart.prepared_key = Some(data_key);
                        chart.error = None;
                        chart.rebuild_scene(cx);
                    }
                    Err(error) => {
                        chart.error = Some(error.to_string());
                        cx.notify();
                    }
                }
            });
        }));
    }

    fn rebuild_scene(&mut self, cx: &mut Context<Self>) {
        self.rebuild_scene_with_motion(cx, true);
    }

    fn rebuild_scene_with_motion(&mut self, cx: &mut Context<Self>, animate: bool) {
        if self.activity != ChartActivity::Active {
            return;
        }
        let (Some(prepared), Some(bounds)) = (self.prepared.clone(), self.bounds) else {
            cx.notify();
            return;
        };
        if self.prepared_key != Some(self.requested_data_key) {
            return;
        }
        let frame_key = self.current_frame_key();
        self.layout_job = self.layout_job.saturating_add(1);
        let job = self.layout_job;
        let theme = primitive_chart_theme(&self.config.theme);
        let geo = self.geo.clone();
        let viewport = ChartViewport {
            zoom: self.zoom,
            pan: self.pan,
        };
        let axis_windows = self.linked_axis_windows.clone();
        let selected = self
            .config
            .selected
            .union(&self.linked_selected)
            .cloned()
            .collect::<BTreeSet<_>>();
        let previous = self.current_scene_sample();
        self.layout_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut scene = layout_chart_scene_with_axis_windows(
                        &prepared,
                        f64::from(bounds.size.width),
                        f64::from(bounds.size.height),
                        &theme,
                        &geo,
                        viewport,
                        &axis_windows,
                    )?;
                    apply_chart_selection(&mut scene, &selected, theme.selection);
                    Ok::<_, super::ChartPrepareError>(scene)
                })
                .await;
            let _ = this.update(cx, |chart, cx| {
                if chart.activity != ChartActivity::Active
                    || chart.layout_job != job
                    || chart.current_frame_key() != frame_key
                {
                    return;
                }
                chart.layout_task = None;
                match result {
                    Ok(scene) => {
                        chart.scene = Some(scene.clone());
                        chart.materialize_effective_projection();
                        chart.presented_key = Some(frame_key);
                        if animate
                            && chart_motion_duration(&chart.config)
                                .is_some_and(|duration| !duration.is_zero())
                        {
                            chart.previous_scene = Some(previous.unwrap_or_else(|| {
                                let mut empty = scene;
                                empty.marks = Vec::new().into();
                                empty
                            }));
                            chart.transition_started = Some(chart.config.theme.now());
                        } else {
                            chart.previous_scene = None;
                            chart.transition_started = None;
                        }
                        chart.error = None;
                    }
                    Err(error) => chart.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
    }

    fn set_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let size_changed = self
            .bounds
            .is_none_or(|previous| previous.size != bounds.size);
        self.bounds = Some(bounds);
        if size_changed && self.activity == ChartActivity::Active {
            self.materialize_effective_projection();
            self.invalidate_frame();
            self.rebuild_scene(cx);
        }
    }

    fn linked_viewport_for(&self, zoom: f64, pan: ChartPoint) -> ChartLinkedViewport {
        let Some(scene) = &self.scene else {
            return ChartLinkedViewport::default();
        };
        let Some(region) = self.config.spec.regions.first() else {
            return ChartLinkedViewport::default();
        };
        let Some(plot) = scene.plot_regions.get(&region.key).copied() else {
            return ChartLinkedViewport::default();
        };
        match region.kind {
            ChartCoordinateKind::Cartesian2d => {
                let x_prefix = format!("{}:x:", region.key);
                let y_prefix = format!("{}:y:", region.key);
                let x = scene.axis_domains.iter().find_map(|(key, domain)| {
                    let identity = key.strip_prefix(&x_prefix)?;
                    if domain.scale == ChartAxisScale::Category {
                        return None;
                    }
                    Some(ChartLinkedAxis {
                        key: identity.to_owned(),
                        visible: linked_visible_domain(*domain, zoom, pan.x, plot.width)?,
                    })
                });
                let y = scene.axis_domains.iter().find_map(|(key, domain)| {
                    let identity = key.strip_prefix(&y_prefix)?;
                    if domain.scale == ChartAxisScale::Category {
                        return None;
                    }
                    Some(ChartLinkedAxis {
                        key: identity.to_owned(),
                        visible: linked_visible_domain(*domain, zoom, pan.y, -plot.height)?,
                    })
                });
                ChartLinkedViewport::Cartesian {
                    region: region.key.clone(),
                    x,
                    y,
                }
            }
            ChartCoordinateKind::Geo2d => ChartLinkedViewport::Geo {
                region: region.key.clone(),
                map: region.map.clone().unwrap_or_default(),
                projection: region
                    .projection
                    .clone()
                    .unwrap_or_else(|| "equirectangular".to_owned()),
                zoom,
                normalized_pan: ChartPoint {
                    x: pan.x / plot.width.max(f64::EPSILON),
                    y: pan.y / plot.height.max(f64::EPSILON),
                },
            },
            ChartCoordinateKind::Polar => ChartLinkedViewport::Unsupported,
        }
    }

    fn apply_linked_projection(&mut self, projection: Option<ChartLinkedProjection>) -> bool {
        let previous_identity = self
            .linked_projection
            .as_ref()
            .map(|state| (&state.key, state.commit));
        let next_identity = projection.as_ref().map(|state| (&state.key, state.commit));
        if previous_identity == next_identity {
            return false;
        }
        let previous_windows = self.linked_axis_windows.clone();
        let previous_viewport = (self.zoom, self.pan);
        self.linked_projection = projection;
        if self.viewport_preview_dirty
            || self.dragging_pan
            || self.wheel_gesture == ChartWheelGesture::Explicit
        {
            return false;
        }
        self.restore_committed_viewport();
        previous_windows != self.linked_axis_windows
            || (previous_viewport.0 - self.zoom).abs() > f64::EPSILON
            || previous_viewport.1 != self.pan
    }

    fn restore_committed_viewport(&mut self) {
        self.effective_projection = self
            .linked_projection
            .as_ref()
            .map(|projection| projection.viewport.clone())
            .or_else(|| self.local_committed_projection.clone());
        if self.effective_projection.is_some() {
            self.materialize_effective_projection();
            return;
        }
        self.zoom = self.config.zoom;
        self.pan = self.config.pan;
        self.linked_axis_windows.clear();
    }

    fn materialize_effective_projection(&mut self) {
        self.linked_axis_windows.clear();
        let Some(projection) = self.effective_projection.clone() else {
            return;
        };
        match projection {
            ChartLinkedViewport::Cartesian { region, x, y } => {
                if !self.config.spec.regions.iter().any(|target| {
                    target.key == region && target.kind == ChartCoordinateKind::Cartesian2d
                }) {
                    self.effective_projection = None;
                    self.zoom = self.config.zoom;
                    self.pan = self.config.pan;
                    return;
                }
                if let Some(axis) = &x {
                    self.linked_axis_windows
                        .insert(format!("{region}:x:{}", axis.key), axis.visible);
                }
                if let Some(axis) = &y {
                    self.linked_axis_windows
                        .insert(format!("{region}:y:{}", axis.key), axis.visible);
                }
                self.sync_scalar_from_cartesian(&region, x.as_ref(), y.as_ref());
            }
            ChartLinkedViewport::Geo {
                region,
                map,
                projection,
                zoom,
                normalized_pan,
            } => {
                let Some(_target) = self.config.spec.regions.iter().find(|target| {
                    target.key == region
                        && target.kind == ChartCoordinateKind::Geo2d
                        && target.map.as_deref() == Some(map.as_str())
                        && target.projection.as_deref().unwrap_or("equirectangular")
                            == projection.as_str()
                }) else {
                    self.effective_projection = None;
                    self.zoom = self.config.zoom;
                    self.pan = self.config.pan;
                    return;
                };
                let theme = primitive_chart_theme(&self.config.theme);
                let plot = self.bounds.and_then(|bounds| {
                    chart_region_rect(
                        &self.config.spec,
                        f64::from(bounds.size.width),
                        f64::from(bounds.size.height),
                        &region,
                        &theme,
                    )
                });
                let size = plot.map_or((1.0, 1.0), |plot| (plot.width, plot.height));
                self.zoom = zoom;
                self.pan = ChartPoint {
                    x: normalized_pan.x * size.0,
                    y: normalized_pan.y * size.1,
                };
            }
            ChartLinkedViewport::Unsupported => {
                self.effective_projection = None;
                self.zoom = self.config.zoom;
                self.pan = self.config.pan;
            }
        }
    }

    fn sync_scalar_from_cartesian(
        &mut self,
        region: &str,
        x: Option<&ChartLinkedAxis>,
        y: Option<&ChartLinkedAxis>,
    ) {
        let Some(scene) = &self.scene else {
            return;
        };
        let theme = primitive_chart_theme(&self.config.theme);
        let plot = self
            .bounds
            .and_then(|bounds| {
                chart_region_rect(
                    &self.config.spec,
                    f64::from(bounds.size.width),
                    f64::from(bounds.size.height),
                    region,
                    &theme,
                )
            })
            .or_else(|| scene.plot_regions.get(region).copied());
        let Some(plot) = plot else { return };
        let axis_state = |channel: &str, axis: &ChartLinkedAxis, range: f64| {
            let domain = scene
                .axis_domains
                .get(&format!("{region}:{channel}:{}", axis.key))
                .copied()?;
            let full = axis_domain_space(domain.full, domain.scale)?;
            let visible = axis_domain_space(axis.visible, domain.scale)?;
            let full_span = full.1 - full.0;
            let visible_span = visible.1 - visible.0;
            if full_span <= f64::EPSILON || visible_span <= f64::EPSILON {
                return None;
            }
            let zoom = (full_span / visible_span).clamp(0.5, 20.0);
            Some((zoom, linked_pan(domain, axis.visible, range, zoom)))
        };
        let x_state = x.and_then(|axis| axis_state("x", axis, plot.width));
        let y_state = y.and_then(|axis| axis_state("y", axis, -plot.height));
        if let Some((zoom, pan)) = x_state {
            self.zoom = zoom;
            self.pan.x = pan;
        } else if let Some((zoom, _)) = y_state {
            self.zoom = zoom;
        }
        if let Some((_, pan)) = y_state {
            self.pan.y = pan;
        }
    }

    fn ensure_effective_projection(&mut self) {
        if self.effective_projection.is_some() {
            return;
        }
        let projection = self.linked_viewport_for(self.zoom, self.pan);
        if !matches!(projection, ChartLinkedViewport::Unsupported) {
            self.effective_projection = Some(projection);
            self.materialize_effective_projection();
        }
    }

    fn zoom_effective_viewport(&mut self, factor: f64) -> bool {
        let next_zoom = (self.zoom * factor).clamp(0.5, 20.0);
        if (next_zoom - self.zoom).abs() <= f64::EPSILON {
            return false;
        }
        let applied_factor = next_zoom / self.zoom;
        self.ensure_effective_projection();
        let scales = self.scene.as_ref().map(|scene| scene.axis_domains.clone());
        if let Some(projection) = self.effective_projection.as_mut() {
            match projection {
                ChartLinkedViewport::Cartesian { region, x, y } => {
                    for (channel, axis) in [("x", x), ("y", y)] {
                        let Some(axis) = axis else { continue };
                        let scale = scales
                            .as_ref()
                            .and_then(|domains| {
                                domains.get(&format!("{region}:{channel}:{}", axis.key))
                            })
                            .map_or(ChartAxisScale::Linear, |domain| domain.scale);
                        axis.visible = zoom_axis_window(axis.visible, scale, applied_factor);
                    }
                }
                ChartLinkedViewport::Geo { zoom, .. } => *zoom = next_zoom,
                ChartLinkedViewport::Unsupported => return false,
            }
            self.materialize_effective_projection();
        } else {
            self.zoom = next_zoom;
        }
        true
    }

    fn pan_effective_viewport(&mut self, delta: ChartPoint) -> bool {
        self.ensure_effective_projection();
        let scene = self.scene.clone();
        if let Some(projection) = self.effective_projection.as_mut() {
            match projection {
                ChartLinkedViewport::Cartesian { region, x, y } => {
                    let Some(scene) = scene.as_ref() else {
                        return false;
                    };
                    let theme = primitive_chart_theme(&self.config.theme);
                    let plot = self
                        .bounds
                        .and_then(|bounds| {
                            chart_region_rect(
                                &self.config.spec,
                                f64::from(bounds.size.width),
                                f64::from(bounds.size.height),
                                region,
                                &theme,
                            )
                        })
                        .or_else(|| scene.plot_regions.get(region).copied());
                    let Some(plot) = plot else { return false };
                    for (channel, axis, pixels, range) in [
                        ("x", x, delta.x, plot.width),
                        ("y", y, delta.y, -plot.height),
                    ] {
                        let Some(axis) = axis else { continue };
                        let Some(domain) = scene
                            .axis_domains
                            .get(&format!("{region}:{channel}:{}", axis.key))
                            .copied()
                        else {
                            continue;
                        };
                        axis.visible = pan_axis_window(axis.visible, domain, pixels, range);
                    }
                }
                ChartLinkedViewport::Geo {
                    region,
                    normalized_pan,
                    ..
                } => {
                    let Some(bounds) = self.bounds else {
                        return false;
                    };
                    let theme = primitive_chart_theme(&self.config.theme);
                    let Some(plot) = chart_region_rect(
                        &self.config.spec,
                        f64::from(bounds.size.width),
                        f64::from(bounds.size.height),
                        region,
                        &theme,
                    ) else {
                        return false;
                    };
                    normalized_pan.x += delta.x / plot.width.max(f64::EPSILON);
                    normalized_pan.y += delta.y / plot.height.max(f64::EPSILON);
                }
                ChartLinkedViewport::Unsupported => return false,
            }
            self.materialize_effective_projection();
        } else {
            self.pan.x += delta.x;
            self.pan.y += delta.y;
        }
        true
    }

    fn local_point(&self, position: Point<Pixels>) -> Option<ChartPoint> {
        self.visual_point(position)
    }

    fn visual_point(&self, position: Point<Pixels>) -> Option<ChartPoint> {
        let bounds = self.bounds?;
        Some(ChartPoint {
            x: f64::from(position.x - bounds.origin.x),
            y: f64::from(position.y - bounds.origin.y),
        })
    }

    fn hit_mark_at(&self, position: Point<Pixels>) -> Option<ChartMark> {
        let scene = self.current_scene_sample()?;
        let visual = self.visual_point(position)?;
        scene.hit_test(visual).cloned()
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.dragging_pan {
            if let Some(origin) = self.pan_origin {
                let changed = self.pan_effective_viewport(ChartPoint {
                    x: f64::from(event.position.x - origin.x),
                    y: f64::from(event.position.y - origin.y),
                });
                if !changed {
                    return;
                }
                self.viewport_input_generation = self.viewport_input_generation.saturating_add(1);
                self.viewport_preview_dirty = true;
                self.pan_origin = Some(event.position);
                self.invalidate_frame();
                self.rebuild_scene_with_motion(cx, false);
            }
            return;
        }
        if event.dragging()
            && !matches!(self.config.spec.brush, ChartBrushMode::None)
            && let Some(point) = self.local_point(event.position)
            && let Some((start, _)) = self.brush
        {
            self.brush = Some((start, point));
            cx.notify();
            return;
        }
        let hovered_mark = self.hit_mark_at(event.position);
        let hovered = hovered_mark.as_ref().map(|mark| mark.key.clone());
        if hovered != self.hovered {
            self.hovered.clone_from(&hovered);
            if let Some(key) = link_key(&self.config.spec) {
                let links = self.links.clone();
                let source = cx.weak_entity();
                let datum = hovered_mark.and_then(|mark| mark.datum.map(|datum| datum.key));
                window.defer(cx, move |_, cx| {
                    links.broadcast_hover(&key, &source, datum.as_deref(), cx);
                });
            }
            cx.notify();
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        match event.button {
            MouseButton::Middle => {
                self.dragging_pan = true;
                self.pan_origin = Some(event.position);
            }
            MouseButton::Left => {
                if let Some(mark) = self.hit_mark_at(event.position)
                    && matches!(mark.role, ChartMarkRole::Legend | ChartMarkRole::Annotation)
                {
                    match mark.role {
                        ChartMarkRole::Legend => self.emit_legend(mark, window, cx),
                        ChartMarkRole::Annotation => self.emit_annotation(mark, window, cx),
                        _ => unreachable!(),
                    }
                } else if !matches!(self.config.spec.brush, ChartBrushMode::None)
                    && let Some(point) = self.local_point(event.position)
                    && self.current_scene_sample().is_some_and(|scene| {
                        scene
                            .plot_regions
                            .values()
                            .any(|region| region.contains(point))
                    })
                {
                    let point = if self.config.spec.brush == ChartBrushMode::GeoRegion {
                        self.current_scene_sample()
                            .and_then(|scene| scene.hit_test(point).cloned())
                            .as_ref()
                            .and_then(mark_center)
                            .unwrap_or(point)
                    } else {
                        point
                    };
                    self.brush = Some((point, point));
                } else if let Some(mark) = self.hit_mark_at(event.position)
                    && mark.role == ChartMarkRole::Data
                {
                    self.emit_select(&mark, window, cx);
                }
            }
            MouseButton::Right | MouseButton::Navigate(_) => {}
        }
        cx.notify();
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.button == MouseButton::Middle {
            self.dragging_pan = false;
            self.pan_origin = None;
            self.emit_viewport_change(window, cx);
        }
        if event.button == MouseButton::Left
            && let Some((start, end)) = self.brush.take()
        {
            self.emit_brush(start, end, window, cx);
        }
        cx.notify();
    }

    fn wheel(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(16.0));
        if matches!(event.touch_phase, gpui::TouchPhase::Ended) {
            self.wheel_gesture = ChartWheelGesture::PhaseLess;
            self.emit_viewport_change(window, cx);
            cx.stop_propagation();
            return;
        }
        if matches!(event.touch_phase, gpui::TouchPhase::Started) {
            self.wheel_gesture = ChartWheelGesture::Explicit;
            self.wheel_generation = self.wheel_generation.saturating_add(1);
            self.wheel_commit_task = None;
        }
        let factor = (-f64::from(delta.y) / 400.0).exp();
        if !self.zoom_effective_viewport(factor) {
            return;
        }
        self.viewport_input_generation = self.viewport_input_generation.saturating_add(1);
        self.viewport_preview_dirty = true;
        self.invalidate_frame();
        self.rebuild_scene_with_motion(cx, false);
        cx.stop_propagation();
        if self.wheel_gesture == ChartWheelGesture::Explicit {
            return;
        }
        if event.delta.precise() {
            self.schedule_wheel_commit(window, cx);
        } else {
            self.emit_viewport_change(window, cx);
        }
    }

    fn schedule_wheel_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.wheel_generation = self.wheel_generation.saturating_add(1);
        let generation = self.wheel_generation;
        self.wheel_commit_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(WHEEL_COMMIT_DELAY).await;
            let _ = this.update_in(cx, |chart, window, cx| {
                if chart.wheel_generation == generation {
                    chart.emit_viewport_change(window, cx);
                }
            });
        }));
    }

    fn emit_viewport_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.viewport_preview_dirty || self.viewport_commit_in_flight {
            return;
        }
        self.viewport_preview_dirty = false;
        self.viewport_commit_in_flight = true;
        self.next_viewport_revision = self
            .next_viewport_revision
            .max(self.config.viewport_revision)
            .saturating_add(1);
        let viewport_revision = self.next_viewport_revision;
        let projection = self.effective_projection.clone();
        self.pending_viewport = Some(ChartViewportProposal {
            revision: viewport_revision,
            input_generation: self.viewport_input_generation,
            zoom: self.zoom,
            pan: self.pan,
            projection: projection.clone(),
        });
        self.wheel_generation = self.wheel_generation.saturating_add(1);
        self.wheel_commit_task = None;
        let events = self.events.clone();
        let entity = cx.weak_entity();
        let payload = UiValue::Map(BTreeMap::from([
            ("zoom".to_owned(), UiValue::Float(self.zoom)),
            ("pan_x".to_owned(), UiValue::Float(self.pan.x)),
            ("pan_y".to_owned(), UiValue::Float(self.pan.y)),
            (
                "viewport".to_owned(),
                projection
                    .as_ref()
                    .map_or(UiValue::Null, linked_viewport_value),
            ),
            (
                "viewport_revision".to_owned(),
                UiValue::Integer(i64::try_from(viewport_revision).unwrap_or(i64::MAX)),
            ),
        ]));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("zoom_change", payload, window, cx);
            let _ = entity.update(cx, |chart, _| {
                chart.viewport_commit_in_flight = false;
            });
        });
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(scene) = self.current_scene_sample() else {
            return;
        };
        let interactive = scene
            .marks
            .iter()
            .filter(|mark| {
                mark.interactive
                    && matches!(mark.role, ChartMarkRole::Data | ChartMarkRole::Annotation)
            })
            .map(|mark| mark.key.clone())
            .collect::<Vec<_>>();
        if interactive.is_empty() {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" | "up" => {
                let current = self
                    .focused
                    .as_ref()
                    .and_then(|key| interactive.iter().position(|value| value == key))
                    .unwrap_or(0);
                let next = if current == 0 {
                    interactive.len() - 1
                } else {
                    current - 1
                };
                self.focused = Some(interactive[next].clone());
                cx.stop_propagation();
                cx.notify();
            }
            "right" | "down" => {
                let current = self
                    .focused
                    .as_ref()
                    .and_then(|key| interactive.iter().position(|value| value == key))
                    .unwrap_or(interactive.len() - 1);
                self.focused = Some(interactive[(current + 1) % interactive.len()].clone());
                cx.stop_propagation();
                cx.notify();
            }
            "enter" | "space" => {
                if let Some(mark) = self
                    .focused
                    .as_ref()
                    .and_then(|key| scene.marks.iter().find(|mark| mark.key == *key))
                    .cloned()
                {
                    if mark.role == ChartMarkRole::Annotation {
                        self.emit_annotation(mark, window, cx);
                    } else {
                        self.emit_select(&mark, window, cx);
                    }
                    cx.stop_propagation();
                }
            }
            "home" => {
                self.focused = interactive.first().cloned();
                cx.stop_propagation();
                cx.notify();
            }
            "end" => {
                self.focused = interactive.last().cloned();
                cx.stop_propagation();
                cx.notify();
            }
            _ => {}
        }
    }

    fn emit_select(&self, mark: &ChartMark, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.events.clone();
        let payload = mark_payload(mark, self.scene.as_ref().map_or(0, |scene| scene.revision));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("select", payload, window, cx);
        });
    }

    fn emit_legend(&self, mark: ChartMark, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.events.clone();
        let payload = UiValue::Map(BTreeMap::from([
            ("series_key".to_owned(), UiValue::String(mark.series_key)),
            ("visible".to_owned(), UiValue::Bool(!mark.selected)),
        ]));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("legend_change", payload, window, cx);
        });
    }

    fn emit_annotation(&self, mark: ChartMark, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.events.clone();
        let payload = UiValue::Map(BTreeMap::from([
            ("key".to_owned(), UiValue::String(mark.datum_key)),
            ("label".to_owned(), UiValue::String(mark.label)),
        ]));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("annotation_activate", payload, window, cx);
        });
    }

    fn emit_brush(
        &self,
        start: ChartPoint,
        end: ChartPoint,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(scene) = self.current_scene_sample() else {
            return;
        };
        let mut rect = ChartRect {
            x: start.x.min(end.x),
            y: start.y.min(end.y),
            width: (end.x - start.x).abs(),
            height: (end.y - start.y).abs(),
        };
        match self.config.spec.brush {
            ChartBrushMode::X => {
                rect.y = 0.0;
                rect.height = scene.height;
            }
            ChartBrushMode::Y => {
                rect.x = 0.0;
                rect.width = scene.width;
            }
            ChartBrushMode::None
            | ChartBrushMode::Xy
            | ChartBrushMode::GeoRectangle
            | ChartBrushMode::GeoRegion => {}
        }
        let keys = scene
            .marks
            .iter()
            .filter(|mark| {
                mark.role == ChartMarkRole::Data
                    && mark.interactive
                    && mark_center(mark).is_some_and(|point| rect.contains(point))
            })
            .filter_map(|mark| {
                mark.datum
                    .as_ref()
                    .map(|datum| UiValue::String(datum.key.clone()))
            })
            .collect::<Vec<_>>();
        let data = scene
            .marks
            .iter()
            .filter(|mark| {
                mark.role == ChartMarkRole::Data
                    && mark.interactive
                    && mark_center(mark).is_some_and(|point| rect.contains(point))
            })
            .filter_map(|mark| {
                mark.datum.as_ref().map(|datum| {
                    UiValue::Map(BTreeMap::from([
                        ("dataset".to_owned(), UiValue::String(datum.dataset.clone())),
                        (
                            "series_key".to_owned(),
                            UiValue::String(datum.series.clone()),
                        ),
                        ("datum_key".to_owned(), UiValue::String(datum.key.clone())),
                        (
                            "region_key".to_owned(),
                            UiValue::String(mark.region_key.clone()),
                        ),
                    ]))
                })
            })
            .collect::<Vec<_>>();
        let events = self.events.clone();
        let payload = UiValue::Map(BTreeMap::from([
            ("keys".to_owned(), UiValue::Array(keys)),
            ("data".to_owned(), UiValue::Array(data)),
            (
                "revision".to_owned(),
                UiValue::Integer(i64::try_from(scene.revision).unwrap_or(i64::MAX)),
            ),
            ("x".to_owned(), UiValue::Float(rect.x)),
            ("y".to_owned(), UiValue::Float(rect.y)),
            ("width".to_owned(), UiValue::Float(rect.width)),
            ("height".to_owned(), UiValue::Float(rect.height)),
        ]));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("brush_change", payload, window, cx);
        });
    }

    fn displayed_scene(&mut self, window: &mut Window) -> Option<PreparedChartScene> {
        let scene = self.current_scene_sample()?;
        let complete = self.transition_started.is_none_or(|started| {
            chart_motion_duration(&self.config).is_none_or(|duration| {
                self.config.theme.now().saturating_duration_since(started) >= duration
            })
        });
        if complete {
            self.transition_started = None;
            self.previous_scene = None;
        } else {
            window.request_animation_frame();
        }
        Some(scene)
    }

    fn current_scene_sample(&self) -> Option<PreparedChartScene> {
        let next = self.scene.clone()?;
        let (Some(started), Some(duration), Some(previous)) = (
            self.transition_started,
            chart_motion_duration(&self.config),
            self.previous_scene.as_ref(),
        ) else {
            return Some(next);
        };
        let progress = self
            .config
            .theme
            .now()
            .saturating_duration_since(started)
            .as_secs_f64()
            / duration.as_secs_f64();
        if progress >= 1.0 {
            return Some(next);
        }
        let easing = self
            .config
            .theme
            .motion()
            .easings
            .get(&self.config.spec.motion.easing_role)
            .copied()
            .unwrap_or(crate::MotionEasing::EaseInOut);
        Some(super::interpolate_chart_scene(
            previous,
            &next,
            easing.sample(progress.clamp(0.0, 1.0)),
        ))
    }
}

impl Render for ChartEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scene = self.displayed_scene(window);
        let hovered = self.hovered.clone();
        let focused = self.focused.clone();
        let chart_theme = primitive_chart_theme(&self.config.theme);
        let background = chart_theme.background;
        let crosshair = self.config.spec.tooltip.crosshair;
        let crosshair_color = chart_theme.crosshair;
        let canvas_scene = scene.clone();
        let chart_canvas = canvas(
            |_, _, _| (),
            move |bounds, (), window, _| {
                if let Some(scene) = &canvas_scene {
                    paint_chart_scene(
                        bounds,
                        scene,
                        hovered.as_deref(),
                        focused.as_deref(),
                        1.0,
                        ChartPoint::default(),
                        crosshair.then_some(crosshair_color),
                        window,
                    );
                }
            },
        )
        .size_full();
        let mut root = div()
            .id("gpui-rhai-chart")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgba(background.as_rgba_hex()))
            .track_focus(&self.focus)
            .tab_stop(true)
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_scroll_wheel(cx.listener(Self::wheel))
            .child(chart_canvas);
        if let Some(scene) = &scene {
            for label in scene.labels.iter() {
                let position = label.position;
                let typography = match label.role {
                    super::ChartLabelRole::Title => &chart_theme.title_typography,
                    super::ChartLabelRole::Label => &chart_theme.label_typography,
                };
                let font_size = typography_length_pixels(typography.size, 12.0);
                let character_count = u32::try_from(label.text.chars().count()).unwrap_or(u32::MAX);
                let label_width =
                    (f64::from(character_count) * font_size * 0.65 + 8.0).clamp(80.0, 320.0);
                let element = apply_chart_typography(
                    div()
                        .absolute()
                        .top(px(f64_to_f32(position.y)))
                        .whitespace_nowrap()
                        .text_color(rgba(label.color.as_rgba_hex())),
                    typography,
                );
                let element = match label.anchor {
                    super::ChartLabelAnchor::Start => element
                        .left(px(f64_to_f32(position.x)))
                        .right(px(0.0))
                        .text_left(),
                    super::ChartLabelAnchor::Center => element
                        .left(px(f64_to_f32(position.x - label_width / 2.0)))
                        .w(px(f64_to_f32(label_width)))
                        .text_center(),
                    super::ChartLabelAnchor::End => element
                        .left(px(0.0))
                        .w(px(f64_to_f32(position.x.max(0.0))))
                        .text_right(),
                };
                root = root.child(element.child(label.text.clone()));
            }
            if self.config.spec.tooltip.visible
                && let Some(mark) = self
                    .hovered
                    .as_deref()
                    .and_then(|key| scene.marks.iter().find(|mark| mark.key == key))
                && let Some(position) = mark_center(mark)
            {
                let chart_theme = primitive_chart_theme(&self.config.theme);
                let tooltip_x = if chart_theme.direction == crate::TextDirection::RightToLeft {
                    position.x - 172.0
                } else {
                    position.x + 12.0
                };
                let tooltip_text = if self.config.spec.tooltip.shared {
                    scene
                        .marks
                        .iter()
                        .filter(|candidate| {
                            candidate.role == ChartMarkRole::Data
                                && candidate.region_key == mark.region_key
                                && mark_center(candidate)
                                    .is_some_and(|center| (center.x - position.x).abs() <= 1.0)
                        })
                        .map(|candidate| match candidate.value {
                            Some(value) => format!(
                                "{}: {}",
                                candidate.label,
                                format_tooltip_value(value, &chart_theme)
                            ),
                            None => candidate.label.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    match mark.value {
                        Some(value) => format!(
                            "{}: {}",
                            mark.label,
                            format_tooltip_value(value, &chart_theme)
                        ),
                        None => mark.label.clone(),
                    }
                };
                root = root.child(apply_chart_typography(
                    div()
                        .absolute()
                        .left(px(f64_to_f32(tooltip_x.max(4.0))))
                        .top(px(f64_to_f32(position.y + 12.0)))
                        .p(px(8.0))
                        .border_1()
                        .border_color(rgba(chart_theme.axis.as_rgba_hex()))
                        .bg(rgba(chart_theme.tooltip_surface.as_rgba_hex()))
                        .text_color(rgba(chart_theme.tooltip_text.as_rgba_hex()))
                        .child(tooltip_text),
                    &chart_theme.label_typography,
                ));
            }
        }
        if let Some((start, end)) = self.brush {
            let selection = primitive_chart_theme(&self.config.theme).selection;
            root = root.child(
                div()
                    .absolute()
                    .left(px(f64_to_f32(start.x.min(end.x))))
                    .top(px(f64_to_f32(start.y.min(end.y))))
                    .w(px(f64_to_f32((end.x - start.x).abs())))
                    .h(px(f64_to_f32((end.y - start.y).abs())))
                    .border_1()
                    .border_color(rgba(selection.as_rgba_hex()))
                    .bg(rgba(with_alpha(selection, 0x22).as_rgba_hex())),
            );
        }
        if let Some(error) = &self.error {
            root = root.child(
                div()
                    .absolute()
                    .left(px(12.0))
                    .right(px(12.0))
                    .bottom(px(12.0))
                    .p(px(8.0))
                    .text_color(rgba(
                        self.config
                            .theme
                            .color("danger")
                            .unwrap_or(crate::Rgba8::from_rgb_hex(0x00ff_4455))
                            .as_rgba_hex(),
                    ))
                    .child(error.clone()),
            );
        }
        root.child(ChartBoundsRecorder { chart: cx.entity() })
    }
}

struct ChartBoundsRecorder {
    chart: Entity<ChartEntity>,
}

impl Element for ChartBoundsRecorder {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = div().absolute().inset_0().into_any_element();
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
        self.chart
            .update(cx, |chart, cx| chart.set_bounds(bounds, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for ChartBoundsRecorder {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct ChartPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<ChartEntity>>,
    sources: BTreeMap<PrimitiveInstanceId, ChartSourceCache>,
    transforms: ChartTransformRegistry,
    geo: ChartGeoRegistry,
    custom_series: ChartSeriesRegistry,
    formatters: ChartFormatterRegistry,
    links: ChartLinkRegistry,
}

impl ChartPrimitiveHandler {
    #[must_use]
    pub fn new(
        transforms: ChartTransformRegistry,
        geo: ChartGeoRegistry,
        custom_series: ChartSeriesRegistry,
        formatters: ChartFormatterRegistry,
    ) -> Self {
        Self {
            instances: BTreeMap::new(),
            sources: BTreeMap::new(),
            transforms,
            geo,
            custom_series,
            formatters,
            links: ChartLinkRegistry::default(),
        }
    }
}

impl PrimitiveHandler for ChartPrimitiveHandler {
    fn effect_cost(&self, instance: &PrimitiveInstance) -> usize {
        match instance.node.props.get("data") {
            Some(PrimitiveValue::Data(UiValue::Array(rows))) => rows.len(),
            Some(PrimitiveValue::ChartData(data)) => data
                .snapshot()
                .datasets()
                .map(|(_, dataset)| dataset.len())
                .fold(0_usize, usize::saturating_add),
            _ => 0,
        }
    }

    fn accessibility(
        &self,
        instance: &PrimitiveInstanceId,
        cx: &App,
    ) -> Option<crate::PrimitiveAccessibilityProjection> {
        let chart = self.instances.get(instance)?.read(cx);
        let scene = chart.scene.as_ref()?;
        let presented_key = chart.presented_key?;
        let active = chart.focused.as_ref().and_then(|key| {
            scene
                .marks
                .iter()
                .find(|mark| mark.key == *key)
                .and_then(|mark| mark.datum.as_ref())
                .and_then(|datum| {
                    scene.semantics.iter().find(|semantic| {
                        semantic.series_key == datum.series && semantic.datum_key == datum.key
                    })
                })
        });
        let mut important = scene
            .semantics
            .iter()
            .filter(|datum| datum.selected)
            .take(16)
            .collect::<Vec<_>>();
        if let Some(active) = active
            && !important.contains(&active)
        {
            important.insert(0, active);
        }
        let details = important
            .iter()
            .map(|datum| match datum.value {
                Some(value) => format!("{} {}={value}", datum.series_key, datum.name),
                None => format!("{} {}", datum.series_key, datum.name),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let description = if details.is_empty() {
            scene.summary.clone()
        } else {
            format!("{}; active or selected data: {details}", scene.summary)
        };
        Some(crate::PrimitiveAccessibilityProjection {
            description,
            value: Some(UiValue::Map(BTreeMap::from([
                (
                    "revision".to_owned(),
                    UiValue::Integer(i64::try_from(scene.revision).unwrap_or(i64::MAX)),
                ),
                (
                    "source_epoch".to_owned(),
                    UiValue::Integer(
                        i64::try_from(presented_key.data.source_epoch).unwrap_or(i64::MAX),
                    ),
                ),
                (
                    "frame_epoch".to_owned(),
                    UiValue::Integer(i64::try_from(presented_key.frame_epoch).unwrap_or(i64::MAX)),
                ),
                (
                    "mark_count".to_owned(),
                    UiValue::Integer(i64::try_from(scene.marks.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "semantic_count".to_owned(),
                    UiValue::Integer(i64::try_from(scene.semantics.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "active".to_owned(),
                    active.map_or(UiValue::Null, |datum| {
                        UiValue::Map(BTreeMap::from([
                            (
                                "series_key".to_owned(),
                                UiValue::String(datum.series_key.clone()),
                            ),
                            (
                                "datum_key".to_owned(),
                                UiValue::String(datum.datum_key.clone()),
                            ),
                            ("name".to_owned(), UiValue::String(datum.name.clone())),
                            (
                                "value".to_owned(),
                                datum.value.map_or(UiValue::Null, UiValue::Float),
                            ),
                        ]))
                    }),
                ),
            ]))),
        })
    }

    fn suspend(&mut self, instance: &PrimitiveInstanceId, cx: &mut App) {
        if let Some(entity) = self.instances.get(instance) {
            entity.update(cx, ChartEntity::suspend);
        }
    }

    fn resume(&mut self, instance: &PrimitiveInstanceId, cx: &mut App) {
        if let Some(entity) = self.instances.get(instance) {
            entity.update(cx, ChartEntity::resume);
        }
    }

    fn commit_resume(&mut self, instance: &PrimitiveInstanceId, cx: &mut App) {
        if let Some(entity) = self.instances.get(instance) {
            entity.update(cx, ChartEntity::commit_resume);
        }
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        _: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "ChartPrimitive requires a stable key".to_owned())?;
        let existing = self.instances.get(&id).cloned();
        let parsed = match parse_config(&instance.node.props, theme, self.sources.get(&id)) {
            Ok(config) => config,
            Err(error) => {
                if let Some(entity) = existing {
                    entity.update(cx, |chart, cx| {
                        chart.error = Some(error);
                        cx.notify();
                    });
                    return Ok(entity.into_any_element());
                }
                return Err(error);
            }
        };
        let (config, source) = parsed;
        if let Some(source) = source {
            self.sources.insert(id.clone(), source);
        }
        let Some(entity) = existing else {
            let entity = cx.new(|cx| {
                ChartEntity::new(
                    config.clone(),
                    events.clone(),
                    self.transforms.clone(),
                    self.geo.clone(),
                    self.custom_series.clone(),
                    self.formatters.clone(),
                    self.links.clone(),
                    cx,
                )
            });
            entity.update(cx, ChartEntity::start);
            self.instances.insert(id, entity.clone());
            let link = link_key(&config.spec);
            let weak = entity.downgrade();
            self.links.register(link.clone(), &weak);
            if let Some(projection) = self.links.linked_viewport(link.as_ref(), &weak) {
                entity.update(cx, |chart, _| {
                    if chart.apply_linked_projection(Some(projection)) {
                        chart.invalidate_frame();
                    }
                });
            }
            if let Some(link) = link {
                self.links
                    .broadcast_selection_set(&link, &weak, &config.selected, cx);
            }
            return Ok(entity.into_any_element());
        };
        let link = link_key(&config.spec);
        let weak = entity.downgrade();
        self.links.register(link.clone(), &weak);
        let linked_selected = self.links.linked_selection(link.as_ref(), &weak);
        let linked_projection = self.links.linked_viewport(link.as_ref(), &weak);
        entity.update(cx, |chart, cx| {
            chart.update_config(
                config,
                events.clone(),
                linked_selected,
                linked_projection,
                cx,
            );
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.sources.remove(instance);
        if let Some(entity) = self.instances.remove(instance) {
            self.links.unregister(&entity.downgrade());
        }
    }
}

fn parse_config(
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
    cached: Option<&ChartSourceCache>,
) -> Result<(ChartConfig, Option<ChartSourceCache>), String> {
    if let Some(cached) = cached.filter(|cached| cached.matches(props)) {
        return Ok((chart_config_from_source(cached, props, theme)?, None));
    }
    let source = parse_chart_source(props)?;
    let config = chart_config_from_source(&source, props, theme)?;
    Ok((config, Some(source)))
}

fn chart_config_from_source(
    source: &ChartSourceCache,
    props: &PrimitiveProps,
    theme: &PrimitiveTheme,
) -> Result<ChartConfig, String> {
    let hidden = string_set_prop(props, "hidden_series");
    let mut spec = source.spec.clone();
    for series in &mut spec.series {
        if hidden.contains(&series.key) {
            series.visible = false;
        }
    }
    Ok(ChartConfig {
        spec,
        data: source.data.clone(),
        selected: string_set_prop(props, "selected_keys"),
        zoom: number_prop(props, "zoom").unwrap_or(1.0),
        pan: ChartPoint {
            x: number_prop(props, "pan_x").unwrap_or(0.0),
            y: number_prop(props, "pan_y").unwrap_or(0.0),
        },
        viewport: viewport_prop(props)?,
        viewport_revision: integer_prop(props, "viewport_revision").unwrap_or(0),
        theme: theme.clone(),
    })
}

fn parse_chart_source(props: &PrimitiveProps) -> Result<ChartSourceCache, String> {
    let spec_prop = props
        .get("spec")
        .cloned()
        .ok_or_else(|| "chart spec is required".to_owned())?;
    let spec = match &spec_prop {
        PrimitiveValue::Data(value) => {
            ChartSpec::from_ui_value(value).map_err(|error| error.to_string())?
        }
        _ => return Err("chart spec must be durable data".to_owned()),
    };
    let key_dimension = data_string_prop(props, "key_dimension");
    let data_prop = props
        .get("data")
        .cloned()
        .ok_or_else(|| "chart data is required".to_owned())?;
    let data = match &data_prop {
        PrimitiveValue::ChartData(data) => ChartDataInput::Native(data.clone()),
        PrimitiveValue::Data(UiValue::Array(rows)) => {
            let rows = rows
                .iter()
                .enumerate()
                .map(|(index, row)| match row {
                    UiValue::Map(row) => Ok(row.clone()),
                    _ => Err(format!("chart inline row {index} must be an object")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if rows.len() > 10_000 {
                return Err(
                    "inline chart data is limited to 10000 rows; use NativeChartData".to_owned(),
                );
            }
            let dataset = ChartDataset::from_rows(
                "main",
                &rows,
                key_dimension.clone(),
                ChartDataLimits::default(),
            )
            .map_err(|error| error.to_string())?;
            let data = NativeChartData::new([dataset], ChartDataLimits::default())
                .map_err(|error| error.to_string())?;
            ChartDataInput::Inline(data.snapshot())
        }
        _ => return Err("chart data must be an array of objects or NativeChartData".to_owned()),
    };
    Ok(ChartSourceCache {
        spec_prop,
        data_prop,
        key_dimension,
        spec,
        data,
    })
}

fn link_key(spec: &ChartSpec) -> Option<(String, String)> {
    spec.link_group.clone().zip(spec.link_domain.clone())
}

fn linked_pan(domain: ChartAxisDomain, visible: (f64, f64), range: f64, zoom: f64) -> f64 {
    let Some(full) = axis_domain_space(domain.full, domain.scale) else {
        return 0.0;
    };
    let Some(visible) = axis_domain_space(visible, domain.scale) else {
        return 0.0;
    };
    let full_span = full.1 - full.0;
    if full_span <= f64::EPSILON {
        return 0.0;
    }
    let full_center = f64::midpoint(full.0, full.1);
    let visible_center = f64::midpoint(visible.0, visible.1);
    let pan = (full_center - visible_center) * range * zoom / full_span;
    if domain.direction == ChartAxisDirection::Reversed {
        -pan
    } else {
        pan
    }
}

fn viewport_matches_proposal(config: &ChartConfig, proposal: &ChartViewportProposal) -> bool {
    if config.viewport.is_some() {
        return config.viewport == proposal.projection;
    }
    (config.zoom - proposal.zoom).abs() <= 0.000_001
        && (config.pan.x - proposal.pan.x).abs() <= 0.000_001
        && (config.pan.y - proposal.pan.y).abs() <= 0.000_001
}

fn zoom_axis_window(visible: (f64, f64), scale: ChartAxisScale, factor: f64) -> (f64, f64) {
    let Some(visible_space) = axis_domain_space(visible, scale) else {
        return visible;
    };
    let center = f64::midpoint(visible_space.0, visible_space.1);
    let half = (visible_space.1 - visible_space.0) / factor / 2.0;
    let next = (center - half, center + half);
    if scale == ChartAxisScale::Log {
        (next.0.exp(), next.1.exp())
    } else {
        next
    }
}

fn pan_axis_window(
    visible: (f64, f64),
    domain: ChartAxisDomain,
    pixels: f64,
    range: f64,
) -> (f64, f64) {
    let Some(visible_space) = axis_domain_space(visible, domain.scale) else {
        return visible;
    };
    if range.abs() <= f64::EPSILON {
        return visible;
    }
    let pixels = if domain.direction == ChartAxisDirection::Reversed {
        -pixels
    } else {
        pixels
    };
    let shift = -pixels * (visible_space.1 - visible_space.0) / range;
    let next = (visible_space.0 + shift, visible_space.1 + shift);
    if domain.scale == ChartAxisScale::Log {
        (next.0.exp(), next.1.exp())
    } else {
        next
    }
}

fn rebase_axis_window(base: (f64, f64), accepted: (f64, f64), current: (f64, f64)) -> (f64, f64) {
    let accepted_span = accepted.1 - accepted.0;
    if accepted_span.abs() <= f64::EPSILON {
        return current;
    }
    let base_span = base.1 - base.0;
    let span_ratio = (current.1 - current.0) / accepted_span;
    let center_offset = (f64::midpoint(current.0, current.1)
        - f64::midpoint(accepted.0, accepted.1))
        / accepted_span;
    let center = f64::midpoint(base.0, base.1) + center_offset * base_span;
    let half = base_span * span_ratio / 2.0;
    (center - half, center + half)
}

fn rebase_linked_axis(
    base: Option<ChartLinkedAxis>,
    accepted: Option<ChartLinkedAxis>,
    current: Option<ChartLinkedAxis>,
) -> Option<ChartLinkedAxis> {
    let current = current?;
    let accepted = accepted.filter(|axis| axis.key == current.key)?;
    let base = base.filter(|axis| axis.key == current.key)?;
    Some(ChartLinkedAxis {
        key: current.key,
        visible: rebase_axis_window(base.visible, accepted.visible, current.visible),
    })
}

fn rebase_viewport_projection(
    base: ChartLinkedViewport,
    accepted: Option<ChartLinkedViewport>,
    current: Option<ChartLinkedViewport>,
) -> Option<ChartLinkedViewport> {
    match (base, accepted?, current?) {
        (
            ChartLinkedViewport::Cartesian {
                region: base_region,
                x: base_x,
                y: base_y,
            },
            ChartLinkedViewport::Cartesian {
                region: accepted_region,
                x: accepted_x,
                y: accepted_y,
            },
            ChartLinkedViewport::Cartesian {
                region: current_region,
                x: current_x,
                y: current_y,
            },
        ) if base_region == accepted_region && accepted_region == current_region => {
            Some(ChartLinkedViewport::Cartesian {
                region: current_region,
                x: rebase_linked_axis(base_x, accepted_x, current_x),
                y: rebase_linked_axis(base_y, accepted_y, current_y),
            })
        }
        (
            ChartLinkedViewport::Geo {
                region: base_region,
                map: base_map,
                projection: base_projection,
                zoom: base_zoom,
                normalized_pan: base_pan,
            },
            ChartLinkedViewport::Geo {
                region: accepted_region,
                map: accepted_map,
                projection: accepted_projection,
                zoom: accepted_zoom,
                normalized_pan: accepted_pan,
            },
            ChartLinkedViewport::Geo {
                region: current_region,
                map: current_map,
                projection: current_projection,
                zoom: current_zoom,
                normalized_pan: current_pan,
            },
        ) if (
            base_region.as_str(),
            base_map.as_str(),
            base_projection.as_str(),
        ) == (
            accepted_region.as_str(),
            accepted_map.as_str(),
            accepted_projection.as_str(),
        ) && (
            accepted_region.as_str(),
            accepted_map.as_str(),
            accepted_projection.as_str(),
        ) == (
            current_region.as_str(),
            current_map.as_str(),
            current_projection.as_str(),
        ) =>
        {
            Some(ChartLinkedViewport::Geo {
                region: current_region,
                map: current_map,
                projection: current_projection,
                zoom: if accepted_zoom.abs() <= f64::EPSILON {
                    current_zoom
                } else {
                    (base_zoom * current_zoom / accepted_zoom).clamp(0.5, 20.0)
                },
                normalized_pan: ChartPoint {
                    x: base_pan.x + current_pan.x - accepted_pan.x,
                    y: base_pan.y + current_pan.y - accepted_pan.y,
                },
            })
        }
        (_, _, current) => Some(current),
    }
}

fn axis_domain_space(domain: (f64, f64), scale: ChartAxisScale) -> Option<(f64, f64)> {
    if scale == ChartAxisScale::Log {
        (domain.0 > 0.0 && domain.1 > 0.0).then(|| (domain.0.ln(), domain.1.ln()))
    } else {
        Some(domain)
    }
}

fn linked_visible_domain(
    domain: ChartAxisDomain,
    zoom: f64,
    pan: f64,
    range: f64,
) -> Option<(f64, f64)> {
    let full = axis_domain_space(domain.full, domain.scale)?;
    let span = full.1 - full.0;
    if span <= f64::EPSILON || range.abs() <= f64::EPSILON {
        return Some(domain.full);
    }
    let visible_span = span / zoom;
    let pan = if domain.direction == ChartAxisDirection::Reversed {
        -pan
    } else {
        pan
    };
    let center = f64::midpoint(full.0, full.1) - pan * span / (range * zoom);
    let visible = (center - visible_span / 2.0, center + visible_span / 2.0);
    if domain.scale == ChartAxisScale::Log {
        Some((visible.0.exp(), visible.1.exp()))
    } else {
        Some(visible)
    }
}

fn viewport_string(map: &BTreeMap<String, UiValue>, name: &str) -> Result<String, String> {
    match map.get(name) {
        Some(UiValue::String(value)) if !value.is_empty() => Ok(value.clone()),
        _ => Err(format!(
            "chart viewport `{name}` must be a non-empty string"
        )),
    }
}

fn viewport_number(map: &BTreeMap<String, UiValue>, name: &str) -> Result<f64, String> {
    let value = match map.get(name) {
        Some(UiValue::Float(value)) => *value,
        Some(UiValue::Integer(value)) => value
            .to_string()
            .parse::<f64>()
            .map_err(|_| format!("chart viewport `{name}` is outside numeric range"))?,
        _ => return Err(format!("chart viewport `{name}` must be a number")),
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| format!("chart viewport `{name}` must be finite"))
}

fn viewport_axis(value: Option<&UiValue>, name: &str) -> Result<Option<ChartLinkedAxis>, String> {
    let Some(value) = value else { return Ok(None) };
    let UiValue::Map(map) = value else {
        return Err(format!("chart viewport `{name}` must be an object"));
    };
    let min = viewport_number(map, "min")?;
    let max = viewport_number(map, "max")?;
    if min >= max {
        return Err(format!("chart viewport `{name}` requires min < max"));
    }
    Ok(Some(ChartLinkedAxis {
        key: viewport_string(map, "key")?,
        visible: (min, max),
    }))
}

fn viewport_prop(props: &PrimitiveProps) -> Result<Option<ChartLinkedViewport>, String> {
    let Some(value) = props.get("viewport") else {
        return Ok(None);
    };
    let PrimitiveValue::Data(value) = value else {
        return Err("chart viewport must be durable data".to_owned());
    };
    if matches!(value, UiValue::Null) {
        return Ok(None);
    }
    let UiValue::Map(map) = value else {
        return Err("chart viewport must be an object".to_owned());
    };
    match viewport_string(map, "kind")?.as_str() {
        "cartesian" => {
            let x = viewport_axis(map.get("x"), "x")?;
            let y = viewport_axis(map.get("y"), "y")?;
            if x.is_none() && y.is_none() {
                return Err("cartesian chart viewport requires x or y".to_owned());
            }
            Ok(Some(ChartLinkedViewport::Cartesian {
                region: viewport_string(map, "region")?,
                x,
                y,
            }))
        }
        "geo" => {
            let zoom = viewport_number(map, "zoom")?;
            if !(0.5..=20.0).contains(&zoom) {
                return Err("geo chart viewport zoom must be between 0.5 and 20".to_owned());
            }
            Ok(Some(ChartLinkedViewport::Geo {
                region: viewport_string(map, "region")?,
                map: viewport_string(map, "map")?,
                projection: viewport_string(map, "projection")?,
                zoom,
                normalized_pan: ChartPoint {
                    x: viewport_number(map, "pan_x")?,
                    y: viewport_number(map, "pan_y")?,
                },
            }))
        }
        kind => Err(format!("unknown chart viewport kind `{kind}`")),
    }
}

fn linked_axis_value(axis: &ChartLinkedAxis) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("key".to_owned(), UiValue::String(axis.key.clone())),
        ("min".to_owned(), UiValue::Float(axis.visible.0)),
        ("max".to_owned(), UiValue::Float(axis.visible.1)),
    ]))
}

fn linked_viewport_value(viewport: &ChartLinkedViewport) -> UiValue {
    match viewport {
        ChartLinkedViewport::Cartesian { region, x, y } => {
            let mut value = BTreeMap::from([
                ("kind".to_owned(), UiValue::String("cartesian".to_owned())),
                ("region".to_owned(), UiValue::String(region.clone())),
            ]);
            if let Some(axis) = x {
                value.insert("x".to_owned(), linked_axis_value(axis));
            }
            if let Some(axis) = y {
                value.insert("y".to_owned(), linked_axis_value(axis));
            }
            UiValue::Map(value)
        }
        ChartLinkedViewport::Geo {
            region,
            map,
            projection,
            zoom,
            normalized_pan,
        } => UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("geo".to_owned())),
            ("region".to_owned(), UiValue::String(region.clone())),
            ("map".to_owned(), UiValue::String(map.clone())),
            ("projection".to_owned(), UiValue::String(projection.clone())),
            ("zoom".to_owned(), UiValue::Float(*zoom)),
            ("pan_x".to_owned(), UiValue::Float(normalized_pan.x)),
            ("pan_y".to_owned(), UiValue::Float(normalized_pan.y)),
        ])),
        ChartLinkedViewport::Unsupported => UiValue::Null,
    }
}

fn data_string_prop(props: &PrimitiveProps, name: &str) -> Option<String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::String(value))) => Some(value.clone()),
        _ => None,
    }
}

fn number_prop(props: &PrimitiveProps, name: &str) -> Option<f64> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Float(value))) => Some(*value),
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn integer_prop(props: &PrimitiveProps, name: &str) -> Option<u64> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => u64::try_from(*value).ok(),
        _ => None,
    }
}

fn string_set_prop(props: &PrimitiveProps, name: &str) -> BTreeSet<String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Array(values))) => values
            .iter()
            .filter_map(|value| match value {
                UiValue::String(value) => Some(value.clone()),
                _ => None,
            })
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn primitive_chart_theme(theme: &PrimitiveTheme) -> ChartTheme {
    let fallback = ChartTheme::default();
    let palette = (1..=8)
        .map(|index| {
            theme
                .color(&format!("charts.palette_{index}"))
                .unwrap_or(fallback.palette[index - 1])
        })
        .collect::<Vec<_>>();
    ChartTheme {
        background: theme.color("surface").unwrap_or(fallback.background),
        text: theme.color("text_primary").unwrap_or(fallback.text),
        muted_text: theme.color("text_muted").unwrap_or(fallback.muted_text),
        axis: theme.color("charts.axis").unwrap_or(fallback.axis),
        grid: theme.color("charts.grid").unwrap_or(fallback.grid),
        positive: theme.color("charts.positive").unwrap_or(fallback.positive),
        negative: theme.color("charts.negative").unwrap_or(fallback.negative),
        selection: theme
            .color("charts.selection")
            .unwrap_or(fallback.selection),
        map_missing: theme
            .color("charts.map_missing")
            .unwrap_or(fallback.map_missing),
        crosshair: theme
            .color("charts.crosshair")
            .unwrap_or(fallback.crosshair),
        tooltip_surface: theme
            .color("charts.tooltip_surface")
            .unwrap_or(fallback.tooltip_surface),
        tooltip_text: theme
            .color("charts.tooltip_text")
            .unwrap_or(fallback.tooltip_text),
        palette: palette.into(),
        locale: theme.locale().to_owned(),
        number: theme.number_metadata().cloned(),
        motion_quality: theme.motion_quality(),
        direction: theme.direction(),
        title_typography: theme
            .typography("title")
            .unwrap_or_else(|| fallback.title_typography.clone()),
        label_typography: theme
            .typography("body_small")
            .unwrap_or_else(|| fallback.label_typography.clone()),
    }
}

fn chart_motion_duration(config: &ChartConfig) -> Option<Duration> {
    if !config.spec.motion.enabled
        || config.theme.motion_preference() == crate::MotionPreference::None
    {
        return None;
    }
    let millis = config
        .theme
        .motion()
        .durations_ms
        .get(&config.spec.motion.duration_role)
        .copied()
        .unwrap_or(180);
    let millis = if config.theme.motion_preference() == crate::MotionPreference::Reduced {
        millis.min(90)
    } else {
        millis
    };
    Some(Duration::from_millis(millis))
}

fn mark_payload(mark: &ChartMark, revision: u64) -> UiValue {
    let dataset = mark
        .datum
        .as_ref()
        .map_or_else(String::new, |datum| datum.dataset.clone());
    let datum_key = mark
        .datum
        .as_ref()
        .map_or_else(|| mark.datum_key.clone(), |datum| datum.key.clone());
    UiValue::Map(BTreeMap::from([
        ("dataset".to_owned(), UiValue::String(dataset)),
        (
            "region_key".to_owned(),
            UiValue::String(mark.region_key.clone()),
        ),
        (
            "revision".to_owned(),
            UiValue::Integer(i64::try_from(revision).unwrap_or(i64::MAX)),
        ),
        (
            "series_key".to_owned(),
            UiValue::String(mark.series_key.clone()),
        ),
        ("datum_key".to_owned(), UiValue::String(datum_key)),
        ("name".to_owned(), UiValue::String(mark.label.clone())),
        (
            "value".to_owned(),
            mark.value.map_or(UiValue::Null, UiValue::Float),
        ),
    ]))
}

fn mark_center(mark: &ChartMark) -> Option<ChartPoint> {
    match &mark.geometry {
        ChartMarkGeometry::Rect(rect) => Some(rect.center()),
        ChartMarkGeometry::Circle { center, .. } => Some(*center),
        ChartMarkGeometry::Polyline { points, .. } | ChartMarkGeometry::Polygon(points) => {
            let count = points.len();
            (count > 0).then(|| ChartPoint {
                x: points.iter().map(|point| point.x).sum::<f64>() / usize_to_f64(count),
                y: points.iter().map(|point| point.y).sum::<f64>() / usize_to_f64(count),
            })
        }
        ChartMarkGeometry::CompoundPolygon(rings) => rings.first().and_then(|points| {
            let count = points.len();
            (count > 0).then(|| ChartPoint {
                x: points.iter().map(|point| point.x).sum::<f64>() / usize_to_f64(count),
                y: points.iter().map(|point| point.y).sum::<f64>() / usize_to_f64(count),
            })
        }),
    }
}

fn mark_is_viewport_fixed(mark: &ChartMark) -> bool {
    matches!(
        mark.role,
        ChartMarkRole::Axis | ChartMarkRole::Grid | ChartMarkRole::Legend
    )
}

fn paint_chart_scene(
    bounds: Bounds<Pixels>,
    scene: &PreparedChartScene,
    hovered: Option<&str>,
    focused: Option<&str>,
    zoom: f64,
    pan: ChartPoint,
    crosshair: Option<crate::Rgba8>,
    window: &mut Window,
) {
    let center = ChartPoint {
        x: f64::from(bounds.size.width) / 2.0,
        y: f64::from(bounds.size.height) / 2.0,
    };
    for mark in scene.marks.iter() {
        let transformed = !mark_is_viewport_fixed(mark);
        let scale = if transformed { zoom } else { 1.0 };
        let map = |point| map_chart_point(bounds, center, zoom, pan, point, transformed);
        let emphasized = hovered == Some(mark.key.as_str()) || focused == Some(mark.key.as_str());
        let fill_color = mark.fill.map(|color| {
            if emphasized {
                lighten(color, 0.18)
            } else {
                color
            }
        });
        let clip = matches!(mark.role, ChartMarkRole::Data | ChartMarkRole::Decoration)
            .then(|| scene.plot_regions.get(&mark.region_key))
            .flatten()
            .map(|region| ContentMask {
                bounds: Bounds::new(
                    point(
                        bounds.origin.x + px(f64_to_f32(region.x)),
                        bounds.origin.y + px(f64_to_f32(region.y)),
                    ),
                    size(px(f64_to_f32(region.width)), px(f64_to_f32(region.height))),
                ),
            });
        window.with_content_mask(clip, |window| match &mark.geometry {
            ChartMarkGeometry::Rect(rect) => {
                let origin = map(ChartPoint {
                    x: rect.x,
                    y: rect.y,
                });
                let chart_size = size(
                    px(f64_to_f32(rect.width * scale)),
                    px(f64_to_f32(rect.height * scale)),
                );
                if let Some(color) = fill_color {
                    window.paint_quad(fill(
                        Bounds::new(origin, chart_size),
                        rgba(color.as_rgba_hex()),
                    ));
                }
                let painted_bounds = Bounds::new(origin, chart_size);
                paint_rect_stroke(painted_bounds, mark.stroke, window);
            }
            ChartMarkGeometry::Circle { center, radius } => {
                let center = map(*center);
                let radius = px(f64_to_f32(radius * scale));
                paint_circle(center, radius, fill_color, mark.stroke, window);
            }
            ChartMarkGeometry::Polyline { points, width } => {
                if points.len() < 2 {
                    return;
                }
                if let Some((color, _)) = mark.stroke {
                    paint_polyline(points, *width, scale, color, &map, window);
                }
            }
            ChartMarkGeometry::Polygon(points) => {
                if points.len() < 3 {
                    return;
                }
                if let Some(color) = fill_color {
                    let mut path = gpui::PathBuilder::fill();
                    path.move_to(map(points[0]));
                    for point_value in &points[1..] {
                        path.line_to(map(*point_value));
                    }
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
                if let Some((color, width)) = mark.stroke {
                    let mut path = gpui::PathBuilder::stroke(px(f64_to_f32(width * scale)));
                    path.move_to(map(points[0]));
                    for point_value in &points[1..] {
                        path.line_to(map(*point_value));
                    }
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
            }
            ChartMarkGeometry::CompoundPolygon(rings) => {
                if rings.is_empty() {
                    return;
                }
                if let Some(color) = fill_color {
                    let mut path = gpui::PathBuilder::fill();
                    for points in rings.iter() {
                        if points.len() < 3 {
                            continue;
                        }
                        path.move_to(map(points[0]));
                        for point_value in &points[1..] {
                            path.line_to(map(*point_value));
                        }
                        path.close();
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, rgba(color.as_rgba_hex()));
                    }
                }
                if let Some((color, width)) = mark.stroke {
                    for points in rings.iter() {
                        if points.len() < 3 {
                            continue;
                        }
                        let mut path = gpui::PathBuilder::stroke(px(f64_to_f32(width * scale)));
                        path.move_to(map(points[0]));
                        for point_value in &points[1..] {
                            path.line_to(map(*point_value));
                        }
                        path.close();
                        if let Ok(path) = path.build() {
                            window.paint_path(path, rgba(color.as_rgba_hex()));
                        }
                    }
                }
            }
        });
    }
    if let Some(color) = crosshair
        && let Some(mark) = hovered.and_then(|key| scene.marks.iter().find(|mark| mark.key == key))
        && let Some(mark_center) = mark_center(mark)
        && let Some(region) = scene
            .plot_regions
            .values()
            .find(|region| region.contains(mark_center))
    {
        let presented_center = map_chart_point(bounds, center, zoom, pan, mark_center, true);
        let fixed_origin = map_chart_point(
            bounds,
            center,
            zoom,
            pan,
            ChartPoint {
                x: region.x,
                y: region.y,
            },
            false,
        );
        let fixed_end = map_chart_point(
            bounds,
            center,
            zoom,
            pan,
            ChartPoint {
                x: region.x + region.width,
                y: region.y + region.height,
            },
            false,
        );
        for (start, end) in [
            (
                point(fixed_origin.x, presented_center.y),
                point(fixed_end.x, presented_center.y),
            ),
            (
                point(presented_center.x, fixed_origin.y),
                point(presented_center.x, fixed_end.y),
            ),
        ] {
            let mut path = gpui::PathBuilder::stroke(px(1.0));
            path.move_to(start);
            path.line_to(end);
            if let Ok(path) = path.build() {
                window.paint_path(path, rgba(with_alpha(color, 0x88).as_rgba_hex()));
            }
        }
    }
}

fn map_chart_point(
    bounds: Bounds<Pixels>,
    center: ChartPoint,
    zoom: f64,
    pan: ChartPoint,
    point_value: ChartPoint,
    transformed: bool,
) -> Point<Pixels> {
    let point_value = if transformed {
        ChartPoint {
            x: center.x + (point_value.x - center.x) * zoom + pan.x,
            y: center.y + (point_value.y - center.y) * zoom + pan.y,
        }
    } else {
        point_value
    };
    point(
        bounds.origin.x + px(f64_to_f32(point_value.x)),
        bounds.origin.y + px(f64_to_f32(point_value.y)),
    )
}

fn paint_circle(
    center: Point<Pixels>,
    radius: Pixels,
    fill_color: Option<crate::Rgba8>,
    stroke: Option<(crate::Rgba8, f64)>,
    window: &mut Window,
) {
    let (border_widths, border_color) = stroke.map_or_else(
        || (gpui::Edges::default(), rgba(0)),
        |(color, width)| {
            (
                gpui::Edges::all(px(f64_to_f32(width))),
                rgba(color.as_rgba_hex()),
            )
        },
    );
    window.paint_quad(gpui::quad(
        Bounds::new(
            point(center.x - radius, center.y - radius),
            size(radius * 2.0, radius * 2.0),
        ),
        gpui::Corners::all(radius),
        fill_color.map_or_else(|| rgba(0), |color| rgba(color.as_rgba_hex())),
        border_widths,
        border_color,
        gpui::BorderStyle::default(),
    ));
}

fn paint_polyline(
    points: &[ChartPoint],
    width: f64,
    zoom: f64,
    color: crate::Rgba8,
    map: &impl Fn(ChartPoint) -> Point<Pixels>,
    window: &mut Window,
) {
    let mut path = gpui::PathBuilder::stroke(px(f64_to_f32(width * zoom)));
    path.move_to(map(points[0]));
    for point in &points[1..] {
        path.line_to(map(*point));
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, rgba(color.as_rgba_hex()));
    }
}

fn paint_rect_stroke(
    bounds: Bounds<Pixels>,
    stroke: Option<(crate::Rgba8, f64)>,
    window: &mut Window,
) {
    let Some((color, width)) = stroke else { return };
    let width = px(f64_to_f32(width));
    for line in [
        Bounds::new(bounds.origin, size(bounds.size.width, width)),
        Bounds::new(
            point(bounds.origin.x, bounds.bottom() - width),
            size(bounds.size.width, width),
        ),
        Bounds::new(bounds.origin, size(width, bounds.size.height)),
        Bounds::new(
            point(bounds.right() - width, bounds.origin.y),
            size(width, bounds.size.height),
        ),
    ] {
        window.paint_quad(fill(line, rgba(color.as_rgba_hex())));
    }
}

fn lighten(color: crate::Rgba8, amount: f64) -> crate::Rgba8 {
    let value = color.as_rgba_hex();
    let channel = |shift: u32| {
        let current = f64::from(((value >> shift) & 0xff) as u8);
        ((current + (255.0 - current) * amount)
            .round()
            .clamp(0.0, 255.0) as u32)
            << shift
    };
    crate::Rgba8::from_rgba_hex(channel(24) | channel(16) | channel(8) | u32::from(value as u8))
}

fn with_alpha(color: crate::Rgba8, alpha: u8) -> crate::Rgba8 {
    crate::Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | u32::from(alpha))
}

fn format_tooltip_value(value: f64, theme: &ChartTheme) -> String {
    theme.number.as_ref().map_or_else(
        || value.to_string(),
        |number| {
            crate::format_number_with_metadata(
                value,
                crate::NumberFormatOptions {
                    min_fraction_digits: 0,
                    max_fraction_digits: 3,
                    grouping: true,
                },
                number,
            )
            .unwrap_or_else(|_| value.to_string())
        },
    )
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}
fn f64_to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(0.0)
}

fn typography_length_pixels(value: crate::Length, fallback: f64) -> f64 {
    match value {
        crate::Length::Pixels(value) => value,
        crate::Length::Rems(value) => value * 16.0,
        crate::Length::Relative(_)
        | crate::Length::ThemeSpacing(_)
        | crate::Length::ThemeRadius(_) => fallback,
    }
}

fn apply_chart_typography(
    mut element: gpui::Div,
    typography: &crate::ResolvedTypography,
) -> gpui::Div {
    element = element
        .text_size(px(f64_to_f32(typography_length_pixels(
            typography.size,
            12.0,
        ))))
        .line_height(px(f64_to_f32(typography_length_pixels(
            typography.line_height,
            16.0,
        ))))
        .font_weight(FontWeight(f32::from(typography.weight)));
    if let Some(family) = &typography.family {
        element = element.font_family(family.clone());
    }
    if !typography.fallbacks.is_empty() {
        element.text_style().font_fallbacks =
            Some(FontFallbacks::from_fonts(typography.fallbacks.clone()));
    }
    element
}

#[must_use]
/// Build the validated native Chart primitive descriptor.
///
/// # Panics
///
/// Panics only if the compile-time primitive ID becomes invalid.
pub fn chart_primitive_descriptor() -> PrimitiveDescriptor {
    let string_array = ValueSchema::Array {
        items: Box::new(ValueSchema::string()),
        max_items: Some(10_000),
    };
    let datum_ref = ValueSchema::Object {
        fields: BTreeMap::from([
            (
                "dataset".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "series_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "datum_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "region_key".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
        ]),
        allow_unknown: false,
    };
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.chart").expect("static primitive ID"),
        export: "ChartPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "spec".to_owned(),
                ObjectField::required(ValueSchema::UiValue),
            ),
            (
                "data".to_owned(),
                ObjectField::required(ValueSchema::OneOf {
                    variants: vec![
                        ValueSchema::Array {
                            items: Box::new(ValueSchema::UiValue),
                            max_items: Some(10_000),
                        },
                        ValueSchema::ChartData,
                    ],
                }),
            ),
            (
                "key_dimension".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
            ),
            (
                "selected_keys".to_owned(),
                ObjectField::optional(string_array.clone())
                    .with_default(UiValue::Array(Vec::new())),
            ),
            (
                "hidden_series".to_owned(),
                ObjectField::optional(string_array).with_default(UiValue::Array(Vec::new())),
            ),
            (
                "zoom".to_owned(),
                ObjectField::optional(ValueSchema::bounded_number(Some(0.5), Some(20.0)))
                    .with_default(UiValue::Float(1.0)),
            ),
            (
                "pan_x".to_owned(),
                ObjectField::optional(ValueSchema::number()).with_default(UiValue::Float(0.0)),
            ),
            (
                "pan_y".to_owned(),
                ObjectField::optional(ValueSchema::number()).with_default(UiValue::Float(0.0)),
            ),
            (
                "viewport".to_owned(),
                ObjectField::optional(ValueSchema::UiValue).with_default(UiValue::Null),
            ),
            (
                "viewport_revision".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(Some(0), None))
                    .with_default(UiValue::Integer(0)),
            ),
            (
                "on_select".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
            (
                "on_zoom_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
            (
                "on_brush_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
            (
                "on_legend_change".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
            (
                "on_annotation_activate".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
        ]),
        events: BTreeMap::from([
            (
                "select".to_owned(),
                EventSchema {
                    payload: ValueSchema::Object {
                        fields: BTreeMap::from([
                            (
                                "dataset".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "region_key".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "revision".to_owned(),
                                ObjectField::required(ValueSchema::bounded_integer(Some(0), None)),
                            ),
                            (
                                "series_key".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "datum_key".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "name".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "value".to_owned(),
                                ObjectField::required(ValueSchema::optional(ValueSchema::number())),
                            ),
                        ]),
                        allow_unknown: false,
                    },
                },
            ),
            (
                "zoom_change".to_owned(),
                EventSchema {
                    payload: ValueSchema::Object {
                        fields: BTreeMap::from([
                            (
                                "zoom".to_owned(),
                                ObjectField::required(ValueSchema::positive_number()),
                            ),
                            (
                                "pan_x".to_owned(),
                                ObjectField::required(ValueSchema::number()),
                            ),
                            (
                                "pan_y".to_owned(),
                                ObjectField::required(ValueSchema::number()),
                            ),
                            (
                                "viewport".to_owned(),
                                ObjectField::required(ValueSchema::UiValue),
                            ),
                            (
                                "viewport_revision".to_owned(),
                                ObjectField::required(ValueSchema::bounded_integer(Some(0), None)),
                            ),
                        ]),
                        allow_unknown: false,
                    },
                },
            ),
            (
                "brush_change".to_owned(),
                EventSchema {
                    payload: ValueSchema::Object {
                        fields: BTreeMap::from([
                            (
                                "keys".to_owned(),
                                ObjectField::required(ValueSchema::Array {
                                    items: Box::new(ValueSchema::string()),
                                    max_items: Some(10_000),
                                }),
                            ),
                            (
                                "data".to_owned(),
                                ObjectField::required(ValueSchema::Array {
                                    items: Box::new(datum_ref),
                                    max_items: Some(10_000),
                                }),
                            ),
                            (
                                "revision".to_owned(),
                                ObjectField::required(ValueSchema::bounded_integer(Some(0), None)),
                            ),
                            ("x".to_owned(), ObjectField::required(ValueSchema::number())),
                            ("y".to_owned(), ObjectField::required(ValueSchema::number())),
                            (
                                "width".to_owned(),
                                ObjectField::required(ValueSchema::number()),
                            ),
                            (
                                "height".to_owned(),
                                ObjectField::required(ValueSchema::number()),
                            ),
                        ]),
                        allow_unknown: false,
                    },
                },
            ),
            (
                "legend_change".to_owned(),
                EventSchema {
                    payload: ValueSchema::Object {
                        fields: BTreeMap::from([
                            (
                                "series_key".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "visible".to_owned(),
                                ObjectField::required(ValueSchema::Bool),
                            ),
                        ]),
                        allow_unknown: false,
                    },
                },
            ),
            (
                "annotation_activate".to_owned(),
                EventSchema {
                    payload: ValueSchema::Object {
                        fields: BTreeMap::from([
                            (
                                "key".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                            (
                                "label".to_owned(),
                                ObjectField::required(ValueSchema::string()),
                            ),
                        ]),
                        allow_unknown: false,
                    },
                },
            ),
        ]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: Some(EffectPrimitiveDescriptor {
            platforms: [
                PrimitivePlatform::MacOs,
                PrimitivePlatform::Linux,
                PrimitivePlatform::Windows,
            ]
            .into_iter()
            .collect(),
            max_instances: 256,
            max_cost_per_instance: 2_000_000,
            reduced_motion: true,
            quality_tiers: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_domain_round_trip_is_independent_of_chart_size_and_full_domain() {
        for direction in [ChartAxisDirection::Normal, ChartAxisDirection::Reversed] {
            let source = ChartAxisDomain {
                full: (0.0, 100.0),
                visible: (0.0, 100.0),
                scale: ChartAxisScale::Linear,
                direction,
            };
            let source_visible = linked_visible_domain(source, 2.0, 40.0, 500.0).unwrap();
            let target = ChartAxisDomain {
                full: (-100.0, 300.0),
                visible: (-100.0, 300.0),
                scale: ChartAxisScale::Linear,
                direction,
            };
            let target_zoom =
                (target.full.1 - target.full.0) / (source_visible.1 - source_visible.0);
            let target_pan = linked_pan(target, source_visible, 900.0, target_zoom);
            let target_visible =
                linked_visible_domain(target, target_zoom, target_pan, 900.0).unwrap();
            assert!((source_visible.0 - target_visible.0).abs() < 0.000_001);
            assert!((source_visible.1 - target_visible.1).abs() < 0.000_001);
        }
    }

    #[test]
    fn typed_viewport_round_trips_without_collapsing_axis_windows() {
        let viewport = ChartLinkedViewport::Cartesian {
            region: "main".to_owned(),
            x: Some(ChartLinkedAxis {
                key: "time".to_owned(),
                visible: (12.5, 41.0),
            }),
            y: Some(ChartLinkedAxis {
                key: "value".to_owned(),
                visible: (-8.0, 240.0),
            }),
        };
        let props = PrimitiveProps::new().with(
            "viewport",
            PrimitiveValue::Data(linked_viewport_value(&viewport)),
        );
        assert_eq!(viewport_prop(&props).unwrap(), Some(viewport));

        let geo = ChartLinkedViewport::Geo {
            region: "map".to_owned(),
            map: "world".to_owned(),
            projection: "mercator".to_owned(),
            zoom: 3.0,
            normalized_pan: ChartPoint { x: 0.25, y: -0.5 },
        };
        let props = PrimitiveProps::new().with(
            "viewport",
            PrimitiveValue::Data(linked_viewport_value(&geo)),
        );
        assert_eq!(viewport_prop(&props).unwrap(), Some(geo));
    }
}
