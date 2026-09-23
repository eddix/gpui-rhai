#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, AppContext, Bounds, ContentMask, Context, Element, ElementId, Entity,
    FocusHandle, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Render, ScrollWheelEvent, Styled, Task, WeakEntity, Window,
    canvas, div, fill, point, px, rgba, size,
};

use super::{
    ChartBrushMode, ChartDataLimits, ChartDataSnapshot, ChartDataset, ChartFormatterRegistry,
    ChartGeoRegistry, ChartMark, ChartMarkGeometry, ChartMarkRole, ChartPoint, ChartPreparedData,
    ChartRect, ChartSeriesRegistry, ChartSpec, ChartTheme, ChartTransformRegistry, ChartViewport,
    NativeChartData, PreparedChartScene, layout_chart_scene_with_viewport, prepare_chart_data,
};
use crate::{
    ComponentStateSchema, EffectPrimitiveDescriptor, EventSchema, ObjectField, PrimitiveDescriptor,
    PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitivePlatform, PrimitiveProps, PrimitiveTheme, PrimitiveValue, UiValue, ValueSchema,
};

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
            (Self::Inline(left), Self::Inline(right)) => {
                left.revision() == right.revision() && left.datasets().eq(right.datasets())
            }
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
}

type ChartLinkKey = (String, String);
type ChartLinkMembers = BTreeMap<ChartLinkKey, BTreeSet<WeakEntity<ChartEntity>>>;

#[derive(Clone, Copy, Debug, Default)]
struct ChartLinkedViewport {
    x: Option<(f64, f64)>,
    y: Option<(f64, f64)>,
}

impl ChartLinkRegistry {
    fn register(&self, key: Option<(String, String)>, entity: WeakEntity<ChartEntity>) {
        let mut members = self.members.borrow_mut();
        for group in members.values_mut() {
            group.remove(&entity);
        }
        members.retain(|_, group| !group.is_empty());
        if let Some(key) = key {
            members.entry(key).or_default().insert(entity);
        }
    }

    fn unregister(&self, entity: &WeakEntity<ChartEntity>) {
        let mut members = self.members.borrow_mut();
        for group in members.values_mut() {
            group.remove(entity);
        }
        members.retain(|_, group| !group.is_empty());
    }

    fn members(&self, key: &(String, String)) -> BTreeSet<WeakEntity<ChartEntity>> {
        self.members.borrow().get(key).cloned().unwrap_or_default()
    }

    fn broadcast_zoom(
        &self,
        key: &(String, String),
        source: &WeakEntity<ChartEntity>,
        viewport: ChartLinkedViewport,
        cx: &mut App,
    ) {
        for member in self.members(key) {
            if &member != source {
                let _ = member.update(cx, |chart, cx| {
                    chart.apply_linked_viewport(viewport);
                    chart.rebuild_scene_with_motion(cx, false);
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
                    chart.hovered =
                        hovered.as_deref().and_then(|datum| {
                            chart.scene.as_ref()?.marks.iter().find_map(|mark| {
                                (mark.datum_key == datum).then(|| mark.key.clone())
                            })
                        });
                    cx.notify();
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
        for member in self.members(key) {
            if &member != source {
                let selected = selected.clone();
                let _ = member.update(cx, |chart, cx| {
                    chart.linked_selected = selected;
                    chart.rebuild_scene(cx);
                });
            }
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
    scene: Option<PreparedChartScene>,
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
    prepare_task: Option<Task<()>>,
    layout_job: u64,
    layout_task: Option<Task<()>>,
    data_task: Option<Task<()>>,
    transition_started: Option<Instant>,
    linked_selected: BTreeSet<String>,
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
            scene: None,
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
            prepare_task: None,
            layout_job: 0,
            layout_task: None,
            data_task: None,
            transition_started: None,
            linked_selected: BTreeSet::new(),
        }
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        self.restart_data_listener(cx);
        self.start_prepare(cx);
    }

    fn update_config(
        &mut self,
        config: ChartConfig,
        events: PrimitiveEventEmitter,
        cx: &mut Context<Self>,
    ) {
        let previous_link = link_key(&self.config.spec);
        let next_link = link_key(&config.spec);
        let data_changed = !self.config.data.same_source(&config.data);
        let spec_changed = self.config.spec != config.spec;
        let selection_changed = self.config.selected != config.selected;
        let viewport_changed =
            (self.zoom - config.zoom).abs() > f64::EPSILON || self.pan != config.pan;
        let theme_changed = primitive_chart_theme(&self.config.theme)
            != primitive_chart_theme(&config.theme)
            || self.config.theme.motion_preference() != config.theme.motion_preference()
            || self.config.theme.motion_quality() != config.theme.motion_quality();
        self.config = config;
        if previous_link != next_link {
            self.linked_selected.clear();
        }
        // Every controlled render is an acknowledgement. Re-applying an
        // unchanged Host value intentionally rejects a transient preview.
        self.zoom = self.config.zoom;
        self.pan = self.config.pan;
        self.events = events;
        if let Some(key) = next_link {
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
        } else if selection_changed || theme_changed {
            self.rebuild_scene(cx);
        } else if viewport_changed {
            self.rebuild_scene_with_motion(cx, false);
        }
    }

    fn restart_data_listener(&mut self, cx: &mut Context<Self>) {
        self.data_task = None;
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
        self.job = self.job.saturating_add(1);
        self.layout_job = self.layout_job.saturating_add(1);
        self.layout_task = None;
        let job = self.job;
        let spec = self.config.spec.clone();
        let data = self.config.data.snapshot();
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
                if chart.job != job {
                    return;
                }
                chart.prepare_task = None;
                match result {
                    Ok(prepared) => {
                        chart.prepared = Some(prepared);
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
        let (Some(prepared), Some(bounds)) = (self.prepared.clone(), self.bounds) else {
            cx.notify();
            return;
        };
        self.layout_job = self.layout_job.saturating_add(1);
        let job = self.layout_job;
        let theme = primitive_chart_theme(&self.config.theme);
        let geo = self.geo.clone();
        let viewport = ChartViewport {
            zoom: self.zoom,
            pan: self.pan,
        };
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
                    let mut scene = layout_chart_scene_with_viewport(
                        &prepared,
                        f64::from(bounds.size.width),
                        f64::from(bounds.size.height),
                        &theme,
                        &geo,
                        viewport,
                    )?;
                    apply_selection(&mut scene, &selected, theme.selection);
                    Ok::<_, super::ChartPrepareError>(scene)
                })
                .await;
            let _ = this.update(cx, |chart, cx| {
                if chart.layout_job != job {
                    return;
                }
                chart.layout_task = None;
                match result {
                    Ok(scene) => {
                        chart.scene = Some(scene.clone());
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
        if size_changed {
            self.rebuild_scene(cx);
        }
    }

    fn linked_viewport(&self) -> ChartLinkedViewport {
        let Some(scene) = &self.scene else {
            return ChartLinkedViewport::default();
        };
        let Some(plot) = scene.plot_regions.values().next().copied() else {
            return ChartLinkedViewport::default();
        };
        ChartLinkedViewport {
            x: scene.axis_domains.iter().find_map(|(key, domain)| {
                key.contains(":x:")
                    .then(|| linked_visible_domain(domain.full, self.zoom, self.pan.x, plot.width))
            }),
            y: scene.axis_domains.iter().find_map(|(key, domain)| {
                key.contains(":y:").then(|| {
                    linked_visible_domain(domain.full, self.zoom, self.pan.y, -plot.height)
                })
            }),
        }
    }

    fn apply_linked_viewport(&mut self, linked: ChartLinkedViewport) {
        let Some(scene) = &self.scene else { return };
        let Some(plot) = scene.plot_regions.values().next().copied() else {
            return;
        };
        let x_domain = scene
            .axis_domains
            .iter()
            .find_map(|(key, domain)| key.contains(":x:").then_some(*domain));
        let y_domain = scene
            .axis_domains
            .iter()
            .find_map(|(key, domain)| key.contains(":y:").then_some(*domain));
        let (Some(visible), Some(full)) = (
            linked.x.or(linked.y),
            x_domain.or(y_domain).map(|domain| domain.full),
        ) else {
            return;
        };
        let visible_span = visible.1 - visible.0;
        let full_span = full.1 - full.0;
        if visible_span <= f64::EPSILON || full_span <= f64::EPSILON {
            return;
        }
        self.zoom = (full_span / visible_span).clamp(0.5, 20.0);
        self.pan.x = linked.x.zip(x_domain).map_or(0.0, |(visible, domain)| {
            linked_pan(domain.full, visible, plot.width, self.zoom)
        });
        self.pan.y = linked.y.zip(y_domain).map_or(0.0, |(visible, domain)| {
            linked_pan(domain.full, visible, -plot.height, self.zoom)
        });
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
                self.pan.x += f64::from(event.position.x - origin.x);
                self.pan.y += f64::from(event.position.y - origin.y);
                self.pan_origin = Some(event.position);
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
                let datum = hovered_mark.map(|mark| mark.datum_key);
                window.defer(cx, move |_, cx| {
                    links.broadcast_hover(&key, &source, datum.as_deref(), cx);
                });
            }
            cx.notify();
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window);
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
            self.emit_viewport_change(window, cx);
            return;
        }
        let factor = (-f64::from(delta.y) / 400.0).exp();
        let next = (self.zoom * factor).clamp(0.5, 20.0);
        if (next - self.zoom).abs() <= f64::EPSILON {
            return;
        }
        self.zoom = next;
        self.rebuild_scene_with_motion(cx, false);
        cx.stop_propagation();
    }

    fn emit_viewport_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        let events = self.events.clone();
        let linked = link_key(&self.config.spec).map(|key| {
            (
                self.links.clone(),
                key,
                cx.weak_entity(),
                self.linked_viewport(),
            )
        });
        let payload = UiValue::Map(BTreeMap::from([
            ("zoom".to_owned(), UiValue::Float(self.zoom)),
            ("pan_x".to_owned(), UiValue::Float(self.pan.x)),
            ("pan_y".to_owned(), UiValue::Float(self.pan.y)),
        ]));
        window.defer(cx, move |window, cx| {
            let _ = events.emit("zoom_change", payload, window, cx);
            if let Some((links, key, source, viewport)) = linked {
                links.broadcast_zoom(&key, &source, viewport, cx);
            }
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
        let linked = link_key(&self.config.spec).map(|key| {
            let selected = BTreeSet::from([mark.datum_key.clone()]);
            (self.links.clone(), key, cx.weak_entity(), selected)
        });
        window.defer(cx, move |window, cx| {
            let _ = events.emit("select", payload, window, cx);
            if let Some((links, key, source, selected)) = linked {
                links.broadcast_selection_set(&key, &source, &selected, cx);
            }
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
            .map(|mark| UiValue::String(mark.datum_key.clone()))
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
                let mut element = div()
                    .absolute()
                    .left(px(f64_to_f32(position.x)))
                    .top(px(f64_to_f32(position.y)))
                    .text_size(px(12.0))
                    .text_color(rgba(label.color.as_rgba_hex()))
                    .child(label.text.clone());
                element = match label.anchor {
                    super::ChartLabelAnchor::Start => element,
                    super::ChartLabelAnchor::Center => element.ml(px(-20.0)),
                    super::ChartLabelAnchor::End => element.ml(px(-52.0)),
                };
                root = root.child(element);
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
                root = root.child(
                    div()
                        .absolute()
                        .left(px(f64_to_f32(tooltip_x.max(4.0))))
                        .top(px(f64_to_f32(position.y + 12.0)))
                        .p(px(8.0))
                        .border_1()
                        .border_color(rgba(chart_theme.axis.as_rgba_hex()))
                        .bg(rgba(chart_theme.tooltip_surface.as_rgba_hex()))
                        .text_color(rgba(chart_theme.tooltip_text.as_rgba_hex()))
                        .text_size(px(12.0))
                        .child(tooltip_text),
                );
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
        self.sources.insert(id.clone(), source);
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
            self.links
                .register(link_key(&config.spec), entity.downgrade());
            return Ok(entity.into_any_element());
        };
        self.links
            .register(link_key(&config.spec), entity.downgrade());
        entity.update(cx, |chart, cx| {
            chart.update_config(config, events.clone(), cx);
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
) -> Result<(ChartConfig, ChartSourceCache), String> {
    let source = if let Some(cached) = cached.filter(|cached| cached.matches(props)) {
        cached.clone()
    } else {
        parse_chart_source(props)?
    };
    let hidden = string_set_prop(props, "hidden_series");
    let mut spec = source.spec.clone();
    for series in &mut spec.series {
        if hidden.contains(&series.key) {
            series.visible = false;
        }
    }
    Ok((
        ChartConfig {
            spec,
            data: source.data.clone(),
            selected: string_set_prop(props, "selected_keys"),
            zoom: number_prop(props, "zoom").unwrap_or(1.0),
            pan: ChartPoint {
                x: number_prop(props, "pan_x").unwrap_or(0.0),
                y: number_prop(props, "pan_y").unwrap_or(0.0),
            },
            theme: theme.clone(),
        },
        source,
    ))
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

fn linked_pan(full: (f64, f64), visible: (f64, f64), range: f64, zoom: f64) -> f64 {
    let full_span = full.1 - full.0;
    if full_span <= f64::EPSILON {
        return 0.0;
    }
    let full_center = f64::midpoint(full.0, full.1);
    let visible_center = f64::midpoint(visible.0, visible.1);
    (full_center - visible_center) * range * zoom / full_span
}

fn linked_visible_domain(full: (f64, f64), zoom: f64, pan: f64, range: f64) -> (f64, f64) {
    let span = full.1 - full.0;
    if span <= f64::EPSILON || range.abs() <= f64::EPSILON {
        return full;
    }
    let visible = span / zoom;
    let center = f64::midpoint(full.0, full.1) - pan * span / (range * zoom);
    (center - visible / 2.0, center + visible / 2.0)
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

fn apply_selection(
    scene: &mut PreparedChartScene,
    selected: &BTreeSet<String>,
    color: crate::Rgba8,
) {
    let mut marks = scene.marks.to_vec();
    for mark in &mut marks {
        if selected.contains(&mark.datum_key) {
            mark.selected = true;
            mark.stroke = Some((color, 2.0));
        }
    }
    scene.marks = marks.into();
    let mut semantics = scene.semantics.to_vec();
    for datum in &mut semantics {
        datum.selected = selected.contains(&datum.datum_key);
    }
    scene.semantics = semantics.into();
}

fn mark_payload(mark: &ChartMark, revision: u64) -> UiValue {
    let dataset = mark
        .datum
        .as_ref()
        .map_or_else(String::new, |datum| datum.dataset.clone());
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
        (
            "datum_key".to_owned(),
            UiValue::String(mark.datum_key.clone()),
        ),
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
        let emphasized = hovered == Some(mark.key.as_str())
            || focused == Some(mark.key.as_str())
            || mark.selected;
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
    if let Some((color, width)) = stroke {
        let width = px(f64_to_f32(width));
        window.paint_quad(gpui::quad(
            Bounds::new(
                point(center.x - radius - width, center.y - radius - width),
                size((radius + width) * 2.0, (radius + width) * 2.0),
            ),
            gpui::Corners::all(radius + width),
            rgba(color.as_rgba_hex()),
            gpui::Edges::default(),
            rgba(0),
            gpui::BorderStyle::default(),
        ));
    }
    if let Some(color) = fill_color {
        window.paint_quad(gpui::quad(
            Bounds::new(
                point(center.x - radius, center.y - radius),
                size(radius * 2.0, radius * 2.0),
            ),
            gpui::Corners::all(radius),
            rgba(color.as_rgba_hex()),
            gpui::Edges::default(),
            rgba(0),
            gpui::BorderStyle::default(),
        ));
    }
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
        let source_visible = linked_visible_domain((0.0, 100.0), 2.0, 40.0, 500.0);
        let target_full = (-100.0, 300.0);
        let target_zoom = (target_full.1 - target_full.0) / (source_visible.1 - source_visible.0);
        let target_pan = linked_pan(target_full, source_visible, 900.0, target_zoom);
        let target_visible = linked_visible_domain(target_full, target_zoom, target_pan, 900.0);
        assert!((source_visible.0 - target_visible.0).abs() < 0.000_001);
        assert!((source_visible.1 - target_visible.1).abs() < 0.000_001);
    }
}
