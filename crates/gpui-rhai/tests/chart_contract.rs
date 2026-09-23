#![cfg(feature = "charts")]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_rhai::*;
use serde_json::{Value, json};

fn ui(value: Value) -> UiValue {
    match value {
        Value::Null => UiValue::Null,
        Value::Bool(value) => UiValue::Bool(value),
        Value::Number(value) => value.as_i64().map_or_else(
            || UiValue::Float(value.as_f64().expect("finite JSON number")),
            UiValue::Integer,
        ),
        Value::String(value) => UiValue::String(value),
        Value::Array(values) => UiValue::Array(values.into_iter().map(ui).collect()),
        Value::Object(values) => UiValue::Map(
            values
                .into_iter()
                .map(|(key, value)| (key, ui(value)))
                .collect(),
        ),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn dataset(name: &str, rows: Value) -> ChartDataset {
    let rows = rows
        .as_array()
        .expect("rows")
        .iter()
        .cloned()
        .map(|row| match ui(row) {
            UiValue::Map(row) => row,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    ChartDataset::from_rows(
        name,
        &rows,
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .unwrap()
}

fn prepared(spec: Value, datasets: impl IntoIterator<Item = ChartDataset>) -> ChartPreparedData {
    let spec = ChartSpec::from_ui_value(&ui(spec)).unwrap();
    let data = NativeChartData::new(datasets, ChartDataLimits::default()).unwrap();
    prepare_chart_data(
        spec,
        &data.snapshot(),
        &ChartTransformRegistry::new(),
        &ChartSeriesRegistry::new(),
        &ChartFormatterRegistry::new(),
    )
    .unwrap()
}

fn scene(spec: Value, dataset: ChartDataset) -> PreparedChartScene {
    layout_chart_scene(
        &prepared(spec, [dataset]),
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap()
}

#[allow(clippy::cast_precision_loss)]
fn center(mark: &ChartMark) -> ChartPoint {
    match &mark.geometry {
        ChartMarkGeometry::Circle { center, .. } => *center,
        ChartMarkGeometry::Rect(rect) => rect.center(),
        ChartMarkGeometry::Polygon(points) | ChartMarkGeometry::Polyline { points, .. } => {
            ChartPoint {
                x: points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
                y: points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
            }
        }
        ChartMarkGeometry::CompoundPolygon(rings) => center(&ChartMark {
            key: "temporary".to_owned(),
            region_key: "main".to_owned(),
            role: ChartMarkRole::Decoration,
            datum: None,
            series_key: "temporary".to_owned(),
            datum_key: "temporary".to_owned(),
            geometry: ChartMarkGeometry::Polygon(rings[0].clone()),
            fill: None,
            stroke: None,
            label: String::new(),
            value: None,
            interactive: false,
            selected: false,
        }),
    }
}

#[test]
fn shared_axes_compile_one_coordinate_fact() {
    let first = dataset(
        "main",
        json!([{"id":"a0","x":0,"y":1},{"id":"a1","x":1,"y":2}]),
    );
    let second = dataset(
        "other",
        json!([{"id":"b0","x":0,"y":10},{"id":"b1","x":100,"y":20}]),
    );
    let scene = layout_chart_scene(
        &prepared(
            json!({
                "axes":[
                    {"key":"x","position":"bottom"},
                    {"key":"ya","position":"left"},
                    {"key":"yb","position":"right"}
                ],
                "series":[
                    {"key":"a","kind":"scatter","x_axis":"x","y_axis":"ya","encode":{"x":"x","y":"y"}},
                    {"key":"b","dataset":"other","kind":"scatter","x_axis":"x","y_axis":"yb","encode":{"x":"x","y":"y"}}
                ]
            }),
            [first, second],
        ),
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    let x1 = center(
        scene
            .marks
            .iter()
            .find(|mark| mark.datum_key == "a1")
            .unwrap(),
    )
    .x;
    let x100 = center(
        scene
            .marks
            .iter()
            .find(|mark| mark.datum_key == "b1")
            .unwrap(),
    )
    .x;
    assert!(
        x1 < x100 - 100.0,
        "shared X axis used private per-pair domains"
    );
}

#[test]
fn category_log_and_viewport_scales_keep_ticks_and_marks_coherent() {
    let category = scene(
        json!({"axes":[{"key":"x","position":"bottom","scale":"category"}],"series":[{"key":"s","kind":"bar","x_axis":"x","encode":{"x":"x","y":"y"}}]}),
        dataset(
            "main",
            json!([{"id":"a","x":1,"y":1},{"id":"b","x":100,"y":10}]),
        ),
    );
    assert_eq!(
        category
            .marks
            .iter()
            .filter(|mark| mark.role == ChartMarkRole::Data)
            .count(),
        2
    );
    assert!(category.labels.iter().any(|label| label.text == "1"));
    assert!(category.labels.iter().any(|label| label.text == "100"));

    let log = scene(
        json!({"axes":[{"key":"y","position":"left","scale":"log","min":1,"max":100}],"series":[{"key":"s","kind":"bar","y_axis":"y","encode":{"x":"x","y":"y"}}]}),
        dataset(
            "main",
            json!([{"id":"a","x":"A","y":1},{"id":"b","x":"B","y":10}]),
        ),
    );
    assert_eq!(
        log.marks
            .iter()
            .filter(|mark| mark.role == ChartMarkRole::Data)
            .count(),
        2
    );

    let prepared = prepared(
        json!({"legend":{"visible":false},"series":[{"key":"s","kind":"scatter","encode":{"x":"x","y":"y"}}]}),
        [dataset(
            "main",
            json!([{"id":"a","x":0,"y":0},{"id":"mid","x":50,"y":50},{"id":"b","x":100,"y":100}]),
        )],
    );
    let viewport = layout_chart_scene_with_viewport(
        &prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
        ChartViewport {
            zoom: 2.0,
            pan: ChartPoint::default(),
        },
    )
    .unwrap();
    let midpoint = center(
        viewport
            .marks
            .iter()
            .find(|mark| mark.datum_key == "mid")
            .unwrap(),
    );
    let tick = viewport
        .labels
        .iter()
        .filter(|label| label.key.starts_with("axis:main:x:"))
        .min_by(|left, right| {
            (left.position.x - midpoint.x)
                .abs()
                .total_cmp(&(right.position.x - midpoint.x).abs())
        })
        .unwrap();
    assert!((midpoint.x - tick.position.x).abs() < 0.01);
    assert!((tick.text.parse::<f64>().unwrap() - 50.0).abs() < 0.01);
}

#[test]
fn stacked_bars_use_interval_domain_and_split_signs() {
    let scene = scene(
        json!({"legend":{"visible":false},"series":[
            {"key":"a","kind":"bar","stack":"g","encode":{"x":"x","y":"a"}},
            {"key":"b","kind":"bar","stack":"g","encode":{"x":"x","y":"b"}}
        ]}),
        dataset(
            "main",
            json!([{"id":"p","x":"P","a":10,"b":10},{"id":"n","x":"N","a":5,"b":-4}]),
        ),
    );
    let plot = scene.plot_regions["main"];
    for mark in scene
        .marks
        .iter()
        .filter(|mark| mark.role == ChartMarkRole::Data)
    {
        let ChartMarkGeometry::Rect(rect) = mark.geometry else {
            continue;
        };
        assert!(rect.y >= plot.y - 0.01);
        assert!(rect.y + rect.height <= plot.y + plot.height + 0.01);
    }
    let positive = scene
        .marks
        .iter()
        .find(|mark| mark.series_key == "a" && mark.datum_key == "n")
        .map(center)
        .unwrap();
    let negative = scene
        .marks
        .iter()
        .find(|mark| mark.series_key == "b" && mark.datum_key == "n")
        .map(center)
        .unwrap();
    assert!(positive.y < negative.y);
}

#[test]
fn transform_pipeline_preserves_schema_types_and_valid_empty_sets() {
    let input = dataset(
        "main",
        json!([{"id":"a","g":1,"v":2},{"id":"b","g":1,"v":3}]),
    );
    let aggregate = apply_chart_transforms(
        &input,
        &[ChartTransformSpec::Aggregate {
            group_by: vec!["g".to_owned()],
            dimension: "v".to_owned(),
            operation: "sum".to_owned(),
            output: "total".to_owned(),
        }],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    )
    .unwrap();
    assert_eq!(
        aggregate.column("g").unwrap().data_type(),
        ChartDataType::Integer
    );
    assert_eq!(
        aggregate.column("total").unwrap().value(0),
        Some(ChartValue::Number(5.0))
    );

    let empty = apply_chart_transforms(
        &input,
        &[ChartTransformSpec::Filter {
            dimension: "v".to_owned(),
            operator: "gt".to_owned(),
            value: UiValue::Integer(99),
        }],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    )
    .unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.columns().count(), input.columns().count());
    assert_eq!(empty.key_dimension(), Some("id"));

    let prepared = prepared(
        json!({"series":[{"key":"s","kind":"bar","transforms":[{"kind":"aggregate","group_by":["g"],"dimension":"v","operation":"sum","output":"total"}],"encode":{"x":"g","y":"total"}}]}),
        [input],
    );
    assert_eq!(prepared.revision(), 1);
}

#[test]
fn null_gaps_survive_automatic_sampling_and_semantics_keep_all_keys() {
    let rows = (0..5000)
        .map(|index| json!({"id":format!("r{index}"),"x":index,"y":if index == 2400 { Value::Null } else { json!(index % 9) }}))
        .collect::<Vec<_>>();
    let scene = scene(
        json!({"legend":{"visible":false},"series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]}),
        dataset("main", Value::Array(rows)),
    );
    assert!(scene.sampled_series.iter().any(|series| series == "s"));
    assert_eq!(scene.semantics.len(), 5000);
    assert!(scene.semantics.iter().any(|datum| datum.datum_key == "r4"));
    let lines = scene
        .marks
        .iter()
        .filter(|mark| {
            matches!(mark.geometry, ChartMarkGeometry::Polyline { .. })
                && mark.role == ChartMarkRole::Decoration
        })
        .count();
    assert_eq!(lines, 2, "null gap was joined into one path");
}

#[test]
fn categorical_line_and_area_survive_the_automatic_sampling_boundary() {
    for kind in ["line", "area"] {
        for row_count in [3_999, 4_000, 4_001, 10_000] {
            let rows = (0..row_count)
                .map(|index| {
                    json!({
                        "id": format!("r{index}"),
                        "x": format!("C{index}"),
                        "y": if index == row_count / 2 {
                            Value::Null
                        } else {
                            json!(index % 7)
                        }
                    })
                })
                .collect::<Vec<_>>();
            let scene = scene(
                json!({
                    "legend":{"visible":false},
                    "series":[{"key":"s","kind":kind,"encode":{"x":"x","y":"y"}}]
                }),
                dataset("main", Value::Array(rows)),
            );
            assert_eq!(scene.semantics.len(), row_count, "{kind} {row_count}");
            assert!(
                scene
                    .marks
                    .iter()
                    .any(|mark| mark.role == ChartMarkRole::Data),
                "{kind} disappeared at {row_count} rows"
            );
            assert!(
                scene.semantics.iter().any(|datum| datum.datum_key == "r0")
                    && scene
                        .semantics
                        .iter()
                        .any(|datum| datum.datum_key == format!("r{}", row_count - 1)),
                "{kind} changed category identity at {row_count} rows"
            );
        }
    }
}

#[test]
fn gauge_and_radar_share_explicit_value_domains() {
    let gauge = scene(
        json!({"regions":[{"key":"main","kind":"polar"}],"axes":[{"key":"r","position":"radial","min":0,"max":100}],"series":[{"key":"g","kind":"gauge","encode":{"name":"id","value":"v"}}]}),
        dataset("main", json!([{"id":"meter","v":25}])),
    );
    let track = gauge
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Decoration && mark.datum_key == "gauge_track")
        .unwrap();
    let value = gauge
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Data && mark.series_key == "g")
        .unwrap();
    assert_ne!(track.geometry, value.geometry);

    let radar = scene(
        json!({"regions":[{"key":"main","kind":"polar"}],"series":[
            {"key":"a","kind":"radar","encode":{"name":"n","value":"a"}},
            {"key":"b","kind":"radar","encode":{"name":"n","value":"b"}}
        ]}),
        dataset(
            "main",
            json!([
                {"id":"i1","n":"A","a":10,"b":100},
                {"id":"i2","n":"B","a":20,"b":200},
                {"id":"i3","n":"C","a":30,"b":300}
            ]),
        ),
    );
    let a = radar
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Decoration && mark.series_key == "a")
        .unwrap();
    let b = radar
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Decoration && mark.series_key == "b")
        .unwrap();
    assert_ne!(a.geometry, b.geometry);
}

#[test]
fn mark_identity_role_and_color_are_stable() {
    let scene = scene(
        json!({"regions":[{"key":"left","kind":"cartesian_2d","column":0},{"key":"right","kind":"cartesian_2d","column":1}],"series":[
            {"key":"a","kind":"line","coordinate":"left","visible":false,"encode":{"x":"x","y":"a"}},
            {"key":"b","kind":"line","coordinate":"right","encode":{"x":"x","y":"b"}}
        ]}),
        dataset(
            "main",
            json!([{"id":"legend","x":0,"a":1,"b":2},{"id":"other","x":1,"a":2,"b":3}]),
        ),
    );
    assert_eq!(
        scene
            .marks
            .iter()
            .map(|mark| &mark.key)
            .collect::<BTreeSet<_>>()
            .len(),
        scene.marks.len()
    );
    let datum = scene
        .marks
        .iter()
        .find(|mark| mark.datum_key == "legend" && mark.role == ChartMarkRole::Data)
        .unwrap();
    assert_eq!(datum.role, ChartMarkRole::Data);
    let legend = scene
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Legend && mark.series_key == "b")
        .unwrap();
    let series = scene
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Data && mark.series_key == "b")
        .unwrap();
    assert_eq!(legend.fill, series.fill);
}

#[test]
fn png_rasterization_keeps_text() {
    let baseline_dataset = dataset(
        "main",
        json!([{"id":"a","x":0,"y":1},{"id":"b","x":1,"y":2}]),
    );
    let first = prepared(
        json!({"title":"FIRST","series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]}),
        [baseline_dataset.clone()],
    );
    let second = prepared(
        json!({"title":"SECOND TITLE","series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]}),
        [baseline_dataset],
    );
    let request = ChartExportRequest::terminal(640, 400, "en");
    let theme = ChartTheme::default();
    let geo = ChartGeoRegistry::new();
    assert_ne!(
        export_chart_png(&first, &request, &theme, &geo).unwrap(),
        export_chart_png(&second, &request, &theme, &geo).unwrap(),
        "different titles produced identical PNG pixels"
    );
    let cjk = prepared(
        json!({"title":"中文标题","series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]}),
        [dataset(
            "main",
            json!([{"id":"a","x":0,"y":1},{"id":"b","x":1,"y":2}]),
        )],
    );
    assert_ne!(
        export_chart_png(&first, &request, &theme, &geo).unwrap(),
        export_chart_png(&cjk, &request, &theme, &geo).unwrap(),
        "CJK title did not affect rasterized pixels"
    );
}

#[test]
fn rejected_geo_projection_does_not_mix_coordinate_spaces() {
    let prepared = prepared(
        json!({"legend":{"visible":false},"regions":[{"key":"main","kind":"geo_2d","map":"world","projection":"mercator"}],"series":[{"key":"g","kind":"geo_scatter","encode":{"longitude":"lon","latitude":"lat","value":"v"}}]}),
        [dataset(
            "main",
            json!([{"id":"polar","lon":0,"lat":89,"v":1}]),
        )],
    );
    let geo = ChartGeoRegistry::new();
    geo.register_map(ChartGeoMap::from_geojson("world", r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"world","properties":{},"geometry":{"type":"Polygon","coordinates":[[[-170,-80],[170,-80],[170,80],[-170,80],[-170,-80]]]}}]}"#).unwrap()).unwrap();
    let scene = layout_chart_scene(&prepared, 640.0, 400.0, &ChartTheme::default(), &geo).unwrap();
    assert!(
        !scene
            .marks
            .iter()
            .any(|mark| mark.role == ChartMarkRole::Data)
    );
    assert!(
        scene
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.contains("projection_rejected"))
    );
}

#[test]
#[allow(clippy::items_after_statements)]
fn failed_duplicate_registration_is_atomic_and_custom_polar_is_dispatched() {
    struct Add(i64);
    impl HostChartTransform for Add {
        fn transform(
            &self,
            input: &ChartDataset,
            _: &UiValue,
            _: ChartTransformContext,
        ) -> Result<ChartDataset, String> {
            let mut rows = input.rows();
            for row in &mut rows {
                row.insert("registered".to_owned(), ChartValue::Integer(self.0));
            }
            ChartDataset::from_chart_rows(
                input.name(),
                &rows,
                input.key_dimension().map(ToOwned::to_owned),
                ChartDataLimits::default(),
            )
            .map_err(|error| error.to_string())
        }
    }
    let transforms = ChartTransformRegistry::new();
    transforms.register("atomic", Add(1)).unwrap();
    assert!(transforms.register("atomic", Add(2)).is_err());
    let input = dataset("main", json!([{"id":"a","v":1}]));
    let output = apply_chart_transforms(
        &input,
        &[ChartTransformSpec::Host {
            id: "atomic".to_owned(),
            options: UiValue::Null,
        }],
        &transforms,
        ChartTransformContext::default(),
    )
    .unwrap();
    assert_eq!(
        output.column("registered").unwrap().value(0),
        Some(ChartValue::Integer(1))
    );

    struct Custom(Arc<AtomicUsize>);
    impl HostChartSeries for Custom {
        fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(vec![ChartMark {
                key: "main:custom:a".to_owned(),
                region_key: "main".to_owned(),
                role: ChartMarkRole::Data,
                datum: Some(ChartDatumRef {
                    dataset: "main".to_owned(),
                    series: context.spec.key.clone(),
                    key: "a".to_owned(),
                }),
                series_key: context.spec.key.clone(),
                datum_key: "a".to_owned(),
                geometry: ChartMarkGeometry::Circle {
                    center: context.bounds.center(),
                    radius: 8.0,
                },
                fill: Some(context.theme.palette[0]),
                stroke: None,
                label: "custom".to_owned(),
                value: None,
                interactive: true,
                selected: false,
            }])
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let series = ChartSeriesRegistry::new();
    series.register("custom", Custom(calls.clone())).unwrap();
    let spec = ChartSpec::from_ui_value(&ui(json!({"regions":[{"key":"main","kind":"polar"}],"series":[{"key":"custom","kind":"custom","renderer":"custom","encode":{}}]}))).unwrap();
    let data = NativeChartData::new([input], ChartDataLimits::default()).unwrap();
    let prepared = prepare_chart_data(
        spec,
        &data.snapshot(),
        &ChartTransformRegistry::new(),
        &series,
        &ChartFormatterRegistry::new(),
    )
    .unwrap();
    let scene = layout_chart_scene(
        &prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(scene.marks.iter().any(|mark| mark.key == "main:custom:a"));
}

#[test]
fn viewport_uses_the_resolved_axis_plan_for_explicit_log_and_category_scales() {
    let explicit = prepared(
        json!({
            "legend":{"visible":false},
            "axes":[
                {"key":"x","position":"bottom","min":0,"max":100},
                {"key":"y","position":"left","min":0,"max":100}
            ],
            "series":[{"key":"s","kind":"scatter","x_axis":"x","y_axis":"y","encode":{"x":"x","y":"y"}}]
        }),
        [dataset(
            "main",
            json!([{"id":"a","x":0,"y":0},{"id":"b","x":100,"y":100}]),
        )],
    );
    let base = layout_chart_scene(
        &explicit,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    let zoomed = layout_chart_scene_with_viewport(
        &explicit,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
        ChartViewport {
            zoom: 2.0,
            pan: ChartPoint::default(),
        },
    )
    .unwrap();
    assert_ne!(base.marks, zoomed.marks);
    assert_ne!(base.labels, zoomed.labels);
    let x = zoomed
        .axis_domains
        .iter()
        .find_map(|(key, domain)| key.contains(":x:").then_some(domain))
        .unwrap();
    assert_eq!(x.full, (0.0, 100.0));
    assert_eq!(x.visible, (25.0, 75.0));

    let log = prepared(
        json!({
            "axes":[{"key":"y","position":"left","scale":"log"}],
            "series":[{"key":"s","kind":"line","y_axis":"y","encode":{"x":"x","y":"y"}}]
        }),
        [dataset(
            "main",
            json!([{"id":"a","x":0,"y":1},{"id":"b","x":1,"y":10}]),
        )],
    );
    let log_scene = layout_chart_scene_with_viewport(
        &log,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
        ChartViewport {
            zoom: 0.5,
            pan: ChartPoint::default(),
        },
    )
    .unwrap();
    let log_y = log_scene
        .axis_domains
        .iter()
        .find_map(|(key, domain)| key.contains(":y:").then_some(domain))
        .unwrap();
    assert!(log_y.visible.0 > 0.0 && log_y.visible.1 > log_y.visible.0);

    let categories = scene(
        json!({
            "axes":[{"key":"x","position":"bottom"}],
            "series":[{"key":"s","kind":"bar","x_axis":"x","encode":{"x":"x","y":"y"}}]
        }),
        dataset(
            "main",
            json!([{"id":"a","x":"Alpha","y":1},{"id":"b","x":"Beta","y":2}]),
        ),
    );
    assert!(categories.labels.iter().any(|label| label.text == "Alpha"));
    assert!(categories.labels.iter().any(|label| label.text == "Beta"));
}

#[test]
fn pan_moves_content_with_the_pointer_on_normal_and_reversed_axes() {
    for direction in ["normal", "reversed"] {
        let prepared = prepared(
            json!({
                "legend":{"visible":false},
                "axes":[
                    {"key":"x","position":"bottom","direction":direction,"min":0,"max":100},
                    {"key":"y","position":"left","direction":direction,"min":0,"max":100}
                ],
                "series":[{"key":"s","kind":"scatter","x_axis":"x","y_axis":"y","encode":{"x":"x","y":"y"}}]
            }),
            [dataset(
                "main",
                json!([{"id":"a","x":20,"y":20},{"id":"b","x":80,"y":80}]),
            )],
        );
        let base = layout_chart_scene(
            &prepared,
            640.0,
            400.0,
            &ChartTheme::default(),
            &ChartGeoRegistry::new(),
        )
        .unwrap();
        let panned = layout_chart_scene_with_viewport(
            &prepared,
            640.0,
            400.0,
            &ChartTheme::default(),
            &ChartGeoRegistry::new(),
            ChartViewport {
                zoom: 1.0,
                pan: ChartPoint { x: 20.0, y: 20.0 },
            },
        )
        .unwrap();
        let base = center(
            base.marks
                .iter()
                .find(|mark| mark.role == ChartMarkRole::Data && mark.datum_key == "a")
                .unwrap(),
        );
        let panned = center(
            panned
                .marks
                .iter()
                .find(|mark| mark.role == ChartMarkRole::Data && mark.datum_key == "a")
                .unwrap(),
        );
        assert!((panned.x - base.x - 20.0).abs() < 0.01, "{direction}");
        assert!((panned.y - base.y - 20.0).abs() < 0.01, "{direction}");
    }
}

#[test]
fn normalized_axis_and_structured_mark_identities_accept_legal_combinations() {
    let data = dataset(
        "main",
        json!([{"id":"line:0","x":0,"y":1},{"id":"b","x":1,"y":2}]),
    );
    for spec in [
        json!({
            "axes":[
                {"key":"x","position":"bottom"},
                {"key":"ya","position":"left"},
                {"key":"yb","position":"right"}
            ],
            "annotations":[{"key":"limit","kind":"baseline","axis":"y","value":1}],
            "series":[
                {"key":"a","kind":"scatter","x_axis":"x","y_axis":"ya","encode":{"x":"x","y":"y"}},
                {"key":"b","kind":"scatter","x_axis":"x","y_axis":"yb","encode":{"x":"x","y":"y"}}
            ]
        }),
        json!({
            "axes":[{"key":"x","position":"bottom"}],
            "series":[
                {"key":"a","kind":"scatter","encode":{"x":"x","y":"y"}},
                {"key":"b","kind":"scatter","x_axis":"x","encode":{"x":"x","y":"y"}}
            ]
        }),
        json!({"series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]}),
    ] {
        let scene = scene(spec, data.clone());
        assert_eq!(
            scene
                .marks
                .iter()
                .map(|mark| &mark.key)
                .collect::<BTreeSet<_>>()
                .len(),
            scene.marks.len()
        );
    }
}

#[test]
fn primary_axis_and_annotations_do_not_depend_on_axis_names_or_empty_groups() {
    let data = dataset(
        "main",
        json!([
            {"id":"a","x":0,"a":0,"b":0},
            {"id":"b","x":1,"a":10,"b":1000}
        ]),
    );
    let mut annotation_positions = Vec::new();
    for (left, right) in [("a_left", "z_right"), ("z_left", "a_right")] {
        let scene = scene(
            json!({
                "legend":{"visible":false},
                "axes":[
                    {"key":"x","position":"bottom"},
                    {"key":left,"position":"left"},
                    {"key":right,"position":"right"}
                ],
                "annotations":[{"key":"threshold","kind":"baseline","axis":"y","value":5}],
                "series":[
                    {"key":"a","kind":"line","x_axis":"x","y_axis":left,"encode":{"x":"x","y":"a"}},
                    {"key":"b","kind":"line","x_axis":"x","y_axis":right,"encode":{"x":"x","y":"b"}}
                ]
            }),
            data.clone(),
        );
        annotation_positions.push(center(
            scene
                .marks
                .iter()
                .find(|mark| mark.role == ChartMarkRole::Annotation)
                .unwrap(),
        ));
    }
    assert!(
        (annotation_positions[0].y - annotation_positions[1].y).abs() < 0.01,
        "renaming axis IDs changed annotation semantics"
    );

    let shared = dataset(
        "main",
        json!([{"id":"a","x":0,"y":0},{"id":"b","x":10,"y":10}]),
    );
    let empty = shared.select_rows(&[]).unwrap();
    let full = ChartDataset::from_chart_rows(
        "other",
        &shared.rows(),
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .unwrap();
    let prepared = prepared(
        json!({
            "axes":[
                {"key":"x","position":"bottom"},
                {"key":"a_empty","position":"left"},
                {"key":"z_full","position":"right"}
            ],
            "annotations":[{"key":"baseline","kind":"baseline","axis":"y","value":5}],
            "series":[
                {"key":"empty","kind":"line","x_axis":"x","y_axis":"a_empty","encode":{"x":"x","y":"y"}},
                {"key":"full","kind":"line","dataset":"other","x_axis":"x","y_axis":"z_full","encode":{"x":"x","y":"y"}}
            ]
        }),
        [empty, full],
    );
    let empty_group_scene = layout_chart_scene(
        &prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    assert!(
        empty_group_scene
            .labels
            .iter()
            .any(|label| label.key.starts_with("axis:main:x:"))
    );
    assert_eq!(
        empty_group_scene
            .marks
            .iter()
            .filter(|mark| mark.role == ChartMarkRole::Annotation)
            .count(),
        1
    );
}

#[test]
fn annotations_compile_independent_crossed_primary_axes() {
    let crossed = scene(
        json!({
            "legend":{"visible":false},
            "axes":[
                {"key":"primary_x","position":"bottom"},
                {"key":"primary_y","position":"left"},
                {"key":"secondary_x","position":"top"},
                {"key":"secondary_y","position":"right"}
            ],
            "series":[
                {"key":"a","kind":"scatter","x_axis":"primary_x","y_axis":"secondary_y","encode":{"x":"x1","y":"y2"}},
                {"key":"b","kind":"scatter","x_axis":"secondary_x","y_axis":"primary_y","encode":{"x":"x2","y":"y1"}}
            ],
            "annotations":[{"key":"center","kind":"mark_point","x":5,"y":5}]
        }),
        dataset(
            "main",
            json!([
                {"id":"a","x1":0,"x2":0,"y1":0,"y2":0},
                {"id":"b","x1":10,"x2":1000,"y1":10,"y2":1000}
            ]),
        ),
    );
    let expected = crossed.plot_regions["main"].center();
    let actual = center(
        crossed
            .marks
            .iter()
            .find(|mark| mark.role == ChartMarkRole::Annotation)
            .unwrap(),
    );
    assert!((actual.x - expected.x).abs() < 0.01, "{actual:?}");
    assert!((actual.y - expected.y).abs() < 0.01, "{actual:?}");
}

#[test]
fn axis_schema_fact_is_shared_by_data_ticks_and_annotations() {
    let empty = dataset("main", json!([{"id":"empty","x":"A","y":1}]))
        .select_rows(&[])
        .unwrap();
    let numeric_source = dataset(
        "main",
        json!([{"id":"zero","x":0,"y":0},{"id":"ten","x":10,"y":10}]),
    );
    let numeric = ChartDataset::from_chart_rows(
        "numeric",
        &numeric_source.rows(),
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .unwrap();
    let prepared = prepared(
        json!({
            "legend":{"visible":false},
            "axes":[
                {"key":"x","position":"bottom"},
                {"key":"empty_y","position":"left"},
                {"key":"numeric_y","position":"right"}
            ],
            "series":[
                {"key":"empty","kind":"scatter","x_axis":"x","y_axis":"empty_y","encode":{"x":"x","y":"y"}},
                {"key":"numeric","kind":"scatter","dataset":"numeric","x_axis":"x","y_axis":"numeric_y","encode":{"x":"x","y":"y"}}
            ],
            "annotations":[{"key":"same_value","kind":"mark_point","x":10,"y":10}]
        }),
        [empty, numeric],
    );
    let scene = layout_chart_scene(
        &prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    let data = center(
        scene
            .marks
            .iter()
            .find(|mark| mark.role == ChartMarkRole::Data && mark.datum_key == "ten")
            .unwrap(),
    );
    let annotation = center(
        scene
            .marks
            .iter()
            .find(|mark| mark.role == ChartMarkRole::Annotation)
            .unwrap(),
    );
    assert!(
        (data.x - annotation.x).abs() < 0.01,
        "{data:?} {annotation:?}"
    );
    assert!(
        (data.y - annotation.y).abs() < 0.01,
        "{data:?} {annotation:?}"
    );
    assert_eq!(
        scene.axis_domains["main:x:x"].scale,
        ChartAxisScale::Category
    );
}

#[test]
fn empty_data_preserves_schema_and_produces_a_valid_scene() {
    let input = dataset(
        "main",
        json!([{"id":"a","g":1,"x":"A","y":1},{"id":"b","g":2,"x":"B","y":2}]),
    );
    let filtered = apply_chart_transforms(
        &input,
        &[
            ChartTransformSpec::Filter {
                dimension: "y".to_owned(),
                operator: "gt".to_owned(),
                value: UiValue::Integer(99),
            },
            ChartTransformSpec::Aggregate {
                group_by: vec!["g".to_owned()],
                dimension: "y".to_owned(),
                operation: "sum".to_owned(),
                output: "total".to_owned(),
            },
        ],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    )
    .unwrap();
    assert!(filtered.is_empty());
    assert_eq!(
        filtered.column("g").unwrap().data_type(),
        ChartDataType::Integer
    );
    assert_eq!(
        filtered.column("total").unwrap().data_type(),
        ChartDataType::Number
    );

    let mut empty_spec = ChartSpec::from_ui_value(&ui(json!({
        "series":[{"key":"s","kind":"bar","encode":{"x":"x","y":"y"}}]
    })))
    .unwrap();
    empty_spec.series[0].transforms = vec![ChartTransformSpec::Filter {
        dimension: "y".to_owned(),
        operator: "gt".to_owned(),
        value: UiValue::Integer(99),
    }];
    let native = NativeChartData::new([input], ChartDataLimits::default()).unwrap();
    let empty_prepared = prepare_chart_data(
        empty_spec,
        &native.snapshot(),
        &ChartTransformRegistry::new(),
        &ChartSeriesRegistry::new(),
        &ChartFormatterRegistry::new(),
    )
    .unwrap();
    let empty_scene = layout_chart_scene(
        &empty_prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    assert!(
        !empty_scene
            .marks
            .iter()
            .any(|mark| mark.role == ChartMarkRole::Data)
    );
}

#[test]
fn draw_sampling_does_not_change_the_semantic_domain() {
    let sample_data = dataset(
        "main",
        json!([
            {"id":"a","x":0,"y":0},
            {"id":"b","x":1,"y":-500},
            {"id":"c","x":2,"y":1000},
            {"id":"d","x":3,"y":0}
        ]),
    );
    let mut sample_spec = ChartSpec::from_ui_value(&ui(json!({
        "series":[{"key":"s","kind":"line","encode":{"x":"x","y":"y"}}]
    })))
    .unwrap();
    sample_spec.series[0].transforms = vec![ChartTransformSpec::Downsample {
        x: "x".to_owned(),
        y: "y".to_owned(),
        threshold: 3,
    }];
    let native = NativeChartData::new([sample_data], ChartDataLimits::default()).unwrap();
    let sampled = prepare_chart_data(
        sample_spec,
        &native.snapshot(),
        &ChartTransformRegistry::new(),
        &ChartSeriesRegistry::new(),
        &ChartFormatterRegistry::new(),
    )
    .unwrap();
    let sampled = layout_chart_scene(
        &sampled,
        640.0,
        400.0,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    let y = sampled
        .axis_domains
        .iter()
        .find_map(|(key, domain)| key.contains(":y:").then_some(domain))
        .unwrap();
    assert!(y.full.0 <= -500.0 && y.full.1 >= 1000.0);
}

#[test]
fn geo_viewport_and_complex_series_keep_business_identity() {
    let data = dataset(
        "main",
        json!([{"id":"measurement-1","name":"feature","v":25}]),
    );
    let gauge = scene(
        json!({
            "regions":[{"key":"main","kind":"polar"}],
            "axes":[{"key":"r","position":"radial","min":0,"max":100}],
            "series":[{"key":"g","kind":"gauge","encode":{"name":"name","value":"v"}}]
        }),
        data.clone(),
    );
    let gauge_mark = gauge
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Data)
        .unwrap();
    assert_eq!(gauge_mark.datum.as_ref().unwrap().key, "measurement-1");
    assert_eq!(gauge_mark.datum_key, "measurement-1");

    let geo = ChartGeoRegistry::new();
    geo.register_map(
        ChartGeoMap::from_geojson(
            "map",
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[20,0],[20,20],[0,20],[0,0]]]}}]}"#,
        )
        .unwrap(),
    )
    .unwrap();
    let prepared = prepared(
        json!({
            "regions":[{"key":"main","kind":"geo_2d","map":"map"}],
            "series":[{"key":"m","kind":"map","encode":{"name":"name","value":"v"}}]
        }),
        [data],
    );
    let base = layout_chart_scene(&prepared, 640.0, 400.0, &ChartTheme::default(), &geo).unwrap();
    let zoomed = layout_chart_scene_with_viewport(
        &prepared,
        640.0,
        400.0,
        &ChartTheme::default(),
        &geo,
        ChartViewport {
            zoom: 2.0,
            pan: ChartPoint { x: 30.0, y: 10.0 },
        },
    )
    .unwrap();
    assert_ne!(base.marks, zoomed.marks);
    let map_mark = base
        .marks
        .iter()
        .find(|mark| mark.role == ChartMarkRole::Data)
        .unwrap();
    assert_eq!(map_mark.datum.as_ref().unwrap().key, "measurement-1");
    assert_eq!(map_mark.datum_key, "measurement-1");
}

#[test]
fn custom_series_failures_are_atomic_across_coordinate_systems() {
    struct Reject;
    impl HostChartSeries for Reject {
        fn layout(&self, _: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
            Err("candidate rejected".to_owned())
        }
    }

    let geo = ChartGeoRegistry::new();
    geo.register_map(
        ChartGeoMap::from_geojson(
            "map",
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[20,0],[20,20],[0,20],[0,0]]]}}]}"#,
        )
        .unwrap(),
    )
    .unwrap();
    for coordinate in ["cartesian_2d", "polar", "geo_2d"] {
        let registry = ChartSeriesRegistry::new();
        registry.register("reject", Reject).unwrap();
        let region = if coordinate == "geo_2d" {
            json!({"key":"main","kind":coordinate,"map":"map"})
        } else {
            json!({"key":"main","kind":coordinate})
        };
        let spec = ChartSpec::from_ui_value(&ui(json!({
            "regions":[region],
            "series":[{"key":"custom","kind":"custom","renderer":"reject","encode":{"x":"x","y":"y"}}]
        })))
        .unwrap();
        let native = NativeChartData::new(
            [dataset("main", json!([{"id":"a","x":1,"y":2}]))],
            ChartDataLimits::default(),
        )
        .unwrap();
        let prepared = prepare_chart_data(
            spec,
            &native.snapshot(),
            &ChartTransformRegistry::new(),
            &registry,
            &ChartFormatterRegistry::new(),
        )
        .unwrap();
        assert!(
            layout_chart_scene(&prepared, 640.0, 400.0, &ChartTheme::default(), &geo,).is_err(),
            "{coordinate} swallowed a custom renderer failure"
        );
    }
}
