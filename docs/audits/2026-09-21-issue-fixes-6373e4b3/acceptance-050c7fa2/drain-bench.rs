use gpui_rhai::*;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
fn main() {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(r#"fn done(ctx,p){} fn view(){text("x").on_click(Fn("done"))}"#)
        .unwrap();
    let node = engine.render(&compiled).unwrap();
    let cb = node.handler("click").unwrap().as_script().unwrap().clone();
    let g = compiled.generation();
    for side in [256, 1024, 2048] {
        let mut samples = vec![];
        let mut starts = vec![];
        for _ in 0..3 {
            let svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}"><defs><linearGradient id="g"><stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/></svg>"##
            );
            let r = AssetRegistry::new();
            r.register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "sample".into(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: svg.into_bytes(),
                    },
                )])),
            )
            .unwrap();
            let start = Instant::now();
            r.start_image_decode(
                &AssetId::parse("app/sample").unwrap(),
                AsyncScope::App,
                g,
                cb.clone(),
                cb.clone(),
            )
            .unwrap();
            starts.push(start.elapsed().as_secs_f64() * 1000.);
            let mut completed = false;
            for _ in 0..1000 {
                let before = Instant::now();
                let events = r.drain_image_decodes(g).unwrap();
                let ms = before.elapsed().as_secs_f64() * 1000.;
                if !events.is_empty() {
                    assert!(
                        matches!(events[0].payload, UiValue::Handle(_)),
                        "{:?}",
                        events[0].payload
                    );
                    samples.push(ms);
                    completed = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(completed);
        }
        samples.sort_by(f64::total_cmp);
        starts.sort_by(f64::total_cmp);
        println!(
            "side={side} start_ms_p50={:.3} foreground_drain_ms_p50={:.3} drain_samples_ms={samples:?}",
            starts[1], samples[1]
        );
    }
}
