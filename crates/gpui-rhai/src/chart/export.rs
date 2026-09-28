use std::collections::BTreeSet;
use std::fmt::Write as _;

use thiserror::Error;

use super::{
    ChartGeoRegistry, ChartMarkGeometry, ChartPrepareError, ChartPreparedData, ChartTheme,
    ChartViewport, PreparedChartScene, layout_chart_scene_with_viewport,
};

const MAX_EXPORT_DIMENSION: u32 = 8_192;
const MAX_EXPORT_PIXELS: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartExportMotion {
    Terminal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartExportRequest {
    pub width: u32,
    pub height: u32,
    pub locale: String,
    pub expected_revision: Option<u64>,
    pub motion: ChartExportMotion,
    pub viewport: ChartViewport,
    pub selected_keys: BTreeSet<String>,
    pub number: Option<crate::NumberMetadata>,
    pub direction: Option<crate::TextDirection>,
}

impl ChartExportRequest {
    #[must_use]
    pub fn terminal(width: u32, height: u32, locale: impl Into<String>) -> Self {
        Self {
            width,
            height,
            locale: locale.into(),
            expected_revision: None,
            motion: ChartExportMotion::Terminal,
            viewport: ChartViewport::default(),
            selected_keys: BTreeSet::new(),
            number: None,
            direction: None,
        }
    }

    #[must_use]
    pub fn with_view_state(
        mut self,
        viewport: ChartViewport,
        selected_keys: impl IntoIterator<Item = String>,
    ) -> Self {
        self.viewport = viewport;
        self.selected_keys = selected_keys.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_locale_context(
        mut self,
        number: Option<crate::NumberMetadata>,
        direction: crate::TextDirection,
    ) -> Self {
        self.number = number;
        self.direction = Some(direction);
        self
    }
}

/// Render a prepared terminal scene as self-contained SVG text.
///
/// # Errors
///
/// Returns for invalid size/locale/revision, layout, or geo preparation.
pub fn export_chart_svg(
    prepared: &ChartPreparedData,
    request: &ChartExportRequest,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
) -> Result<String, ChartExportError> {
    let scene = export_scene(prepared, request, theme, geo)?;
    Ok(scene_to_svg(&scene, theme, &request.locale))
}

/// Render the same terminal scene as encoded PNG bytes.
///
/// # Errors
///
/// Returns SVG preparation, bounded allocation, rasterization, or encoding
/// diagnostics.
pub fn export_chart_png(
    prepared: &ChartPreparedData,
    request: &ChartExportRequest,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
) -> Result<Vec<u8>, ChartExportError> {
    let svg = export_chart_svg(prepared, request, theme, geo)?;
    let tree = usvg::Tree::from_str(&svg, &crate::asset::svg_options())
        .map_err(|error| ChartExportError::Svg(error.to_string()))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(request.width, request.height)
        .ok_or(ChartExportError::Allocation)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| ChartExportError::Png(error.to_string()))
}

fn export_scene(
    prepared: &ChartPreparedData,
    request: &ChartExportRequest,
    theme: &ChartTheme,
    geo: &ChartGeoRegistry,
) -> Result<PreparedChartScene, ChartExportError> {
    validate_request(request)?;
    if let Some(expected) = request.expected_revision
        && expected != prepared.revision()
    {
        return Err(ChartExportError::Revision {
            expected,
            actual: prepared.revision(),
        });
    }
    let mut export_theme = theme.clone();
    export_theme.locale.clone_from(&request.locale);
    if request.number.is_some() {
        export_theme.number.clone_from(&request.number);
    }
    if let Some(direction) = request.direction {
        export_theme.direction = direction;
    }
    let mut scene = layout_chart_scene_with_viewport(
        prepared,
        f64::from(request.width),
        f64::from(request.height),
        &export_theme,
        geo,
        request.viewport,
    )?;
    super::apply_chart_selection(&mut scene, &request.selected_keys, export_theme.selection);
    Ok(scene)
}

fn validate_request(request: &ChartExportRequest) -> Result<(), ChartExportError> {
    if request.width == 0
        || request.height == 0
        || request.width > MAX_EXPORT_DIMENSION
        || request.height > MAX_EXPORT_DIMENSION
        || u64::from(request.width).saturating_mul(u64::from(request.height)) > MAX_EXPORT_PIXELS
    {
        return Err(ChartExportError::Dimensions {
            width: request.width,
            height: request.height,
        });
    }
    if request.locale.trim().is_empty() || request.locale.len() > 64 {
        return Err(ChartExportError::Locale(request.locale.clone()));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn scene_to_svg(scene: &PreparedChartScene, theme: &ChartTheme, locale: &str) -> String {
    let mut output = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-label="{}" lang="{}"><rect width="100%" height="100%" fill="{}"/>"#,
        format_number(scene.width),
        format_number(scene.height),
        format_number(scene.width),
        format_number(scene.height),
        escape_xml(&scene.summary),
        escape_xml(locale),
        color_hex(theme.background)
    );
    output.push_str("<defs>");
    for (key, region) in &scene.plot_regions {
        let _ = write!(
            output,
            r#"<clipPath id="clip-{}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath>"#,
            escape_xml(key),
            format_number(region.x),
            format_number(region.y),
            format_number(region.width),
            format_number(region.height),
        );
    }
    output.push_str("</defs>");
    for mark in scene.marks.iter() {
        let clipped = matches!(
            mark.role,
            super::ChartMarkRole::Data | super::ChartMarkRole::Decoration
        ) && scene.plot_regions.contains_key(&mark.region_key);
        if clipped {
            let _ = write!(
                output,
                r#"<g clip-path="url(#clip-{})">"#,
                escape_xml(&mark.region_key)
            );
        }
        let fill = mark.fill.map_or_else(|| "none".to_owned(), color_hex);
        let (stroke, stroke_width) = mark.stroke.map_or_else(
            || ("none".to_owned(), 0.0),
            |(color, width)| (color_hex(color), width),
        );
        match &mark.geometry {
            ChartMarkGeometry::Rect(rect) => {
                let _ = write!(
                    output,
                    r#"<rect data-key="{}" x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    escape_xml(&mark.key),
                    format_number(rect.x),
                    format_number(rect.y),
                    format_number(rect.width),
                    format_number(rect.height),
                    fill,
                    stroke,
                    format_number(stroke_width)
                );
            }
            ChartMarkGeometry::Circle { center, radius } => {
                let _ = write!(
                    output,
                    r#"<circle data-key="{}" cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    escape_xml(&mark.key),
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius),
                    fill,
                    stroke,
                    format_number(stroke_width)
                );
            }
            ChartMarkGeometry::Polyline { points, width } => {
                let _ = write!(
                    output,
                    r#"<polyline data-key="{}" points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                    escape_xml(&mark.key),
                    svg_points(points),
                    stroke,
                    format_number(*width)
                );
            }
            ChartMarkGeometry::Polygon(points) => {
                let _ = write!(
                    output,
                    r#"<polygon data-key="{}" points="{}" fill="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
                    escape_xml(&mark.key),
                    svg_points(points),
                    fill,
                    stroke,
                    format_number(stroke_width)
                );
            }
            ChartMarkGeometry::CompoundPolygon(rings) => {
                let path = rings
                    .iter()
                    .filter(|ring| !ring.is_empty())
                    .map(|ring| {
                        format!(
                            "M {} Z",
                            ring.iter()
                                .map(|point| format!(
                                    "{} {}",
                                    format_number(point.x),
                                    format_number(point.y)
                                ))
                                .collect::<Vec<_>>()
                                .join(" L ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = write!(
                    output,
                    r#"<path data-key="{}" d="{}" fill="{}" fill-rule="evenodd" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
                    escape_xml(&mark.key),
                    path,
                    fill,
                    stroke,
                    format_number(stroke_width)
                );
            }
        }
        if clipped {
            output.push_str("</g>");
        }
    }
    for label in scene.labels.iter() {
        let anchor = match label.anchor {
            super::scene::ChartLabelAnchor::Start => "start",
            super::scene::ChartLabelAnchor::Center => "middle",
            super::scene::ChartLabelAnchor::End => "end",
        };
        let typography = match label.role {
            super::scene::ChartLabelRole::Title => &theme.title_typography,
            super::scene::ChartLabelRole::Label => &theme.label_typography,
        };
        let font_size = typography_pixels(typography.size, 12.0);
        let font_family = svg_font_family(typography);
        let _ = write!(
            output,
            r#"<text data-key="{}" x="{}" y="{}" fill="{}" font-family="{}" font-size="{}" font-weight="{}" text-anchor="{}" dominant-baseline="hanging">{}</text>"#,
            escape_xml(&label.key),
            format_number(label.position.x),
            format_number(label.position.y),
            color_hex(label.color),
            escape_xml(&font_family),
            format_number(font_size),
            typography.weight,
            anchor,
            escape_xml(&label.text)
        );
    }
    output.push_str("</svg>");
    output
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

fn svg_font_family(typography: &crate::ResolvedTypography) -> String {
    let mut families = typography
        .family
        .iter()
        .cloned()
        .chain(typography.fallbacks.iter().cloned())
        .collect::<Vec<_>>();
    if families.is_empty() {
        families.extend(["system-ui".to_owned(), "sans-serif".to_owned()]);
    }
    families.join(", ")
}

fn svg_points(points: &[super::ChartPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_hex(color: crate::Rgba8) -> String {
    format!("#{:08x}", color.as_rgba_hex())
}

fn format_number(value: f64) -> String {
    let mut value = format!("{value:.3}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartExportError {
    #[error(transparent)]
    Prepare(#[from] ChartPrepareError),
    #[error("chart export size {width}x{height} exceeds the bounded static export policy")]
    Dimensions { width: u32, height: u32 },
    #[error("chart export locale `{0}` is invalid")]
    Locale(String),
    #[error("chart export requested data revision {expected}, but prepared revision is {actual}")]
    Revision { expected: u64, actual: u64 },
    #[error("chart export SVG could not be parsed: {0}")]
    Svg(String),
    #[error("chart export could not allocate the PNG surface")]
    Allocation,
    #[error("chart export PNG encoding failed: {0}")]
    Png(String),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::chart::{
        ChartBrushMode, ChartChannel, ChartCoordinateRegion, ChartDataLimits, ChartDataset,
        ChartEncode, ChartLegendSpec, ChartMotionSpec, ChartSeriesKind, ChartSeriesSpec, ChartSpec,
        ChartTooltipSpec, ChartTransformRegistry, ChartValue, NativeChartData, prepare_chart_data,
    };

    fn prepared_with_id(id: &str) -> ChartPreparedData {
        let rows = vec![
            BTreeMap::from([
                ("id".to_owned(), ChartValue::String(id.to_owned())),
                ("x".to_owned(), ChartValue::String("A".to_owned())),
                ("y".to_owned(), ChartValue::Number(4.0)),
            ]),
            BTreeMap::from([
                ("id".to_owned(), ChartValue::String("b".to_owned())),
                ("x".to_owned(), ChartValue::String("B".to_owned())),
                ("y".to_owned(), ChartValue::Number(7.0)),
            ]),
        ];
        let dataset = ChartDataset::from_chart_rows(
            "main",
            &rows,
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let data = NativeChartData::new([dataset], ChartDataLimits::default()).unwrap();
        let spec = ChartSpec {
            title: Some("A & B".to_owned()),
            description: None,
            regions: vec![ChartCoordinateRegion::default()],
            axes: Vec::new(),
            series: vec![ChartSeriesSpec {
                key: "bars".to_owned(),
                name: "Bars".to_owned(),
                kind: ChartSeriesKind::Bar,
                dataset: "main".to_owned(),
                coordinate: "main".to_owned(),
                x_axis: None,
                y_axis: None,
                encode: ChartEncode::new([
                    (ChartChannel::X, "x".to_owned()),
                    (ChartChannel::Y, "y".to_owned()),
                ]),
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
        };
        prepare_chart_data(
            spec,
            &data.snapshot(),
            &ChartTransformRegistry::new(),
            &crate::ChartSeriesRegistry::new(),
            &crate::ChartFormatterRegistry::new(),
        )
        .unwrap()
    }

    fn prepared() -> ChartPreparedData {
        prepared_with_id("a")
    }

    #[test]
    fn svg_and_png_export_the_same_prepared_terminal_scene() {
        let prepared = prepared();
        let request = ChartExportRequest::terminal(640, 400, "en-US");
        let theme = ChartTheme::default();
        let geo = ChartGeoRegistry::new();
        let svg = export_chart_svg(&prepared, &request, &theme, &geo).unwrap();
        assert!(svg.contains("A &amp; B"));
        assert!(svg.contains("data-key=\"data|4:main|4:bars|1:a|4:body\""));
        let png = export_chart_png(&prepared, &request, &theme, &geo).unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[test]
    fn export_and_layout_share_theme_typography() {
        let prepared = prepared();
        let request = ChartExportRequest::terminal(640, 400, "zh-CN");
        let theme = ChartTheme {
            title_typography: crate::ResolvedTypography {
                family: Some("JetBrains Mono".to_owned()),
                fallbacks: vec!["PingFang SC".to_owned()],
                size: crate::Length::Pixels(24.0),
                line_height: crate::Length::Pixels(32.0),
                weight: 700,
            },
            label_typography: crate::ResolvedTypography {
                family: Some("JetBrains Mono".to_owned()),
                fallbacks: vec!["PingFang SC".to_owned()],
                size: crate::Length::Pixels(18.0),
                line_height: crate::Length::Pixels(26.0),
                weight: 500,
            },
            ..ChartTheme::default()
        };
        let geo = ChartGeoRegistry::new();
        let svg = export_chart_svg(&prepared, &request, &theme, &geo).unwrap();
        assert!(svg.contains(r#"font-family="JetBrains Mono, PingFang SC""#));
        assert!(svg.contains(r#"font-size="24" font-weight="700""#));
        assert!(svg.contains(r#"font-size="18" font-weight="500""#));

        let scene = export_scene(&prepared, &request, &theme, &geo).unwrap();
        assert!(scene.plot_regions["main"].x > 52.0);
    }

    #[test]
    fn selected_business_key_does_not_style_the_legend_control() {
        let prepared = prepared_with_id("legend");
        let request = ChartExportRequest::terminal(640, 400, "en-US")
            .with_view_state(ChartViewport::default(), ["legend".to_owned()]);
        let svg = export_chart_svg(
            &prepared,
            &request,
            &ChartTheme::default(),
            &ChartGeoRegistry::new(),
        )
        .unwrap();
        let legend = svg
            .split("data-key=\"legend:bars\"")
            .nth(1)
            .unwrap()
            .split("/>")
            .next()
            .unwrap();
        assert!(legend.contains("stroke=\"none\""), "{legend}");
        assert!(legend.contains("stroke-width=\"0\""), "{legend}");
    }
}
