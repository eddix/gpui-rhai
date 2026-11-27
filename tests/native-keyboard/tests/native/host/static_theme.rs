//! Host-owned theme data uses the same renderer and primitive snapshots, without
//! a running script engine, theme manager or replacement native input Entity.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext, WeakEntity};
use gpui_rhai::{
    ColorValue, EventPropagation, HostCallback, Length, PrimitiveNode, PrimitiveProps,
    PrimitiveRegistry, PrimitiveValue, RuntimeEngine, StaticUiView, Style,
    TextInputPrimitiveHandler, ThemeTokenOverrides, ThemeVariant, UiNode, UiValue, init_text_input,
    load_theme_with_layers, load_token_base, text_input_primitive_descriptor,
};

use crate::support::filled;

fn designs() -> Vec<ThemeVariant> {
    let runtime = RuntimeEngine::new();
    let base = load_token_base(
        runtime.engine(),
        "tokens",
        gpui_rhai_registry::TOKEN_BASE_SOURCE,
    )
    .unwrap();
    gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .map(|&(name, source)| {
            load_theme_with_layers(
                runtime.engine(),
                Some(&base),
                name,
                source,
                &ThemeTokenOverrides::default(),
            )
            .unwrap()
        })
        .collect()
}

fn themed_root() -> UiNode {
    UiNode::column(vec![
        UiNode::text("Host typography")
            .with_key("label")
            .with_style(
                &Style::new()
                    .typography("body")
                    .unwrap()
                    .background(ColorValue::token("surface_hover").unwrap()),
            ),
    ])
    .with_style(
        &Style::new()
            .width(Length::Pixels(300.0))
            .height(Length::Pixels(200.0))
            .padding(Length::theme_spacing("sm").unwrap())
            .background(ColorValue::token("surface").unwrap())
            .text_color(ColorValue::token("text_primary").unwrap()),
    )
}

#[gpui::test]
fn static_view_renders_all_four_designs_and_rejects_invalid_replacements(cx: &mut TestAppContext) {
    let themes = designs();
    assert_eq!(themes.len(), 4);
    let root = themed_root();
    let initial = themes[0].clone();
    let window = cx.add_window(|_, _| {
        StaticUiView::new(root)
            .unwrap()
            .with_theme(initial)
            .unwrap()
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let node_ids = window
        .read_with(&visual, |view, _| {
            view.retained()
                .nodes()
                .map(|node| node.id())
                .collect::<Vec<_>>()
        })
        .unwrap();
    for theme in themes {
        window
            .update(&mut visual, |view, _, cx| {
                view.set_theme(Some(theme.clone()), cx)
            })
            .unwrap()
            .unwrap();
        visual.run_until_parked();
        visual.refresh().unwrap();
        visual.run_until_parked();
        assert!(
            !filled(&mut visual, theme.tokens.colors["surface"].as_rgba_hex()).is_empty(),
            "{} / {} must paint its own background",
            theme.family,
            theme.name
        );
        let label = filled(
            &mut visual,
            theme.tokens.colors["surface_hover"].as_rgba_hex(),
        )
        .pop()
        .expect("the text's native background is painted")
        .bounds;
        let Length::Pixels(inset) = theme
            .tokens
            .resolve_length(
                Length::theme_spacing("sm").unwrap(),
                &gpui_rhai::Environment::EMPTY,
            )
            .unwrap()
        else {
            panic!("pixel inset")
        };
        assert!((f64::from(label.x) - inset).abs() < 0.1);
        let Length::Pixels(line) = theme.typography("body").unwrap().line_height else {
            panic!("pixel typography")
        };
        assert!((f64::from(label.height) - line).abs() < 0.1);
        window
            .update(&mut visual, |view, _, cx| {
                assert_eq!(
                    view.retained()
                        .nodes()
                        .map(|node| node.id())
                        .collect::<Vec<_>>(),
                    node_ids
                );
                let accepted = view.root().unwrap().clone();
                assert!(!view.set_theme(Some(theme.clone()), cx).unwrap());
                let mut invalid = theme.clone();
                invalid.family.clear();
                assert!(view.set_theme(Some(invalid), cx).is_err());
                assert_eq!(view.root(), Some(&accepted));
                assert!(!view.set_theme(Some(theme.clone()), cx).unwrap());
            })
            .unwrap();
    }
}

fn input_root(
    value: &str,
    owner: &WeakEntity<StaticUiView>,
    changes: &Rc<RefCell<Vec<String>>>,
) -> UiNode {
    let weak = owner.clone();
    let changes = Rc::clone(changes);
    let callback = HostCallback::new("host.changed", move |payload, _, app| {
        if let UiValue::String(value) = payload {
            changes.borrow_mut().push(value.clone());
            weak.update(app, |view, cx| {
                view.set_root(input_root(&value, &weak, &changes), cx)
                    .unwrap();
            })
            .unwrap();
        }
        EventPropagation::Handled
    });
    UiNode::custom(PrimitiveNode {
        primitive: text_input_primitive_descriptor().id,
        key: Some("field".into()),
        props: PrimitiveProps::new()
            .with("value", PrimitiveValue::Data(UiValue::String(value.into())))
            .with(
                "typography",
                PrimitiveValue::Data(UiValue::String("control".into())),
            )
            .with("autofocus", PrimitiveValue::Data(UiValue::Bool(true)))
            .with("on_change", PrimitiveValue::Callback(callback.into())),
    })
    .with_style(
        &Style::new()
            .width(Length::Pixels(240.0))
            .min_height(Length::token("metrics.field_h").unwrap())
            .background(ColorValue::token("surface.field").unwrap())
            .text_color(ColorValue::token("text_primary").unwrap()),
    )
}

#[gpui::test]
fn static_theme_changes_keep_native_input_focus_selection_and_undo(cx: &mut TestAppContext) {
    cx.update(init_text_input);
    let themes = designs();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let registry = PrimitiveRegistry::new();
    registry
        .register(
            text_input_primitive_descriptor(),
            TextInputPrimitiveHandler::default(),
        )
        .unwrap();
    let initial = themes[0].clone();
    let window = cx.add_window(|_, cx| {
        let root = input_root("a", &cx.entity().downgrade(), &changes);
        StaticUiView::with_primitives(root, registry)
            .unwrap()
            .with_theme(initial)
            .unwrap()
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let focus = visual.update(|window, cx| window.focused(cx));
    assert!(focus.is_some());
    // Establish edit history before the switches, not merely an edit after
    // them: a remounted input could otherwise pass the first undo assertion.
    visual.simulate_input("b");
    visual.run_until_parked();
    assert_eq!(changes.borrow().last().unwrap(), "ab");
    visual.simulate_keystrokes("end shift-left");
    for theme in themes {
        window
            .update(&mut visual, |view, _, cx| view.set_theme(Some(theme), cx))
            .unwrap()
            .unwrap();
        visual.run_until_parked();
        assert_eq!(visual.update(|window, cx| window.focused(cx)), focus);
    }
    visual.simulate_input("c");
    visual.run_until_parked();
    assert_eq!(
        changes.borrow().last().unwrap(),
        "ac",
        "selection must survive every theme change"
    );
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-z"));
    visual.run_until_parked();
    assert_eq!(
        changes.borrow().last().unwrap(),
        "ab",
        "native edit history must survive"
    );
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-z"));
    visual.run_until_parked();
    assert_eq!(
        changes.borrow().last().unwrap(),
        "a",
        "pre-switch history must survive too"
    );
}

#[gpui::test]
fn static_theme_can_be_removed_without_losing_literal_rendering(cx: &mut TestAppContext) {
    let root = UiNode::text("Literal Host").with_style(
        &Style::new()
            .width(Length::Pixels(160.0))
            .height(Length::Pixels(40.0))
            .background(ColorValue::Literal(gpui_rhai::Rgba8::from_rgba_hex(
                0x1234_56ff,
            ))),
    );
    let theme = designs().remove(0);
    let window = cx.add_window(|_, _| StaticUiView::new(root).unwrap().with_theme(theme).unwrap());
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    window
        .update(&mut visual, |view, _, cx| {
            assert!(view.set_theme(None, cx).unwrap());
            assert!(!view.set_theme(None, cx).unwrap());
        })
        .unwrap();
    visual.run_until_parked();
    visual.refresh().unwrap();
    visual.run_until_parked();
    assert!(!filled(&mut visual, 0x1234_56ff).is_empty());
}

#[gpui::test]
fn static_theme_reaches_primitive_snapshots_and_inherited_environment(cx: &mut TestAppContext) {
    use gpui::{IntoElement, div};
    use gpui_rhai::{
        Environment, PrimitiveContext, PrimitiveHandler, PrimitiveInstance, PrimitiveTheme, Symbol,
    };

    struct Probe(Rc<RefCell<Option<PrimitiveTheme>>>);
    impl PrimitiveHandler for Probe {
        fn render(
            &mut self,
            _: &PrimitiveInstance,
            _: &PrimitiveContext,
            theme: &PrimitiveTheme,
            _: &mut gpui::Window,
            _: &mut gpui::App,
        ) -> Result<gpui::AnyElement, String> {
            *self.0.borrow_mut() = Some(theme.clone());
            Ok(div().into_any_element())
        }
    }
    let themes = designs();
    let received = Rc::new(RefCell::new(None));
    let registry = PrimitiveRegistry::new();
    registry
        .register(
            text_input_primitive_descriptor(),
            Probe(Rc::clone(&received)),
        )
        .unwrap();
    let root = UiNode::column(vec![UiNode::custom(PrimitiveNode {
        primitive: text_input_primitive_descriptor().id,
        key: Some("probe".into()),
        props: PrimitiveProps::new()
            .with(
                "value",
                PrimitiveValue::Data(UiValue::String(String::new())),
            )
            .with(
                "typography",
                PrimitiveValue::Data(UiValue::String("control".into())),
            ),
    })])
    .with_attribute(
        "environment",
        UiValue::Map(std::collections::BTreeMap::from([
            ("density".into(), UiValue::String("compact".into())),
            ("scale".into(), UiValue::String("150".into())),
        ])),
    );
    let initial = themes[0].clone();
    let window = cx.add_window(|_, _| {
        StaticUiView::with_primitives(root, registry)
            .unwrap()
            .with_theme(initial)
            .unwrap()
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    for theme in themes {
        window
            .update(&mut visual, |view, _, cx| {
                view.set_theme(Some(theme.clone()), cx)
            })
            .unwrap()
            .unwrap();
        visual.run_until_parked();
        visual.refresh().unwrap();
        visual.run_until_parked();
        let received = received.borrow();
        let snapshot = received.as_ref().expect("primitive rendered");
        let expected = Environment::EMPTY
            .with(Symbol::intern("density"), Symbol::intern("compact"))
            .unwrap()
            .with(Symbol::intern("scale"), Symbol::intern("150"))
            .unwrap();
        assert_eq!(snapshot.environment(), expected);
        assert_eq!(
            snapshot.color("surface.field"),
            theme.tokens.color("surface.field")
        );
        let length = Length::token("metrics.field_h").unwrap();
        assert_eq!(
            snapshot.resolve_length(length),
            theme.tokens.resolve_length(length, &expected)
        );
        assert_eq!(
            snapshot.typography("control"),
            theme.tokens.resolve_typography("control", &expected)
        );
    }
}
