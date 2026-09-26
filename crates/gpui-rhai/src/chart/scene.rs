#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use thiserror::Error;

use super::{
    ChartAnnotationKind, ChartAnnotationValue, ChartAxisDirection, ChartAxisPosition,
    ChartAxisScale, ChartChannel, ChartCoordinateKind, ChartDataError, ChartDataSnapshot,
    ChartDataset, ChartDiagnostic, ChartDiagnosticSeverity, ChartFormatterError,
    ChartFormatterRegistry, ChartGeoError, ChartGeoMap, ChartGeoPoint, ChartGeoRegistry,
    ChartScale, ChartScaleError, ChartSeriesExtensionError, ChartSeriesKind, ChartSeriesRegistry,
    ChartSeriesSpec, ChartSpec, ChartSpecError, ChartTransformContext, ChartTransformError,
    ChartTransformRegistry, ChartValue, apply_chart_transforms,
};
use crate::Rgba8;

const MAX_DIAGNOSTICS: usize = 100;
const MAX_INTERACTIVE_MARKS: usize = 10_000;
const MAX_SCENE_MARKS: usize = 200_000;
const MAX_SCENE_VERTICES: usize = 2_000_000;
const DEFAULT_SAMPLE_THRESHOLD: usize = 4_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChartPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChartRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartViewport {
    pub zoom: f64,
    pub pan: ChartPoint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartAxisDomain {
    pub full: (f64, f64),
    pub visible: (f64, f64),
    pub scale: ChartAxisScale,
    pub direction: ChartAxisDirection,
}

impl Default for ChartViewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: ChartPoint::default(),
        }
    }
}

impl ChartRect {
    #[must_use]
    pub fn contains(self, point: ChartPoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }

    #[must_use]
    pub fn center(self) -> ChartPoint {
        ChartPoint {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartMarkGeometry {
    Rect(ChartRect),
    Circle {
        center: ChartPoint,
        radius: f64,
    },
    Polyline {
        points: Arc<[ChartPoint]>,
        width: f64,
    },
    Polygon(Arc<[ChartPoint]>),
    CompoundPolygon(Arc<[Arc<[ChartPoint]>]>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ChartDatumRef {
    pub dataset: String,
    pub series: String,
    pub key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartMarkRole {
    Data,
    Legend,
    Annotation,
    Axis,
    Grid,
    Decoration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartMark {
    pub key: String,
    pub region_key: String,
    pub role: ChartMarkRole,
    pub datum: Option<ChartDatumRef>,
    pub series_key: String,
    pub datum_key: String,
    pub geometry: ChartMarkGeometry,
    pub fill: Option<Rgba8>,
    pub stroke: Option<(Rgba8, f64)>,
    pub label: String,
    pub value: Option<f64>,
    pub interactive: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartLabel {
    pub key: String,
    pub position: ChartPoint,
    pub text: String,
    pub color: Rgba8,
    pub anchor: ChartLabelAnchor,
    pub role: ChartLabelRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartLabelAnchor {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartLabelRole {
    Title,
    Label,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartSemanticDatum {
    pub series_key: String,
    pub datum_key: String,
    pub name: String,
    pub value: Option<f64>,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartTheme {
    pub background: Rgba8,
    pub text: Rgba8,
    pub muted_text: Rgba8,
    pub axis: Rgba8,
    pub grid: Rgba8,
    pub positive: Rgba8,
    pub negative: Rgba8,
    pub selection: Rgba8,
    pub map_missing: Rgba8,
    pub crosshair: Rgba8,
    pub tooltip_surface: Rgba8,
    pub tooltip_text: Rgba8,
    pub palette: Arc<[Rgba8]>,
    pub locale: String,
    pub number: Option<crate::NumberMetadata>,
    pub motion_quality: crate::MotionQuality,
    pub direction: crate::TextDirection,
    pub title_typography: crate::ResolvedTypography,
    pub label_typography: crate::ResolvedTypography,
}

impl Default for ChartTheme {
    fn default() -> Self {
        Self {
            background: Rgba8::from_rgb_hex(0x0011_1317),
            text: Rgba8::from_rgb_hex(0x00e6_e9ef),
            muted_text: Rgba8::from_rgb_hex(0x009a_a2b0),
            axis: Rgba8::from_rgb_hex(0x0074_7d8c),
            grid: Rgba8::from_rgba_hex(0x747d_8c44),
            positive: Rgba8::from_rgb_hex(0x007c_cf91),
            negative: Rgba8::from_rgb_hex(0x00f0_7b83),
            selection: Rgba8::from_rgb_hex(0x007a_a2f7),
            map_missing: Rgba8::from_rgb_hex(0x0030_353f),
            crosshair: Rgba8::from_rgb_hex(0x00c0_caf5),
            tooltip_surface: Rgba8::from_rgb_hex(0x0018_1b22),
            tooltip_text: Rgba8::from_rgb_hex(0x00e6_e9ef),
            palette: vec![
                Rgba8::from_rgb_hex(0x007a_a2f7),
                Rgba8::from_rgb_hex(0x007d_cf91),
                Rgba8::from_rgb_hex(0x00e0_af68),
                Rgba8::from_rgb_hex(0x00bb_9af7),
                Rgba8::from_rgb_hex(0x007d_cfe4),
                Rgba8::from_rgb_hex(0x00f7_768e),
                Rgba8::from_rgb_hex(0x00ff_9e64),
                Rgba8::from_rgb_hex(0x009e_ce6a),
            ]
            .into(),
            locale: "en".to_owned(),
            number: None,
            motion_quality: crate::MotionQuality::High,
            direction: crate::TextDirection::LeftToRight,
            title_typography: chart_typography(16.0, 22.0, 700),
            label_typography: chart_typography(12.0, 16.0, 400),
        }
    }
}

fn chart_typography(size: f64, line_height: f64, weight: u16) -> crate::ResolvedTypography {
    crate::ResolvedTypography {
        family: None,
        fallbacks: Vec::new(),
        size: crate::Length::Pixels(size),
        line_height: crate::Length::Pixels(line_height),
        weight,
    }
}

fn typography_pixels(value: crate::Length, fallback: f64) -> f64 {
    match value {
        crate::Length::Pixels(value) => value,
        crate::Length::Rems(value) => value * 16.0,
        crate::Length::Relative(_)
        | crate::Length::ThemeSpacing(_)
        | crate::Length::ThemeRadius(_) => fallback,
    }
}

#[derive(Clone, Debug)]
struct PreparedSeries {
    spec: ChartSeriesSpec,
    semantic_dataset: ChartDataset,
    dataset: ChartDataset,
    sampled: bool,
    color_index: usize,
}

#[derive(Clone, Debug)]
pub struct ChartPreparedData {
    revision: u64,
    spec: ChartSpec,
    series: Arc<[PreparedSeries]>,
    diagnostics: Arc<[ChartDiagnostic]>,
    custom_series: ChartSeriesRegistry,
    formatters: ChartFormatterRegistry,
}

impl ChartPreparedData {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn spec(&self) -> &ChartSpec {
        &self.spec
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[ChartDiagnostic] {
        &self.diagnostics
    }

    pub fn sampled_series(&self) -> impl Iterator<Item = &str> {
        self.series
            .iter()
            .filter(|series| series.sampled)
            .map(|series| series.spec.key.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedChartScene {
    pub revision: u64,
    pub width: f64,
    pub height: f64,
    pub plot_regions: BTreeMap<String, ChartRect>,
    pub axis_domains: BTreeMap<String, ChartAxisDomain>,
    pub marks: Arc<[ChartMark]>,
    pub labels: Arc<[ChartLabel]>,
    /// Bounded semantic data is derived from the transformed data set, not the
    /// draw sample, so downsampling never makes a selected datum disappear.
    pub semantics: Arc<[ChartSemanticDatum]>,
    pub diagnostics: Arc<[ChartDiagnostic]>,
    pub summary: String,
    pub sampled_series: Arc<[String]>,
}

impl PreparedChartScene {
    #[must_use]
    pub fn hit_test(&self, point: ChartPoint) -> Option<&ChartMark> {
        self.marks.iter().rev().find(|mark| {
            mark.interactive
                && (mark.role != ChartMarkRole::Data
                    || self
                        .plot_regions
                        .get(&mark.region_key)
                        .is_some_and(|region| region.contains(point)))
                && mark_hit_test(mark, point)
        })
    }

    #[must_use]
    pub fn semantic_projection(&self, limit: usize) -> Vec<ChartSemanticDatum> {
        self.semantics
            .iter()
            .take(limit.min(1_000))
            .cloned()
            .collect()
    }
}

/// Interpolate two prepared scenes by stable mark key. Compatible geometry
/// morphs directly; entering geometry grows from a deterministic collapsed
/// shape and removed geometry fades without changing datum identity.
#[must_use]
pub fn interpolate_chart_scene(
    previous: &PreparedChartScene,
    next: &PreparedChartScene,
    progress: f64,
) -> PreparedChartScene {
    let progress = progress.clamp(0.0, 1.0);
    let previous_by_key = previous
        .marks
        .iter()
        .map(|mark| (mark.key.as_str(), mark))
        .collect::<BTreeMap<_, _>>();
    let next_keys = next
        .marks
        .iter()
        .map(|mark| mark.key.as_str())
        .collect::<BTreeSet<_>>();
    let mut marks = next
        .marks
        .iter()
        .map(|mark| {
            let mut sampled = mark.clone();
            sampled.geometry = interpolate_geometry(
                previous_by_key
                    .get(mark.key.as_str())
                    .map(|mark| &mark.geometry),
                &mark.geometry,
                progress,
            );
            sampled.fill = interpolate_optional_color(
                previous_by_key
                    .get(mark.key.as_str())
                    .and_then(|mark| mark.fill),
                mark.fill,
                progress,
            );
            sampled
        })
        .collect::<Vec<_>>();
    marks.extend(
        previous
            .marks
            .iter()
            .filter(|mark| !next_keys.contains(mark.key.as_str()))
            .map(|mark| {
                let mut exiting = mark.clone();
                exiting.fill = mark.fill.map(|color| scale_alpha(color, 1.0 - progress));
                exiting.stroke = mark
                    .stroke
                    .map(|(color, width)| (scale_alpha(color, 1.0 - progress), width));
                exiting.interactive = false;
                exiting
            }),
    );
    PreparedChartScene {
        marks: marks.into(),
        ..next.clone()
    }
}

fn interpolate_geometry(
    previous: Option<&ChartMarkGeometry>,
    next: &ChartMarkGeometry,
    progress: f64,
) -> ChartMarkGeometry {
    match (previous, next) {
        (Some(ChartMarkGeometry::Rect(left)), ChartMarkGeometry::Rect(right)) => {
            ChartMarkGeometry::Rect(ChartRect {
                x: lerp(left.x, right.x, progress),
                y: lerp(left.y, right.y, progress),
                width: lerp(left.width, right.width, progress),
                height: lerp(left.height, right.height, progress),
            })
        }
        (
            Some(ChartMarkGeometry::Circle {
                center: left,
                radius: left_radius,
            }),
            ChartMarkGeometry::Circle {
                center: right,
                radius: right_radius,
            },
        ) => ChartMarkGeometry::Circle {
            center: interpolate_point(*left, *right, progress),
            radius: lerp(*left_radius, *right_radius, progress),
        },
        (
            Some(ChartMarkGeometry::Polyline {
                points: left,
                width: left_width,
            }),
            ChartMarkGeometry::Polyline {
                points: right,
                width: right_width,
            },
        ) if left.len() == right.len() => ChartMarkGeometry::Polyline {
            points: left
                .iter()
                .zip(right.iter())
                .map(|(left, right)| interpolate_point(*left, *right, progress))
                .collect::<Vec<_>>()
                .into(),
            width: lerp(*left_width, *right_width, progress),
        },
        (Some(ChartMarkGeometry::Polygon(left)), ChartMarkGeometry::Polygon(right))
            if left.len() == right.len() =>
        {
            ChartMarkGeometry::Polygon(
                left.iter()
                    .zip(right.iter())
                    .map(|(left, right)| interpolate_point(*left, *right, progress))
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        (
            Some(ChartMarkGeometry::CompoundPolygon(left)),
            ChartMarkGeometry::CompoundPolygon(right),
        ) if left.len() == right.len()
            && left
                .iter()
                .zip(right.iter())
                .all(|(left, right)| left.len() == right.len()) =>
        {
            ChartMarkGeometry::CompoundPolygon(
                left.iter()
                    .zip(right.iter())
                    .map(|(left, right)| {
                        left.iter()
                            .zip(right.iter())
                            .map(|(left, right)| interpolate_point(*left, *right, progress))
                            .collect::<Vec<_>>()
                            .into()
                    })
                    .collect::<Vec<Arc<[ChartPoint]>>>()
                    .into(),
            )
        }
        (None, ChartMarkGeometry::Rect(rect)) => ChartMarkGeometry::Rect(ChartRect {
            x: rect.x,
            y: rect.y + rect.height * (1.0 - progress),
            width: rect.width,
            height: rect.height * progress,
        }),
        (None, ChartMarkGeometry::Circle { center, radius }) => ChartMarkGeometry::Circle {
            center: *center,
            radius: radius * progress,
        },
        (None, ChartMarkGeometry::Polyline { points, width }) => ChartMarkGeometry::Polyline {
            points: trim_polyline(points, progress).into(),
            width: *width,
        },
        (None, ChartMarkGeometry::Polygon(points)) => {
            let center = polygon_center(points);
            ChartMarkGeometry::Polygon(
                points
                    .iter()
                    .map(|point| interpolate_point(center, *point, progress))
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        (None, ChartMarkGeometry::CompoundPolygon(rings)) => ChartMarkGeometry::CompoundPolygon(
            rings
                .iter()
                .map(|points| {
                    let center = polygon_center(points);
                    points
                        .iter()
                        .map(|point| interpolate_point(center, *point, progress))
                        .collect::<Vec<_>>()
                        .into()
                })
                .collect::<Vec<Arc<[ChartPoint]>>>()
                .into(),
        ),
        (Some(_), _) => next.clone(),
    }
}

fn trim_polyline(points: &[ChartPoint], progress: f64) -> Vec<ChartPoint> {
    if points.len() < 2 || progress >= 1.0 {
        return points.to_vec();
    }
    let lengths = points
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y))
        .collect::<Vec<_>>();
    let target = lengths.iter().sum::<f64>() * progress;
    let mut result = vec![points[0]];
    let mut consumed = 0.0;
    for (index, length) in lengths.iter().copied().enumerate() {
        if consumed + length <= target {
            result.push(points[index + 1]);
            consumed += length;
            continue;
        }
        if length > 0.0 {
            result.push(interpolate_point(
                points[index],
                points[index + 1],
                ((target - consumed) / length).clamp(0.0, 1.0),
            ));
        }
        break;
    }
    result
}

fn polygon_center(points: &[ChartPoint]) -> ChartPoint {
    let count = usize_to_f64(points.len().max(1));
    ChartPoint {
        x: points.iter().map(|point| point.x).sum::<f64>() / count,
        y: points.iter().map(|point| point.y).sum::<f64>() / count,
    }
}

fn interpolate_point(left: ChartPoint, right: ChartPoint, progress: f64) -> ChartPoint {
    ChartPoint {
        x: lerp(left.x, right.x, progress),
        y: lerp(left.y, right.y, progress),
    }
}

fn lerp(left: f64, right: f64, progress: f64) -> f64 {
    left + (right - left) * progress
}

fn interpolate_optional_color(
    left: Option<Rgba8>,
    right: Option<Rgba8>,
    progress: f64,
) -> Option<Rgba8> {
    match (left, right) {
        (Some(left), Some(right)) => Some(mix_color(left, right, progress)),
        (None, Some(right)) => Some(scale_alpha(right, progress)),
        (Some(left), None) => Some(scale_alpha(left, 1.0 - progress)),
        (None, None) => None,
    }
}

fn scale_alpha(color: Rgba8, scale: f64) -> Rgba8 {
    let value = color.as_rgba_hex();
    let alpha = f64::from((value & 0xff) as u8) * scale.clamp(0.0, 1.0);
    Rgba8::from_rgba_hex((value & 0xffff_ff00) | alpha.round() as u32)
}

/// Validate, transform, and downsample one immutable chart revision.
///
/// # Errors
///
/// Returns specification, dataset, transform, or extension diagnostics. The
/// caller retains its previous prepared candidate on failure.
pub fn prepare_chart_data(
    spec: ChartSpec,
    data: &ChartDataSnapshot,
    transforms: &ChartTransformRegistry,
    custom_series: &ChartSeriesRegistry,
    formatters: &ChartFormatterRegistry,
) -> Result<ChartPreparedData, ChartPrepareError> {
    spec.validate()?;
    let mut diagnostics = Vec::new();
    let mut prepared = Vec::with_capacity(spec.series.len());
    for (color_index, series) in spec.series.iter().enumerate() {
        let source = data
            .dataset(&series.dataset)
            .ok_or_else(|| ChartPrepareError::MissingDataset(series.dataset.clone()))?;
        let semantic_transforms = series
            .transforms
            .iter()
            .filter(|transform| !matches!(transform, super::ChartTransformSpec::Downsample { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let mut dataset = apply_chart_transforms(
            source,
            &semantic_transforms,
            transforms,
            ChartTransformContext::default(),
        )?;
        validate_series_dimensions(series, &dataset)?;
        if dataset.key_dimension().is_none() {
            dataset = dataset.with_replacement_identity(&series.key)?;
            push_diagnostic(
                &mut diagnostics,
                ChartDiagnostic {
                    severity: ChartDiagnosticSeverity::Warning,
                    code: "chart.replacement_identity".to_owned(),
                    message: format!(
                        "series `{}` has no datum key; any data change replaces the full series",
                        series.key
                    ),
                    series: Some(series.key.clone()),
                    datum: None,
                },
            );
        }
        let semantic_dataset = dataset.clone();
        let original_len = semantic_dataset.len();
        let explicit_downsample = series
            .transforms
            .iter()
            .filter(|transform| matches!(transform, super::ChartTransformSpec::Downsample { .. }))
            .cloned()
            .collect::<Vec<_>>();
        if !explicit_downsample.is_empty() {
            dataset = apply_chart_transforms(
                &dataset,
                &explicit_downsample,
                transforms,
                ChartTransformContext::default(),
            )?;
        } else if matches!(series.kind, ChartSeriesKind::Line | ChartSeriesKind::Area)
            && dataset.len() > DEFAULT_SAMPLE_THRESHOLD
            && let (Some(x), Some(y)) = (
                series.encode.dimension(ChartChannel::X),
                series.encode.dimension(ChartChannel::Y),
            )
        {
            dataset = apply_chart_transforms(
                &dataset,
                &[super::ChartTransformSpec::Downsample {
                    x: x.to_owned(),
                    y: y.to_owned(),
                    threshold: DEFAULT_SAMPLE_THRESHOLD,
                }],
                transforms,
                ChartTransformContext::default(),
            )?;
        }
        let sampled = dataset.len() < original_len;
        if sampled {
            push_diagnostic(
                &mut diagnostics,
                ChartDiagnostic {
                    severity: ChartDiagnosticSeverity::Warning,
                    code: "chart.downsampled".to_owned(),
                    message: format!(
                        "series `{}` draws {} of {} rows; interactions retain original keys",
                        series.key,
                        dataset.len(),
                        original_len
                    ),
                    series: Some(series.key.clone()),
                    datum: None,
                },
            );
        }
        prepared.push(PreparedSeries {
            spec: series.clone(),
            semantic_dataset,
            dataset,
            sampled,
            color_index,
        });
    }
    Ok(ChartPreparedData {
        revision: data.revision(),
        spec,
        series: prepared.into(),
        diagnostics: diagnostics.into(),
        custom_series: custom_series.clone(),
        formatters: formatters.clone(),
    })
}

/// Lay out one prepared data revision for an exact viewport and theme.
///
/// # Errors
///
/// Returns for an invalid viewport, scale, geo source, or custom series.
pub fn layout_chart_scene(
    prepared: &ChartPreparedData,
    width: f64,
    height: f64,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
) -> Result<PreparedChartScene, ChartPrepareError> {
    layout_chart_scene_with_viewport(
        prepared,
        width,
        height,
        theme,
        geo,
        ChartViewport::default(),
    )
}

/// Lay out one acknowledged viewport. Cartesian marks and ticks share the
/// same visible domain instead of applying a post-layout pixel transform.
///
/// # Errors
///
/// Returns the same validated viewport, scale, geo, formatter, or custom
/// series errors as [`layout_chart_scene`].
pub fn layout_chart_scene_with_viewport(
    prepared: &ChartPreparedData,
    width: f64,
    height: f64,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
    viewport: ChartViewport,
) -> Result<PreparedChartScene, ChartPrepareError> {
    layout_chart_scene_with_axis_windows(
        prepared,
        width,
        height,
        theme,
        geo,
        viewport,
        &BTreeMap::new(),
    )
}

pub(crate) fn layout_chart_scene_with_axis_windows(
    prepared: &ChartPreparedData,
    width: f64,
    height: f64,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
    viewport: ChartViewport,
    axis_windows: &BTreeMap<String, (f64, f64)>,
) -> Result<PreparedChartScene, ChartPrepareError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(ChartPrepareError::InvalidViewport { width, height });
    }
    let mut diagnostics = prepared.diagnostics.to_vec();
    let regions = region_rects(&prepared.spec, width, height, theme);
    let mut marks = Vec::new();
    let mut labels = Vec::new();
    let mut axis_domains = BTreeMap::new();
    add_title_and_legend(
        &prepared.spec,
        &prepared.series,
        width,
        height,
        theme,
        &mut marks,
        &mut labels,
    );
    for region in &prepared.spec.regions {
        let bounds = regions[&region.key];
        let series = prepared
            .series
            .iter()
            .filter(|series| series.spec.visible && series.spec.coordinate == region.key)
            .collect::<Vec<_>>();
        match region.kind {
            ChartCoordinateKind::Cartesian2d => layout_cartesian(
                &prepared.spec,
                &series,
                bounds,
                theme,
                &mut marks,
                &mut labels,
                &mut diagnostics,
                &prepared.custom_series,
                &prepared.formatters,
                viewport,
                axis_windows,
                &mut axis_domains,
            )?,
            ChartCoordinateKind::Polar => layout_polar(
                &prepared.spec,
                &region.key,
                &series,
                bounds,
                theme,
                &mut marks,
                &mut labels,
                &mut diagnostics,
                &prepared.custom_series,
            )?,
            ChartCoordinateKind::Geo2d => {
                let map = region
                    .map
                    .as_ref()
                    .ok_or_else(|| ChartPrepareError::MissingGeoMap(region.key.clone()))?;
                let projection = region.projection.as_deref().unwrap_or("equirectangular");
                let source_map = geo.map(map)?;
                let data_projection = if source_map.projected {
                    None
                } else {
                    Some(geo.projection(projection)?)
                };
                let map = geo.projected_map(map, projection)?;
                let mark_start = marks.len();
                layout_geo(
                    &series,
                    &map,
                    bounds,
                    theme,
                    &mut marks,
                    &mut labels,
                    &mut diagnostics,
                    data_projection.as_deref(),
                    &prepared.custom_series,
                )?;
                apply_geo_viewport(&mut marks[mark_start..], bounds, viewport);
            }
        }
    }
    let mut mark_ids = BTreeSet::new();
    if let Some(duplicate) = marks
        .iter()
        .find_map(|mark| (!mark_ids.insert(mark.key.clone())).then(|| mark.key.clone()))
    {
        return Err(ChartPrepareError::DuplicateMarkIdentity(duplicate));
    }
    let vertices = marks
        .iter()
        .map(|mark| geometry_vertex_count(&mark.geometry))
        .fold(0_usize, usize::saturating_add);
    if marks.len() > MAX_SCENE_MARKS || vertices > MAX_SCENE_VERTICES {
        return Err(ChartPrepareError::SceneBudget {
            marks: marks.len(),
            vertices,
        });
    }
    let interactive = marks.iter().filter(|mark| mark.interactive).count();
    if interactive > MAX_INTERACTIVE_MARKS {
        let mut retained = 0_usize;
        for mark in &mut marks {
            if mark.interactive {
                retained += 1;
                if retained > MAX_INTERACTIVE_MARKS {
                    mark.interactive = false;
                }
            }
        }
        push_diagnostic(
            &mut diagnostics,
            ChartDiagnostic {
                severity: ChartDiagnosticSeverity::Warning,
                code: "chart.interaction_budget".to_owned(),
                message: format!(
                    "interactive mark projection capped at {MAX_INTERACTIVE_MARKS} of {interactive}"
                ),
                series: None,
                datum: None,
            },
        );
    }
    let visible_series = prepared
        .series
        .iter()
        .filter(|series| series.spec.visible)
        .count();
    let summary = prepared.spec.description.clone().unwrap_or_else(|| {
        format!(
            "{} chart with {visible_series} visible series and {} rendered marks",
            prepared.spec.title.as_deref().unwrap_or("Untitled"),
            marks.len()
        )
    });
    let semantics = semantic_data(&prepared.series);
    Ok(PreparedChartScene {
        revision: prepared.revision,
        width,
        height,
        plot_regions: regions,
        axis_domains,
        marks: marks.into(),
        labels: labels.into(),
        semantics: semantics.into(),
        diagnostics: diagnostics.into(),
        summary,
        sampled_series: prepared
            .sampled_series()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
            .into(),
    })
}

fn semantic_data(series: &[PreparedSeries]) -> Vec<ChartSemanticDatum> {
    let mut output = Vec::new();
    for prepared in series.iter().filter(|series| series.spec.visible) {
        let name = prepared
            .spec
            .encode
            .dimension(ChartChannel::Name)
            .and_then(|dimension| prepared.semantic_dataset.column(dimension));
        let value = [ChartChannel::Value, ChartChannel::Y]
            .into_iter()
            .find_map(|channel| {
                prepared
                    .spec
                    .encode
                    .dimension(channel)
                    .and_then(|dimension| prepared.semantic_dataset.column(dimension))
            });
        for row in 0..prepared.semantic_dataset.len() {
            output.push(ChartSemanticDatum {
                series_key: prepared.spec.key.clone(),
                datum_key: datum_key(&prepared.semantic_dataset, row),
                name: datum_label(name, &prepared.semantic_dataset, row, &prepared.spec.name),
                value: value
                    .and_then(|column| column.value(row))
                    .and_then(|value| value.as_number()),
                selected: false,
            });
        }
    }
    output
}

pub(crate) fn apply_chart_selection(
    scene: &mut PreparedChartScene,
    selected: &BTreeSet<String>,
    color: Rgba8,
) {
    let mut marks = scene.marks.to_vec();
    for mark in &mut marks {
        if mark.role == ChartMarkRole::Data
            && mark
                .datum
                .as_ref()
                .is_some_and(|datum| selected.contains(&datum.key))
        {
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

fn validate_series_dimensions(
    series: &ChartSeriesSpec,
    dataset: &ChartDataset,
) -> Result<(), ChartPrepareError> {
    for (_, dimension) in series.encode.iter() {
        if dataset.column(dimension).is_none() {
            return Err(ChartPrepareError::MissingDimension {
                series: series.key.clone(),
                dimension: dimension.to_owned(),
            });
        }
    }
    Ok(())
}

fn region_rects(
    spec: &ChartSpec,
    width: f64,
    height: f64,
    theme: &ChartTheme,
) -> BTreeMap<String, ChartRect> {
    let title_line = typography_pixels(theme.title_typography.line_height, 22.0);
    let label_line = typography_pixels(theme.label_typography.line_height, 16.0);
    let label_size = typography_pixels(theme.label_typography.size, 12.0);
    let top = if spec.title.is_some() {
        14.0 + title_line + 16.0
    } else {
        label_line + 8.0
    } + if spec.legend.visible
        && matches!(spec.legend.position, super::ChartLegendPosition::Top)
    {
        label_line + 14.0
    } else {
        0.0
    };
    let bottom = label_line
        + 20.0
        + if spec.legend.visible
            && matches!(spec.legend.position, super::ChartLegendPosition::Bottom)
        {
            label_line + 14.0
        } else {
            0.0
        };
    let left = (label_size * 4.0 + 4.0).max(52.0)
        + if spec.legend.visible && matches!(spec.legend.position, super::ChartLegendPosition::Left)
        {
            100.0
        } else {
            0.0
        };
    let right = (label_size * 2.0).max(24.0)
        + if spec.legend.visible
            && matches!(spec.legend.position, super::ChartLegendPosition::Right)
        {
            100.0
        } else {
            0.0
        };
    let max_row = spec
        .regions
        .iter()
        .map(|region| usize::from(region.row + region.row_span))
        .max()
        .unwrap_or(1);
    let max_column = spec
        .regions
        .iter()
        .map(|region| usize::from(region.column + region.column_span))
        .max()
        .unwrap_or(1);
    let available_width = (width - left - right).max(1.0);
    let available_height = (height - top - bottom).max(1.0);
    spec.regions
        .iter()
        .map(|region| {
            let column_width = available_width / usize_to_f64(max_column);
            let row_height = available_height / usize_to_f64(max_row);
            (
                region.key.clone(),
                ChartRect {
                    x: left + f64::from(region.column) * column_width,
                    y: top + f64::from(region.row) * row_height,
                    width: (f64::from(region.column_span) * column_width - 18.0).max(1.0),
                    height: (f64::from(region.row_span) * row_height - 18.0).max(1.0),
                },
            )
        })
        .collect()
}

pub(crate) fn chart_region_rect(
    spec: &ChartSpec,
    width: f64,
    height: f64,
    region: &str,
    theme: &ChartTheme,
) -> Option<ChartRect> {
    region_rects(spec, width, height, theme).remove(region)
}

fn add_title_and_legend(
    spec: &ChartSpec,
    series: &[PreparedSeries],
    width: f64,
    height: f64,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
) {
    let title_line = typography_pixels(theme.title_typography.line_height, 22.0);
    let label_line = typography_pixels(theme.label_typography.line_height, 16.0);
    if let Some(title) = &spec.title {
        labels.push(ChartLabel {
            key: "title".to_owned(),
            position: ChartPoint {
                x: if theme.direction == crate::TextDirection::RightToLeft {
                    width - 20.0
                } else {
                    20.0
                },
                y: 14.0,
            },
            text: title.clone(),
            color: theme.text,
            anchor: if theme.direction == crate::TextDirection::RightToLeft {
                ChartLabelAnchor::End
            } else {
                ChartLabelAnchor::Start
            },
            role: ChartLabelRole::Title,
        });
    }
    if !spec.legend.visible {
        return;
    }
    let horizontal = matches!(
        spec.legend.position,
        super::ChartLegendPosition::Top | super::ChartLegendPosition::Bottom
    );
    let mut cursor = 20.0;
    for series in series {
        let color = series_color(series, theme);
        let logical_x = if horizontal && theme.direction == crate::TextDirection::RightToLeft {
            width - cursor - 12.0
        } else {
            cursor
        };
        let position = match spec.legend.position {
            super::ChartLegendPosition::Top => ChartPoint {
                x: logical_x,
                y: if spec.title.is_some() {
                    14.0 + title_line + 6.0
                } else {
                    14.0
                },
            },
            super::ChartLegendPosition::Bottom => ChartPoint {
                x: logical_x,
                y: height - 22.0,
            },
            super::ChartLegendPosition::Left => ChartPoint {
                x: 16.0,
                y: 60.0 + cursor,
            },
            super::ChartLegendPosition::Right => ChartPoint {
                x: width - 92.0,
                y: 60.0 + cursor,
            },
        };
        marks.push(ChartMark {
            key: format!("legend:{}", series.spec.key),
            region_key: "__viewport".to_owned(),
            role: ChartMarkRole::Legend,
            datum: None,
            series_key: series.spec.key.clone(),
            datum_key: "legend".to_owned(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x: position.x,
                y: position.y,
                width: 12.0,
                height: 12.0,
            }),
            fill: Some(color),
            stroke: None,
            label: series.spec.name.clone(),
            value: None,
            interactive: spec.legend.interactive,
            selected: series.spec.visible,
        });
        labels.push(ChartLabel {
            key: format!("legend-label:{}", series.spec.key),
            position: ChartPoint {
                x: if horizontal && theme.direction == crate::TextDirection::RightToLeft {
                    position.x - 6.0
                } else {
                    position.x + 18.0
                },
                y: position.y - 2.0,
            },
            text: series.spec.name.clone(),
            color: if series.spec.visible {
                theme.text
            } else {
                theme.muted_text
            },
            anchor: if horizontal && theme.direction == crate::TextDirection::RightToLeft {
                ChartLabelAnchor::End
            } else {
                ChartLabelAnchor::Start
            },
            role: ChartLabelRole::Label,
        });
        cursor += if horizontal {
            72.0 + usize_to_f64(series.spec.name.len()) * 4.0
        } else {
            label_line + 8.0
        };
    }
}

fn resolved_axis_key(spec: &ChartSpec, series: &ChartSeriesSpec, channel: ChartChannel) -> String {
    let explicit = match channel {
        ChartChannel::X => series.x_axis.as_ref(),
        ChartChannel::Y => series.y_axis.as_ref(),
        _ => None,
    };
    if let Some(explicit) = explicit {
        return explicit.clone();
    }
    let position_matches = |position| match channel {
        ChartChannel::X => matches!(position, ChartAxisPosition::Top | ChartAxisPosition::Bottom),
        ChartChannel::Y => matches!(position, ChartAxisPosition::Left | ChartAxisPosition::Right),
        _ => false,
    };
    spec.axes
        .iter()
        .find(|axis| axis.region == series.coordinate && position_matches(axis.position))
        .map_or_else(
            || match channel {
                ChartChannel::X => "__implicit_x".to_owned(),
                ChartChannel::Y => "__implicit_y".to_owned(),
                _ => "__implicit".to_owned(),
            },
            |axis| axis.key.clone(),
        )
}

fn compile_cartesian_axis(
    spec: &ChartSpec,
    series: &[&PreparedSeries],
    channel: ChartChannel,
    identity: &str,
    bounds: ChartRect,
    viewport: ChartViewport,
    visible_override: Option<(f64, f64)>,
) -> Result<(ChartScale, ChartAxisDomain), ChartPrepareError> {
    let axis = spec.axes.iter().find(|axis| axis.key == identity);
    let categorical = axis.is_some_and(|axis| axis.scale == ChartAxisScale::Category)
        || series.iter().any(|prepared| {
            prepared
                .spec
                .encode
                .dimension(channel)
                .and_then(|name| prepared.semantic_dataset.column(name))
                .is_some_and(|column| {
                    matches!(
                        column.data_type(),
                        super::ChartDataType::String | super::ChartDataType::Bool
                    )
                })
        });
    let categories = if categorical {
        collect_categories(series, channel)
    } else {
        Vec::new()
    };
    let inferred = if categorical {
        (0.0, usize_to_f64(categories.len().max(1)))
    } else if channel == ChartChannel::Y {
        cartesian_y_domain(
            series,
            !axis.is_some_and(|axis| axis.scale == ChartAxisScale::Log),
        )
        .unwrap_or((0.0, 1.0))
    } else {
        numeric_domain(series, channel, false).unwrap_or((0.0, 1.0))
    };
    let full = resolve_axis_domain(axis, inferred);
    let direction = axis.map_or(ChartAxisDirection::Normal, |axis| axis.direction);
    let scale_kind = if categorical {
        ChartAxisScale::Category
    } else {
        axis.map_or(ChartAxisScale::Linear, |axis| match axis.scale {
            ChartAxisScale::Auto | ChartAxisScale::Category => ChartAxisScale::Linear,
            scale => scale,
        })
    };
    let (range_start, range_end, pan) = if channel == ChartChannel::Y {
        (
            bounds.y + bounds.height,
            bounds.y,
            direction_adjusted_pan(viewport.pan.y, direction),
        )
    } else {
        (
            bounds.x,
            bounds.x + bounds.width,
            direction_adjusted_pan(viewport.pan.x, direction),
        )
    };
    let visible = if categorical {
        full
    } else if let Some(visible) = visible_override {
        visible
    } else {
        viewport_domain(
            full,
            viewport.zoom,
            pan,
            range_end - range_start,
            scale_kind,
        )
    };
    let scale = if categorical {
        ChartScale::category(categories, range_start, range_end, direction)?
    } else {
        build_continuous_scale(axis, visible, range_start, range_end)?
    };
    Ok((
        scale,
        ChartAxisDomain {
            full,
            visible,
            scale: scale_kind,
            direction,
        },
    ))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_cartesian(
    spec: &ChartSpec,
    series: &[&PreparedSeries],
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    custom_series: &ChartSeriesRegistry,
    formatters: &ChartFormatterRegistry,
    viewport: ChartViewport,
    axis_windows: &BTreeMap<String, (f64, f64)>,
    axis_domains: &mut BTreeMap<String, ChartAxisDomain>,
) -> Result<(), ChartPrepareError> {
    let mut groups = BTreeMap::<(String, String), Vec<&PreparedSeries>>::new();
    for series in series {
        groups
            .entry((
                resolved_axis_key(spec, &series.spec, ChartChannel::X),
                resolved_axis_key(spec, &series.spec, ChartChannel::Y),
            ))
            .or_default()
            .push(*series);
    }
    let groups = groups
        .into_iter()
        .filter(|(_, group)| {
            group
                .iter()
                .any(|series| !series.semantic_dataset.is_empty())
        })
        .collect::<Vec<_>>();
    if groups.is_empty() {
        if let Some(series) = series.first() {
            push_diagnostic(
                diagnostics,
                ChartDiagnostic {
                    severity: ChartDiagnosticSeverity::Warning,
                    code: "chart.empty".to_owned(),
                    message: format!(
                        "coordinate region `{}` has no transformed rows",
                        series.spec.coordinate
                    ),
                    series: None,
                    datum: None,
                },
            );
        }
        return Ok(());
    }
    let region = &groups[0].1[0].spec.coordinate;
    let primary_axis = |channel| {
        spec.axes
            .iter()
            .filter(|axis| axis.region == *region)
            .find(|axis| {
                let orientation_matches = match channel {
                    ChartChannel::X => matches!(
                        axis.position,
                        ChartAxisPosition::Top | ChartAxisPosition::Bottom
                    ),
                    ChartChannel::Y => matches!(
                        axis.position,
                        ChartAxisPosition::Left | ChartAxisPosition::Right
                    ),
                    _ => false,
                };
                orientation_matches
                    && groups.iter().any(|((x_key, y_key), _)| match channel {
                        ChartChannel::X => x_key == &axis.key,
                        ChartChannel::Y => y_key == &axis.key,
                        _ => false,
                    })
            })
            .map(|axis| axis.key.clone())
    };
    let primary_x = primary_axis(ChartChannel::X).unwrap_or_else(|| groups[0].0.0.clone());
    let primary_y = primary_axis(ChartChannel::Y).unwrap_or_else(|| groups[0].0.1.clone());
    let mut horizontal_plans = BTreeMap::<String, (ChartScale, ChartAxisDomain)>::new();
    let mut vertical_plans = BTreeMap::<String, (ChartScale, ChartAxisDomain)>::new();
    for key in groups
        .iter()
        .map(|(keys, _)| &keys.0)
        .collect::<BTreeSet<_>>()
    {
        let contributors = series
            .iter()
            .copied()
            .filter(|series| resolved_axis_key(spec, &series.spec, ChartChannel::X) == *key)
            .collect::<Vec<_>>();
        let plan = compile_cartesian_axis(
            spec,
            &contributors,
            ChartChannel::X,
            key,
            bounds,
            viewport,
            axis_windows.get(&format!("{region}:x:{key}")).copied(),
        )?;
        axis_domains.insert(format!("{region}:x:{key}"), plan.1);
        horizontal_plans.insert(key.clone(), plan);
    }
    for key in groups
        .iter()
        .map(|(keys, _)| &keys.1)
        .collect::<BTreeSet<_>>()
    {
        let contributors = series
            .iter()
            .copied()
            .filter(|series| resolved_axis_key(spec, &series.spec, ChartChannel::Y) == *key)
            .collect::<Vec<_>>();
        let plan = compile_cartesian_axis(
            spec,
            &contributors,
            ChartChannel::Y,
            key,
            bounds,
            viewport,
            axis_windows.get(&format!("{region}:y:{key}")).copied(),
        )?;
        axis_domains.insert(format!("{region}:y:{key}"), plan.1);
        vertical_plans.insert(key.clone(), plan);
    }
    let mut drawn_x = BTreeSet::new();
    let mut drawn_y = BTreeSet::new();
    for ((x_key, y_key), group) in groups {
        let (x_scale, _) = &horizontal_plans[&x_key];
        let (y_scale, y_domain) = &vertical_plans[&y_key];
        layout_cartesian_group(
            spec,
            &group,
            x_scale,
            y_scale,
            *y_domain,
            bounds,
            theme,
            marks,
            labels,
            diagnostics,
            custom_series,
            formatters,
            drawn_x.insert(x_key.clone()),
            drawn_y.insert(y_key.clone()),
            &x_key,
            &y_key,
        )?;
    }
    if spec
        .annotations
        .iter()
        .any(|annotation| annotation.region == *region)
    {
        let x_scale = &horizontal_plans[&primary_x].0;
        let y_scale = &vertical_plans[&primary_y].0;
        layout_cartesian_annotations(spec, region, x_scale, y_scale, bounds, theme, marks, labels);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_cartesian_group(
    spec: &ChartSpec,
    series: &[&PreparedSeries],
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    y_domain: ChartAxisDomain,
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    custom_series: &ChartSeriesRegistry,
    formatters: &ChartFormatterRegistry,
    render_horizontal_axis: bool,
    render_vertical_axis: bool,
    x_identity: &str,
    y_identity: &str,
) -> Result<(), ChartPrepareError> {
    if series.is_empty() {
        return Ok(());
    }
    if series
        .iter()
        .all(|series| series.semantic_dataset.is_empty())
    {
        push_diagnostic(
            diagnostics,
            ChartDiagnostic {
                severity: ChartDiagnosticSeverity::Warning,
                code: "chart.empty".to_owned(),
                message: format!(
                    "coordinate region `{}` has no transformed rows",
                    series[0].spec.coordinate
                ),
                series: None,
                datum: None,
            },
        );
        return Ok(());
    }
    let x_axis = spec.axes.iter().find(|axis| axis.key == x_identity);
    let y_axis = spec.axes.iter().find(|axis| axis.key == y_identity);
    add_cartesian_axes(
        &series[0].spec.coordinate,
        x_scale,
        y_scale,
        x_axis,
        y_axis,
        bounds,
        theme,
        formatters,
        marks,
        labels,
        render_horizontal_axis,
        render_vertical_axis,
    )?;
    let bar_slots = series
        .iter()
        .filter(|series| series.spec.kind == ChartSeriesKind::Bar)
        .map(|series| {
            series.spec.stack.as_ref().map_or_else(
                || format!("series:{}", series.spec.key),
                |stack| format!("stack:{stack}"),
            )
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect::<BTreeMap<_, _>>();
    let slot_count = bar_slots.len().max(1);
    let mut positive_stacks = BTreeMap::<(String, String), f64>::new();
    let mut negative_stacks = BTreeMap::<(String, String), f64>::new();
    for prepared in series {
        let color = series_color(prepared, theme);
        match prepared.spec.kind {
            ChartSeriesKind::Bar => {
                let x_name = prepared.spec.encode.dimension(ChartChannel::X).unwrap();
                let y_name = prepared.spec.encode.dimension(ChartChannel::Y).unwrap();
                let x = prepared.dataset.column(x_name).unwrap();
                let y = prepared.dataset.column(y_name).unwrap();
                let band = x_scale
                    .band_width()
                    .unwrap_or(bounds.width / usize_to_f64(prepared.dataset.len().max(1)));
                let stack_key = prepared.spec.stack.clone();
                let slot_key = stack_key.as_ref().map_or_else(
                    || format!("series:{}", prepared.spec.key),
                    |stack| format!("stack:{stack}"),
                );
                let slot = bar_slots.get(&slot_key).copied().unwrap_or(0);
                let series_bar_width = band * 0.72 / usize_to_f64(slot_count);
                for row in 0..prepared.dataset.len() {
                    let Some(x_value) = x.value(row) else {
                        continue;
                    };
                    let Some(value) = y.value(row).and_then(|value| value.as_number()) else {
                        datum_diagnostic(
                            diagnostics,
                            &prepared.spec,
                            &prepared.dataset,
                            row,
                            "non_numeric_y",
                        );
                        continue;
                    };
                    let category = x_value.display_text();
                    let Some(center) = map_value(x_scale, &x_value) else {
                        continue;
                    };
                    let (start, end) = if let Some(stack) = &stack_key {
                        let running = if value >= 0.0 {
                            positive_stacks
                                .entry((stack.clone(), category))
                                .or_default()
                        } else {
                            negative_stacks
                                .entry((stack.clone(), category))
                                .or_default()
                        };
                        let start = *running;
                        *running += value;
                        (start, *running)
                    } else {
                        let baseline = if matches!(y_scale, ChartScale::Log(_)) {
                            y_domain.visible.0
                        } else {
                            0.0
                        };
                        (baseline, value)
                    };
                    let Some(y_start) = y_scale.map_number(start) else {
                        continue;
                    };
                    let Some(y_end) = y_scale.map_number(end) else {
                        continue;
                    };
                    let offset = (usize_to_f64(slot) - (usize_to_f64(slot_count) - 1.0) / 2.0)
                        * series_bar_width;
                    add_rect_mark(
                        marks,
                        &prepared.spec,
                        &prepared.dataset,
                        row,
                        ChartRect {
                            x: center - series_bar_width / 2.0 + offset,
                            y: y_start.min(y_end),
                            width: series_bar_width.max(1.0),
                            height: (y_end - y_start).abs().max(1.0),
                        },
                        Some(if value >= 0.0 {
                            datum_color(prepared, row, theme, color)
                        } else {
                            theme.negative
                        }),
                        None,
                        value,
                    );
                }
            }
            ChartSeriesKind::Line | ChartSeriesKind::Area => {
                let segments = series_segments(prepared, x_scale, y_scale, diagnostics);
                for (segment_index, points) in segments.into_iter().enumerate() {
                    if points.is_empty() {
                        continue;
                    }
                    let line_points = if prepared.spec.smooth {
                        smooth_polyline(&points.iter().map(|point| point.0).collect::<Vec<_>>(), 8)
                    } else {
                        points.iter().map(|point| point.0).collect::<Vec<_>>()
                    };
                    if prepared.spec.kind == ChartSeriesKind::Area && line_points.len() >= 2 {
                        let baseline = y_scale.map_number(0.0).unwrap_or(bounds.y + bounds.height);
                        let mut polygon = Vec::with_capacity(line_points.len() + 2);
                        polygon.push(ChartPoint {
                            x: line_points[0].x,
                            y: baseline,
                        });
                        polygon.extend(line_points.iter().copied());
                        polygon.push(ChartPoint {
                            x: line_points.last().unwrap().x,
                            y: baseline,
                        });
                        marks.push(ChartMark {
                            key: mark_identity(
                                "decoration",
                                &prepared.spec.coordinate,
                                &prepared.spec.key,
                                "",
                                &format!("area:{segment_index}"),
                            ),
                            region_key: prepared.spec.coordinate.clone(),
                            role: ChartMarkRole::Decoration,
                            datum: None,
                            series_key: prepared.spec.key.clone(),
                            datum_key: "area".to_owned(),
                            geometry: ChartMarkGeometry::Polygon(polygon.into()),
                            fill: Some(with_alpha(color, 0x44)),
                            stroke: None,
                            label: prepared.spec.name.clone(),
                            value: None,
                            interactive: false,
                            selected: false,
                        });
                    }
                    if line_points.len() >= 2 {
                        marks.push(ChartMark {
                            key: mark_identity(
                                "decoration",
                                &prepared.spec.coordinate,
                                &prepared.spec.key,
                                "",
                                &format!("line:{segment_index}"),
                            ),
                            region_key: prepared.spec.coordinate.clone(),
                            role: ChartMarkRole::Decoration,
                            datum: None,
                            series_key: prepared.spec.key.clone(),
                            datum_key: "line".to_owned(),
                            geometry: ChartMarkGeometry::Polyline {
                                points: line_points.into(),
                                width: 2.0,
                            },
                            fill: None,
                            stroke: Some((color, 2.0)),
                            label: prepared.spec.name.clone(),
                            value: None,
                            interactive: false,
                            selected: false,
                        });
                    }
                    for (point, row, value) in points {
                        add_circle_mark(
                            marks,
                            &prepared.spec,
                            &prepared.dataset,
                            row,
                            point,
                            4.0,
                            datum_color(prepared, row, theme, color),
                            value,
                        );
                    }
                }
            }
            ChartSeriesKind::Scatter => {
                let points = series_segments(prepared, x_scale, y_scale, diagnostics)
                    .into_iter()
                    .flatten();
                let size_column = prepared
                    .spec
                    .encode
                    .dimension(ChartChannel::Size)
                    .and_then(|name| prepared.dataset.column(name));
                for (point, row, value) in points {
                    let radius = size_column
                        .and_then(|column| column.value(row))
                        .and_then(|value| value.as_number())
                        .map_or(5.0, |value| value.abs().sqrt().clamp(3.0, 18.0));
                    add_circle_mark(
                        marks,
                        &prepared.spec,
                        &prepared.dataset,
                        row,
                        point,
                        radius,
                        datum_color(prepared, row, theme, color),
                        value,
                    );
                }
            }
            ChartSeriesKind::Heatmap => layout_heatmap(
                prepared,
                x_scale,
                y_scale,
                bounds,
                theme,
                marks,
                diagnostics,
            ),
            ChartSeriesKind::Candlestick => layout_candlestick(
                prepared,
                x_scale,
                y_scale,
                bounds,
                theme,
                marks,
                diagnostics,
            ),
            ChartSeriesKind::Pie
            | ChartSeriesKind::Donut
            | ChartSeriesKind::Map
            | ChartSeriesKind::GeoScatter
            | ChartSeriesKind::GeoLines
            | ChartSeriesKind::Radar
            | ChartSeriesKind::Gauge
            | ChartSeriesKind::Funnel => {}
            ChartSeriesKind::Custom => {
                let renderer = prepared
                    .spec
                    .renderer
                    .as_deref()
                    .expect("validated custom series renderer");
                marks.extend(custom_series.layout(
                    renderer,
                    super::ChartCustomSeriesContext {
                        spec: &prepared.spec,
                        dataset: &prepared.dataset,
                        bounds,
                        theme,
                        x_scale: Some(x_scale),
                        y_scale: Some(y_scale),
                        coordinate: super::ChartCustomCoordinateContext::Cartesian {
                            x: x_scale,
                            y: y_scale,
                        },
                    },
                )?);
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn layout_cartesian_annotations(
    spec: &ChartSpec,
    region: &str,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
) {
    let label_line = typography_pixels(theme.label_typography.line_height, 16.0);
    for annotation in spec.annotations.iter().filter(|item| item.region == region) {
        let color = annotation
            .color
            .as_deref()
            .and_then(parse_hex_color)
            .unwrap_or(theme.selection);
        match &annotation.kind {
            ChartAnnotationKind::MarkPoint { x, y } => {
                let (Some(x), Some(y)) = (
                    map_annotation_value(x_scale, x),
                    map_annotation_value(y_scale, y),
                ) else {
                    continue;
                };
                marks.push(ChartMark {
                    key: format!("{region}:annotation:{}", annotation.key),
                    region_key: region.to_owned(),
                    role: ChartMarkRole::Annotation,
                    datum: None,
                    series_key: "__annotation".to_owned(),
                    datum_key: annotation.key.clone(),
                    geometry: ChartMarkGeometry::Circle {
                        center: ChartPoint { x, y },
                        radius: 6.0,
                    },
                    fill: Some(color),
                    stroke: Some((theme.background, 2.0)),
                    label: annotation
                        .label
                        .clone()
                        .unwrap_or_else(|| annotation.key.clone()),
                    value: None,
                    interactive: true,
                    selected: false,
                });
                if let Some(label) = &annotation.label {
                    labels.push(ChartLabel {
                        key: format!("annotation-label:{}", annotation.key),
                        position: ChartPoint {
                            x: x + 9.0,
                            y: y - label_line,
                        },
                        text: label.clone(),
                        color,
                        anchor: ChartLabelAnchor::Start,
                        role: ChartLabelRole::Label,
                    });
                }
            }
            ChartAnnotationKind::MarkLine { axis, value }
            | ChartAnnotationKind::Baseline { axis, value } => {
                let geometry = annotation_line(*axis, value, x_scale, y_scale, bounds);
                if let Some((start, end)) = geometry {
                    let mut mark = line_mark(
                        format!("{region}:annotation:{}", annotation.key),
                        region,
                        ChartMarkRole::Annotation,
                        start,
                        end,
                        color,
                        1.5,
                    );
                    "__annotation".clone_into(&mut mark.series_key);
                    mark.datum_key.clone_from(&annotation.key);
                    mark.label = annotation
                        .label
                        .clone()
                        .unwrap_or_else(|| annotation.key.clone());
                    mark.interactive = true;
                    marks.push(mark);
                }
            }
            ChartAnnotationKind::MarkArea {
                x_min,
                x_max,
                y_min,
                y_max,
            } => {
                let values = (
                    map_annotation_value(x_scale, x_min),
                    map_annotation_value(x_scale, x_max),
                    map_annotation_value(y_scale, y_min),
                    map_annotation_value(y_scale, y_max),
                );
                if let (Some(x_min), Some(x_max), Some(y_min), Some(y_max)) = values {
                    marks.push(annotation_area(
                        annotation,
                        ChartRect {
                            x: x_min.min(x_max),
                            y: y_min.min(y_max),
                            width: (x_max - x_min).abs(),
                            height: (y_max - y_min).abs(),
                        },
                        color,
                    ));
                }
            }
            ChartAnnotationKind::ThresholdBand { axis, min, max } => {
                let values = match axis {
                    ChartChannel::X => (
                        map_annotation_value(x_scale, min),
                        map_annotation_value(x_scale, max),
                    ),
                    ChartChannel::Y => (
                        map_annotation_value(y_scale, min),
                        map_annotation_value(y_scale, max),
                    ),
                    _ => (None, None),
                };
                if let (Some(min), Some(max)) = values {
                    let rect = if *axis == ChartChannel::X {
                        ChartRect {
                            x: min.min(max),
                            y: bounds.y,
                            width: (max - min).abs(),
                            height: bounds.height,
                        }
                    } else {
                        ChartRect {
                            x: bounds.x,
                            y: min.min(max),
                            width: bounds.width,
                            height: (max - min).abs(),
                        }
                    };
                    marks.push(annotation_area(annotation, rect, color));
                }
            }
        }
    }
}

fn map_annotation_value(scale: &ChartScale, value: &ChartAnnotationValue) -> Option<f64> {
    match (scale, value) {
        (ChartScale::Category(_), ChartAnnotationValue::Number(value)) => {
            scale.map_category(&value.to_string())
        }
        (_, ChartAnnotationValue::Number(value)) => scale.map_number(*value),
        (_, ChartAnnotationValue::Category(value)) => scale.map_category(value),
    }
}

fn annotation_line(
    axis: ChartChannel,
    value: &ChartAnnotationValue,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    bounds: ChartRect,
) -> Option<(ChartPoint, ChartPoint)> {
    match axis {
        ChartChannel::X => {
            let x = map_annotation_value(x_scale, value)?;
            Some((
                ChartPoint { x, y: bounds.y },
                ChartPoint {
                    x,
                    y: bounds.y + bounds.height,
                },
            ))
        }
        ChartChannel::Y => {
            let y = map_annotation_value(y_scale, value)?;
            Some((
                ChartPoint { x: bounds.x, y },
                ChartPoint {
                    x: bounds.x + bounds.width,
                    y,
                },
            ))
        }
        _ => None,
    }
}

fn annotation_area(
    annotation: &super::ChartAnnotation,
    rect: ChartRect,
    color: Rgba8,
) -> ChartMark {
    ChartMark {
        key: format!("{}:annotation:{}", annotation.region, annotation.key),
        region_key: annotation.region.clone(),
        role: ChartMarkRole::Annotation,
        datum: None,
        series_key: "__annotation".to_owned(),
        datum_key: annotation.key.clone(),
        geometry: ChartMarkGeometry::Rect(rect),
        fill: Some(with_alpha(color, 0x28)),
        stroke: Some((color, 1.0)),
        label: annotation
            .label
            .clone()
            .unwrap_or_else(|| annotation.key.clone()),
        value: None,
        interactive: true,
        selected: false,
    }
}

fn build_continuous_scale(
    axis: Option<&super::ChartAxisSpec>,
    inferred: (f64, f64),
    range_start: f64,
    range_end: f64,
) -> Result<ChartScale, ChartScaleError> {
    let (mut min, mut max) = inferred;
    if (min - max).abs() <= f64::EPSILON {
        let padding = min.abs().max(1.0) * 0.05;
        min -= padding;
        max += padding;
    }
    let direction = axis.map_or(ChartAxisDirection::Normal, |axis| axis.direction);
    match axis.map_or(ChartAxisScale::Linear, |axis| axis.scale) {
        ChartAxisScale::Log => ChartScale::logarithmic(min, max, range_start, range_end, direction),
        ChartAxisScale::Time => ChartScale::time_with_zone(
            f64_to_i64(min),
            f64_to_i64(max),
            range_start,
            range_end,
            direction,
            axis.map_or(super::ChartTimeZone::Utc, |axis| axis.timezone.clone()),
        ),
        ChartAxisScale::Auto | ChartAxisScale::Linear | ChartAxisScale::Category => {
            ChartScale::linear(min, max, range_start, range_end, direction)
        }
    }
}

fn resolve_axis_domain(axis: Option<&super::ChartAxisSpec>, inferred: (f64, f64)) -> (f64, f64) {
    axis.map_or(inferred, |axis| {
        (
            axis.min.unwrap_or(inferred.0),
            axis.max.unwrap_or(inferred.1),
        )
    })
}

fn add_cartesian_axes(
    region: &str,
    x: &ChartScale,
    y: &ChartScale,
    x_axis: Option<&super::ChartAxisSpec>,
    y_axis: Option<&super::ChartAxisSpec>,
    bounds: ChartRect,
    theme: &ChartTheme,
    formatters: &ChartFormatterRegistry,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    draw_x: bool,
    draw_y: bool,
) -> Result<(), ChartPrepareError> {
    let label_line = typography_pixels(theme.label_typography.line_height, 16.0);
    let x_key = x_axis.map_or("x", |axis| axis.key.as_str());
    let y_key = y_axis.map_or("y", |axis| axis.key.as_str());
    let y_right = y_axis.is_some_and(|axis| axis.position == ChartAxisPosition::Right);
    let x_top = x_axis.is_some_and(|axis| axis.position == ChartAxisPosition::Top);
    if draw_y {
        for (index, tick) in y.ticks(6).into_iter().enumerate() {
            marks.push(line_mark(
                format!("{region}:grid:{y_key}:{index}"),
                region,
                ChartMarkRole::Grid,
                ChartPoint {
                    x: bounds.x,
                    y: tick.position,
                },
                ChartPoint {
                    x: bounds.x + bounds.width,
                    y: tick.position,
                },
                theme.grid,
                1.0,
            ));
            labels.push(ChartLabel {
                key: format!("axis:{region}:{y_key}:{index}"),
                position: ChartPoint {
                    x: if y_right {
                        bounds.x + bounds.width + 8.0
                    } else {
                        bounds.x - 8.0
                    },
                    y: tick.position - label_line / 2.0,
                },
                text: format_axis_tick(&tick, y, y_axis, theme, formatters)?,
                color: theme.muted_text,
                anchor: if y_right {
                    ChartLabelAnchor::Start
                } else {
                    ChartLabelAnchor::End
                },
                role: ChartLabelRole::Label,
            });
        }
        marks.push(line_mark(
            format!("{region}:axis:{y_key}"),
            region,
            ChartMarkRole::Axis,
            ChartPoint {
                x: if y_right {
                    bounds.x + bounds.width
                } else {
                    bounds.x
                },
                y: bounds.y,
            },
            ChartPoint {
                x: if y_right {
                    bounds.x + bounds.width
                } else {
                    bounds.x
                },
                y: bounds.y + bounds.height,
            },
            theme.axis,
            1.0,
        ));
        if let Some(title) = y_axis.and_then(|axis| axis.title.as_ref()) {
            labels.push(ChartLabel {
                key: format!("axis-title:{region}:{y_key}"),
                position: ChartPoint {
                    x: if y_right {
                        bounds.x + bounds.width + 8.0
                    } else {
                        bounds.x - 8.0
                    },
                    y: bounds.y - label_line - 2.0,
                },
                text: title.clone(),
                color: theme.text,
                anchor: if y_right {
                    ChartLabelAnchor::Start
                } else {
                    ChartLabelAnchor::End
                },
                role: ChartLabelRole::Label,
            });
        }
    }
    if draw_x {
        for (index, tick) in x.ticks(8).into_iter().enumerate() {
            marks.push(line_mark(
                format!("{region}:grid:{x_key}:{index}"),
                region,
                ChartMarkRole::Grid,
                ChartPoint {
                    x: tick.position,
                    y: bounds.y,
                },
                ChartPoint {
                    x: tick.position,
                    y: bounds.y + bounds.height,
                },
                theme.grid,
                1.0,
            ));
            labels.push(ChartLabel {
                key: format!("axis:{region}:{x_key}:{index}"),
                position: ChartPoint {
                    x: tick.position,
                    y: if x_top {
                        bounds.y - label_line - 2.0
                    } else {
                        bounds.y + bounds.height + 6.0
                    },
                },
                text: format_axis_tick(&tick, x, x_axis, theme, formatters)?,
                color: theme.muted_text,
                anchor: ChartLabelAnchor::Center,
                role: ChartLabelRole::Label,
            });
        }
        marks.push(line_mark(
            format!("{region}:axis:{x_key}"),
            region,
            ChartMarkRole::Axis,
            ChartPoint {
                x: bounds.x,
                y: if x_top {
                    bounds.y
                } else {
                    bounds.y + bounds.height
                },
            },
            ChartPoint {
                x: bounds.x + bounds.width,
                y: if x_top {
                    bounds.y
                } else {
                    bounds.y + bounds.height
                },
            },
            theme.axis,
            1.0,
        ));
        if let Some(title) = x_axis.and_then(|axis| axis.title.as_ref()) {
            labels.push(ChartLabel {
                key: format!("axis-title:{region}:{x_key}"),
                position: ChartPoint {
                    x: bounds.x + bounds.width / 2.0,
                    y: if x_top {
                        bounds.y - label_line * 2.0 - 2.0
                    } else {
                        bounds.y + bounds.height + label_line + 8.0
                    },
                },
                text: title.clone(),
                color: theme.text,
                anchor: ChartLabelAnchor::Center,
                role: ChartLabelRole::Label,
            });
        }
    }
    Ok(())
}

fn format_axis_tick(
    tick: &super::ChartTick,
    scale: &ChartScale,
    axis: Option<&super::ChartAxisSpec>,
    theme: &ChartTheme,
    formatters: &ChartFormatterRegistry,
) -> Result<String, ChartFormatterError> {
    let Some(axis) = axis else {
        return Ok(tick.label.clone());
    };
    if matches!(scale, ChartScale::Category(_)) {
        return Ok(tick.label.clone());
    }
    let format = &axis.format;
    if let Some(formatter) = &format.formatter {
        return formatters.format(formatter, tick.value, &theme.locale, format);
    }
    let mut value = if format.percent {
        tick.value * 100.0
    } else {
        tick.value
    };
    if !value.is_finite() {
        value = 0.0;
    }
    let body = if format.compact && value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if format.compact && value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if let Some(number) = &theme.number
        && !matches!(axis.scale, ChartAxisScale::Time)
    {
        crate::format_number_with_metadata(
            value,
            crate::NumberFormatOptions {
                min_fraction_digits: format.precision.unwrap_or(0),
                max_fraction_digits: format.precision.unwrap_or(3),
                grouping: true,
            },
            number,
        )
        .unwrap_or_else(|_| tick.label.clone())
    } else if let Some(precision) = format.precision {
        format!("{value:.precision$}", precision = usize::from(precision))
    } else {
        tick.label.clone()
    };
    Ok(format!(
        "{}{}{}{}",
        format.prefix,
        body,
        if format.percent { "%" } else { "" },
        format.suffix
    ))
}

fn smooth_polyline(points: &[ChartPoint], steps: usize) -> Vec<ChartPoint> {
    if points.len() < 3 || steps == 0 {
        return points.to_vec();
    }
    let mut output = Vec::with_capacity((points.len() - 1) * steps + 1);
    output.push(points[0]);
    for index in 0..points.len() - 1 {
        let first = points[index.saturating_sub(1)];
        let second = points[index];
        let third = points[index + 1];
        let fourth = points[(index + 2).min(points.len() - 1)];
        for step in 1..=steps {
            let t = usize_to_f64(step) / usize_to_f64(steps);
            let t2 = t * t;
            let t3 = t2 * t;
            output.push(ChartPoint {
                x: 0.5
                    * ((2.0 * second.x)
                        + (-first.x + third.x) * t
                        + (2.0 * first.x - 5.0 * second.x + 4.0 * third.x - fourth.x) * t2
                        + (-first.x + 3.0 * second.x - 3.0 * third.x + fourth.x) * t3),
                y: 0.5
                    * ((2.0 * second.y)
                        + (-first.y + third.y) * t
                        + (2.0 * first.y - 5.0 * second.y + 4.0 * third.y - fourth.y) * t2
                        + (-first.y + 3.0 * second.y - 3.0 * third.y + fourth.y) * t3),
            });
        }
    }
    output
}

fn series_segments(
    prepared: &PreparedSeries,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    diagnostics: &mut Vec<ChartDiagnostic>,
) -> Vec<Vec<(ChartPoint, usize, f64)>> {
    let x = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::X).unwrap())
        .unwrap();
    let y = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::Y).unwrap())
        .unwrap();
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for row in 0..prepared.dataset.len() {
        let Some(x_value) = x.value(row) else {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        };
        let Some(y_value) = y.value(row).and_then(|value| value.as_number()) else {
            datum_diagnostic(
                diagnostics,
                &prepared.spec,
                &prepared.dataset,
                row,
                "non_numeric_y",
            );
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        };
        let Some(mapped_x) = map_value(x_scale, &x_value) else {
            datum_diagnostic(
                diagnostics,
                &prepared.spec,
                &prepared.dataset,
                row,
                "invalid_x",
            );
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        };
        let Some(mapped_y) = y_scale.map_number(y_value) else {
            datum_diagnostic(
                diagnostics,
                &prepared.spec,
                &prepared.dataset,
                row,
                "invalid_y",
            );
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        };
        current.push((
            ChartPoint {
                x: mapped_x,
                y: mapped_y,
            },
            row,
            y_value,
        ));
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

#[allow(clippy::too_many_arguments)]
fn layout_heatmap(
    prepared: &PreparedSeries,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    diagnostics: &mut Vec<ChartDiagnostic>,
) {
    let x = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::X).unwrap())
        .unwrap();
    let y = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::Y).unwrap())
        .unwrap();
    let value = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::Value).unwrap())
        .unwrap();
    let values = (0..prepared.dataset.len())
        .filter_map(|row| value.value(row).and_then(|value| value.as_number()))
        .collect::<Vec<_>>();
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let cell_width = x_scale
        .band_width()
        .unwrap_or((bounds.width / 20.0).max(4.0));
    let cell_height = y_scale
        .band_width()
        .unwrap_or((bounds.height / 20.0).max(4.0));
    for row in 0..prepared.dataset.len() {
        let Some(x) = x.value(row).and_then(|value| map_value(x_scale, &value)) else {
            continue;
        };
        let Some(y) = y.value(row).and_then(|value| map_value(y_scale, &value)) else {
            continue;
        };
        let Some(value) = value.value(row).and_then(|value| value.as_number()) else {
            datum_diagnostic(
                diagnostics,
                &prepared.spec,
                &prepared.dataset,
                row,
                "non_numeric_value",
            );
            continue;
        };
        let ratio = normalize(value, min, max);
        add_rect_mark(
            marks,
            &prepared.spec,
            &prepared.dataset,
            row,
            ChartRect {
                x: x - cell_width * 0.45,
                y: y - cell_height * 0.45,
                width: cell_width * 0.9,
                height: cell_height * 0.9,
            },
            Some(mix_color(theme.map_missing, theme.positive, ratio)),
            None,
            value,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn layout_candlestick(
    prepared: &PreparedSeries,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    diagnostics: &mut Vec<ChartDiagnostic>,
) {
    let column = |channel| {
        prepared
            .dataset
            .column(prepared.spec.encode.dimension(channel).unwrap())
            .unwrap()
    };
    let x = column(ChartChannel::X);
    let open = column(ChartChannel::Open);
    let close = column(ChartChannel::Close);
    let low = column(ChartChannel::Low);
    let high = column(ChartChannel::High);
    let width = x_scale
        .band_width()
        .unwrap_or(bounds.width / usize_to_f64(prepared.dataset.len().max(1)))
        * 0.58;
    for row in 0..prepared.dataset.len() {
        let Some(center) = x.value(row).and_then(|value| map_value(x_scale, &value)) else {
            continue;
        };
        let values = [open, close, low, high]
            .map(|column| column.value(row).and_then(|value| value.as_number()));
        let [Some(open), Some(close), Some(low), Some(high)] = values else {
            datum_diagnostic(
                diagnostics,
                &prepared.spec,
                &prepared.dataset,
                row,
                "invalid_candlestick",
            );
            continue;
        };
        let [Some(open_y), Some(close_y), Some(low_y), Some(high_y)] =
            [open, close, low, high].map(|value| y_scale.map_number(value))
        else {
            continue;
        };
        let color = if close >= open {
            theme.positive
        } else {
            theme.negative
        };
        marks.push(line_mark(
            mark_identity(
                "decoration",
                &prepared.spec.coordinate,
                &prepared.spec.key,
                &datum_key(&prepared.dataset, row),
                "wick",
            ),
            &prepared.spec.coordinate,
            ChartMarkRole::Decoration,
            ChartPoint {
                x: center,
                y: high_y,
            },
            ChartPoint {
                x: center,
                y: low_y,
            },
            color,
            1.0,
        ));
        add_rect_mark(
            marks,
            &prepared.spec,
            &prepared.dataset,
            row,
            ChartRect {
                x: center - width / 2.0,
                y: open_y.min(close_y),
                width: width.max(1.0),
                height: (close_y - open_y).abs().max(1.0),
            },
            Some(with_alpha(color, 0xcc)),
            Some((color, 1.0)),
            close,
        );
    }
}

fn radar_indicator_order(series: &[&PreparedSeries]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut indicators = Vec::new();
    for prepared in series
        .iter()
        .filter(|series| series.spec.kind == ChartSeriesKind::Radar)
    {
        let Some(name) = prepared
            .spec
            .encode
            .dimension(ChartChannel::Name)
            .and_then(|dimension| prepared.dataset.column(dimension))
        else {
            continue;
        };
        for row in 0..prepared.dataset.len() {
            if let Some(indicator) = name.value(row).map(|value| value.display_text())
                && seen.insert(indicator.clone())
            {
                indicators.push(indicator);
            }
        }
    }
    indicators
}

fn polar_value_domain(
    series: &[&PreparedSeries],
    kind: ChartSeriesKind,
    axis: Option<&super::ChartAxisSpec>,
) -> (f64, f64) {
    let mut min = axis.and_then(|axis| axis.min);
    let mut max = axis.and_then(|axis| axis.max);
    for prepared in series.iter().filter(|series| series.spec.kind == kind) {
        let Some(value) = prepared
            .spec
            .encode
            .dimension(ChartChannel::Value)
            .and_then(|dimension| prepared.dataset.column(dimension))
        else {
            continue;
        };
        for row in 0..prepared.dataset.len() {
            if let Some(value) = value.value(row).and_then(|value| value.as_number()) {
                if axis.and_then(|axis| axis.min).is_none() {
                    min = Some(min.map_or(value, |current| current.min(value)));
                }
                if axis.and_then(|axis| axis.max).is_none() {
                    max = Some(max.map_or(value, |current| current.max(value)));
                }
            }
        }
    }
    let mut min = min.unwrap_or(0.0);
    let mut max = max.unwrap_or_else(|| {
        if kind == ChartSeriesKind::Gauge {
            100.0
        } else {
            1.0
        }
    });
    if (max - min).abs() <= f64::EPSILON {
        if kind == ChartSeriesKind::Gauge {
            min = 0.0;
            max = 100.0_f64.max(max);
        } else {
            max = min + min.abs().max(1.0);
        }
    }
    (min, max)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_polar(
    spec: &ChartSpec,
    region: &str,
    series: &[&PreparedSeries],
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    custom_series: &ChartSeriesRegistry,
) -> Result<(), ChartPrepareError> {
    let center = bounds.center();
    let radius = bounds.width.min(bounds.height) * 0.42;
    let radial_axis = spec
        .axes
        .iter()
        .find(|axis| axis.region == region && axis.position == ChartAxisPosition::Radial);
    let radar_indicators = radar_indicator_order(series);
    let radar_domain = polar_value_domain(series, ChartSeriesKind::Radar, radial_axis);
    for prepared in series {
        let color = series_color(prepared, theme);
        if prepared.spec.kind == ChartSeriesKind::Custom {
            let renderer = prepared
                .spec
                .renderer
                .as_deref()
                .expect("validated custom series renderer");
            marks.extend(custom_series.layout(
                renderer,
                super::ChartCustomSeriesContext {
                    spec: &prepared.spec,
                    dataset: &prepared.dataset,
                    bounds,
                    theme,
                    x_scale: None,
                    y_scale: None,
                    coordinate: super::ChartCustomCoordinateContext::Polar {
                        center,
                        radius,
                        value_domain: (
                            radial_axis.and_then(|axis| axis.min).unwrap_or(0.0),
                            radial_axis.and_then(|axis| axis.max).unwrap_or(1.0),
                        ),
                    },
                },
            )?);
            continue;
        }
        let name = prepared
            .spec
            .encode
            .dimension(ChartChannel::Name)
            .and_then(|name| prepared.dataset.column(name));
        let value = prepared
            .spec
            .encode
            .dimension(ChartChannel::Value)
            .and_then(|name| prepared.dataset.column(name));
        let Some(value) = value else { continue };
        match prepared.spec.kind {
            ChartSeriesKind::Pie | ChartSeriesKind::Donut => {
                let values = (0..prepared.dataset.len())
                    .map(|row| {
                        value
                            .value(row)
                            .and_then(|value| value.as_number())
                            .unwrap_or(0.0)
                            .max(0.0)
                    })
                    .collect::<Vec<_>>();
                let total = values.iter().sum::<f64>();
                if total <= 0.0 {
                    continue;
                }
                let mut angle = -std::f64::consts::FRAC_PI_2;
                for (row, value) in values.into_iter().enumerate() {
                    if value <= 0.0 {
                        continue;
                    }
                    let sweep = value / total * std::f64::consts::TAU;
                    let inner = if prepared.spec.kind == ChartSeriesKind::Donut {
                        radius * 0.55
                    } else {
                        0.0
                    };
                    let polygon = arc_polygon(
                        center,
                        inner,
                        radius,
                        angle,
                        angle + sweep,
                        theme.motion_quality,
                    );
                    let datum_color = datum_color(
                        prepared,
                        row,
                        theme,
                        theme.palette[(prepared.color_index + row) % theme.palette.len()],
                    );
                    let datum = datum_key(&prepared.dataset, row);
                    marks.push(ChartMark {
                        key: mark_identity(
                            "data",
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            &datum,
                            "sector",
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: ChartMarkRole::Data,
                        datum: Some(ChartDatumRef {
                            dataset: prepared.spec.dataset.clone(),
                            series: prepared.spec.key.clone(),
                            key: datum.clone(),
                        }),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum,
                        geometry: ChartMarkGeometry::Polygon(polygon.into()),
                        fill: Some(datum_color),
                        stroke: Some((theme.background, 1.0)),
                        label: datum_label(name, &prepared.dataset, row, &prepared.spec.name),
                        value: Some(value),
                        interactive: true,
                        selected: false,
                    });
                    angle += sweep;
                }
            }
            ChartSeriesKind::Radar => {
                let values = (0..prepared.dataset.len())
                    .filter_map(|row| {
                        Some((
                            name.and_then(|column| column.value(row))?.display_text(),
                            value.value(row)?.as_number()?,
                        ))
                    })
                    .collect::<BTreeMap<_, _>>();
                let count = radar_indicators.len().max(1);
                let points = radar_indicators
                    .iter()
                    .enumerate()
                    .map(|(index, indicator)| {
                        let value = values.get(indicator).copied().unwrap_or(radar_domain.0);
                        let angle = -std::f64::consts::FRAC_PI_2
                            + std::f64::consts::TAU * usize_to_f64(index) / usize_to_f64(count);
                        polar_point(
                            center,
                            radius * normalize(value, radar_domain.0, radar_domain.1),
                            angle,
                        )
                    })
                    .collect::<Vec<_>>();
                if points.len() >= 3 {
                    marks.push(ChartMark {
                        key: mark_identity(
                            "decoration",
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            "",
                            "radar",
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: ChartMarkRole::Decoration,
                        datum: None,
                        series_key: prepared.spec.key.clone(),
                        datum_key: "radar".to_owned(),
                        geometry: ChartMarkGeometry::Polygon(points.into()),
                        fill: Some(with_alpha(color, 0x44)),
                        stroke: Some((color, 2.0)),
                        label: prepared.spec.name.clone(),
                        value: None,
                        interactive: false,
                        selected: false,
                    });
                }
            }
            ChartSeriesKind::Gauge => {
                let current = value
                    .value(0)
                    .and_then(|value| value.as_number())
                    .unwrap_or(0.0);
                let domain = polar_value_domain(
                    std::slice::from_ref(prepared),
                    ChartSeriesKind::Gauge,
                    radial_axis,
                );
                let start = std::f64::consts::PI * 0.75;
                let sweep = std::f64::consts::PI * 1.5;
                let business_key = datum_key(&prepared.dataset, 0);
                for (key, end, fill) in [
                    ("track", start + sweep, with_alpha(theme.axis, 0x55)),
                    (
                        "value",
                        start + sweep * normalize(current, domain.0, domain.1),
                        color,
                    ),
                ] {
                    let is_value = key == "value";
                    marks.push(ChartMark {
                        key: mark_identity(
                            if is_value { "data" } else { "decoration" },
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            if is_value { &business_key } else { "" },
                            if is_value {
                                "gauge_value"
                            } else {
                                "gauge_track"
                            },
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: if is_value {
                            ChartMarkRole::Data
                        } else {
                            ChartMarkRole::Decoration
                        },
                        datum: is_value.then(|| ChartDatumRef {
                            dataset: prepared.spec.dataset.clone(),
                            series: prepared.spec.key.clone(),
                            key: business_key.clone(),
                        }),
                        series_key: prepared.spec.key.clone(),
                        datum_key: if is_value {
                            business_key.clone()
                        } else {
                            "gauge_track".to_owned()
                        },
                        geometry: ChartMarkGeometry::Polygon(
                            arc_polygon(
                                center,
                                radius * 0.68,
                                radius,
                                start,
                                end,
                                theme.motion_quality,
                            )
                            .into(),
                        ),
                        fill: Some(fill),
                        stroke: None,
                        label: prepared.spec.name.clone(),
                        value: Some(current),
                        interactive: is_value,
                        selected: false,
                    });
                }
                labels.push(ChartLabel {
                    key: format!("{}:gauge-label", prepared.spec.key),
                    position: ChartPoint {
                        x: center.x,
                        y: center.y
                            - typography_pixels(theme.label_typography.line_height, 16.0) / 2.0,
                    },
                    text: current.to_string(),
                    color: theme.text,
                    anchor: ChartLabelAnchor::Center,
                    role: ChartLabelRole::Label,
                });
            }
            ChartSeriesKind::Funnel => {
                let rows = (0..prepared.dataset.len())
                    .filter_map(|row| {
                        value
                            .value(row)
                            .and_then(|value| value.as_number())
                            .map(|value| (row, value.max(0.0)))
                    })
                    .collect::<Vec<_>>();
                let max = rows
                    .iter()
                    .map(|row| row.1)
                    .reduce(f64::max)
                    .unwrap_or(1.0)
                    .max(1.0);
                let row_height = bounds.height / usize_to_f64(rows.len().max(1));
                for (index, (row, value)) in rows.iter().copied().enumerate() {
                    let top_width = bounds.width * value / max;
                    let next_width = rows
                        .get(index + 1)
                        .map_or(top_width, |next| bounds.width * next.1 / max);
                    let y = bounds.y + usize_to_f64(index) * row_height;
                    let polygon = vec![
                        ChartPoint {
                            x: center.x - top_width / 2.0,
                            y,
                        },
                        ChartPoint {
                            x: center.x + top_width / 2.0,
                            y,
                        },
                        ChartPoint {
                            x: center.x + next_width / 2.0,
                            y: y + row_height * 0.9,
                        },
                        ChartPoint {
                            x: center.x - next_width / 2.0,
                            y: y + row_height * 0.9,
                        },
                    ];
                    let datum = datum_key(&prepared.dataset, row);
                    marks.push(ChartMark {
                        key: mark_identity(
                            "data",
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            &datum,
                            "funnel",
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: ChartMarkRole::Data,
                        datum: Some(ChartDatumRef {
                            dataset: prepared.spec.dataset.clone(),
                            series: prepared.spec.key.clone(),
                            key: datum.clone(),
                        }),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum,
                        geometry: ChartMarkGeometry::Polygon(polygon.into()),
                        fill: Some(datum_color(
                            prepared,
                            row,
                            theme,
                            theme.palette[(prepared.color_index + index) % theme.palette.len()],
                        )),
                        stroke: None,
                        label: datum_label(name, &prepared.dataset, row, &prepared.spec.name),
                        value: Some(value),
                        interactive: true,
                        selected: false,
                    });
                }
            }
            ChartSeriesKind::Custom => unreachable!("custom series are dispatched above"),
            _ => {
                push_diagnostic(
                    diagnostics,
                    ChartDiagnostic {
                        severity: ChartDiagnosticSeverity::Warning,
                        code: "chart.unsupported_polar_series".to_owned(),
                        message: format!("series `{}` cannot render in Polar", prepared.spec.key),
                        series: Some(prepared.spec.key.clone()),
                        datum: None,
                    },
                );
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_geo(
    series: &[&PreparedSeries],
    map: &ChartGeoMap,
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    _labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    projection: Option<&dyn super::ChartGeoProjection>,
    custom_series: &ChartSeriesRegistry,
) -> Result<(), ChartPrepareError> {
    let scale = geo_scale(map, bounds);
    for prepared in series {
        let color = series_color(prepared, theme);
        match prepared.spec.kind {
            ChartSeriesKind::Map => {
                let name = prepared
                    .dataset
                    .column(prepared.spec.encode.dimension(ChartChannel::Name).unwrap())
                    .unwrap();
                let value = prepared
                    .dataset
                    .column(prepared.spec.encode.dimension(ChartChannel::Value).unwrap())
                    .unwrap();
                let values = (0..prepared.dataset.len())
                    .filter_map(|row| {
                        Some((
                            name.value(row)?.display_text(),
                            (
                                value.value(row)?.as_number()?,
                                datum_key(&prepared.dataset, row),
                            ),
                        ))
                    })
                    .collect::<BTreeMap<_, _>>();
                let min = values
                    .values()
                    .map(|(value, _)| *value)
                    .reduce(f64::min)
                    .unwrap_or(0.0);
                let max = values
                    .values()
                    .map(|(value, _)| *value)
                    .reduce(f64::max)
                    .unwrap_or(1.0);
                for feature in map.features.iter() {
                    let matched = values
                        .get(&feature.key)
                        .or_else(|| values.get(&feature.name))
                        .cloned();
                    let feature_value = matched.as_ref().map(|(value, _)| *value);
                    let business_key = matched.as_ref().map(|(_, key)| key.clone());
                    let rings = feature
                        .rings
                        .iter()
                        .map(|ring| {
                            ring.points
                                .iter()
                                .map(|point| scale_geo_point(*point, scale))
                                .collect::<Vec<_>>()
                                .into()
                        })
                        .collect::<Vec<Arc<[ChartPoint]>>>();
                    marks.push(ChartMark {
                        key: mark_identity(
                            if business_key.is_some() {
                                "data"
                            } else {
                                "decoration"
                            },
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            business_key.as_deref().unwrap_or(""),
                            &format!("map_feature:{}", feature.key),
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: if business_key.is_some() {
                            ChartMarkRole::Data
                        } else {
                            ChartMarkRole::Decoration
                        },
                        datum: business_key.as_ref().map(|key| ChartDatumRef {
                            dataset: prepared.spec.dataset.clone(),
                            series: prepared.spec.key.clone(),
                            key: key.clone(),
                        }),
                        series_key: prepared.spec.key.clone(),
                        datum_key: business_key
                            .clone()
                            .unwrap_or_else(|| format!("map_feature:{}", feature.key)),
                        geometry: ChartMarkGeometry::CompoundPolygon(rings.into()),
                        fill: Some(feature_value.map_or(theme.map_missing, |value| {
                            mix_color(theme.map_missing, color, normalize(value, min, max))
                        })),
                        stroke: Some((theme.background, 0.75)),
                        label: feature.name.clone(),
                        value: feature_value,
                        interactive: business_key.is_some(),
                        selected: false,
                    });
                }
            }
            ChartSeriesKind::GeoScatter => {
                let longitude = prepared
                    .dataset
                    .column(
                        prepared
                            .spec
                            .encode
                            .dimension(ChartChannel::Longitude)
                            .unwrap(),
                    )
                    .unwrap();
                let latitude = prepared
                    .dataset
                    .column(
                        prepared
                            .spec
                            .encode
                            .dimension(ChartChannel::Latitude)
                            .unwrap(),
                    )
                    .unwrap();
                let value = prepared
                    .spec
                    .encode
                    .dimension(ChartChannel::Value)
                    .and_then(|name| prepared.dataset.column(name));
                for row in 0..prepared.dataset.len() {
                    let Some(x) = longitude.value(row).and_then(|value| value.as_number()) else {
                        continue;
                    };
                    let Some(y) = latitude.value(row).and_then(|value| value.as_number()) else {
                        continue;
                    };
                    let point = if let Some(projection) = projection {
                        let Some(point) = projection.project(x, y) else {
                            datum_diagnostic(
                                diagnostics,
                                &prepared.spec,
                                &prepared.dataset,
                                row,
                                "projection_rejected",
                            );
                            continue;
                        };
                        point
                    } else {
                        ChartGeoPoint { x, y }
                    };
                    let point = scale_geo_point(point, scale);
                    let numeric = value
                        .and_then(|column| column.value(row))
                        .and_then(|value| value.as_number());
                    add_circle_mark(
                        marks,
                        &prepared.spec,
                        &prepared.dataset,
                        row,
                        point,
                        numeric.map_or(5.0, |value| value.abs().sqrt().clamp(3.0, 16.0)),
                        color,
                        numeric.unwrap_or(0.0),
                    );
                }
            }
            ChartSeriesKind::GeoLines => {
                let column = |channel| {
                    prepared
                        .dataset
                        .column(prepared.spec.encode.dimension(channel).unwrap())
                        .unwrap()
                };
                let sx = column(ChartChannel::SourceLongitude);
                let sy = column(ChartChannel::SourceLatitude);
                let tx = column(ChartChannel::TargetLongitude);
                let ty = column(ChartChannel::TargetLatitude);
                for row in 0..prepared.dataset.len() {
                    let values = [sx, sy, tx, ty]
                        .map(|column| column.value(row).and_then(|value| value.as_number()));
                    let [Some(sx), Some(sy), Some(tx), Some(ty)] = values else {
                        continue;
                    };
                    let (start, end) = if let Some(projection) = projection {
                        let (Some(start), Some(end)) =
                            (projection.project(sx, sy), projection.project(tx, ty))
                        else {
                            datum_diagnostic(
                                diagnostics,
                                &prepared.spec,
                                &prepared.dataset,
                                row,
                                "projection_rejected",
                            );
                            continue;
                        };
                        (start, end)
                    } else {
                        (
                            ChartGeoPoint { x: sx, y: sy },
                            ChartGeoPoint { x: tx, y: ty },
                        )
                    };
                    let start = scale_geo_point(start, scale);
                    let end = scale_geo_point(end, scale);
                    let datum = datum_key(&prepared.dataset, row);
                    marks.push(ChartMark {
                        key: mark_identity(
                            "data",
                            &prepared.spec.coordinate,
                            &prepared.spec.key,
                            &datum,
                            "geo_line",
                        ),
                        region_key: prepared.spec.coordinate.clone(),
                        role: ChartMarkRole::Data,
                        datum: Some(ChartDatumRef {
                            dataset: prepared.spec.dataset.clone(),
                            series: prepared.spec.key.clone(),
                            key: datum.clone(),
                        }),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum,
                        geometry: ChartMarkGeometry::Polyline {
                            points: vec![start, end].into(),
                            width: 1.5,
                        },
                        fill: None,
                        stroke: Some((color, 1.5)),
                        label: prepared.spec.name.clone(),
                        value: None,
                        interactive: true,
                        selected: false,
                    });
                }
            }
            ChartSeriesKind::Custom => {
                let renderer = prepared
                    .spec
                    .renderer
                    .as_deref()
                    .expect("validated custom series renderer");
                marks.extend(custom_series.layout(
                    renderer,
                    super::ChartCustomSeriesContext {
                        spec: &prepared.spec,
                        dataset: &prepared.dataset,
                        bounds,
                        theme,
                        x_scale: None,
                        y_scale: None,
                        coordinate: super::ChartCustomCoordinateContext::Geo { projection, scale },
                    },
                )?);
            }
            _ => push_diagnostic(
                diagnostics,
                ChartDiagnostic {
                    severity: ChartDiagnosticSeverity::Warning,
                    code: "chart.unsupported_geo_series".to_owned(),
                    message: format!("series `{}` cannot render in Geo2D", prepared.spec.key),
                    series: Some(prepared.spec.key.clone()),
                    datum: None,
                },
            ),
        }
    }
    Ok(())
}

fn apply_geo_viewport(marks: &mut [ChartMark], bounds: ChartRect, viewport: ChartViewport) {
    if (viewport.zoom - 1.0).abs() <= f64::EPSILON && viewport.pan == ChartPoint::default() {
        return;
    }
    let center = bounds.center();
    let map = |point: ChartPoint| ChartPoint {
        x: center.x + (point.x - center.x) * viewport.zoom + viewport.pan.x,
        y: center.y + (point.y - center.y) * viewport.zoom + viewport.pan.y,
    };
    for mark in marks {
        mark.geometry = match &mark.geometry {
            ChartMarkGeometry::Rect(rect) => {
                let origin = map(ChartPoint {
                    x: rect.x,
                    y: rect.y,
                });
                ChartMarkGeometry::Rect(ChartRect {
                    x: origin.x,
                    y: origin.y,
                    width: rect.width * viewport.zoom,
                    height: rect.height * viewport.zoom,
                })
            }
            ChartMarkGeometry::Circle { center, radius } => ChartMarkGeometry::Circle {
                center: map(*center),
                radius: radius * viewport.zoom,
            },
            ChartMarkGeometry::Polyline { points, width } => ChartMarkGeometry::Polyline {
                points: points.iter().copied().map(map).collect::<Vec<_>>().into(),
                width: *width * viewport.zoom,
            },
            ChartMarkGeometry::Polygon(points) => ChartMarkGeometry::Polygon(
                points.iter().copied().map(map).collect::<Vec<_>>().into(),
            ),
            ChartMarkGeometry::CompoundPolygon(rings) => ChartMarkGeometry::CompoundPolygon(
                rings
                    .iter()
                    .map(|ring| ring.iter().copied().map(map).collect::<Vec<_>>().into())
                    .collect::<Vec<Arc<[ChartPoint]>>>()
                    .into(),
            ),
        };
    }
}

fn collect_categories(series: &[&PreparedSeries], channel: ChartChannel) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut categories = Vec::new();
    for prepared in series {
        let Some(column) = prepared
            .spec
            .encode
            .dimension(channel)
            .and_then(|name| prepared.semantic_dataset.column(name))
        else {
            continue;
        };
        for row in 0..prepared.semantic_dataset.len() {
            if let Some(value) = column.value(row) {
                let value = value.display_text();
                if seen.insert(value.clone()) {
                    categories.push(value);
                }
            }
        }
    }
    categories
}

fn numeric_domain(
    series: &[&PreparedSeries],
    channel: ChartChannel,
    include_zero: bool,
) -> Option<(f64, f64)> {
    let mut min = include_zero.then_some(0.0);
    let mut max = include_zero.then_some(0.0);
    for prepared in series {
        let dimensions: &[ChartChannel] =
            if channel == ChartChannel::Y && prepared.spec.kind == ChartSeriesKind::Candlestick {
                &[
                    ChartChannel::Open,
                    ChartChannel::Close,
                    ChartChannel::Low,
                    ChartChannel::High,
                ]
            } else if channel == ChartChannel::Y && prepared.spec.kind == ChartSeriesKind::Heatmap {
                &[ChartChannel::Y]
            } else {
                std::slice::from_ref(&channel)
            };
        for dimension in dimensions {
            let Some(column) = prepared
                .spec
                .encode
                .dimension(*dimension)
                .and_then(|name| prepared.semantic_dataset.column(name))
            else {
                continue;
            };
            for row in 0..prepared.semantic_dataset.len() {
                if let Some(value) = column.value(row).and_then(|value| value.as_number()) {
                    min = Some(min.map_or(value, |min: f64| min.min(value)));
                    max = Some(max.map_or(value, |max: f64| max.max(value)));
                }
            }
        }
    }
    min.zip(max)
}

fn cartesian_y_domain(series: &[&PreparedSeries], include_zero: bool) -> Option<(f64, f64)> {
    let mut min = include_zero.then_some(0.0);
    let mut max = include_zero.then_some(0.0);
    let mut positive = BTreeMap::<(String, String), f64>::new();
    let mut negative = BTreeMap::<(String, String), f64>::new();
    for prepared in series {
        if prepared.spec.kind == ChartSeriesKind::Bar
            && let Some(stack) = &prepared.spec.stack
            && let (Some(x_name), Some(y_name)) = (
                prepared.spec.encode.dimension(ChartChannel::X),
                prepared.spec.encode.dimension(ChartChannel::Y),
            )
            && let (Some(x), Some(y)) = (
                prepared.semantic_dataset.column(x_name),
                prepared.semantic_dataset.column(y_name),
            )
        {
            for row in 0..prepared.semantic_dataset.len() {
                let Some(category) = x.value(row).map(|value| value.display_text()) else {
                    continue;
                };
                let Some(value) = y.value(row).and_then(|value| value.as_number()) else {
                    continue;
                };
                let running = if value >= 0.0 {
                    positive.entry((stack.clone(), category)).or_default()
                } else {
                    negative.entry((stack.clone(), category)).or_default()
                };
                *running += value;
                min = Some(min.map_or(*running, |current| current.min(*running)));
                max = Some(max.map_or(*running, |current| current.max(*running)));
            }
        } else if let Some((series_min, series_max)) =
            numeric_domain(std::slice::from_ref(prepared), ChartChannel::Y, false)
        {
            min = Some(min.map_or(series_min, |current| current.min(series_min)));
            max = Some(max.map_or(series_max, |current| current.max(series_max)));
        }
    }
    min.zip(max)
}

fn direction_adjusted_pan(pan: f64, direction: ChartAxisDirection) -> f64 {
    if direction == ChartAxisDirection::Reversed {
        -pan
    } else {
        pan
    }
}

fn viewport_domain(
    domain: (f64, f64),
    zoom: f64,
    pan_pixels: f64,
    range_delta: f64,
    scale: ChartAxisScale,
) -> (f64, f64) {
    let zoom = if zoom.is_finite() {
        zoom.clamp(0.5, 20.0)
    } else {
        1.0
    };
    if scale == ChartAxisScale::Log {
        if domain.0 <= 0.0 || domain.1 <= domain.0 {
            return domain;
        }
        let log = (domain.0.ln(), domain.1.ln());
        let visible = viewport_domain(log, zoom, pan_pixels, range_delta, ChartAxisScale::Linear);
        return (visible.0.exp(), visible.1.exp());
    }
    let span = domain.1 - domain.0;
    if !span.is_finite() || span <= f64::EPSILON || range_delta.abs() <= f64::EPSILON {
        return domain;
    }
    let visible = span / zoom;
    let center = domain.0 + span / 2.0 - pan_pixels * span / (range_delta * zoom);
    (center - visible / 2.0, center + visible / 2.0)
}

fn map_value(scale: &ChartScale, value: &ChartValue) -> Option<f64> {
    if matches!(scale, ChartScale::Category(_)) {
        scale.map_category(&value.display_text())
    } else {
        value.as_number().and_then(|value| scale.map_number(value))
    }
}

fn add_rect_mark(
    marks: &mut Vec<ChartMark>,
    spec: &ChartSeriesSpec,
    dataset: &ChartDataset,
    row: usize,
    rect: ChartRect,
    fill: Option<Rgba8>,
    stroke: Option<(Rgba8, f64)>,
    value: f64,
) {
    let datum = datum_key(dataset, row);
    marks.push(ChartMark {
        key: mark_identity("data", &spec.coordinate, &spec.key, &datum, "body"),
        region_key: spec.coordinate.clone(),
        role: ChartMarkRole::Data,
        datum: Some(ChartDatumRef {
            dataset: spec.dataset.clone(),
            series: spec.key.clone(),
            key: datum.clone(),
        }),
        series_key: spec.key.clone(),
        datum_key: datum,
        geometry: ChartMarkGeometry::Rect(rect),
        fill,
        stroke,
        label: datum_label(None, dataset, row, &spec.name),
        value: Some(value),
        interactive: true,
        selected: false,
    });
}

#[allow(clippy::too_many_arguments)]
fn add_circle_mark(
    marks: &mut Vec<ChartMark>,
    spec: &ChartSeriesSpec,
    dataset: &ChartDataset,
    row: usize,
    center: ChartPoint,
    radius: f64,
    fill: Rgba8,
    value: f64,
) {
    let datum = datum_key(dataset, row);
    let geometry = match series_pattern(&spec.key) {
        0 => ChartMarkGeometry::Circle { center, radius },
        1 => ChartMarkGeometry::Rect(ChartRect {
            x: center.x - radius,
            y: center.y - radius,
            width: radius * 2.0,
            height: radius * 2.0,
        }),
        2 => ChartMarkGeometry::Polygon(
            vec![
                ChartPoint {
                    x: center.x,
                    y: center.y - radius,
                },
                ChartPoint {
                    x: center.x + radius,
                    y: center.y,
                },
                ChartPoint {
                    x: center.x,
                    y: center.y + radius,
                },
                ChartPoint {
                    x: center.x - radius,
                    y: center.y,
                },
            ]
            .into(),
        ),
        _ => ChartMarkGeometry::Polygon(
            vec![
                ChartPoint {
                    x: center.x,
                    y: center.y - radius,
                },
                ChartPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                },
                ChartPoint {
                    x: center.x - radius,
                    y: center.y + radius,
                },
            ]
            .into(),
        ),
    };
    marks.push(ChartMark {
        key: mark_identity("data", &spec.coordinate, &spec.key, &datum, "point"),
        region_key: spec.coordinate.clone(),
        role: ChartMarkRole::Data,
        datum: Some(ChartDatumRef {
            dataset: spec.dataset.clone(),
            series: spec.key.clone(),
            key: datum.clone(),
        }),
        series_key: spec.key.clone(),
        datum_key: datum,
        geometry,
        fill: Some(fill),
        stroke: None,
        label: datum_label(None, dataset, row, &spec.name),
        value: Some(value),
        interactive: true,
        selected: false,
    });
}

fn line_mark(
    key: String,
    region: &str,
    role: ChartMarkRole,
    start: ChartPoint,
    end: ChartPoint,
    color: Rgba8,
    width: f64,
) -> ChartMark {
    ChartMark {
        key,
        region_key: region.to_owned(),
        role,
        datum: None,
        series_key: "__axis".to_owned(),
        datum_key: String::new(),
        geometry: ChartMarkGeometry::Polyline {
            points: vec![start, end].into(),
            width,
        },
        fill: None,
        stroke: Some((color, width)),
        label: String::new(),
        value: None,
        interactive: false,
        selected: false,
    }
}

fn datum_key(dataset: &ChartDataset, row: usize) -> String {
    dataset.key(row).unwrap_or_else(|| format!("row_{row}"))
}

fn mark_identity(role: &str, region: &str, series: &str, datum: &str, part: &str) -> String {
    format!(
        "{role}|{}:{region}|{}:{series}|{}:{datum}|{}:{part}",
        region.len(),
        series.len(),
        datum.len(),
        part.len()
    )
}

fn datum_label(
    name: Option<&super::ChartColumn>,
    dataset: &ChartDataset,
    row: usize,
    fallback: &str,
) -> String {
    name.and_then(|column| column.value(row)).map_or_else(
        || dataset.key(row).unwrap_or_else(|| fallback.to_owned()),
        |value| value.display_text(),
    )
}

fn datum_diagnostic(
    diagnostics: &mut Vec<ChartDiagnostic>,
    series: &ChartSeriesSpec,
    dataset: &ChartDataset,
    row: usize,
    code: &str,
) {
    push_diagnostic(
        diagnostics,
        ChartDiagnostic {
            severity: ChartDiagnosticSeverity::Warning,
            code: format!("chart.datum.{code}"),
            message: format!("series `{}` skipped invalid datum at row {row}", series.key),
            series: Some(series.key.clone()),
            datum: Some(datum_key(dataset, row)),
        },
    );
}

fn push_diagnostic(diagnostics: &mut Vec<ChartDiagnostic>, diagnostic: ChartDiagnostic) {
    if diagnostics.len() < MAX_DIAGNOSTICS {
        diagnostics.push(diagnostic);
    }
}

fn series_color(prepared: &PreparedSeries, theme: &ChartTheme) -> Rgba8 {
    prepared
        .spec
        .color
        .as_deref()
        .and_then(parse_hex_color)
        .unwrap_or_else(|| theme.palette[prepared.color_index % theme.palette.len()])
}

fn datum_color(
    prepared: &PreparedSeries,
    row: usize,
    theme: &ChartTheme,
    fallback: Rgba8,
) -> Rgba8 {
    let Some(value) = prepared
        .spec
        .encode
        .dimension(ChartChannel::Color)
        .and_then(|dimension| prepared.dataset.column(dimension))
        .and_then(|column| column.value(row))
    else {
        return fallback;
    };
    let identity = value.display_text();
    parse_hex_color(&identity).unwrap_or_else(|| {
        let index = identity.as_bytes().iter().fold(0_usize, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(usize::from(*byte))
        });
        theme.palette[index % theme.palette.len()]
    })
}

fn parse_hex_color(value: &str) -> Option<Rgba8> {
    let value = value.strip_prefix('#')?;
    match value.len() {
        6 => u32::from_str_radix(value, 16)
            .ok()
            .map(|value| Rgba8::from_rgb_hex((value << 8) | 0xff)),
        8 => u32::from_str_radix(value, 16)
            .ok()
            .map(Rgba8::from_rgba_hex),
        _ => None,
    }
}

fn arc_polygon(
    center: ChartPoint,
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
    quality: crate::MotionQuality,
) -> Vec<ChartPoint> {
    let steps = match quality {
        crate::MotionQuality::Low => 24_usize,
        crate::MotionQuality::Medium => 40_usize,
        crate::MotionQuality::High => 64_usize,
    };
    let mut points = (0..=steps)
        .map(|step| {
            polar_point(
                center,
                outer,
                start + (end - start) * usize_to_f64(step) / usize_to_f64(steps),
            )
        })
        .collect::<Vec<_>>();
    if inner > 0.0 {
        points.extend((0..=steps).rev().map(|step| {
            polar_point(
                center,
                inner,
                start + (end - start) * usize_to_f64(step) / usize_to_f64(steps),
            )
        }));
    } else {
        points.push(center);
    }
    points
}

fn polar_point(center: ChartPoint, radius: f64, angle: f64) -> ChartPoint {
    ChartPoint {
        x: center.x + angle.cos() * radius,
        y: center.y + angle.sin() * radius,
    }
}

fn geo_scale(map: &ChartGeoMap, bounds: ChartRect) -> (f64, f64, f64) {
    let scale = (bounds.width / map.bounds.width()).min(bounds.height / map.bounds.height());
    let x = bounds.x + (bounds.width - map.bounds.width() * scale) / 2.0 - map.bounds.min_x * scale;
    let y =
        bounds.y + (bounds.height - map.bounds.height() * scale) / 2.0 - map.bounds.min_y * scale;
    (scale, x, y)
}

fn scale_geo_point(point: ChartGeoPoint, scale: (f64, f64, f64)) -> ChartPoint {
    ChartPoint {
        x: point.x * scale.0 + scale.1,
        y: point.y * scale.0 + scale.2,
    }
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    if max > min {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn mix_color(left: Rgba8, right: Rgba8, ratio: f64) -> Rgba8 {
    let left = left.as_rgba_hex();
    let right = right.as_rgba_hex();
    let mix = |shift: u32| {
        let left = f64::from(((left >> shift) & 0xff_u32) as u8);
        let right = f64::from(((right >> shift) & 0xff_u32) as u8);
        ((left + (right - left) * ratio).round().clamp(0.0, 255.0) as u32) << shift
    };
    Rgba8::from_rgba_hex(mix(24) | mix(16) | mix(8) | mix(0))
}

fn with_alpha(color: Rgba8, alpha: u8) -> Rgba8 {
    Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | u32::from(alpha))
}

pub(crate) fn series_pattern(key: &str) -> usize {
    if key.starts_with("__") {
        return 0;
    }
    key.as_bytes().iter().fold(0_usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(usize::from(*byte))
    }) % 4
}

pub(crate) fn mark_hit_test(mark: &ChartMark, point: ChartPoint) -> bool {
    match &mark.geometry {
        ChartMarkGeometry::Rect(rect) => rect.contains(point),
        ChartMarkGeometry::Circle { center, radius } => {
            (point.x - center.x).hypot(point.y - center.y) <= *radius + 3.0
        }
        ChartMarkGeometry::Polygon(points) => point_in_polygon(point, points),
        ChartMarkGeometry::CompoundPolygon(rings) => {
            rings
                .iter()
                .filter(|ring| point_in_polygon(point, ring))
                .count()
                % 2
                == 1
        }
        ChartMarkGeometry::Polyline { points, width } => points
            .windows(2)
            .any(|segment| distance_to_segment(point, segment[0], segment[1]) <= width / 2.0 + 4.0),
    }
}

fn point_in_polygon(point: ChartPoint, polygon: &[ChartPoint]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let left = polygon[current];
        let right = polygon[previous];
        if (left.y > point.y) != (right.y > point.y)
            && point.x < (right.x - left.x) * (point.y - left.y) / (right.y - left.y) + left.x
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn geometry_vertex_count(geometry: &ChartMarkGeometry) -> usize {
    match geometry {
        ChartMarkGeometry::Rect(_) => 4,
        ChartMarkGeometry::Circle { .. } => 32,
        ChartMarkGeometry::Polyline { points, .. } | ChartMarkGeometry::Polygon(points) => {
            points.len()
        }
        ChartMarkGeometry::CompoundPolygon(rings) => rings.iter().map(|ring| ring.len()).sum(),
    }
}

fn distance_to_segment(point: ChartPoint, start: ChartPoint, end: ChartPoint) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return (point.x - start.x).hypot(point.y - start.y);
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    (point.x - (start.x + t * dx)).hypot(point.y - (start.y + t * dy))
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_i64(value: f64) -> i64 {
    value.round() as i64
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartPrepareError {
    #[error(transparent)]
    Spec(#[from] ChartSpecError),
    #[error(transparent)]
    Data(#[from] ChartDataError),
    #[error(transparent)]
    Transform(#[from] ChartTransformError),
    #[error(transparent)]
    Scale(#[from] ChartScaleError),
    #[error(transparent)]
    Geo(#[from] ChartGeoError),
    #[error(transparent)]
    CustomSeries(#[from] ChartSeriesExtensionError),
    #[error(transparent)]
    Formatter(#[from] ChartFormatterError),
    #[error("chart scene contains duplicate mark identity `{0}`")]
    DuplicateMarkIdentity(String),
    #[error("chart scene exceeds resource budget: {marks} marks, {vertices} vertices")]
    SceneBudget { marks: usize, vertices: usize },
    #[error("chart dataset `{0}` does not exist")]
    MissingDataset(String),
    #[error("chart series `{series}` encodes missing dimension `{dimension}`")]
    MissingDimension { series: String, dimension: String },
    #[error("chart Geo2D region `{0}` has no map")]
    MissingGeoMap(String),
    #[error("chart viewport {width}x{height} must be finite and positive")]
    InvalidViewport { width: f64, height: f64 },
}

#[cfg(test)]
mod tests {
    use super::super::{
        ChartBrushMode, ChartCoordinateRegion, ChartDataLimits, ChartEncode, ChartLegendSpec,
        ChartMotionSpec, ChartTooltipSpec,
    };
    use super::*;

    fn dataset(count: usize) -> ChartDataset {
        let rows = (0..count)
            .map(|index| {
                BTreeMap::from([
                    ("id".to_owned(), ChartValue::String(format!("r{index}"))),
                    (
                        "category".to_owned(),
                        ChartValue::String(format!("c{index}")),
                    ),
                    (
                        "value".to_owned(),
                        ChartValue::Number(usize_to_f64(index + 1)),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        ChartDataset::from_chart_rows(
            "main",
            &rows,
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap()
    }

    fn spec(kind: ChartSeriesKind) -> ChartSpec {
        let encode = match kind {
            ChartSeriesKind::Bar => ChartEncode::new([
                (ChartChannel::X, "category".to_owned()),
                (ChartChannel::Y, "value".to_owned()),
            ]),
            ChartSeriesKind::Pie => ChartEncode::new([
                (ChartChannel::Name, "category".to_owned()),
                (ChartChannel::Value, "value".to_owned()),
            ]),
            _ => unreachable!(),
        };
        let region = ChartCoordinateRegion {
            kind: kind.default_coordinate(),
            ..ChartCoordinateRegion::default()
        };
        ChartSpec {
            title: Some("Test".to_owned()),
            description: None,
            regions: vec![region],
            axes: Vec::new(),
            series: vec![ChartSeriesSpec {
                key: "series".to_owned(),
                name: "Series".to_owned(),
                kind,
                dataset: "main".to_owned(),
                coordinate: "main".to_owned(),
                x_axis: None,
                y_axis: None,
                encode,
                stack: None,
                color: None,
                visible: true,
                smooth: false,
                transforms: Vec::new(),
                renderer: None,
                options: crate::UiValue::Null,
            }],
            legend: ChartLegendSpec::default(),
            tooltip: ChartTooltipSpec::default(),
            motion: ChartMotionSpec::default(),
            brush: ChartBrushMode::None,
            annotations: Vec::new(),
            link_group: None,
            link_domain: None,
            interaction: crate::ChartInteractionSpec::default(),
        }
    }

    #[test]
    fn bar_scene_uses_stable_original_keys_for_hit_testing_and_semantics() {
        let data = super::super::NativeChartData::new([dataset(4)], ChartDataLimits::default())
            .unwrap()
            .snapshot();
        let prepared = prepare_chart_data(
            spec(ChartSeriesKind::Bar),
            &data,
            &ChartTransformRegistry::new(),
            &ChartSeriesRegistry::new(),
            &ChartFormatterRegistry::new(),
        )
        .unwrap();
        let scene = layout_chart_scene(
            &prepared,
            800.0,
            500.0,
            &ChartTheme::default(),
            &ChartGeoRegistry::new(),
        )
        .unwrap();
        let mark = scene
            .marks
            .iter()
            .find(|mark| mark.datum_key == "r2")
            .unwrap();
        let point = match mark.geometry {
            ChartMarkGeometry::Rect(rect) => rect.center(),
            _ => panic!("bar is not rect"),
        };
        assert_eq!(
            scene.hit_test(point).map(|mark| mark.datum_key.as_str()),
            Some("r2")
        );
        assert!(
            scene
                .semantic_projection(10)
                .iter()
                .any(|datum| datum.datum_key == "r2")
        );
    }

    #[test]
    fn pie_scene_produces_distinct_keyed_sector_geometry() {
        let data = super::super::NativeChartData::new([dataset(4)], ChartDataLimits::default())
            .unwrap()
            .snapshot();
        let prepared = prepare_chart_data(
            spec(ChartSeriesKind::Pie),
            &data,
            &ChartTransformRegistry::new(),
            &ChartSeriesRegistry::new(),
            &ChartFormatterRegistry::new(),
        )
        .unwrap();
        let scene = layout_chart_scene(
            &prepared,
            600.0,
            500.0,
            &ChartTheme::default(),
            &ChartGeoRegistry::new(),
        )
        .unwrap();
        let sectors = scene
            .marks
            .iter()
            .filter(|mark| {
                mark.series_key == "series"
                    && mark.interactive
                    && matches!(mark.geometry, ChartMarkGeometry::Polygon(_))
            })
            .collect::<Vec<_>>();
        assert_eq!(sectors.len(), 4);
        assert_eq!(
            sectors
                .iter()
                .map(|mark| mark.datum_key.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
    }

    #[test]
    fn complete_builtin_series_matrix_produces_real_scene_geometry() {
        let rows = (0..4)
            .map(|index| {
                let value = usize_to_f64(index + 1);
                BTreeMap::from([
                    ("id".to_owned(), ChartValue::String(format!("r{index}"))),
                    (
                        "category".to_owned(),
                        ChartValue::String(if index < 2 { "north" } else { "south" }.to_owned()),
                    ),
                    (
                        "indicator".to_owned(),
                        ChartValue::String(format!("metric_{index}")),
                    ),
                    ("x".to_owned(), ChartValue::Number(value)),
                    ("y".to_owned(), ChartValue::Number(value * 2.0)),
                    ("value".to_owned(), ChartValue::Number(value * 3.0)),
                    ("open".to_owned(), ChartValue::Number(value + 2.0)),
                    ("close".to_owned(), ChartValue::Number(value + 3.0)),
                    ("low".to_owned(), ChartValue::Number(value)),
                    ("high".to_owned(), ChartValue::Number(value + 5.0)),
                    ("lon".to_owned(), ChartValue::Number(value + 1.0)),
                    ("lat".to_owned(), ChartValue::Number(value + 1.0)),
                    ("source_lon".to_owned(), ChartValue::Number(1.0)),
                    ("source_lat".to_owned(), ChartValue::Number(1.0)),
                    ("target_lon".to_owned(), ChartValue::Number(8.0)),
                    ("target_lat".to_owned(), ChartValue::Number(8.0)),
                ])
            })
            .collect::<Vec<_>>();
        let dataset = ChartDataset::from_chart_rows(
            "main",
            &rows,
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let data = super::super::NativeChartData::new([dataset], ChartDataLimits::default())
            .unwrap()
            .snapshot();
        let geo = ChartGeoRegistry::new();
        geo.register_map(
            super::super::ChartGeoMap::from_geojson(
                "demo",
                r#"{"type":"FeatureCollection","features":[
                  {"type":"Feature","id":"north","geometry":{"type":"Polygon","coordinates":[[[0,5],[10,5],[10,10],[0,10],[0,5]]]}},
                  {"type":"Feature","id":"south","geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,5],[0,5],[0,0]]]}}
                ]}"#,
            )
            .unwrap(),
        )
        .unwrap();
        for kind in [
            ChartSeriesKind::Bar,
            ChartSeriesKind::Line,
            ChartSeriesKind::Area,
            ChartSeriesKind::Scatter,
            ChartSeriesKind::Pie,
            ChartSeriesKind::Donut,
            ChartSeriesKind::Heatmap,
            ChartSeriesKind::Map,
            ChartSeriesKind::GeoScatter,
            ChartSeriesKind::GeoLines,
            ChartSeriesKind::Candlestick,
            ChartSeriesKind::Radar,
            ChartSeriesKind::Gauge,
            ChartSeriesKind::Funnel,
        ] {
            let encode = match kind {
                ChartSeriesKind::Bar
                | ChartSeriesKind::Line
                | ChartSeriesKind::Area
                | ChartSeriesKind::Scatter => ChartEncode::new([
                    (ChartChannel::X, "x".to_owned()),
                    (ChartChannel::Y, "y".to_owned()),
                ]),
                ChartSeriesKind::Heatmap => ChartEncode::new([
                    (ChartChannel::X, "x".to_owned()),
                    (ChartChannel::Y, "y".to_owned()),
                    (ChartChannel::Value, "value".to_owned()),
                ]),
                ChartSeriesKind::Radar => ChartEncode::new([
                    (ChartChannel::Name, "indicator".to_owned()),
                    (ChartChannel::Value, "value".to_owned()),
                ]),
                ChartSeriesKind::Pie
                | ChartSeriesKind::Donut
                | ChartSeriesKind::Gauge
                | ChartSeriesKind::Funnel
                | ChartSeriesKind::Map => ChartEncode::new([
                    (ChartChannel::Name, "category".to_owned()),
                    (ChartChannel::Value, "value".to_owned()),
                ]),
                ChartSeriesKind::GeoScatter => ChartEncode::new([
                    (ChartChannel::Longitude, "lon".to_owned()),
                    (ChartChannel::Latitude, "lat".to_owned()),
                    (ChartChannel::Value, "value".to_owned()),
                ]),
                ChartSeriesKind::GeoLines => ChartEncode::new([
                    (ChartChannel::SourceLongitude, "source_lon".to_owned()),
                    (ChartChannel::SourceLatitude, "source_lat".to_owned()),
                    (ChartChannel::TargetLongitude, "target_lon".to_owned()),
                    (ChartChannel::TargetLatitude, "target_lat".to_owned()),
                ]),
                ChartSeriesKind::Candlestick => ChartEncode::new([
                    (ChartChannel::X, "x".to_owned()),
                    (ChartChannel::Open, "open".to_owned()),
                    (ChartChannel::Close, "close".to_owned()),
                    (ChartChannel::Low, "low".to_owned()),
                    (ChartChannel::High, "high".to_owned()),
                ]),
                ChartSeriesKind::Custom => unreachable!(),
            };
            let mut region = ChartCoordinateRegion {
                kind: kind.default_coordinate(),
                ..ChartCoordinateRegion::default()
            };
            if region.kind == ChartCoordinateKind::Geo2d {
                region.map = Some("demo".to_owned());
                region.projection = Some("equirectangular".to_owned());
            }
            let spec = ChartSpec {
                title: None,
                description: None,
                regions: vec![region],
                axes: Vec::new(),
                series: vec![ChartSeriesSpec {
                    key: "matrix".to_owned(),
                    name: format!("{kind:?}"),
                    kind,
                    dataset: "main".to_owned(),
                    coordinate: "main".to_owned(),
                    x_axis: None,
                    y_axis: None,
                    encode,
                    stack: None,
                    color: None,
                    visible: true,
                    smooth: true,
                    transforms: Vec::new(),
                    renderer: None,
                    options: crate::UiValue::Null,
                }],
                legend: ChartLegendSpec::default(),
                tooltip: ChartTooltipSpec::default(),
                motion: ChartMotionSpec::default(),
                brush: ChartBrushMode::None,
                annotations: Vec::new(),
                link_group: None,
                link_domain: None,
                interaction: crate::ChartInteractionSpec::default(),
            };
            let prepared = prepare_chart_data(
                spec,
                &data,
                &ChartTransformRegistry::new(),
                &ChartSeriesRegistry::new(),
                &ChartFormatterRegistry::new(),
            )
            .unwrap();
            let scene =
                layout_chart_scene(&prepared, 640.0, 400.0, &ChartTheme::default(), &geo).unwrap();
            assert!(
                scene
                    .marks
                    .iter()
                    .any(|mark| mark.series_key == "matrix" && mark.datum_key != "legend"),
                "{kind:?} produced no series geometry: {:?}",
                scene.diagnostics
            );
        }
    }

    #[test]
    fn sampled_motion_scene_is_the_hit_test_scene() {
        let mark = |x| ChartMark {
            key: "series:datum".to_owned(),
            region_key: "main".to_owned(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: "main".to_owned(),
                series: "series".to_owned(),
                key: "datum".to_owned(),
            }),
            series_key: "series".to_owned(),
            datum_key: "datum".to_owned(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x,
                y: 10.0,
                width: 20.0,
                height: 20.0,
            }),
            fill: Some(Rgba8::from_rgb_hex(0x0011_2233)),
            stroke: None,
            label: "Datum".to_owned(),
            value: Some(1.0),
            interactive: true,
            selected: false,
        };
        let scene = |mark| PreparedChartScene {
            revision: 1,
            width: 200.0,
            height: 100.0,
            plot_regions: BTreeMap::from([(
                "main".to_owned(),
                ChartRect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0,
                },
            )]),
            axis_domains: BTreeMap::new(),
            marks: vec![mark].into(),
            labels: Vec::new().into(),
            semantics: Vec::new().into(),
            diagnostics: Vec::new().into(),
            summary: String::new(),
            sampled_series: Vec::new().into(),
        };
        let previous = scene(mark(0.0));
        let next = scene(mark(100.0));
        let sampled = interpolate_chart_scene(&previous, &next, 0.5);

        assert!(sampled.hit_test(ChartPoint { x: 60.0, y: 20.0 }).is_some());
        assert!(sampled.hit_test(ChartPoint { x: 110.0, y: 20.0 }).is_none());
    }
}
