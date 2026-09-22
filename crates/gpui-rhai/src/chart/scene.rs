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

#[derive(Clone, Debug, PartialEq)]
pub struct ChartMark {
    pub key: String,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartLabelAnchor {
    Start,
    Center,
    End,
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
    pub palette: Arc<[Rgba8]>,
    pub locale: String,
    pub number: Option<crate::NumberMetadata>,
    pub motion_quality: crate::MotionQuality,
    pub direction: crate::TextDirection,
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
        }
    }
}

#[derive(Clone, Debug)]
struct PreparedSeries {
    spec: ChartSeriesSpec,
    dataset: ChartDataset,
    sampled: bool,
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
    pub marks: Arc<[ChartMark]>,
    pub labels: Arc<[ChartLabel]>,
    pub diagnostics: Arc<[ChartDiagnostic]>,
    pub summary: String,
    pub sampled_series: Arc<[String]>,
}

impl PreparedChartScene {
    #[must_use]
    pub fn hit_test(&self, point: ChartPoint) -> Option<&ChartMark> {
        self.marks
            .iter()
            .rev()
            .find(|mark| mark.interactive && mark_hit_test(mark, point))
    }

    #[must_use]
    pub fn semantic_projection(&self, limit: usize) -> Vec<ChartSemanticDatum> {
        self.marks
            .iter()
            .filter(|mark| mark.interactive)
            .take(limit.min(1_000))
            .map(|mark| ChartSemanticDatum {
                series_key: mark.series_key.clone(),
                datum_key: mark.datum_key.clone(),
                name: mark.label.clone(),
                value: mark.value,
                selected: mark.selected,
            })
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
    for series in &spec.series {
        let source = data
            .dataset(&series.dataset)
            .ok_or_else(|| ChartPrepareError::MissingDataset(series.dataset.clone()))?;
        validate_series_dimensions(series, source)?;
        let original_len = source.len();
        let mut dataset = apply_chart_transforms(
            source,
            &series.transforms,
            transforms,
            ChartTransformContext::default(),
        )?;
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
        let explicitly_sampled = series
            .transforms
            .iter()
            .any(|transform| matches!(transform, super::ChartTransformSpec::Downsample { .. }));
        if !explicitly_sampled
            && matches!(series.kind, ChartSeriesKind::Line | ChartSeriesKind::Area)
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
            dataset,
            sampled,
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
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(ChartPrepareError::InvalidViewport { width, height });
    }
    let mut diagnostics = prepared.diagnostics.to_vec();
    let regions = region_rects(&prepared.spec, width, height);
    let mut marks = Vec::new();
    let mut labels = Vec::new();
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
            )?,
            ChartCoordinateKind::Polar => layout_polar(
                &series,
                bounds,
                theme,
                &mut marks,
                &mut labels,
                &mut diagnostics,
                &prepared.custom_series,
            ),
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
                );
            }
        }
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
    Ok(PreparedChartScene {
        revision: prepared.revision,
        width,
        height,
        plot_regions: regions,
        marks: marks.into(),
        labels: labels.into(),
        diagnostics: diagnostics.into(),
        summary,
        sampled_series: prepared
            .sampled_series()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
            .into(),
    })
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

fn region_rects(spec: &ChartSpec, width: f64, height: f64) -> BTreeMap<String, ChartRect> {
    let top = if spec.title.is_some() { 52.0 } else { 24.0 }
        + if spec.legend.visible && matches!(spec.legend.position, super::ChartLegendPosition::Top)
        {
            30.0
        } else {
            0.0
        };
    let bottom = 36.0
        + if spec.legend.visible
            && matches!(spec.legend.position, super::ChartLegendPosition::Bottom)
        {
            30.0
        } else {
            0.0
        };
    let left = 52.0
        + if spec.legend.visible && matches!(spec.legend.position, super::ChartLegendPosition::Left)
        {
            100.0
        } else {
            0.0
        };
    let right = 24.0
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
                    width: f64::from(region.column_span) * column_width - 18.0,
                    height: f64::from(region.row_span) * row_height - 18.0,
                },
            )
        })
        .collect()
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
    for (index, series) in series.iter().enumerate() {
        let color = series_color(&series.spec, index, theme);
        let logical_x = if horizontal && theme.direction == crate::TextDirection::RightToLeft {
            width - cursor - 12.0
        } else {
            cursor
        };
        let position = match spec.legend.position {
            super::ChartLegendPosition::Top => ChartPoint {
                x: logical_x,
                y: if spec.title.is_some() { 42.0 } else { 14.0 },
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
        });
        cursor += if horizontal {
            72.0 + usize_to_f64(series.spec.name.len()) * 4.0
        } else {
            24.0
        };
    }
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
) -> Result<(), ChartPrepareError> {
    let mut groups = BTreeMap::<(String, String), Vec<&PreparedSeries>>::new();
    for series in series {
        groups
            .entry((
                series.spec.x_axis.clone().unwrap_or_default(),
                series.spec.y_axis.clone().unwrap_or_default(),
            ))
            .or_default()
            .push(*series);
    }
    for group in groups.into_values() {
        layout_cartesian_group(
            spec,
            &group,
            bounds,
            theme,
            marks,
            labels,
            diagnostics,
            custom_series,
            formatters,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_cartesian_group(
    spec: &ChartSpec,
    series: &[&PreparedSeries],
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    custom_series: &ChartSeriesRegistry,
    formatters: &ChartFormatterRegistry,
) -> Result<(), ChartPrepareError> {
    if series.is_empty() {
        return Ok(());
    }
    let x_dimension = series[0]
        .spec
        .encode
        .dimension(ChartChannel::X)
        .ok_or_else(|| ChartPrepareError::MissingDimension {
            series: series[0].spec.key.clone(),
            dimension: "x".to_owned(),
        })?;
    let mut x_categorical = series.iter().any(|series| {
        series
            .dataset
            .column(
                series
                    .spec
                    .encode
                    .dimension(ChartChannel::X)
                    .unwrap_or(x_dimension),
            )
            .is_some_and(|column| {
                matches!(
                    column.data_type(),
                    super::ChartDataType::String | super::ChartDataType::Bool
                )
            })
    });
    let mut y_categorical = series.iter().any(|series| {
        series
            .spec
            .encode
            .dimension(ChartChannel::Y)
            .and_then(|name| series.dataset.column(name))
            .is_some_and(|column| {
                matches!(
                    column.data_type(),
                    super::ChartDataType::String | super::ChartDataType::Bool
                )
            })
    });
    let x_axis = series[0]
        .spec
        .x_axis
        .as_ref()
        .and_then(|key| spec.axes.iter().find(|axis| axis.key == *key))
        .or_else(|| {
            spec.axes.iter().find(|axis| {
                axis.region == series[0].spec.coordinate
                    && matches!(
                        axis.position,
                        ChartAxisPosition::Top | ChartAxisPosition::Bottom
                    )
            })
        });
    let y_axis = series[0]
        .spec
        .y_axis
        .as_ref()
        .and_then(|key| spec.axes.iter().find(|axis| axis.key == *key))
        .or_else(|| {
            spec.axes.iter().find(|axis| {
                axis.region == series[0].spec.coordinate
                    && matches!(
                        axis.position,
                        ChartAxisPosition::Left | ChartAxisPosition::Right
                    )
            })
        });
    x_categorical |= x_axis.is_some_and(|axis| axis.scale == ChartAxisScale::Category);
    y_categorical |= y_axis.is_some_and(|axis| axis.scale == ChartAxisScale::Category);
    let categories = if x_categorical {
        collect_categories(series, ChartChannel::X)
    } else {
        Vec::new()
    };
    let y_categories = if y_categorical {
        collect_categories(series, ChartChannel::Y)
    } else {
        Vec::new()
    };
    let x_domain = if x_categorical {
        (0.0, usize_to_f64(categories.len().max(1)))
    } else {
        numeric_domain(series, ChartChannel::X, false).unwrap_or((0.0, 1.0))
    };
    let y_domain = if y_categorical {
        (0.0, usize_to_f64(y_categories.len().max(1)))
    } else {
        numeric_domain(series, ChartChannel::Y, true).unwrap_or((0.0, 1.0))
    };
    let x_scale = if x_categorical {
        ChartScale::category(
            categories.clone(),
            bounds.x,
            bounds.x + bounds.width,
            x_axis.map_or(ChartAxisDirection::Normal, |axis| axis.direction),
        )?
    } else {
        build_continuous_scale(x_axis, x_domain, bounds.x, bounds.x + bounds.width)?
    };
    let y_scale = if y_categorical {
        ChartScale::category(
            y_categories,
            bounds.y + bounds.height,
            bounds.y,
            y_axis.map_or(ChartAxisDirection::Normal, |axis| axis.direction),
        )?
    } else {
        build_continuous_scale(y_axis, y_domain, bounds.y + bounds.height, bounds.y)?
    };
    add_cartesian_axes(
        &x_scale, &y_scale, x_axis, y_axis, bounds, theme, formatters, marks, labels,
    )?;
    layout_cartesian_annotations(
        spec,
        &series[0].spec.coordinate,
        &x_scale,
        &y_scale,
        bounds,
        theme,
        marks,
        labels,
    );

    let bar_series = series
        .iter()
        .filter(|series| series.spec.kind == ChartSeriesKind::Bar)
        .count()
        .max(1);
    let mut bar_index = 0_usize;
    let mut stacks = BTreeMap::<(String, String), f64>::new();
    for (series_index, prepared) in series.iter().enumerate() {
        let color = series_color(&prepared.spec, series_index, theme);
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
                let series_bar_width = if stack_key.is_some() {
                    band * 0.72
                } else {
                    band * 0.72 / usize_to_f64(bar_series)
                };
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
                    let Some(center) = map_value(&x_scale, &x_value) else {
                        continue;
                    };
                    let (start, end) = if let Some(stack) = &stack_key {
                        let running = stacks.entry((stack.clone(), category)).or_default();
                        let start = *running;
                        *running += value;
                        (start, *running)
                    } else {
                        (0.0, value)
                    };
                    let Some(y_start) = y_scale.map_number(start) else {
                        continue;
                    };
                    let Some(y_end) = y_scale.map_number(end) else {
                        continue;
                    };
                    let offset = if stack_key.is_some() {
                        0.0
                    } else {
                        (usize_to_f64(bar_index) - (usize_to_f64(bar_series) - 1.0) / 2.0)
                            * series_bar_width
                    };
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
                        Some(if value >= 0.0 { color } else { theme.negative }),
                        None,
                        value,
                    );
                }
                bar_index += 1;
            }
            ChartSeriesKind::Line | ChartSeriesKind::Area => {
                let points = series_points(prepared, &x_scale, &y_scale, diagnostics);
                if points.len() >= 2 {
                    let line_points = if prepared.spec.smooth {
                        smooth_polyline(&points.iter().map(|point| point.0).collect::<Vec<_>>(), 8)
                    } else {
                        points.iter().map(|point| point.0).collect::<Vec<_>>()
                    };
                    if prepared.spec.kind == ChartSeriesKind::Area {
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
                            key: format!("{}:area", prepared.spec.key),
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
                    marks.push(ChartMark {
                        key: format!("{}:line", prepared.spec.key),
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
                    for (point, row, value) in points {
                        add_circle_mark(
                            marks,
                            &prepared.spec,
                            &prepared.dataset,
                            row,
                            point,
                            4.0,
                            color,
                            value,
                        );
                    }
                }
            }
            ChartSeriesKind::Scatter => {
                let points = series_points(prepared, &x_scale, &y_scale, diagnostics);
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
                        color,
                        value,
                    );
                }
            }
            ChartSeriesKind::Heatmap => layout_heatmap(
                prepared,
                &x_scale,
                &y_scale,
                bounds,
                theme,
                marks,
                diagnostics,
            ),
            ChartSeriesKind::Candlestick => layout_candlestick(
                prepared,
                &x_scale,
                &y_scale,
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
                        x_scale: Some(&x_scale),
                        y_scale: Some(&y_scale),
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
                    key: format!("annotation:{}", annotation.key),
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
                            y: y - 14.0,
                        },
                        text: label.clone(),
                        color,
                        anchor: ChartLabelAnchor::Start,
                    });
                }
            }
            ChartAnnotationKind::MarkLine { axis, value }
            | ChartAnnotationKind::Baseline { axis, value } => {
                let geometry = annotation_line(*axis, value, x_scale, y_scale, bounds);
                if let Some((start, end)) = geometry {
                    let mut mark = line_mark(
                        format!("annotation:{}", annotation.key),
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
    match value {
        ChartAnnotationValue::Number(value) => scale.map_number(*value),
        ChartAnnotationValue::Category(value) => scale.map_category(value),
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
        key: format!("annotation:{}", annotation.key),
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
    if let Some(axis) = axis {
        min = axis.min.unwrap_or(min);
        max = axis.max.unwrap_or(max);
    }
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

fn add_cartesian_axes(
    x: &ChartScale,
    y: &ChartScale,
    x_axis: Option<&super::ChartAxisSpec>,
    y_axis: Option<&super::ChartAxisSpec>,
    bounds: ChartRect,
    theme: &ChartTheme,
    formatters: &ChartFormatterRegistry,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
) -> Result<(), ChartPrepareError> {
    let x_key = x_axis.map_or("x", |axis| axis.key.as_str());
    let y_key = y_axis.map_or("y", |axis| axis.key.as_str());
    let y_right = y_axis.is_some_and(|axis| axis.position == ChartAxisPosition::Right);
    let x_top = x_axis.is_some_and(|axis| axis.position == ChartAxisPosition::Top);
    for (index, tick) in y.ticks(6).into_iter().enumerate() {
        marks.push(line_mark(
            format!("grid:{y_key}:{index}"),
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
            key: format!("axis:{y_key}:{index}"),
            position: ChartPoint {
                x: if y_right {
                    bounds.x + bounds.width + 8.0
                } else {
                    bounds.x - 8.0
                },
                y: tick.position - 8.0,
            },
            text: format_axis_tick(&tick, y_axis, theme, formatters)?,
            color: theme.muted_text,
            anchor: if y_right {
                ChartLabelAnchor::Start
            } else {
                ChartLabelAnchor::End
            },
        });
    }
    for (index, tick) in x.ticks(8).into_iter().enumerate() {
        marks.push(line_mark(
            format!("grid:{x_key}:{index}"),
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
            key: format!("axis:{x_key}:{index}"),
            position: ChartPoint {
                x: tick.position,
                y: if x_top {
                    bounds.y - 18.0
                } else {
                    bounds.y + bounds.height + 6.0
                },
            },
            text: format_axis_tick(&tick, x_axis, theme, formatters)?,
            color: theme.muted_text,
            anchor: ChartLabelAnchor::Center,
        });
    }
    marks.push(line_mark(
        format!("axis:{x_key}"),
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
    marks.push(line_mark(
        format!("axis:{y_key}"),
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
    Ok(())
}

fn format_axis_tick(
    tick: &super::ChartTick,
    axis: Option<&super::ChartAxisSpec>,
    theme: &ChartTheme,
    formatters: &ChartFormatterRegistry,
) -> Result<String, ChartFormatterError> {
    let Some(axis) = axis else {
        return Ok(tick.label.clone());
    };
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
    let body = if let Some(number) = &theme.number
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
    } else if format.compact && value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if format.compact && value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
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

fn series_points(
    prepared: &PreparedSeries,
    x_scale: &ChartScale,
    y_scale: &ChartScale,
    diagnostics: &mut Vec<ChartDiagnostic>,
) -> Vec<(ChartPoint, usize, f64)> {
    let x = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::X).unwrap())
        .unwrap();
    let y = prepared
        .dataset
        .column(prepared.spec.encode.dimension(ChartChannel::Y).unwrap())
        .unwrap();
    (0..prepared.dataset.len())
        .filter_map(|row| {
            let x_value = x.value(row)?;
            let Some(y_value) = y.value(row).and_then(|value| value.as_number()) else {
                datum_diagnostic(
                    diagnostics,
                    &prepared.spec,
                    &prepared.dataset,
                    row,
                    "non_numeric_y",
                );
                return None;
            };
            let Some(mapped_x) = map_value(x_scale, &x_value) else {
                datum_diagnostic(
                    diagnostics,
                    &prepared.spec,
                    &prepared.dataset,
                    row,
                    "invalid_x",
                );
                return None;
            };
            let Some(mapped_y) = y_scale.map_number(y_value) else {
                datum_diagnostic(
                    diagnostics,
                    &prepared.spec,
                    &prepared.dataset,
                    row,
                    "invalid_y",
                );
                return None;
            };
            Some((
                ChartPoint {
                    x: mapped_x,
                    y: mapped_y,
                },
                row,
                y_value,
            ))
        })
        .collect()
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
            format!(
                "{}:{}:wick",
                prepared.spec.key,
                datum_key(&prepared.dataset, row)
            ),
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn layout_polar(
    series: &[&PreparedSeries],
    bounds: ChartRect,
    theme: &ChartTheme,
    marks: &mut Vec<ChartMark>,
    labels: &mut Vec<ChartLabel>,
    diagnostics: &mut Vec<ChartDiagnostic>,
    custom_series: &ChartSeriesRegistry,
) {
    let center = bounds.center();
    let radius = bounds.width.min(bounds.height) * 0.42;
    for (series_index, prepared) in series.iter().enumerate() {
        let color = series_color(&prepared.spec, series_index, theme);
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
                    let datum_color = theme.palette[(series_index + row) % theme.palette.len()];
                    marks.push(ChartMark {
                        key: format!(
                            "{}:{}",
                            prepared.spec.key,
                            datum_key(&prepared.dataset, row)
                        ),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum_key(&prepared.dataset, row),
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
                        value
                            .value(row)
                            .and_then(|value| value.as_number())
                            .map(|value| (row, value.max(0.0)))
                    })
                    .collect::<Vec<_>>();
                let max = values
                    .iter()
                    .map(|(_, value)| *value)
                    .reduce(f64::max)
                    .unwrap_or(1.0)
                    .max(1.0);
                let count = values.len().max(1);
                let points = values
                    .iter()
                    .enumerate()
                    .map(|(index, (_, value))| {
                        let angle = -std::f64::consts::FRAC_PI_2
                            + std::f64::consts::TAU * usize_to_f64(index) / usize_to_f64(count);
                        polar_point(center, radius * value / max, angle)
                    })
                    .collect::<Vec<_>>();
                if points.len() >= 3 {
                    marks.push(ChartMark {
                        key: format!("{}:radar", prepared.spec.key),
                        series_key: prepared.spec.key.clone(),
                        datum_key: "radar".to_owned(),
                        geometry: ChartMarkGeometry::Polygon(points.into()),
                        fill: Some(with_alpha(color, 0x44)),
                        stroke: Some((color, 2.0)),
                        label: prepared.spec.name.clone(),
                        value: None,
                        interactive: true,
                        selected: false,
                    });
                }
            }
            ChartSeriesKind::Gauge => {
                let current = value
                    .value(0)
                    .and_then(|value| value.as_number())
                    .unwrap_or(0.0);
                let max = (0..prepared.dataset.len())
                    .filter_map(|row| value.value(row).and_then(|value| value.as_number()))
                    .reduce(f64::max)
                    .unwrap_or(100.0)
                    .max(current)
                    .max(1.0);
                let start = std::f64::consts::PI * 0.75;
                let sweep = std::f64::consts::PI * 1.5;
                for (key, end, fill) in [
                    ("track", start + sweep, with_alpha(theme.axis, 0x55)),
                    (
                        "value",
                        start + sweep * (current / max).clamp(0.0, 1.0),
                        color,
                    ),
                ] {
                    marks.push(ChartMark {
                        key: format!("{}:gauge:{key}", prepared.spec.key),
                        series_key: prepared.spec.key.clone(),
                        datum_key: key.to_owned(),
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
                        interactive: key == "value",
                        selected: false,
                    });
                }
                labels.push(ChartLabel {
                    key: format!("{}:gauge-label", prepared.spec.key),
                    position: ChartPoint {
                        x: center.x,
                        y: center.y - 8.0,
                    },
                    text: current.to_string(),
                    color: theme.text,
                    anchor: ChartLabelAnchor::Center,
                });
            }
            ChartSeriesKind::Funnel => {
                let mut rows = (0..prepared.dataset.len())
                    .filter_map(|row| {
                        value
                            .value(row)
                            .and_then(|value| value.as_number())
                            .map(|value| (row, value.max(0.0)))
                    })
                    .collect::<Vec<_>>();
                rows.sort_by(|left, right| right.1.total_cmp(&left.1));
                let max = rows.first().map_or(1.0, |row| row.1.max(1.0));
                let row_height = bounds.height / usize_to_f64(rows.len().max(1));
                for (index, (row, value)) in rows.into_iter().enumerate() {
                    let top_width = bounds.width * value / max;
                    let next_width = if index + 1 < prepared.dataset.len() {
                        top_width * 0.82
                    } else {
                        top_width * 0.68
                    };
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
                    marks.push(ChartMark {
                        key: format!(
                            "{}:{}",
                            prepared.spec.key,
                            datum_key(&prepared.dataset, row)
                        ),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum_key(&prepared.dataset, row),
                        geometry: ChartMarkGeometry::Polygon(polygon.into()),
                        fill: Some(theme.palette[(series_index + index) % theme.palette.len()]),
                        stroke: None,
                        label: datum_label(name, &prepared.dataset, row, &prepared.spec.name),
                        value: Some(value),
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
                match custom_series.layout(
                    renderer,
                    super::ChartCustomSeriesContext {
                        spec: &prepared.spec,
                        dataset: &prepared.dataset,
                        bounds,
                        theme,
                        x_scale: None,
                        y_scale: None,
                    },
                ) {
                    Ok(custom) => marks.extend(custom),
                    Err(error) => push_diagnostic(
                        diagnostics,
                        ChartDiagnostic {
                            severity: ChartDiagnosticSeverity::Error,
                            code: "chart.custom_series".to_owned(),
                            message: error.to_string(),
                            series: Some(prepared.spec.key.clone()),
                            datum: None,
                        },
                    ),
                }
            }
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
) {
    let scale = geo_scale(map, bounds);
    for (series_index, prepared) in series.iter().enumerate() {
        let color = series_color(&prepared.spec, series_index, theme);
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
                            value.value(row)?.as_number()?,
                        ))
                    })
                    .collect::<BTreeMap<_, _>>();
                let min = values.values().copied().reduce(f64::min).unwrap_or(0.0);
                let max = values.values().copied().reduce(f64::max).unwrap_or(1.0);
                for feature in map.features.iter() {
                    let feature_value = values
                        .get(&feature.key)
                        .or_else(|| values.get(&feature.name))
                        .copied();
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
                        key: format!("{}:{}", prepared.spec.key, feature.key),
                        series_key: prepared.spec.key.clone(),
                        datum_key: feature.key.clone(),
                        geometry: ChartMarkGeometry::CompoundPolygon(rings.into()),
                        fill: Some(feature_value.map_or(theme.map_missing, |value| {
                            mix_color(theme.map_missing, color, normalize(value, min, max))
                        })),
                        stroke: Some((theme.background, 0.75)),
                        label: feature.name.clone(),
                        value: feature_value,
                        interactive: true,
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
                    let point = projection
                        .and_then(|projection| projection.project(x, y))
                        .unwrap_or(ChartGeoPoint { x, y });
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
                    let start = projection
                        .and_then(|projection| projection.project(sx, sy))
                        .unwrap_or(ChartGeoPoint { x: sx, y: sy });
                    let end = projection
                        .and_then(|projection| projection.project(tx, ty))
                        .unwrap_or(ChartGeoPoint { x: tx, y: ty });
                    let start = scale_geo_point(start, scale);
                    let end = scale_geo_point(end, scale);
                    marks.push(ChartMark {
                        key: format!(
                            "{}:{}",
                            prepared.spec.key,
                            datum_key(&prepared.dataset, row)
                        ),
                        series_key: prepared.spec.key.clone(),
                        datum_key: datum_key(&prepared.dataset, row),
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
                match custom_series.layout(
                    renderer,
                    super::ChartCustomSeriesContext {
                        spec: &prepared.spec,
                        dataset: &prepared.dataset,
                        bounds,
                        theme,
                        x_scale: None,
                        y_scale: None,
                    },
                ) {
                    Ok(custom) => marks.extend(custom),
                    Err(error) => push_diagnostic(
                        diagnostics,
                        ChartDiagnostic {
                            severity: ChartDiagnosticSeverity::Error,
                            code: "chart.custom_series".to_owned(),
                            message: error.to_string(),
                            series: Some(prepared.spec.key.clone()),
                            datum: None,
                        },
                    ),
                }
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
}

fn collect_categories(series: &[&PreparedSeries], channel: ChartChannel) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut categories = Vec::new();
    for prepared in series {
        let Some(column) = prepared
            .spec
            .encode
            .dimension(channel)
            .and_then(|name| prepared.dataset.column(name))
        else {
            continue;
        };
        for row in 0..prepared.dataset.len() {
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
                .and_then(|name| prepared.dataset.column(name))
            else {
                continue;
            };
            for row in 0..prepared.dataset.len() {
                if let Some(value) = column.value(row).and_then(|value| value.as_number()) {
                    min = Some(min.map_or(value, |min: f64| min.min(value)));
                    max = Some(max.map_or(value, |max: f64| max.max(value)));
                }
            }
        }
    }
    min.zip(max)
}

fn map_value(scale: &ChartScale, value: &ChartValue) -> Option<f64> {
    value
        .as_number()
        .and_then(|value| scale.map_number(value))
        .or_else(|| {
            matches!(value, ChartValue::String(_) | ChartValue::Bool(_))
                .then(|| scale.map_category(&value.display_text()))
                .flatten()
        })
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
        key: format!("{}:{datum}", spec.key),
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
    marks.push(ChartMark {
        key: format!("{}:{datum}", spec.key),
        series_key: spec.key.clone(),
        datum_key: datum,
        geometry: ChartMarkGeometry::Circle { center, radius },
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
    start: ChartPoint,
    end: ChartPoint,
    color: Rgba8,
    width: f64,
) -> ChartMark {
    ChartMark {
        key,
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

fn series_color(spec: &ChartSeriesSpec, index: usize, theme: &ChartTheme) -> Rgba8 {
    spec.color
        .as_deref()
        .and_then(parse_hex_color)
        .unwrap_or_else(|| theme.palette[index % theme.palette.len()])
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
            }],
            legend: ChartLegendSpec::default(),
            tooltip: ChartTooltipSpec::default(),
            motion: ChartMotionSpec::default(),
            brush: ChartBrushMode::None,
            annotations: Vec::new(),
            link_group: None,
            link_domain: None,
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
                ChartSeriesKind::Pie
                | ChartSeriesKind::Donut
                | ChartSeriesKind::Radar
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
                }],
                legend: ChartLegendSpec::default(),
                tooltip: ChartTooltipSpec::default(),
                motion: ChartMotionSpec::default(),
                brush: ChartBrushMode::None,
                annotations: Vec::new(),
                link_group: None,
                link_domain: None,
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
            plot_regions: BTreeMap::new(),
            marks: vec![mark].into(),
            labels: Vec::new().into(),
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
