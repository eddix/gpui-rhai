#![allow(dead_code)]
use gpui_rhai::{ComponentInstancePath, UiValue};
#[path = "../../../../../../../crates/gpui-rhai/src/native_collection.rs"]
mod native_collection;
#[path = "../../../../../../../crates/gpui-rhai/src/collection_projection.rs"]
mod collection_projection;
use std::{collections::BTreeMap, hint::black_box, time::Instant};

fn main() {
    // Product modules are included unchanged so private key lookup is exactly
    // the one used by VirtualListEntityElement::prepaint. Only the surrounding
    // frame/render work is omitted. This measures key scan, not total frame time.
    for count in [1_000, 10_000, 100_000, 1_000_000] {
        let data = native_collection::VirtualCollectionData::Native(native_collection::NativeCollection::new("key", (0..count).map(|i| BTreeMap::from([("key".to_owned(), UiValue::String(format!("k{i:09}")))]))).unwrap());
        let mut timings = Vec::new();
        let source = "absent0000";
        for step in 0..60 {
            let mut lookups = 0;
            let start = Instant::now();
            let hit = (0..black_box(data.len())).find(|index| {
                lookups += 1;
                data.key(*index) == Some(black_box(source))
            });
            black_box(hit);
            assert_eq!(lookups, count);
            if step >= 10 { timings.push(start.elapsed()); }
        }
        timings.sort();
        println!("absent drag source: rows={count}, key_calls_per_frame={count}, median={:?}, p95={:?}", timings[25], timings[47]);
    }
}
