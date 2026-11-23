//! Mount the actual Rust extension example, including its task and subscription.
//! The native primitive supplies content; the official Card owns preset chrome.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    AccessibilityNode, Environment, Length, MotionPreference, ScriptViewConfig, ScriptViewHandle,
    ScriptViewHost, ScriptViewState, ThemeVariant, UiValue,
};

use crate::support::{
    ActiveHost, Step, filled_including_rounded_clips, last_error, node_by_test_id, settle,
    snapshot, text_of,
};

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/extension_host.rs"]
mod extension_host;

fn mounted(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    // This real example uses OS threads and channels, not deterministic fakes.
    // Permit their scheduler wakes only in this mixed-I/O fixture.
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let prepared = extension_host::view()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("extension-example", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("example"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        ActiveHost { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(600.0), px(320.0)));
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Tick, 3);
    (visual, view)
}

fn message(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> AccessibilityNode {
    snapshot(visual, view)
        .find_by_role_and_name("status", "Host extension message")
        .next()
        .expect("the actual native message is mounted")
        .clone()
}

fn pixels(theme: &ThemeVariant, value: Length) -> f64 {
    let Length::Pixels(value) = theme
        .tokens
        .resolve_length(value, &Environment::EMPTY)
        .unwrap()
    else {
        panic!("resolved pixel length")
    };
    value
}

#[gpui::test]
fn real_extension_keeps_async_state_and_card_design_through_all_presets(cx: &mut TestAppContext) {
    let (mut visual, view) = mounted(cx);
    // Advance controller/throttle time and allow real worker threads to deliver.
    // A prepared-only or copied script fixture would miss this path.
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        settle(&mut visual, Step::Tick, 1);
        assert_eq!(last_error(&mut visual, &view), None);
        if message(&mut visual, &view).value == Some(UiValue::String("BACKGROUND READY".into()))
            && text_of(&mut visual, &view, "extension-tick") == "Subscription tick: 3"
        {
            break;
        }
        assert!(Instant::now() < deadline, "real host work did not finish");
        std::thread::yield_now();
    }
    let message_id = message(&mut visual, &view).id;
    let ticker_id = node_by_test_id(&mut visual, &view, "extension-tick").id;

    for (family, variant) in [
        ("Modern", "Light"),
        ("Clean", "Dark"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, variant, cx))
            .unwrap();
        for width in [600.0, 360.0] {
            visual.simulate_resize(size(px(width), px(320.0)));
            settle(&mut visual, Step::Tick, 3);
            assert_eq!(view.state(), ScriptViewState::Active);
            assert_eq!(last_error(&mut visual, &view), None);
            let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
            assert_eq!(
                (theme.family.as_str(), theme.name.as_str()),
                (family, variant)
            );
            let status = message(&mut visual, &view);
            assert_eq!(
                status.id, message_id,
                "a design switch must not remount content"
            );
            assert_eq!(
                status.value,
                Some(UiValue::String("BACKGROUND READY".into()))
            );
            assert!(status.geometry.unwrap().visual.height > 0.0);
            let ticker = node_by_test_id(&mut visual, &view, "extension-tick");
            assert_eq!(ticker.id, ticker_id);
            assert_eq!(
                ticker.name, "Subscription tick: 3",
                "switching must not restart effects"
            );

            let card = node_by_test_id(&mut visual, &view, "extension-status-card")
                .geometry
                .unwrap()
                .visual;
            let inset = pixels(&theme, Length::theme_spacing("lg").unwrap());
            assert!((card.x - inset).abs() < 0.1);
            assert!((card.width - (f64::from(width) - 2.0 * inset)).abs() < 0.1);
            let radius = pixels(&theme, Length::token("metrics.card_outer_radius").unwrap());
            assert!(
                filled_including_rounded_clips(
                    &mut visual,
                    theme.tokens.color("card.tray").unwrap().as_rgba_hex(),
                )
                .iter()
                .any(|quad| {
                    (f64::from(quad.bounds.x) - card.x).abs() < 0.1
                        && (f64::from(quad.bounds.y) - card.y).abs() < 0.1
                        && (f64::from(quad.bounds.width) - card.width).abs() < 0.1
                        && quad
                            .corner_radii
                            .iter()
                            .all(|r| (f64::from(*r) - radius).abs() < 0.1)
                }),
                "{family}/{variant} at {width}px must paint the official Card recipe"
            );
        }
    }
    visual.update(|_, cx| view.dispose(cx)).unwrap();
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(view.state(), ScriptViewState::Disposed);
}
