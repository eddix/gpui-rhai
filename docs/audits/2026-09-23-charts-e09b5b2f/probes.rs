use gpui_rhai::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn ui(v: Value) -> UiValue {
    match v {
        Value::Null => UiValue::Null,
        Value::Bool(v) => UiValue::Bool(v),
        Value::Number(v) => v
            .as_i64()
            .map_or_else(|| UiValue::Float(v.as_f64().unwrap()), UiValue::Integer),
        Value::String(v) => UiValue::String(v),
        Value::Array(a) => UiValue::Array(a.into_iter().map(ui).collect()),
        Value::Object(m) => UiValue::Map(m.into_iter().map(|(k, v)| (k, ui(v))).collect()),
    }
}
fn dataset(rows: Value) -> ChartDataset {
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .map(|r| match ui(r) {
            UiValue::Map(m) => m,
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    ChartDataset::from_rows("main", &rows, Some("id".into()), ChartDataLimits::default()).unwrap()
}
fn prep(v: Value, d: &ChartDataset) -> Result<ChartPreparedData, ChartPrepareError> {
    let s = ChartSpec::from_ui_value(&ui(v)).unwrap();
    let n = NativeChartData::new([d.clone()], ChartDataLimits::default()).unwrap();
    prepare_chart_data(
        s,
        &n.snapshot(),
        &ChartTransformRegistry::new(),
        &ChartSeriesRegistry::new(),
        &ChartFormatterRegistry::new(),
    )
}
fn scene(v: Value, d: &ChartDataset) -> Result<PreparedChartScene, ChartPrepareError> {
    layout_chart_scene(
        &prep(v, d)?,
        640.,
        400.,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
}
fn series(kind: &str, x: &str, y: &str) -> Value {
    json!({"key":"s","kind":kind,"encode":{"x":x,"y":y}})
}
fn main() {
    let d = dataset(json!([{"id":"a","x":"A","a":10,"b":10},{"id":"b","x":"B","a":5,"b":-4}]));
    let s=scene(json!({"legend":{"visible":false},"series":[{"key":"a","kind":"bar","stack":"g","encode":{"x":"x","y":"a"}},{"key":"b","kind":"bar","stack":"g","encode":{"x":"x","y":"b"}}]}),&d).unwrap();
    let region = s.plot_regions["main"];
    println!(
        "STACK plot={region:?} bars={:?}",
        s.marks
            .iter()
            .filter(|m| m.interactive)
            .map(|m| (&m.key, &m.geometry))
            .collect::<Vec<_>>()
    );
    let d = dataset(json!([{"id":"a","x":0,"y":1},{"id":"b","x":1,"y":10}]));
    println!("AUTO_LOG_Y result={:?}",scene(json!({"series":[series("line","x","y")],"axes":[{"key":"logy","position":"left","scale":"log"}]}),&d).map(|s|s.marks.len()));
    println!("NUMERIC_CATEGORY interactive={:?}",scene(json!({"series":[series("bar","x","y")],"axes":[{"key":"catx","position":"bottom","scale":"category"}]}),&d).map(|s|s.marks.iter().filter(|m|m.interactive&&m.datum_key!="legend").count()));
    let d =
        dataset(json!([{"id":"a","x":0,"y":1},{"id":"gap","x":1,"y":null},{"id":"c","x":2,"y":3}]));
    let s = scene(
        json!({"legend":{"visible":false},"series":[series("line","x","y")]}),
        &d,
    )
    .unwrap();
    println!(
        "NULL_GAP line_geometry={:?}",
        s.marks
            .iter()
            .find(|m| m.key == "s:line")
            .map(|m| &m.geometry)
    );
    let rows = (0..5000)
        .map(|i| json!({"id":format!("r{i}"),"x":i,"y":if i==2400{Value::Null}else{json!(i%9)}}))
        .collect::<Vec<_>>();
    println!(
        "LARGE_NULL_LINE prepare={:?}",
        prep(
            json!({"series":[series("line","x","y")]}),
            &dataset(Value::Array(rows))
        )
        .map(|_| "ok")
    );
    let d = dataset(json!([{"id":"a","x":"A","a":10,"b":5}]));
    let s=scene(json!({"series":[{"key":"a","kind":"bar","visible":false,"encode":{"x":"x","y":"a"}},{"key":"b","kind":"bar","encode":{"x":"x","y":"b"}}]}),&d).unwrap();
    println!(
        "HIDDEN_PALETTE b_marks={:?}",
        s.marks
            .iter()
            .filter(|m| m.series_key == "b")
            .map(|m| (&m.datum_key, m.fill))
            .collect::<Vec<_>>()
    );
    let d = dataset(json!([{"id":"a","x":0,"y":1},{"id":"b","x":1,"y":3}]));
    let req = ChartExportRequest::terminal(640, 400, "en");
    let a = prep(
        json!({"title":"FIRST","series":[series("line","x","y")]}),
        &d,
    )
    .unwrap();
    let b = prep(
        json!({"title":"SECOND TITLE","series":[series("line","x","y")]}),
        &d,
    )
    .unwrap();
    let theme = ChartTheme::default();
    let geo = ChartGeoRegistry::new();
    let pa = export_chart_png(&a, &req, &theme, &geo).unwrap();
    let pb = export_chart_png(&b, &req, &theme, &geo).unwrap();
    println!(
        "PNG_LABELS png_equal_despite_different_title={} svg_equal={}",
        pa == pb,
        export_chart_svg(&a, &req, &theme, &geo).unwrap()
            == export_chart_svg(&b, &req, &theme, &geo).unwrap()
    );
    let out = std::env::var_os("GPUI_RHAI_CHART_AUDIT_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("gpui-rhai-chart-audit-e09b5b2f"));
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("export.png"), pa).unwrap();
    std::fs::write(
        out.join("export.svg"),
        export_chart_svg(&a, &req, &theme, &geo).unwrap(),
    )
    .unwrap();
    println!("EXPORT_OUTPUT={}", out.display());
    let spec = json!({"regions":[{"key":"left","kind":"cartesian_2d","column":0},{"key":"right","kind":"cartesian_2d","column":1}],"series":[{"key":"a","kind":"line","coordinate":"left","encode":{"x":"x","y":"y"}},{"key":"b","kind":"line","coordinate":"right","encode":{"x":"x","y":"y"}}]});
    let s = scene(spec, &d).unwrap();
    let mut seen = BTreeSet::new();
    let dup = s
        .marks
        .iter()
        .filter_map(|m| (!seen.insert(&m.key)).then_some(&m.key))
        .collect::<Vec<_>>();
    println!("MULTI_REGION duplicate_mark_keys={dup:?}");
    let d =
        dataset(json!([{"id":"a","g":1,"y":1},{"id":"b","g":1,"y":null},{"id":"c","g":2,"y":3}]));
    let t = ChartTransformSpec::Aggregate {
        group_by: vec!["g".into()],
        dimension: "y".into(),
        operation: "sum".into(),
        output: "total".into(),
    };
    let out = apply_chart_transforms(
        &d,
        &[t],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    )
    .unwrap();
    println!(
        "AGGREGATE group_column_type={:?} rows={:?}",
        out.column("g").unwrap().data_type(),
        out.rows()
    );
    struct AddOne(i64);
    impl HostChartTransform for AddOne {
        fn transform(
            &self,
            input: &ChartDataset,
            _: &UiValue,
            _: ChartTransformContext,
        ) -> Result<ChartDataset, String> {
            let mut rows = input.rows();
            for r in &mut rows {
                r.insert("newval".into(), ChartValue::Integer(self.0));
            }
            ChartDataset::from_chart_rows(
                input.name(),
                &rows,
                input.key_dimension().map(str::to_owned),
                ChartDataLimits::default(),
            )
            .map_err(|e| e.to_string())
        }
    }
    let r = ChartTransformRegistry::new();
    r.register("test", AddOne(1)).unwrap();
    let duplicate = r.register("test", AddOne(2));
    let out = apply_chart_transforms(
        &d,
        &[ChartTransformSpec::Host {
            id: "test".into(),
            options: UiValue::Null,
        }],
        &r,
        ChartTransformContext::default(),
    )
    .unwrap();
    println!(
        "DUPLICATE_REGISTRATION err={} subsequent_result={:?}",
        duplicate.is_err(),
        out.column("newval").unwrap().value(0)
    );
    let d = dataset(json!([{"id":"meter","v":25}]));
    let s=scene(json!({"regions":[{"key":"main","kind":"polar"}],"axes":[{"key":"r","position":"radial","min":0,"max":100}],"series":[{"key":"g","kind":"gauge","encode":{"value":"v","name":"id"}}]}),&d).unwrap();
    let track = &s
        .marks
        .iter()
        .find(|m| m.key == "g:gauge:track")
        .unwrap()
        .geometry;
    let value = &s
        .marks
        .iter()
        .find(|m| m.key == "g:gauge:value")
        .unwrap()
        .geometry;
    println!("GAUGE25_OVER100 value_equals_full_track={}", track == value);
    let d = dataset(
        json!([{"id":"a","n":"A","a":10,"b":100},{"id":"b","n":"B","a":20,"b":200},{"id":"c","n":"C","a":30,"b":300}]),
    );
    let s=scene(json!({"regions":[{"key":"main","kind":"polar"}],"series":[{"key":"a","kind":"radar","encode":{"name":"n","value":"a"}},{"key":"b","kind":"radar","encode":{"name":"n","value":"b"}}]}),&d).unwrap();
    println!(
        "RADAR_TENFOLD identical_polygons={}",
        s.marks
            .iter()
            .find(|m| m.key == "a:radar")
            .unwrap()
            .geometry
            == s.marks
                .iter()
                .find(|m| m.key == "b:radar")
                .unwrap()
                .geometry
    );
    let a = dataset(json!([{"id":"a0","x":0,"y":1},{"id":"a1","x":1,"y":2}]));
    let b = dataset(json!([{"id":"b0","x":0,"y":10},{"id":"b1","x":100,"y":20}]));
    let b = ChartDataset::from_chart_rows(
        "other",
        &b.rows(),
        Some("id".into()),
        ChartDataLimits::default(),
    )
    .unwrap();
    let n = NativeChartData::new([a, b], ChartDataLimits::default()).unwrap();
    let spec=ChartSpec::from_ui_value(&ui(json!({"axes":[{"key":"x","position":"bottom"},{"key":"ya","position":"left"},{"key":"yb","position":"right"}],"series":[{"key":"a","kind":"scatter","x_axis":"x","y_axis":"ya","encode":{"x":"x","y":"y"}},{"key":"b","dataset":"other","kind":"scatter","x_axis":"x","y_axis":"yb","encode":{"x":"x","y":"y"}}]}))).unwrap();
    let p = prepare_chart_data(
        spec,
        &n.snapshot(),
        &ChartTransformRegistry::new(),
        &ChartSeriesRegistry::new(),
        &ChartFormatterRegistry::new(),
    )
    .unwrap();
    let s = layout_chart_scene(
        &p,
        640.,
        400.,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    println!(
        "SHARED_X_DIFFERENT_Y x1={:?} x100={:?}",
        s.marks.iter().find(|m| m.key == "a:a1").unwrap().geometry,
        s.marks.iter().find(|m| m.key == "b:b1").unwrap().geometry
    );
    let d = dataset(json!([{"id":"a","x":"Alpha","y":1},{"id":"b","x":"Beta","y":2}]));
    let p=prep(json!({"axes":[{"key":"category","position":"bottom","scale":"category"}],"series":[series("bar","x","y")]}),&d).unwrap();
    let mut t = ChartTheme::default();
    t.number = Some(NumberMetadata {
        digits: (0..10).map(|v| v.to_string()).collect(),
        decimal_separator: ".".into(),
        grouping_separator: ",".into(),
        primary_group_size: 3,
        secondary_group_size: 3,
        minus_sign: "-".into(),
    });
    let s = layout_chart_scene(&p, 640., 400., &t, &ChartGeoRegistry::new()).unwrap();
    println!(
        "CATEGORY_LOCALE_LABELS {:?}",
        s.labels
            .iter()
            .filter(|l| l.key.starts_with("axis:category"))
            .map(|l| &l.text)
            .collect::<Vec<_>>()
    );
    let invalid = apply_chart_transforms(
        &d,
        &[ChartTransformSpec::Filter {
            dimension: "y".into(),
            operator: "typo".into(),
            value: UiValue::Integer(1),
        }],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    );
    println!("FILTER_TYPO {:?}", invalid.map(|d| d.len()));
    let sampled_rows = (0..5000)
        .map(|i| json!({"id":format!("r{i}"),"x":i,"y":(i%13)}))
        .collect::<Vec<_>>();
    let s = scene(
        json!({"legend":{"visible":false},"series":[series("line","x","y")]}),
        &dataset(Value::Array(sampled_rows)),
    )
    .unwrap();
    let keys = s
        .marks
        .iter()
        .filter(|m| m.interactive)
        .map(|m| m.datum_key.as_str())
        .collect::<BTreeSet<_>>();
    let missing = (0..5000)
        .map(|i| format!("r{i}"))
        .find(|id| !keys.contains(id.as_str()))
        .unwrap();
    println!(
        "SAMPLED_IDENTITY source_rows=5000 interactive={} unreachable_example={missing}",
        keys.len()
    );

    let d = dataset(json!([{"id":"a","x":"A","y":1},{"id":"b","x":"B","y":2}]));
    let empty = apply_chart_transforms(
        &d,
        &[ChartTransformSpec::Filter {
            dimension: "y".into(),
            operator: "gt".into(),
            value: UiValue::Integer(99),
        }],
        &ChartTransformRegistry::new(),
        ChartTransformContext::default(),
    );
    println!(
        "VALID_FILTER_NO_MATCHES={:?}",
        empty.map(|d| (d.len(), d.columns().count()))
    );
    println!("LOG_BAR_EXPLICIT_MIN marks={:?}",scene(json!({"legend":{"visible":false},"axes":[{"key":"ly","position":"left","scale":"log","min":1,"max":100}],"series":[series("bar","x","y")]}),&d).map(|s|s.marks.iter().filter(|m|m.interactive).count()));
    struct Custom(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl HostChartSeries for Custom {
        fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(vec![ChartMark {
                key: "custom-mark".into(),
                series_key: c.spec.key.clone(),
                datum_key: "a".into(),
                geometry: ChartMarkGeometry::Circle {
                    center: c.bounds.center(),
                    radius: 10.,
                },
                fill: Some(c.theme.palette[0]),
                stroke: None,
                label: "Custom".into(),
                value: None,
                interactive: true,
                selected: false,
            }])
        }
    }
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reg = ChartSeriesRegistry::new();
    reg.register("custom", Custom(calls.clone())).unwrap();
    let n = NativeChartData::new([d], ChartDataLimits::default()).unwrap();
    let spec=ChartSpec::from_ui_value(&ui(json!({"regions":[{"key":"main","kind":"polar"}],"series":[{"key":"custom","kind":"custom","renderer":"custom","encode":{}}]}))).unwrap();
    let p = prepare_chart_data(
        spec,
        &n.snapshot(),
        &ChartTransformRegistry::new(),
        &reg,
        &ChartFormatterRegistry::new(),
    )
    .unwrap();
    let s = layout_chart_scene(
        &p,
        640.,
        400.,
        &ChartTheme::default(),
        &ChartGeoRegistry::new(),
    )
    .unwrap();
    println!(
        "CUSTOM_POLAR renderer_calls={} returned_data_marks={} diagnostics={:?}",
        calls.load(std::sync::atomic::Ordering::Relaxed),
        s.marks
            .iter()
            .filter(|m| m.datum_key != "legend" && m.interactive)
            .count(),
        s.diagnostics
    );

    let d = dataset(json!([{"id":"polar","lon":0,"lat":89,"v":1}]));
    let p=prep(json!({"legend":{"visible":false},"regions":[{"key":"main","kind":"geo_2d","map":"world","projection":"mercator"}],"series":[{"key":"g","kind":"geo_scatter","encode":{"longitude":"lon","latitude":"lat","value":"v"}}]}),&d).unwrap();
    let g = ChartGeoRegistry::new();
    g.register_map(ChartGeoMap::from_geojson("world",r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"world","properties":{},"geometry":{"type":"Polygon","coordinates":[[[-170,-80],[170,-80],[170,80],[-170,80],[-170,-80]]]}}]}"#).unwrap()).unwrap();
    let s = layout_chart_scene(&p, 640., 400., &ChartTheme::default(), &g).unwrap();
    println!(
        "GEO_REJECTED_PROJECTION result={:?} emitted={:?} diagnostics={:?}",
        MercatorProjection.project(0., 89.),
        s.marks
            .iter()
            .filter(|m| m.interactive)
            .map(|m| &m.geometry)
            .collect::<Vec<_>>(),
        s.diagnostics
    );

    let d = dataset(json!([{"id":"a","g":1,"v":2},{"id":"b","g":1,"v":3}]));
    let n = NativeChartData::new([d], ChartDataLimits::default()).unwrap();
    let mut spec = ChartSpec::from_ui_value(&ui(
        json!({"series":[{"key":"s","kind":"bar","encode":{"x":"g","y":"total"}}]}),
    ))
    .unwrap();
    spec.series[0].transforms = vec![ChartTransformSpec::Aggregate {
        group_by: vec!["g".into()],
        dimension: "v".into(),
        operation: "sum".into(),
        output: "total".into(),
    }];
    println!(
        "ENCODE_TRANSFORM_OUTPUT={:?}",
        prepare_chart_data(
            spec,
            &n.snapshot(),
            &ChartTransformRegistry::new(),
            &ChartSeriesRegistry::new(),
            &ChartFormatterRegistry::new()
        )
        .map(|_| "ok")
    );
}
