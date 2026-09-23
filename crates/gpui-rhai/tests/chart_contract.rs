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
fn gauge_and_radar_share_explicit_value_domains() {
    let gauge = scene(
        json!({"regions":[{"key":"main","kind":"polar"}],"axes":[{"key":"r","position":"radial","min":0,"max":100}],"series":[{"key":"g","kind":"gauge","encode":{"name":"id","value":"v"}}]}),
        dataset("main", json!([{"id":"meter","v":25}])),
    );
    let track = gauge
        .marks
        .iter()
        .find(|mark| mark.key.ends_with("gauge:track"))
        .unwrap();
    let value = gauge
        .marks
        .iter()
        .find(|mark| mark.key.ends_with("gauge:value"))
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
        .find(|mark| mark.key.ends_with("a:radar"))
        .unwrap();
    let b = radar
        .marks
        .iter()
        .find(|mark| mark.key.ends_with("b:radar"))
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
