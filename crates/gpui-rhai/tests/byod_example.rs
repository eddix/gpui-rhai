//! The bring-your-own-design reference example prepares without the token
//! base and renders its own vocabulary: the runtime imposes no design.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::*;

const MAIN: &str = include_str!("../../../examples/byod_treemap/ui/main.rhai");
const THEME: &str = include_str!("../../../examples/byod_treemap/ui/theme.rhai");

#[test]
fn byod_treemap_prepares_without_the_token_base() {
    let entry = ModuleId::parse("main").unwrap();
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    EmbeddedScriptView::new(entry, scripts, THEME)
        .prepare()
        .expect("a theme with its own vocabulary needs no token base");
}

#[test]
fn byod_treemap_lays_out_every_item_inside_its_area() {
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile_named("main.rhai", MAIN).unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("the treemap renders a column");
    };
    let UiNodeKind::Box { children: tiles } = children[1].kind() else {
        panic!("the second child holds the tiles");
    };
    assert_eq!(tiles.len(), 9);
    let mut area = 0.0;
    for tile in tiles {
        let base = &tile.style().base;
        let pixels = |length: Option<LayoutLength>| match length {
            Some(LayoutLength::Definite(Length::Pixels(value))) => value,
            other => panic!("tile geometry must be literal pixels, got {other:?}"),
        };
        let (x, y) = (pixels(base.left), pixels(base.top));
        let (width, height) = (pixels(base.width), pixels(base.height));
        assert!(x >= -0.01 && y >= -0.01 && x + width <= 760.01 && y + height <= 440.01);
        area += width * height;
        // Colors come from the app's own vocabulary, never the design language.
        assert!(
            matches!(&base.background, Some(ColorValue::Token(token)) if token.starts_with("tile."))
        );
    }
    assert!(
        (area - 760.0 * 440.0).abs() < 1.0,
        "tiles cover the area: {area}"
    );
}
