use gpui_rhai::*;
use std::time::{Duration, Instant};
fn main() {
    let now = Instant::now();
    for age in [1, 8] {
        for count in [100, 1000] {
            for inertia in [false, true] {
                let mut r = MotionRuntime::new(MotionPreference::Normal);
                for i in 0..count {
                    let source = if inertia {
                        MotionSource::Inertia(MotionInertia {
                            property: MotionProperty::TranslateX,
                            from: 0.,
                            velocity: 1000.,
                            friction: 0.5,
                            min: None,
                            max: None,
                            bounce: 0.,
                            snap_points: vec![],
                            intent: MotionIntent::Decorative,
                        })
                    } else {
                        MotionSource::Transition(MotionTransition::new(
                            MotionProperty::TranslateX,
                            0.,
                            1000.,
                            10000,
                        ))
                    };
                    r.start(
                        ComponentInstancePath::root("UiNode", format!("root/{i}")),
                        source,
                        now,
                    )
                    .unwrap();
                }
                let time = now + Duration::from_secs(age);
                for _ in 0..10 {
                    std::hint::black_box(r.tick(time));
                    std::hint::black_box(r.snapshot(time));
                }
                let mut samples = vec![];
                for _ in 0..100 {
                    let started = Instant::now();
                    std::hint::black_box(r.tick(time));
                    std::hint::black_box(r.snapshot(time));
                    samples.push(started.elapsed().as_secs_f64() * 1000.);
                }
                samples.sort_by(f64::total_cmp);
                println!(
                    "age_s={age} count={count} inertia={inertia} p50_ms={:.4} p95_ms={:.4}",
                    samples[50], samples[95]
                );
            }
        }
    }
}
