//! Runtime composition coverage of the canonical Gallery, not an import list.
//!
//! This gate proves a formal component reached the committed composition.
//! Opening overlays, navigating virtual content, actual painted states and
//! source fidelity remain separate native interaction/visual gates.

use std::collections::{BTreeMap, BTreeSet};

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AutomationCommand, MotionPreference, PrimitiveValue, UiNode, UiNodeKind, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use crate::support::{Host, Mount, Step, last_error, settle};

fn component_exports(node: &UiNode, reached: &mut BTreeSet<String>) {
    if let Some(owner) = node.component_root() {
        // ComponentInstancePath already has a public lossless serialization.
        // Do not split Display paths: application keys can contain delimiters.
        let segments = serde_json::to_value(owner).unwrap();
        for segment in segments.as_array().unwrap() {
            reached.insert(segment["component"].as_str().unwrap().to_owned());
        }
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                component_exports(child, reached);
            }
        }
        UiNodeKind::Custom { primitive } => {
            for (_, value) in primitive.props.iter() {
                match value {
                    PrimitiveValue::Node(child) => component_exports(child, reached),
                    PrimitiveValue::Nodes(children) => {
                        for child in children {
                            component_exports(child, reached);
                        }
                    }
                    _ => {}
                }
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            component_exports(trigger, reached);
            component_exports(content, reached);
        }
        UiNodeKind::Layer { content, .. } => component_exports(content, reached),
        UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                component_exports(child, reached);
            }
        }
        UiNodeKind::ErrorBoundary { child, fallback } => {
            component_exports(child, reached);
            component_exports(fallback, reached);
        }
        _ => {}
    }
}

fn expected_modules() -> BTreeMap<String, String> {
    let mut expected = BTreeMap::new();
    for (module, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_MOTION_SOURCES_BY_ID)
        .chain(gpui_rhai_registry::BUNDLED_CHART_SOURCES_BY_ID)
    {
        let metadata = gpui_rhai::parse_component_header(source).unwrap();
        assert_eq!(metadata.id.as_str(), *module);
        assert!(
            expected
                .insert(metadata.export, (*module).to_owned())
                .is_none(),
            "ambiguous export: module coverage must not confuse two modules"
        );
    }
    expected
}

#[gpui::test]
fn every_public_module_reaches_the_canonical_gallery_composition(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch::default())
        .unwrap()
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("module-coverage", "module-coverage"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(900.)));
    let expected = expected_modules();
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Tick, 3);
        let mut covered = BTreeMap::<String, BTreeSet<String>>::new();
        for page in acceptance::page_ids() {
            visual
                .update(|window, cx| {
                    view.automate(
                        AutomationCommand::Action {
                            id: "gallery.go".to_owned(),
                            payload: Some(UiValue::String(page.clone())),
                        },
                        window,
                        cx,
                    )
                })
                .unwrap();
            settle(&mut visual, Step::Tick, 3);
            assert_eq!(last_error(&mut visual, &view), None, "{page}");
            let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
            let mut exports = BTreeSet::new();
            component_exports(&root, &mut exports);
            for export in exports {
                if let Some(module) = expected.get(&export) {
                    covered
                        .entry(module.clone())
                        .or_default()
                        .insert(page.clone());
                }
            }
        }
        let missing = expected
            .values()
            .filter(|id| !covered.contains_key(*id))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "{family}/{appearance}: {} of {} modules never reached the canonical Gallery: {missing:?}\nCovered: {covered:#?}",
            missing.len(),
            expected.len()
        );
    }
}

#[gpui::test]
fn importing_or_naming_a_component_is_not_runtime_composition_coverage(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(
        r#"
        import "components/button" as button;
        fn view(ctx) {text("Button")}
    "#,
    )
    .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut reached = BTreeSet::new();
    component_exports(&root, &mut reached);
    assert!(!reached.contains("Button"));

    let (mut positive, called) = Mount::components(
        r#"
        import "components/button" as button;
        fn view(ctx) {button::Button(#{text:"Button"})}
    "#,
    )
    .id("coverage-positive")
    .mount(cx);
    settle(&mut positive, Step::Frame, 2);
    let root = positive.update(|_, cx| called.root(cx).unwrap().unwrap());
    let mut reached = BTreeSet::new();
    component_exports(&root, &mut reached);
    assert!(
        reached.contains("Button"),
        "an actual formal invocation must count"
    );
}
