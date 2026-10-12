//! Combobox pointer picks update transactional caller state, and a relative width tracks the
//! flex space and sizes the panel.

use crate::support::{Mount, bounds, by_role, click_center, node_texts};
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};

#[gpui::test]
fn combobox_pointer_updates_transactional_rhai_caller_state(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
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
"#,
    )
    .registry("components/combobox")
    .registry("components/input")
    .bundled_assets()
    .ids("combobox-window", "combobox-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let host = window.read_with(cx, |root, _| root.host.clone()).unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let trigger = bounds(&mut visual, &view, "combobox", "Theme");
    click_center(&mut visual, trigger);
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
    let (window, view) = Mount::new(
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
"#,
    )
    .registry("components/combobox")
    .registry("components/select")
    .registry("components/input")
    .bundled_assets()
    .ids("adaptive-width-window", "adaptive-width-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let host = window.read_with(cx, |root, _| root.host.clone()).unwrap();
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
                            locator: by_role("combobox", name),
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
