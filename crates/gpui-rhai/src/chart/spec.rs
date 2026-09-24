#![allow(clippy::cast_precision_loss)]

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::UiValue;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChartSeriesKind {
    Bar,
    Line,
    Area,
    Scatter,
    Pie,
    Donut,
    Heatmap,
    Map,
    GeoScatter,
    GeoLines,
    Candlestick,
    Radar,
    Gauge,
    Funnel,
    Custom,
}

impl ChartSeriesKind {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "bar" => Self::Bar,
            "line" => Self::Line,
            "area" => Self::Area,
            "scatter" => Self::Scatter,
            "pie" => Self::Pie,
            "donut" => Self::Donut,
            "heatmap" => Self::Heatmap,
            "map" | "choropleth" => Self::Map,
            "geo_scatter" => Self::GeoScatter,
            "geo_lines" => Self::GeoLines,
            "candlestick" => Self::Candlestick,
            "radar" => Self::Radar,
            "gauge" => Self::Gauge,
            "funnel" => Self::Funnel,
            "custom" => Self::Custom,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn default_coordinate(self) -> ChartCoordinateKind {
        match self {
            Self::Bar
            | Self::Line
            | Self::Area
            | Self::Scatter
            | Self::Heatmap
            | Self::Candlestick
            | Self::Custom => ChartCoordinateKind::Cartesian2d,
            Self::Pie | Self::Donut | Self::Radar | Self::Gauge | Self::Funnel => {
                ChartCoordinateKind::Polar
            }
            Self::Map | Self::GeoScatter | Self::GeoLines => ChartCoordinateKind::Geo2d,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartCoordinateKind {
    Cartesian2d,
    Polar,
    Geo2d,
}

impl ChartCoordinateKind {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "cartesian" | "cartesian_2d" => Self::Cartesian2d,
            "polar" => Self::Polar,
            "geo" | "geo_2d" => Self::Geo2d,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChartChannel {
    X,
    Y,
    Value,
    Name,
    Size,
    Color,
    Open,
    Close,
    Low,
    High,
    Longitude,
    Latitude,
    SourceLongitude,
    SourceLatitude,
    TargetLongitude,
    TargetLatitude,
}

impl ChartChannel {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "x" => Self::X,
            "y" => Self::Y,
            "value" => Self::Value,
            "name" => Self::Name,
            "size" => Self::Size,
            "color" => Self::Color,
            "open" => Self::Open,
            "close" => Self::Close,
            "low" => Self::Low,
            "high" => Self::High,
            "longitude" | "lng" | "lon" => Self::Longitude,
            "latitude" | "lat" => Self::Latitude,
            "source_longitude" | "source_lng" => Self::SourceLongitude,
            "source_latitude" | "source_lat" => Self::SourceLatitude,
            "target_longitude" | "target_lng" => Self::TargetLongitude,
            "target_latitude" | "target_lat" => Self::TargetLatitude,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChartEncode(BTreeMap<ChartChannel, String>);

impl ChartEncode {
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = (ChartChannel, String)>) -> Self {
        Self(values.into_iter().collect())
    }

    #[must_use]
    pub fn dimension(&self, channel: ChartChannel) -> Option<&str> {
        self.0.get(&channel).map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ChartChannel, &str)> {
        self.0
            .iter()
            .map(|(channel, dimension)| (*channel, dimension.as_str()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartAxisScale {
    Auto,
    Linear,
    Log,
    Category,
    Time,
}

impl ChartAxisScale {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "auto" => Self::Auto,
            "linear" => Self::Linear,
            "log" => Self::Log,
            "category" => Self::Category,
            "time" => Self::Time,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartAxisDirection {
    Normal,
    Reversed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartAxisPosition {
    Top,
    Right,
    Bottom,
    Left,
    Radial,
    Angular,
}

impl ChartAxisPosition {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            "radial" => Self::Radial,
            "angular" => Self::Angular,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ChartTimeZone {
    #[default]
    Utc,
    FixedOffsetMinutes(i32),
    Iana(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartFormatSpec {
    pub precision: Option<u8>,
    pub compact: bool,
    pub percent: bool,
    pub prefix: String,
    pub suffix: String,
    pub formatter: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartAxisSpec {
    pub key: String,
    pub region: String,
    pub dimension: Option<String>,
    pub scale: ChartAxisScale,
    pub direction: ChartAxisDirection,
    pub position: ChartAxisPosition,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub title: Option<String>,
    pub format: ChartFormatSpec,
    pub timezone: ChartTimeZone,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartCoordinateRegion {
    pub key: String,
    pub kind: ChartCoordinateKind,
    pub map: Option<String>,
    pub projection: Option<String>,
    pub row: u16,
    pub column: u16,
    pub row_span: u16,
    pub column_span: u16,
}

impl Default for ChartCoordinateRegion {
    fn default() -> Self {
        Self {
            key: "main".to_owned(),
            kind: ChartCoordinateKind::Cartesian2d,
            map: None,
            projection: None,
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartTransformSpec {
    Filter {
        dimension: String,
        operator: String,
        value: UiValue,
    },
    Sort {
        dimension: String,
        descending: bool,
    },
    Aggregate {
        group_by: Vec<String>,
        dimension: String,
        operation: String,
        output: String,
    },
    Bin {
        dimension: String,
        bins: u16,
        output_start: String,
        output_end: String,
    },
    Stack {
        group_by: Vec<String>,
        dimension: String,
        output_start: String,
        output_end: String,
    },
    Normalize {
        group_by: Vec<String>,
        dimension: String,
        output: String,
    },
    MovingWindow {
        dimension: String,
        window: usize,
        operation: String,
        output: String,
    },
    Downsample {
        x: String,
        y: String,
        threshold: usize,
    },
    Host {
        id: String,
        options: UiValue,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeriesSpec {
    pub key: String,
    pub name: String,
    pub kind: ChartSeriesKind,
    pub dataset: String,
    pub coordinate: String,
    pub x_axis: Option<String>,
    pub y_axis: Option<String>,
    pub encode: ChartEncode,
    pub stack: Option<String>,
    pub color: Option<String>,
    pub visible: bool,
    pub smooth: bool,
    pub transforms: Vec<ChartTransformSpec>,
    pub renderer: Option<String>,
    pub options: UiValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartLegendPosition {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartLegendSpec {
    pub visible: bool,
    pub position: ChartLegendPosition,
    pub interactive: bool,
}

impl Default for ChartLegendSpec {
    fn default() -> Self {
        Self {
            visible: true,
            position: ChartLegendPosition::Top,
            interactive: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartTooltipSpec {
    pub visible: bool,
    pub shared: bool,
    pub crosshair: bool,
}

impl Default for ChartTooltipSpec {
    fn default() -> Self {
        Self {
            visible: true,
            shared: false,
            crosshair: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartMotionSpec {
    pub enabled: bool,
    pub duration_role: String,
    pub easing_role: String,
}

impl Default for ChartMotionSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_role: "normal".to_owned(),
            easing_role: "standard".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartBrushMode {
    None,
    X,
    Y,
    Xy,
    GeoRectangle,
    GeoRegion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartAnnotationValue {
    Number(f64),
    Category(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartAnnotationKind {
    MarkPoint {
        x: ChartAnnotationValue,
        y: ChartAnnotationValue,
    },
    MarkLine {
        axis: ChartChannel,
        value: ChartAnnotationValue,
    },
    MarkArea {
        x_min: ChartAnnotationValue,
        x_max: ChartAnnotationValue,
        y_min: ChartAnnotationValue,
        y_max: ChartAnnotationValue,
    },
    ThresholdBand {
        axis: ChartChannel,
        min: ChartAnnotationValue,
        max: ChartAnnotationValue,
    },
    Baseline {
        axis: ChartChannel,
        value: ChartAnnotationValue,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartAnnotation {
    pub key: String,
    pub region: String,
    pub label: Option<String>,
    pub color: Option<String>,
    pub kind: ChartAnnotationKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartSpec {
    pub title: Option<String>,
    pub description: Option<String>,
    pub regions: Vec<ChartCoordinateRegion>,
    pub axes: Vec<ChartAxisSpec>,
    pub series: Vec<ChartSeriesSpec>,
    pub legend: ChartLegendSpec,
    pub tooltip: ChartTooltipSpec,
    pub motion: ChartMotionSpec,
    pub brush: ChartBrushMode,
    pub annotations: Vec<ChartAnnotation>,
    pub link_group: Option<String>,
    pub link_domain: Option<String>,
}

impl ChartSpec {
    /// Decode a durable Rhai map into the typed chart contract.
    ///
    /// # Errors
    ///
    /// Returns precise shape, enum, identity, and cross-reference diagnostics.
    pub fn from_ui_value(value: &UiValue) -> Result<Self, ChartSpecError> {
        let root = map(value, "chart")?;
        reject_unknown(
            root,
            &[
                "title",
                "description",
                "regions",
                "axes",
                "series",
                "legend",
                "tooltip",
                "motion",
                "brush",
                "annotations",
                "link_group",
                "link_domain",
            ],
            "chart",
        )?;
        let regions = array_optional(root, "regions")?
            .map(|values| {
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| parse_region(value, index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_else(|| vec![ChartCoordinateRegion::default()]);
        let axes = array_optional(root, "axes")?
            .map(|values| {
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| parse_axis(value, index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let series = array(root, "series")?
            .iter()
            .enumerate()
            .map(|(index, value)| parse_series(value, index))
            .collect::<Result<Vec<_>, _>>()?;
        let spec = Self {
            title: optional_string(root, "title")?,
            description: optional_string(root, "description")?,
            regions,
            axes,
            series,
            legend: map_optional(root, "legend")?
                .map_or_else(|| Ok(ChartLegendSpec::default()), parse_legend)?,
            tooltip: map_optional(root, "tooltip")?
                .map_or_else(|| Ok(ChartTooltipSpec::default()), parse_tooltip)?,
            motion: map_optional(root, "motion")?
                .map_or_else(|| Ok(ChartMotionSpec::default()), parse_motion)?,
            brush: optional_string(root, "brush")?.map_or(Ok(ChartBrushMode::None), |value| {
                parse_brush(&value).ok_or_else(|| ChartSpecError::InvalidEnum {
                    path: "chart.brush".to_owned(),
                    value,
                })
            })?,
            annotations: array_optional(root, "annotations")?
                .map(|values| {
                    values
                        .iter()
                        .enumerate()
                        .map(|(index, value)| parse_annotation(value, index))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .unwrap_or_default(),
            link_group: optional_identifier(root, "link_group")?,
            link_domain: optional_identifier(root, "link_domain")?,
        };
        spec.validate()?;
        Ok(spec)
    }

    /// Validate cross-references and semantic requirements atomically.
    ///
    /// # Errors
    ///
    /// Returns duplicate, missing, invalid-domain, or binding diagnostics.
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> Result<(), ChartSpecError> {
        if self.series.is_empty() {
            return Err(ChartSpecError::NoSeries);
        }
        let mut region_keys = BTreeSet::new();
        for region in &self.regions {
            validate_identifier(&region.key, "region key")?;
            if !region_keys.insert(region.key.clone()) {
                return Err(ChartSpecError::DuplicateRegion(region.key.clone()));
            }
            if region.row_span == 0 || region.column_span == 0 {
                return Err(ChartSpecError::InvalidGridSpan(region.key.clone()));
            }
            if region.kind == ChartCoordinateKind::Geo2d && region.map.is_none() {
                return Err(ChartSpecError::MissingGeoMap(region.key.clone()));
            }
        }
        let mut axis_keys = BTreeSet::new();
        for axis in &self.axes {
            validate_identifier(&axis.key, "axis key")?;
            if !axis_keys.insert(axis.key.clone()) {
                return Err(ChartSpecError::DuplicateAxis(axis.key.clone()));
            }
            if !region_keys.contains(&axis.region) {
                return Err(ChartSpecError::UnknownRegion(axis.region.clone()));
            }
            if axis.min.is_some_and(|value| !value.is_finite())
                || axis.max.is_some_and(|value| !value.is_finite())
                || matches!((axis.min, axis.max), (Some(min), Some(max)) if max <= min)
            {
                return Err(ChartSpecError::InvalidAxisDomain(axis.key.clone()));
            }
        }
        let mut series_keys = BTreeSet::new();
        for series in &self.series {
            validate_identifier(&series.key, "series key")?;
            validate_identifier(&series.dataset, "dataset")?;
            if !series_keys.insert(series.key.clone()) {
                return Err(ChartSpecError::DuplicateSeries(series.key.clone()));
            }
            let region = self
                .regions
                .iter()
                .find(|region| region.key == series.coordinate)
                .ok_or_else(|| ChartSpecError::UnknownRegion(series.coordinate.clone()))?;
            if series.kind != ChartSeriesKind::Custom
                && region.kind != series.kind.default_coordinate()
            {
                return Err(ChartSpecError::CoordinateMismatch {
                    series: series.key.clone(),
                    expected: series.kind.default_coordinate(),
                    actual: region.kind,
                });
            }
            validate_required_channels(series)?;
            if series.kind == ChartSeriesKind::Custom && series.renderer.is_none() {
                return Err(ChartSpecError::MissingCustomRenderer(series.key.clone()));
            }
            if series.kind != ChartSeriesKind::Custom && series.options != UiValue::Null {
                return Err(ChartSpecError::UnsupportedSeriesOptions(series.key.clone()));
            }
            for axis in [&series.x_axis, &series.y_axis].into_iter().flatten() {
                let axis = self
                    .axes
                    .iter()
                    .find(|candidate| candidate.key == *axis)
                    .ok_or_else(|| ChartSpecError::UnknownAxis(axis.clone()))?;
                if axis.region != series.coordinate {
                    return Err(ChartSpecError::AxisRegionMismatch {
                        series: series.key.clone(),
                        axis: axis.key.clone(),
                    });
                }
            }
            if let Some(axis) = &series.x_axis {
                let Some(axis) = self.axes.iter().find(|candidate| candidate.key == *axis) else {
                    return Err(ChartSpecError::UnknownAxis(axis.clone()));
                };
                if !matches!(
                    axis.position,
                    ChartAxisPosition::Top | ChartAxisPosition::Bottom
                ) {
                    return Err(ChartSpecError::AxisOrientationMismatch {
                        series: series.key.clone(),
                        axis: axis.key.clone(),
                        channel: ChartChannel::X,
                    });
                }
                if let Some(dimension) = &axis.dimension
                    && series.encode.dimension(ChartChannel::X) != Some(dimension.as_str())
                {
                    return Err(ChartSpecError::AxisDimensionMismatch {
                        series: series.key.clone(),
                        axis: axis.key.clone(),
                        dimension: dimension.clone(),
                    });
                }
            }
            if let Some(axis) = &series.y_axis {
                let Some(axis) = self.axes.iter().find(|candidate| candidate.key == *axis) else {
                    return Err(ChartSpecError::UnknownAxis(axis.clone()));
                };
                if !matches!(
                    axis.position,
                    ChartAxisPosition::Left | ChartAxisPosition::Right
                ) {
                    return Err(ChartSpecError::AxisOrientationMismatch {
                        series: series.key.clone(),
                        axis: axis.key.clone(),
                        channel: ChartChannel::Y,
                    });
                }
                if let Some(dimension) = &axis.dimension
                    && series.encode.dimension(ChartChannel::Y) != Some(dimension.as_str())
                {
                    return Err(ChartSpecError::AxisDimensionMismatch {
                        series: series.key.clone(),
                        axis: axis.key.clone(),
                        dimension: dimension.clone(),
                    });
                }
            }
            for (_, dimension) in series.encode.iter() {
                validate_identifier(dimension, "encoded dimension")?;
            }
        }
        if self.link_group.is_some() != self.link_domain.is_some() {
            return Err(ChartSpecError::IncompleteLink);
        }
        let mut annotation_keys = BTreeSet::new();
        for annotation in &self.annotations {
            validate_identifier(&annotation.key, "annotation key")?;
            if !annotation_keys.insert(annotation.key.clone()) {
                return Err(ChartSpecError::DuplicateAnnotation(annotation.key.clone()));
            }
            if !region_keys.contains(&annotation.region) {
                return Err(ChartSpecError::UnknownRegion(annotation.region.clone()));
            }
        }
        Ok(())
    }
}

fn validate_required_channels(series: &ChartSeriesSpec) -> Result<(), ChartSpecError> {
    let required: &[ChartChannel] = match series.kind {
        ChartSeriesKind::Bar
        | ChartSeriesKind::Line
        | ChartSeriesKind::Area
        | ChartSeriesKind::Scatter => &[ChartChannel::X, ChartChannel::Y],
        ChartSeriesKind::Heatmap => &[ChartChannel::X, ChartChannel::Y, ChartChannel::Value],
        ChartSeriesKind::Pie
        | ChartSeriesKind::Donut
        | ChartSeriesKind::Radar
        | ChartSeriesKind::Gauge
        | ChartSeriesKind::Funnel
        | ChartSeriesKind::Map => &[ChartChannel::Name, ChartChannel::Value],
        ChartSeriesKind::GeoScatter => &[ChartChannel::Longitude, ChartChannel::Latitude],
        ChartSeriesKind::GeoLines => &[
            ChartChannel::SourceLongitude,
            ChartChannel::SourceLatitude,
            ChartChannel::TargetLongitude,
            ChartChannel::TargetLatitude,
        ],
        ChartSeriesKind::Candlestick => &[
            ChartChannel::X,
            ChartChannel::Open,
            ChartChannel::Close,
            ChartChannel::Low,
            ChartChannel::High,
        ],
        ChartSeriesKind::Custom => &[],
    };
    if let Some(channel) = required
        .iter()
        .find(|channel| series.encode.dimension(**channel).is_none())
    {
        return Err(ChartSpecError::MissingChannel {
            series: series.key.clone(),
            channel: *channel,
        });
    }
    Ok(())
}

fn parse_region(value: &UiValue, index: usize) -> Result<ChartCoordinateRegion, ChartSpecError> {
    let path = format!("chart.regions[{index}]");
    let value = map(value, &path)?;
    reject_unknown(
        value,
        &[
            "key",
            "kind",
            "map",
            "projection",
            "row",
            "column",
            "row_span",
            "column_span",
        ],
        &path,
    )?;
    let kind_text = string(value, "kind", &path)?;
    let kind =
        ChartCoordinateKind::parse(&kind_text).ok_or_else(|| ChartSpecError::InvalidEnum {
            path: format!("{path}.kind"),
            value: kind_text,
        })?;
    Ok(ChartCoordinateRegion {
        key: string(value, "key", &path)?,
        kind,
        map: optional_string(value, "map")?,
        projection: optional_string(value, "projection")?,
        row: optional_u16(value, "row")?.unwrap_or(0),
        column: optional_u16(value, "column")?.unwrap_or(0),
        row_span: optional_u16(value, "row_span")?.unwrap_or(1),
        column_span: optional_u16(value, "column_span")?.unwrap_or(1),
    })
}

fn parse_axis(value: &UiValue, index: usize) -> Result<ChartAxisSpec, ChartSpecError> {
    let path = format!("chart.axes[{index}]");
    let value = map(value, &path)?;
    reject_unknown(
        value,
        &[
            "key",
            "region",
            "dimension",
            "scale",
            "direction",
            "position",
            "min",
            "max",
            "title",
            "format",
            "timezone",
        ],
        &path,
    )?;
    let position_text = string(value, "position", &path)?;
    let position =
        ChartAxisPosition::parse(&position_text).ok_or_else(|| ChartSpecError::InvalidEnum {
            path: format!("{path}.position"),
            value: position_text,
        })?;
    let scale_text = optional_string(value, "scale")?.unwrap_or_else(|| "auto".to_owned());
    let scale = ChartAxisScale::parse(&scale_text).ok_or_else(|| ChartSpecError::InvalidEnum {
        path: format!("{path}.scale"),
        value: scale_text,
    })?;
    let direction_text =
        optional_string(value, "direction")?.unwrap_or_else(|| "normal".to_owned());
    let direction = match direction_text.as_str() {
        "normal" => ChartAxisDirection::Normal,
        "reversed" => ChartAxisDirection::Reversed,
        _ => {
            return Err(ChartSpecError::InvalidEnum {
                path: format!("{path}.direction"),
                value: direction_text,
            });
        }
    };
    Ok(ChartAxisSpec {
        key: string(value, "key", &path)?,
        region: optional_string(value, "region")?.unwrap_or_else(|| "main".to_owned()),
        dimension: optional_string(value, "dimension")?,
        scale,
        direction,
        position,
        min: optional_number(value, "min")?,
        max: optional_number(value, "max")?,
        title: optional_string(value, "title")?,
        format: map_optional(value, "format")?
            .map_or_else(|| Ok(ChartFormatSpec::default()), parse_format)?,
        timezone: optional_string(value, "timezone")?
            .map_or_else(|| Ok(ChartTimeZone::Utc), |value| parse_timezone(&value))?,
    })
}

fn parse_series(value: &UiValue, index: usize) -> Result<ChartSeriesSpec, ChartSpecError> {
    let path = format!("chart.series[{index}]");
    let value = map(value, &path)?;
    reject_unknown(
        value,
        &[
            "key",
            "name",
            "kind",
            "dataset",
            "coordinate",
            "x_axis",
            "y_axis",
            "encode",
            "stack",
            "color",
            "visible",
            "smooth",
            "transforms",
            "renderer",
            "options",
        ],
        &path,
    )?;
    let kind_text = string(value, "kind", &path)?;
    let kind = ChartSeriesKind::parse(&kind_text).ok_or_else(|| ChartSpecError::InvalidEnum {
        path: format!("{path}.kind"),
        value: kind_text,
    })?;
    let encode = map_required(value, "encode", &path)?;
    let mut channels = BTreeMap::new();
    for (name, dimension) in encode {
        let channel = ChartChannel::parse(name).ok_or_else(|| ChartSpecError::InvalidEnum {
            path: format!("{path}.encode.{name}"),
            value: name.clone(),
        })?;
        let dimension = match dimension {
            UiValue::String(value) => value.clone(),
            _ => {
                return Err(ChartSpecError::ExpectedString(format!(
                    "{path}.encode.{name}"
                )));
            }
        };
        channels.insert(channel, dimension);
    }
    let key = string(value, "key", &path)?;
    Ok(ChartSeriesSpec {
        name: optional_string(value, "name")?.unwrap_or_else(|| key.clone()),
        key,
        kind,
        dataset: optional_string(value, "dataset")?.unwrap_or_else(|| "main".to_owned()),
        coordinate: optional_string(value, "coordinate")?
            .unwrap_or_else(|| default_region_key(kind).to_owned()),
        x_axis: optional_identifier(value, "x_axis")?,
        y_axis: optional_identifier(value, "y_axis")?,
        encode: ChartEncode(channels),
        stack: optional_identifier(value, "stack")?,
        color: optional_string(value, "color")?,
        visible: optional_bool(value, "visible")?.unwrap_or(true),
        smooth: optional_bool(value, "smooth")?.unwrap_or(false),
        transforms: array_optional(value, "transforms")?
            .map(|values| {
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        parse_transform(value, &format!("{path}.transforms[{index}]"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default(),
        renderer: optional_identifier(value, "renderer")?,
        options: value.get("options").cloned().unwrap_or(UiValue::Null),
    })
}

fn default_region_key(_: ChartSeriesKind) -> &'static str {
    "main"
}

#[allow(clippy::too_many_lines)]
fn parse_transform(value: &UiValue, path: &str) -> Result<ChartTransformSpec, ChartSpecError> {
    let value = map(value, path)?;
    reject_unknown(
        value,
        &[
            "kind",
            "dimension",
            "operator",
            "value",
            "descending",
            "group_by",
            "operation",
            "output",
            "bins",
            "output_start",
            "output_end",
            "window",
            "x",
            "y",
            "threshold",
            "id",
            "options",
        ],
        path,
    )?;
    let kind = string(value, "kind", path)?;
    match kind.as_str() {
        "filter" => {
            let operator = string(value, "operator", path)?;
            require_enum_value(
                &operator,
                &["eq", "ne", "lt", "lte", "gt", "gte", "contains"],
                &format!("{path}.operator"),
            )?;
            Ok(ChartTransformSpec::Filter {
                dimension: string(value, "dimension", path)?,
                operator,
                value: value
                    .get("value")
                    .cloned()
                    .ok_or_else(|| ChartSpecError::Missing(format!("{path}.value")))?,
            })
        }
        "sort" => Ok(ChartTransformSpec::Sort {
            dimension: string(value, "dimension", path)?,
            descending: optional_bool(value, "descending")?.unwrap_or(false),
        }),
        "aggregate" => {
            let operation = string(value, "operation", path)?;
            require_enum_value(
                &operation,
                &["sum", "mean", "min", "max", "count"],
                &format!("{path}.operation"),
            )?;
            Ok(ChartTransformSpec::Aggregate {
                group_by: string_array(value, "group_by", path)?,
                dimension: string(value, "dimension", path)?,
                operation,
                output: string(value, "output", path)?,
            })
        }
        "bin" => Ok(ChartTransformSpec::Bin {
            dimension: string(value, "dimension", path)?,
            bins: optional_u16(value, "bins")?.unwrap_or(20).max(1),
            output_start: string(value, "output_start", path)?,
            output_end: string(value, "output_end", path)?,
        }),
        "stack" => Ok(ChartTransformSpec::Stack {
            group_by: string_array(value, "group_by", path)?,
            dimension: string(value, "dimension", path)?,
            output_start: string(value, "output_start", path)?,
            output_end: string(value, "output_end", path)?,
        }),
        "normalize" => Ok(ChartTransformSpec::Normalize {
            group_by: string_array(value, "group_by", path)?,
            dimension: string(value, "dimension", path)?,
            output: string(value, "output", path)?,
        }),
        "moving_window" => {
            let operation = string(value, "operation", path)?;
            require_enum_value(
                &operation,
                &["sum", "mean", "min", "max", "count"],
                &format!("{path}.operation"),
            )?;
            Ok(ChartTransformSpec::MovingWindow {
                dimension: string(value, "dimension", path)?,
                window: optional_usize(value, "window")?.unwrap_or(5).max(1),
                operation,
                output: string(value, "output", path)?,
            })
        }
        "downsample" => Ok(ChartTransformSpec::Downsample {
            x: string(value, "x", path)?,
            y: string(value, "y", path)?,
            threshold: optional_usize(value, "threshold")?.unwrap_or(2_000).max(3),
        }),
        "host" => Ok(ChartTransformSpec::Host {
            id: string(value, "id", path)?,
            options: value.get("options").cloned().unwrap_or(UiValue::Null),
        }),
        _ => Err(ChartSpecError::InvalidEnum {
            path: format!("{path}.kind"),
            value: kind,
        }),
    }
}

fn parse_legend(value: &BTreeMap<String, UiValue>) -> Result<ChartLegendSpec, ChartSpecError> {
    reject_unknown(
        value,
        &["visible", "position", "interactive"],
        "chart.legend",
    )?;
    let position_text = optional_string(value, "position")?.unwrap_or_else(|| "top".to_owned());
    let position = match position_text.as_str() {
        "top" => ChartLegendPosition::Top,
        "right" => ChartLegendPosition::Right,
        "bottom" => ChartLegendPosition::Bottom,
        "left" => ChartLegendPosition::Left,
        _ => {
            return Err(ChartSpecError::InvalidEnum {
                path: "chart.legend.position".to_owned(),
                value: position_text,
            });
        }
    };
    Ok(ChartLegendSpec {
        visible: optional_bool(value, "visible")?.unwrap_or(true),
        position,
        interactive: optional_bool(value, "interactive")?.unwrap_or(true),
    })
}

fn parse_tooltip(value: &BTreeMap<String, UiValue>) -> Result<ChartTooltipSpec, ChartSpecError> {
    reject_unknown(value, &["visible", "shared", "crosshair"], "chart.tooltip")?;
    Ok(ChartTooltipSpec {
        visible: optional_bool(value, "visible")?.unwrap_or(true),
        shared: optional_bool(value, "shared")?.unwrap_or(false),
        crosshair: optional_bool(value, "crosshair")?.unwrap_or(true),
    })
}

fn parse_motion(value: &BTreeMap<String, UiValue>) -> Result<ChartMotionSpec, ChartSpecError> {
    reject_unknown(value, &["enabled", "duration", "easing"], "chart.motion")?;
    Ok(ChartMotionSpec {
        enabled: optional_bool(value, "enabled")?.unwrap_or(true),
        duration_role: optional_string(value, "duration")?.unwrap_or_else(|| "normal".to_owned()),
        easing_role: optional_string(value, "easing")?.unwrap_or_else(|| "standard".to_owned()),
    })
}

fn parse_format(value: &BTreeMap<String, UiValue>) -> Result<ChartFormatSpec, ChartSpecError> {
    reject_unknown(
        value,
        &[
            "precision",
            "compact",
            "percent",
            "prefix",
            "suffix",
            "formatter",
        ],
        "chart.axis.format",
    )?;
    let precision = optional_u16(value, "precision")?;
    if precision.is_some_and(|precision| precision > 12) {
        return Err(ChartSpecError::InvalidPrecision);
    }
    Ok(ChartFormatSpec {
        precision: precision.map(|value| u8::try_from(value).unwrap_or(12)),
        compact: optional_bool(value, "compact")?.unwrap_or(false),
        percent: optional_bool(value, "percent")?.unwrap_or(false),
        prefix: optional_string(value, "prefix")?.unwrap_or_default(),
        suffix: optional_string(value, "suffix")?.unwrap_or_default(),
        formatter: optional_identifier(value, "formatter")?,
    })
}

fn parse_timezone(value: &str) -> Result<ChartTimeZone, ChartSpecError> {
    if value.eq_ignore_ascii_case("utc") {
        return Ok(ChartTimeZone::Utc);
    }
    if let Some(minutes) = value.strip_prefix("offset:") {
        let minutes = minutes
            .parse::<i32>()
            .map_err(|_| ChartSpecError::InvalidTimeZone(value.to_owned()))?;
        if !(-24 * 60..=24 * 60).contains(&minutes) {
            return Err(ChartSpecError::InvalidTimeZone(value.to_owned()));
        }
        return Ok(ChartTimeZone::FixedOffsetMinutes(minutes));
    }
    if value.split('/').count() >= 2
        && value.split('/').all(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
                && segment.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '+')
                })
        })
    {
        Ok(ChartTimeZone::Iana(value.to_owned()))
    } else {
        Err(ChartSpecError::InvalidTimeZone(value.to_owned()))
    }
}

fn parse_brush(value: &str) -> Option<ChartBrushMode> {
    Some(match value {
        "none" => ChartBrushMode::None,
        "x" => ChartBrushMode::X,
        "y" => ChartBrushMode::Y,
        "xy" => ChartBrushMode::Xy,
        "geo_rectangle" => ChartBrushMode::GeoRectangle,
        "geo_region" => ChartBrushMode::GeoRegion,
        _ => return None,
    })
}

fn parse_annotation(value: &UiValue, index: usize) -> Result<ChartAnnotation, ChartSpecError> {
    let path = format!("chart.annotations[{index}]");
    let value = map(value, &path)?;
    reject_unknown(
        value,
        &[
            "key", "region", "label", "color", "kind", "x", "y", "axis", "value", "x_min", "x_max",
            "y_min", "y_max", "min", "max",
        ],
        &path,
    )?;
    let kind = string(value, "kind", &path)?;
    let annotation_kind = match kind.as_str() {
        "mark_point" => ChartAnnotationKind::MarkPoint {
            x: annotation_value(value, "x", &path)?,
            y: annotation_value(value, "y", &path)?,
        },
        "mark_line" => ChartAnnotationKind::MarkLine {
            axis: annotation_axis(value, &path)?,
            value: annotation_value(value, "value", &path)?,
        },
        "mark_area" => ChartAnnotationKind::MarkArea {
            x_min: annotation_value(value, "x_min", &path)?,
            x_max: annotation_value(value, "x_max", &path)?,
            y_min: annotation_value(value, "y_min", &path)?,
            y_max: annotation_value(value, "y_max", &path)?,
        },
        "threshold_band" => ChartAnnotationKind::ThresholdBand {
            axis: annotation_axis(value, &path)?,
            min: annotation_value(value, "min", &path)?,
            max: annotation_value(value, "max", &path)?,
        },
        "baseline" => ChartAnnotationKind::Baseline {
            axis: annotation_axis(value, &path)?,
            value: annotation_value(value, "value", &path)?,
        },
        _ => {
            return Err(ChartSpecError::InvalidEnum {
                path: format!("{path}.kind"),
                value: kind,
            });
        }
    };
    Ok(ChartAnnotation {
        key: string(value, "key", &path)?,
        region: optional_string(value, "region")?.unwrap_or_else(|| "main".to_owned()),
        label: optional_string(value, "label")?,
        color: optional_string(value, "color")?,
        kind: annotation_kind,
    })
}

fn annotation_axis(
    value: &BTreeMap<String, UiValue>,
    path: &str,
) -> Result<ChartChannel, ChartSpecError> {
    let value = string(value, "axis", path)?;
    match value.as_str() {
        "x" => Ok(ChartChannel::X),
        "y" => Ok(ChartChannel::Y),
        _ => Err(ChartSpecError::InvalidEnum {
            path: format!("{path}.axis"),
            value,
        }),
    }
}

fn annotation_value(
    value: &BTreeMap<String, UiValue>,
    name: &str,
    path: &str,
) -> Result<ChartAnnotationValue, ChartSpecError> {
    match value.get(name) {
        Some(UiValue::Float(value)) if value.is_finite() => {
            Ok(ChartAnnotationValue::Number(*value))
        }
        Some(UiValue::Integer(value)) => Ok(ChartAnnotationValue::Number(*value as f64)),
        Some(UiValue::String(value)) => Ok(ChartAnnotationValue::Category(value.clone())),
        _ => Err(ChartSpecError::InvalidAnnotationValue(format!(
            "{path}.{name}"
        ))),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartDiagnostic {
    pub severity: ChartDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub series: Option<String>,
    pub datum: Option<String>,
}

fn map<'a>(
    value: &'a UiValue,
    path: &str,
) -> Result<&'a BTreeMap<String, UiValue>, ChartSpecError> {
    match value {
        UiValue::Map(value) => Ok(value),
        _ => Err(ChartSpecError::ExpectedMap(path.to_owned())),
    }
}

fn map_required<'a>(
    value: &'a BTreeMap<String, UiValue>,
    name: &str,
    path: &str,
) -> Result<&'a BTreeMap<String, UiValue>, ChartSpecError> {
    value
        .get(name)
        .ok_or_else(|| ChartSpecError::Missing(format!("{path}.{name}")))
        .and_then(|value| map(value, &format!("{path}.{name}")))
}

fn map_optional<'a>(
    value: &'a BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<&'a BTreeMap<String, UiValue>>, ChartSpecError> {
    value
        .get(name)
        .filter(|value| !matches!(value, UiValue::Null))
        .map(|value| map(value, &format!("chart.{name}")))
        .transpose()
}

fn array<'a>(
    value: &'a BTreeMap<String, UiValue>,
    name: &str,
) -> Result<&'a [UiValue], ChartSpecError> {
    match value.get(name) {
        Some(UiValue::Array(value)) => Ok(value),
        Some(_) => Err(ChartSpecError::ExpectedArray(format!("chart.{name}"))),
        None => Err(ChartSpecError::Missing(format!("chart.{name}"))),
    }
}

fn array_optional<'a>(
    value: &'a BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<&'a [UiValue]>, ChartSpecError> {
    match value.get(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::Array(value)) => Ok(Some(value)),
        Some(_) => Err(ChartSpecError::ExpectedArray(format!("chart.{name}"))),
    }
}

fn string(
    value: &BTreeMap<String, UiValue>,
    name: &str,
    path: &str,
) -> Result<String, ChartSpecError> {
    match value.get(name) {
        Some(UiValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(ChartSpecError::ExpectedString(format!("{path}.{name}"))),
        None => Err(ChartSpecError::Missing(format!("{path}.{name}"))),
    }
}

fn optional_string(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<String>, ChartSpecError> {
    match value.get(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ChartSpecError::ExpectedString(format!("chart.{name}"))),
    }
}

fn optional_identifier(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<String>, ChartSpecError> {
    let value = optional_string(value, name)?;
    if let Some(value) = &value {
        validate_identifier(value, name)?;
    }
    Ok(value)
}

fn optional_bool(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<bool>, ChartSpecError> {
    match value.get(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(ChartSpecError::ExpectedBool(format!("chart.{name}"))),
    }
}

fn optional_number(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<f64>, ChartSpecError> {
    match value.get(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::Float(value)) => Ok(Some(*value)),
        Some(UiValue::Integer(value)) => Ok(Some(*value as f64)),
        Some(_) => Err(ChartSpecError::ExpectedNumber(format!("chart.{name}"))),
    }
}

fn optional_usize(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<usize>, ChartSpecError> {
    match value.get(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::Integer(value)) => usize::try_from(*value)
            .map(Some)
            .map_err(|_| ChartSpecError::ExpectedUnsigned(format!("chart.{name}"))),
        Some(_) => Err(ChartSpecError::ExpectedUnsigned(format!("chart.{name}"))),
    }
}

fn optional_u16(
    value: &BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<u16>, ChartSpecError> {
    optional_usize(value, name)?.map_or(Ok(None), |value| {
        u16::try_from(value)
            .map(Some)
            .map_err(|_| ChartSpecError::ExpectedUnsigned(format!("chart.{name}")))
    })
}

fn string_array(
    value: &BTreeMap<String, UiValue>,
    name: &str,
    path: &str,
) -> Result<Vec<String>, ChartSpecError> {
    let values = match value.get(name) {
        Some(UiValue::Array(values)) => values,
        Some(_) => return Err(ChartSpecError::ExpectedArray(format!("{path}.{name}"))),
        None => return Ok(Vec::new()),
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            UiValue::String(value) => Ok(value.clone()),
            _ => Err(ChartSpecError::ExpectedString(format!(
                "{path}.{name}[{index}]"
            ))),
        })
        .collect()
}

fn validate_identifier(value: &str, category: &str) -> Result<(), ChartSpecError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(ChartSpecError::InvalidIdentifier {
            category: category.to_owned(),
            value: value.to_owned(),
        })
    }
}

fn reject_unknown(
    value: &BTreeMap<String, UiValue>,
    allowed: &[&str],
    path: &str,
) -> Result<(), ChartSpecError> {
    if let Some(name) = value.keys().find(|name| !allowed.contains(&name.as_str())) {
        Err(ChartSpecError::UnknownField {
            path: path.to_owned(),
            name: name.clone(),
        })
    } else {
        Ok(())
    }
}

fn require_enum_value(value: &str, allowed: &[&str], path: &str) -> Result<(), ChartSpecError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ChartSpecError::InvalidEnum {
            path: path.to_owned(),
            value: value.to_owned(),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartSpecError {
    #[error("chart configuration requires at least one series")]
    NoSeries,
    #[error("chart configuration is missing `{0}`")]
    Missing(String),
    #[error("chart value `{0}` must be an object")]
    ExpectedMap(String),
    #[error("chart value `{0}` must be an array")]
    ExpectedArray(String),
    #[error("chart value `{0}` must be a string")]
    ExpectedString(String),
    #[error("chart value `{0}` must be a boolean")]
    ExpectedBool(String),
    #[error("chart value `{0}` must be a finite number")]
    ExpectedNumber(String),
    #[error("chart value `{0}` must be an unsigned integer")]
    ExpectedUnsigned(String),
    #[error("chart enum at `{path}` has unknown value `{value}`")]
    InvalidEnum { path: String, value: String },
    #[error("chart object `{path}` contains unknown field `{name}`")]
    UnknownField { path: String, name: String },
    #[error("chart {category} `{value}` must be a safe identifier")]
    InvalidIdentifier { category: String, value: String },
    #[error("chart coordinate region `{0}` is duplicated")]
    DuplicateRegion(String),
    #[error("chart coordinate region `{0}` has an invalid grid span")]
    InvalidGridSpan(String),
    #[error("Geo2D coordinate region `{0}` requires a registered map")]
    MissingGeoMap(String),
    #[error("chart axis `{0}` is duplicated")]
    DuplicateAxis(String),
    #[error("chart axis `{0}` has an invalid explicit domain")]
    InvalidAxisDomain(String),
    #[error("chart series `{0}` is duplicated")]
    DuplicateSeries(String),
    #[error("chart coordinate region `{0}` does not exist")]
    UnknownRegion(String),
    #[error("chart axis `{0}` does not exist")]
    UnknownAxis(String),
    #[error("chart series `{series}` binds axis `{axis}` from a different region")]
    AxisRegionMismatch { series: String, axis: String },
    #[error("chart series `{series}` binds {channel:?} to incompatible axis `{axis}`")]
    AxisOrientationMismatch {
        series: String,
        axis: String,
        channel: ChartChannel,
    },
    #[error(
        "chart series `{series}` does not encode dimension `{dimension}` required by axis `{axis}`"
    )]
    AxisDimensionMismatch {
        series: String,
        axis: String,
        dimension: String,
    },
    #[error("chart series `{series}` requires {expected:?}, but region uses {actual:?}")]
    CoordinateMismatch {
        series: String,
        expected: ChartCoordinateKind,
        actual: ChartCoordinateKind,
    },
    #[error("chart series `{series}` is missing the {channel:?} encode channel")]
    MissingChannel {
        series: String,
        channel: ChartChannel,
    },
    #[error("chart link_group and link_domain must be supplied together")]
    IncompleteLink,
    #[error("custom chart series `{0}` requires a registered renderer ID")]
    MissingCustomRenderer(String),
    #[error("built-in chart series `{0}` does not accept custom options")]
    UnsupportedSeriesOptions(String),
    #[error("chart annotation `{0}` is duplicated")]
    DuplicateAnnotation(String),
    #[error("chart annotation value `{0}` must be a finite number or category string")]
    InvalidAnnotationValue(String),
    #[error("chart numeric precision must be between 0 and 12")]
    InvalidPrecision,
    #[error("chart timezone `{0}` must be UTC, offset:<minutes>, or an IANA name")]
    InvalidTimeZone(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_series_and_named_regions_validate_as_one_scene() {
        let spec = ChartSpec {
            title: Some("Operations".to_owned()),
            description: None,
            regions: vec![
                ChartCoordinateRegion::default(),
                ChartCoordinateRegion {
                    key: "share".to_owned(),
                    kind: ChartCoordinateKind::Polar,
                    row: 0,
                    column: 1,
                    ..ChartCoordinateRegion::default()
                },
            ],
            axes: Vec::new(),
            series: vec![
                ChartSeriesSpec {
                    key: "latency".to_owned(),
                    name: "Latency".to_owned(),
                    kind: ChartSeriesKind::Line,
                    dataset: "main".to_owned(),
                    coordinate: "main".to_owned(),
                    x_axis: None,
                    y_axis: None,
                    encode: ChartEncode::new([
                        (ChartChannel::X, "time".to_owned()),
                        (ChartChannel::Y, "latency".to_owned()),
                    ]),
                    stack: None,
                    color: None,
                    visible: true,
                    smooth: true,
                    transforms: Vec::new(),
                    renderer: None,
                    options: UiValue::Null,
                },
                ChartSeriesSpec {
                    key: "share".to_owned(),
                    name: "Share".to_owned(),
                    kind: ChartSeriesKind::Donut,
                    dataset: "main".to_owned(),
                    coordinate: "share".to_owned(),
                    x_axis: None,
                    y_axis: None,
                    encode: ChartEncode::new([
                        (ChartChannel::Name, "service".to_owned()),
                        (ChartChannel::Value, "requests".to_owned()),
                    ]),
                    stack: None,
                    color: None,
                    visible: true,
                    smooth: false,
                    transforms: Vec::new(),
                    renderer: None,
                    options: UiValue::Null,
                },
            ],
            legend: ChartLegendSpec::default(),
            tooltip: ChartTooltipSpec::default(),
            motion: ChartMotionSpec::default(),
            brush: ChartBrushMode::None,
            annotations: Vec::new(),
            link_group: Some("ops".to_owned()),
            link_domain: Some("time".to_owned()),
        };
        spec.validate().unwrap();
    }

    #[test]
    fn coordinate_and_channel_mismatches_fail_before_scene_install() {
        let mut root = BTreeMap::new();
        root.insert(
            "series".to_owned(),
            UiValue::Array(vec![UiValue::Map(BTreeMap::from([
                ("key".to_owned(), UiValue::String("bad".to_owned())),
                ("kind".to_owned(), UiValue::String("line".to_owned())),
                (
                    "encode".to_owned(),
                    UiValue::Map(BTreeMap::from([(
                        "x".to_owned(),
                        UiValue::String("time".to_owned()),
                    )])),
                ),
            ]))]),
        );
        assert!(matches!(
            ChartSpec::from_ui_value(&UiValue::Map(root)),
            Err(ChartSpecError::MissingChannel {
                channel: ChartChannel::Y,
                ..
            })
        ));
    }

    #[test]
    fn timezones_require_explicit_unambiguous_contracts() {
        assert_eq!(parse_timezone("UTC").unwrap(), ChartTimeZone::Utc);
        assert_eq!(
            parse_timezone("offset:480").unwrap(),
            ChartTimeZone::FixedOffsetMinutes(480)
        );
        assert_eq!(
            parse_timezone("Asia/Shanghai").unwrap(),
            ChartTimeZone::Iana("Asia/Shanghai".to_owned())
        );
        assert!(parse_timezone("09/22/2026").is_err());
    }
}
