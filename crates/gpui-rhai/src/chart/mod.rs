//! Native, retained 2D visualization runtime.
//!
//! The chart feature keeps typed data preparation, layout, painting, hit
//! testing, motion, and streaming updates in Rust. Rhai supplies declarative
//! specifications and receives bounded semantic events.

mod data;
mod export;
mod extension;
mod geo;
mod primitive;
mod scale;
mod scene;
mod spec;
mod transform;

pub use data::{
    ChartColumn, ChartColumnValues, ChartDataError, ChartDataLimits, ChartDataSnapshot,
    ChartDataType, ChartDataset, ChartNullBitmap, ChartValue, NativeChartData,
};
pub use export::{
    ChartExportError, ChartExportMotion, ChartExportRequest, export_chart_png, export_chart_svg,
};
pub use extension::{
    ChartCustomCoordinateContext, ChartCustomSeriesContext, ChartFormatterError,
    ChartFormatterRegistry, ChartSeriesExtensionError, ChartSeriesRegistry, HostChartFormatter,
    HostChartSeries,
};
pub use geo::{
    ChartGeoBounds, ChartGeoError, ChartGeoFeature, ChartGeoMap, ChartGeoPoint, ChartGeoProjection,
    ChartGeoRegistry, ChartGeoRing, EquirectangularProjection, MercatorProjection,
};
pub use primitive::{ChartPrimitiveHandler, chart_primitive_descriptor};
pub use scale::{ChartScale, ChartScaleError, ChartTick};
pub use scene::{
    ChartAxisDomain, ChartDatumRef, ChartLabel, ChartLabelAnchor, ChartMark, ChartMarkGeometry,
    ChartMarkRole, ChartPoint, ChartPrepareError, ChartPreparedData, ChartRect, ChartSemanticDatum,
    ChartTheme, ChartViewport, PreparedChartScene, interpolate_chart_scene, layout_chart_scene,
    layout_chart_scene_with_viewport, prepare_chart_data,
};
pub use spec::{
    ChartAnnotation, ChartAnnotationKind, ChartAnnotationValue, ChartAxisDirection,
    ChartAxisPosition, ChartAxisScale, ChartAxisSpec, ChartBrushMode, ChartChannel,
    ChartCoordinateKind, ChartCoordinateRegion, ChartDiagnostic, ChartDiagnosticSeverity,
    ChartEncode, ChartFormatSpec, ChartLegendPosition, ChartLegendSpec, ChartMotionSpec,
    ChartSeriesKind, ChartSeriesSpec, ChartSpec, ChartSpecError, ChartTimeZone, ChartTooltipSpec,
    ChartTransformSpec,
};
pub use transform::{
    ChartTransformContext, ChartTransformError, ChartTransformRegistry, HostChartTransform,
    apply_chart_transforms,
};

pub(crate) use data::register_chart_data_api;
