//! Combobox pointer picks update transactional caller state, and a relative width tracks the
//! flex space and sizes the panel.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, node_texts, official_icon_assets};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn combobox_pointer_updates_transactional_rhai_caller_state(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/combobox" as combobox;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: false } },
                        selected: #{ schema: #{ type: "array", max_items: 1,
                            items: #{ type: "string" } }, "default": #{ type: "array",
                            value: [#{ type: "string", value: "default-dark" }] } }
                    } } }
                    fn changed(ctx, values) {
                        ctx.set_state("selected", values);
                        ctx.set_state("open", false);
                    }
                    fn opened(ctx, open) { ctx.set_state("open", open); }
                    fn view(ctx) {
                        let selected = ctx.get_state("selected");
                        column([
                            text(`Selected: ${selected[0]}`),
                            combobox::Combobox(#{
                                key: "theme",
                                label: "Theme",
                                options: [
                                    #{ value: "default-dark", label: "Default Dark" },
                                    #{ value: "tokyo-night", label: "Tokyo Night" },
                                    #{ value: "tokyo-storm", label: "Tokyo Storm" }
                                ],
                                selected: selected, open: ctx.get_state("open"), query: "",
                                searchable: false,
                                on_change: Fn("changed"), on_open_change: Fn("opened")
                            })
                        ])
                    }
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/combobox").unwrap(),
                include_str!("../../../../../registry/components/combobox.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/input").unwrap(),
                include_str!("../../../../../registry/components/input.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("combobox-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("combobox-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some((host.clone(), view.clone()));
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (host, view) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let trigger = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("combobox", "Theme")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    visual.simulate_click(
        point(
            px((trigger.visual.x + trigger.visual.width / 2.0) as f32),
            px((trigger.visual.y + trigger.visual.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let placement = host
        .overlay_placement("combobox-view", "theme")
        .unwrap()
        .unwrap();
    visual.simulate_click(
        point(
            px((placement.bounds.x + placement.bounds.width / 2.0) as f32),
            px((placement.bounds.y + 4.0 + 32.0 + 16.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(
        texts.contains(&"Selected: tokyo-night".to_owned()),
        "{texts:?}; placement={placement:?}"
    );
}

#[gpui::test]
fn combobox_relative_width_tracks_flex_space_and_sizes_its_panel(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/combobox" as combobox;
                    import "components/select" as select;
                    fn view(ctx) {
                        let options = [
                            #{ value: "one", label: "One" },
                            #{ value: "two", label: "Two" },
                        ];
                        row([
                            combobox::Combobox(#{
                                key: "adaptive", label: "Adaptive", options: options,
                                selected: [], open: true, query: "", searchable: false,
                                placeholder: "Adaptive value", width: relative(1.0),
                            }).with_style(style().flex_grow().min_width(px(0))),
                            select::Select(#{
                                key: "fixed", label: "Fixed", options: options,
                                value: "one", open: false, query: "", width: px(140),
                                empty_text: "No results",
                            }).with_style(style().flex_shrink(false)),
                        ]).with_style(style().width(relative(1.0)).padding(px(20)).gap(px(10)))
                    }
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/combobox").unwrap(),
                include_str!("../../../../../registry/components/combobox.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/select").unwrap(),
                include_str!("../../../../../registry/components/select.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/input").unwrap(),
                include_str!("../../../../../registry/components/input.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("adaptive-width-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("adaptive-width-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some((host.clone(), view.clone()));
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (host, view) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let measure = |visual: &mut VisualTestContext| {
        for _ in 0..3 {
            cx.background_executor
                .advance_clock(std::time::Duration::from_millis(16));
            visual.run_until_parked();
        }
        let query = |visual: &mut VisualTestContext, name: &str| {
            let result = visual
                .update(|window, cx| {
                    view.automate(
                        gpui_rhai::AutomationCommand::Query {
                            locator: gpui_rhai::AutomationLocator::RoleName {
                                role: "combobox".to_owned(),
                                name: name.to_owned(),
                            },
                        },
                        window,
                        cx,
                    )
                })
                .unwrap();
            let gpui_rhai::AutomationResult::Node { node } = result else {
                panic!("expected {name} combobox");
            };
            node.bounds.expect("combobox trigger has committed bounds")
        };
        let adaptive = query(visual, "Adaptive");
        let fixed = query(visual, "Fixed");
        let panel = host
            .overlay_placement("adaptive-width-view", "adaptive")
            .unwrap()
            .expect("open adaptive panel has placement");
        (adaptive, fixed, panel.bounds)
    };

    visual.simulate_resize(size(px(820.0), px(620.0)));
    let compact = measure(&mut visual);
    visual.simulate_resize(size(px(1_180.0), px(620.0)));
    let expanded = measure(&mut visual);

    for (_, fixed, _) in [compact, expanded] {
        assert!(
            (139.0..=141.0).contains(&fixed.width),
            "fixed Select should retain 140px width, got {fixed:?}"
        );
    }
    assert!(
        expanded.0.width > compact.0.width + 300.0,
        "adaptive Combobox did not consume newly available flex width: compact={:?}, expanded={:?}",
        compact.0,
        expanded.0
    );
    for (trigger, _, panel) in [compact, expanded] {
        assert!(
            (trigger.width - panel.width).abs() <= 1.0,
            "panel should match realized trigger width: trigger={trigger:?}, panel={panel:?}"
        );
    }
}
