use gpui_rhai::*;
use std::{collections::BTreeMap, time::Instant};
fn main() {
    for n in [1000, 10000] {
        let rows = (0..n)
            .map(|i| {
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String(format!("r{i}"))),
                    ("x".to_owned(), UiValue::Integer(i)),
                    ("y".to_owned(), UiValue::Float((i as f64).sin())),
                ])
            })
            .collect::<Vec<_>>();
        let mut samples = vec![];
        for i in 0..25 {
            let start = Instant::now();
            let cloned = rows.clone();
            let d = ChartDataset::from_rows(
                "main",
                &cloned,
                Some("id".into()),
                ChartDataLimits::default(),
            )
            .unwrap();
            std::hint::black_box(NativeChartData::new([d], ChartDataLimits::default()).unwrap());
            if i >= 5 {
                samples.push(start.elapsed().as_secs_f64() * 1000.);
            }
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "inline_config_data rows={n} p50_ms={:.3} p95_ms={:.3}",
            samples[9], samples[18]
        );
    }
}
