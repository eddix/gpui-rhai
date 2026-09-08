use gpui_rhai::*;
use std::{collections::BTreeMap, time::Instant};
fn main() {
    for n in [200, 400, 800, 1600] {
        let mut samples = Vec::new();
        for _ in 0..5 {
            let mut store = StateStore::new();
            let start = Instant::now();
            for i in 0..n {
                store
                    .mount_instance(
                        ComponentInstancePath::root("Empty", i.to_string()),
                        &ComponentStateSchema::default(),
                    )
                    .unwrap();
            }
            std::hint::black_box(store);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "release_mount_empty_instances n={n} median_ms={:.3}",
            samples[2]
        );
    }
    let payload = UiValue::Array(
        (0..10_000)
            .map(|i| {
                UiValue::Map(BTreeMap::from([
                    ("id".into(), UiValue::Integer(i)),
                    ("label".into(), UiValue::String("x".repeat(100))),
                ]))
            })
            .collect(),
    );
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "items".into(),
        StateField::new(ValueSchema::UiValue, payload),
    )]))
    .unwrap();
    let mut runtime = UiRuntimeState::new();
    runtime
        .stores
        .declare(StoreId::app("bulk"), schema)
        .unwrap();
    let mut samples = Vec::new();
    for _ in 0..30 {
        let start = Instant::now();
        std::hint::black_box(runtime.snapshot().unwrap());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "release_snapshot_10000_items p50_ms={:.3} p95_ms={:.3}",
        samples[15], samples[28]
    );
}
