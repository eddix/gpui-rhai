//! Real focused native editing: draft changes never escape before Apply; Escape cancels.
use crate::support::{
    Mount, ReplaceOnce, Step, bounds, by_test_id, click_center, dispatch, last_error,
    node_by_test_id, settle, snapshot,
};
use gpui::TestAppContext;
use gpui_rhai::WallClock;
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

const APP: &str = r#"
import "components/time_range_picker" as picker;
fn state_schema(){#{fields:#{
    range:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{kind:#{type:"string",value:"absolute"},start:#{type:"integer",value:1791947400000},end:#{type:"integer",value:1791952200000}}}},
    changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,value){ctx.set_state("range",value);ctx.set_state("changes",ctx.get_state("changes")+1);}
fn view(ctx){column([
    picker::TimeRangePicker(#{key:"range",label:"Query range",value:ctx.get_state("range"),time_zone:"UTC",on_change:Fn("changed")}),
    text(`${ctx.get_state("changes")}`).test_id("changes"),
])}
"#;

#[gpui::test]
fn range_native_draft_apply_and_escape_cancel(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(APP)
        .locale("en.rhai", EN_LOCALE)
        .with(|v| v.wall_clock(WallClock::fixed(1_791_952_200_000)))
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let trigger = bounds(&mut visual, &view, "combobox", "Query range");
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 3);
    assert!(
        last_error(&mut visual, &view).is_none(),
        "{:?}",
        last_error(&mut visual, &view)
    );
    let end = bounds(&mut visual, &view, "text_field", "End");
    click_center(&mut visual, end);
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("05:30 AM");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "0");
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "0");
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 3);
    let end = bounds(&mut visual, &view, "text_field", "End");
    click_center(&mut visual, end);
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("05:30 AM");
    settle(&mut visual, Step::Frame, 3);
    let apply = bounds(&mut visual, &view, "button", "Apply");
    click_center(&mut visual, apply);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "1");
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn timeline_uses_one_focus_owner_for_selection_and_folding(cx: &mut TestAppContext) {
    let app = r#"
        import "components/timeline" as timeline;
        fn state_schema(){#{fields:#{
            selected:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}},
            expanded:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}}
        }}}
        fn selected(ctx,value){ctx.set_state("selected",value);}
        fn expanded(ctx,value){ctx.set_state("expanded",value);}
        fn view(ctx){column([
            timeline::Timeline(#{key:"timeline",label:"Pipeline",time_zone:"UTC",height:240,selection_mode:"single",
                selected_keys:ctx.get_state("selected"),on_selection_change:Fn("selected"),
                expanded:ctx.get_state("expanded"),on_expanded_change:Fn("expanded"),items:[
                    #{key:"stage",title:"Stage",status:"success",children:[#{key:"step",title:"Step",status:"success"}]},
                    #{key:"now",title:"Current",status:"running",current:true,start:1791947400000}
                ]}),
            text(`${ctx.get_state("selected")}`).test_id("selected"),
            text(`${ctx.get_state("expanded")}`).test_id("expanded"),
        ])}
    "#;
    let (mut visual, view) = Mount::components(app)
        .locale("en.rhai", EN_LOCALE)
        .with(|v| v.wall_clock(WallClock::fixed(1_791_952_200_000)))
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let stage = snapshot(&mut visual, &view)
        .nodes()
        .find(|n| n.role == "listitem" && n.name.starts_with("Stage,"))
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    click_center(&mut visual, stage);
    visual.simulate_keystrokes("down");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        node_by_test_id(&mut visual, &view, "selected")
            .name
            .contains("now")
    );
    visual.simulate_keystrokes("home");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        node_by_test_id(&mut visual, &view, "selected")
            .name
            .contains("stage")
    );
    visual.simulate_keystrokes("right");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        node_by_test_id(&mut visual, &view, "expanded")
            .name
            .contains("stage")
    );
    visual.simulate_keystrokes("down");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        node_by_test_id(&mut visual, &view, "selected")
            .name
            .contains("step")
    );
    visual.simulate_keystrokes("left");
    settle(&mut visual, Step::Frame, 2);
    visual.simulate_keystrokes("left");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(node_by_test_id(&mut visual, &view, "expanded").name, "[]");
    let ax = snapshot(&mut visual, &view);
    assert!(
        ax.nodes()
            .any(|node| node.current.as_deref() == Some("step") && node.name.contains("Current"))
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn time_input_steps_localized_native_text_at_midnight(cx: &mut TestAppContext) {
    let app = r#"
        import "components/time_input" as time_input;
        fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:"11:59 PM"}}}}}
        fn changed(ctx,value){ctx.set_state("value",value);}
        fn view(ctx){column([
            time_input::TimeInput(#{key:"clock",label:"Clock",value:ctx.get_state("value"),on_change:Fn("changed")}),
            text(ctx.get_state("value")).test_id("value"),
        ])}
    "#;
    let (mut visual, view) = Mount::components(app)
        .locale("en.rhai", EN_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_keystrokes("up");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        node_by_test_id(&mut visual, &view, "value").name,
        "12:00 AM"
    );
    visual.simulate_keystrokes("down");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        node_by_test_id(&mut visual, &view, "value").name,
        "11:59 PM"
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn range_fold_requires_a_real_explicit_instant_choice(cx: &mut TestAppContext) {
    let start =
        gpui_rhai::time::local_time_candidates("2026-11-01", "00:30:00", "America/New_York")
            .unwrap()[0];
    let end = gpui_rhai::time::local_time_candidates("2026-11-01", "02:30:00", "America/New_York")
        .unwrap()[0];
    let source = APP
        .replace_once("1791947400000", &start.to_string())
        .replace_once("1791952200000", &end.to_string())
        .replace_once("time_zone:\"UTC\"", "time_zone:\"America/New_York\"");
    let (mut visual, view) = Mount::components(source)
        .locale("en.rhai", EN_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let trigger = bounds(&mut visual, &view, "combobox", "Query range");
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 2);
    let field = bounds(&mut visual, &view, "text_field", "Start");
    click_center(&mut visual, field);
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("01:30 AM");
    settle(&mut visual, Step::Frame, 3);
    let ax = snapshot(&mut visual, &view);
    assert!(
        ax.find_by_role_and_name("button", "Apply")
            .next()
            .unwrap()
            .disabled
    );
    let choices = ax
        .nodes()
        .filter(|n| n.role == "button" && n.name.contains("Choose which occurrence"))
        .collect::<Vec<_>>();
    assert_eq!(choices.len(), 2);
    assert!(choices.iter().any(|n| n.name.contains("-04:00")));
    assert!(choices.iter().any(|n| n.name.contains("-05:00")));
    let later = choices
        .iter()
        .find(|n| n.name.contains("-05:00"))
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    click_center(&mut visual, later);
    settle(&mut visual, Step::Frame, 2);
    assert!(
        !snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Apply")
            .next()
            .unwrap()
            .disabled
    );
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "0");
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn external_range_and_zone_replacement_rebases_the_open_draft(cx: &mut TestAppContext) {
    let app = r#"
        import "components/time_range_picker" as picker;
        fn state_schema(){#{fields:#{
            value:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{kind:#{type:"string",value:"absolute"},start:#{type:"integer",value:1791947400000},end:#{type:"integer",value:1791952200000}}}},
            zone:#{schema:#{type:"string"},"default":#{type:"string",value:"UTC"}},
            precision:#{schema:#{type:"string"},"default":#{type:"string",value:"minute"}},
            revision:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn replaced(ctx,payload){ctx.set_state("value",#{kind:"absolute",start:1792033800000,end:1792038600000});
            ctx.set_state("zone","offset:60");ctx.set_state("precision","second");ctx.set_state("revision",1);}
        fn changed(ctx,value){ctx.set_state("value",value);ctx.set_state("changes",ctx.get_state("changes")+1);}
        fn view(ctx){column([
            picker::TimeRangePicker(#{key:"range",label:"Range",value:ctx.get_state("value"),time_zone:ctx.get_state("zone"),
                precision:ctx.get_state("precision"),revision:ctx.get_state("revision"),on_change:Fn("changed")}),
            row([text("External update")]).with_key("replace").test_id("replace").accessibility_role("button").accessibility_label("External update").on_click(Fn("replaced")),
            text(`${ctx.get_state("changes")}`).test_id("changes"),
            text(`${ctx.get_state("value").end}`).test_id("end"),
        ])}
    "#;
    let (mut visual, view) = Mount::components(app)
        .locale("en.rhai", EN_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let trigger = bounds(&mut visual, &view, "combobox", "Range");
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 2);
    let end = bounds(&mut visual, &view, "text_field", "End");
    click_center(&mut visual, end);
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("06:30 AM");
    settle(&mut visual, Step::Frame, 2);
    dispatch(&mut visual, &view, by_test_id("replace"), "click").unwrap();
    settle(&mut visual, Step::Frame, 4);
    assert!(
        last_error(&mut visual, &view).is_none(),
        "{:?}",
        last_error(&mut visual, &view)
    );
    assert_eq!(
        node_by_test_id(&mut visual, &view, "end").name,
        "1792038600000",
        "external controller update must commit"
    );
    let ax = snapshot(&mut visual, &view);
    assert!(ax.find_by_role_and_name("dialog", "Range").next().is_some());
    assert_eq!(
        ax.find_by_role_and_name("text_field", "End")
            .next()
            .unwrap()
            .value,
        Some(gpui_rhai::UiValue::String("05:30:00 AM".into()))
    );
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "0");
    let apply = bounds(&mut visual, &view, "button", "Apply");
    click_center(&mut visual, apply);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "1");
    assert_eq!(
        node_by_test_id(&mut visual, &view, "end").name,
        "1792038600000"
    );
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn range_calendar_is_reachable_and_operable_from_the_presets(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(APP)
        .locale("en.rhai", EN_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let trigger = bounds(&mut visual, &view, "combobox", "Query range");
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 2);
    visual.simulate_keystrokes("tab");
    visual.simulate_keystrokes("right");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 3);
    let ax = snapshot(&mut visual, &view);
    assert_eq!(
        ax.find_by_role_and_name("text_field", "Start Custom range")
            .next()
            .unwrap()
            .value,
        Some(gpui_rhai::UiValue::String("2026-10-15".into()))
    );
    assert_eq!(node_by_test_id(&mut visual, &view, "changes").name, "0");
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn calendar_and_date_picker_use_runtime_rtl_arrow_mapping_once(cx: &mut TestAppContext) {
    for picker in [false, true] {
        let control = if picker {
            r#"picker::DatePicker(#{key:"date",label:"Date",value:"2026-10-14",on_change:Fn("chosen")})"#
        } else {
            r#"calendar::Calendar(#{key:"calendar",label:"Calendar",value:"2026-10-14",on_change:Fn("chosen")})"#
        };
        let app = format!(
            r#"
            import "components/calendar" as calendar;import "components/date_picker" as picker;
            fn state_schema(){{#{{fields:#{{picked:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
            fn init(ctx){{ctx.set_locale("ar");}}
            fn chosen(ctx,value){{ctx.set_state("picked",value);}}
            fn view(ctx){{column([{control},text(ctx.get_state("picked")).test_id("picked")])}}
        "#
        );
        let (mut visual, view) = Mount::components(app)
            .locale("en.rhai", EN_LOCALE)
            .locale("ar.rhai", AR_LOCALE)
            .mount(cx);
        settle(&mut visual, Step::Frame, 2);
        visual.update(|window, cx| window.focus_next(cx));
        if picker {
            visual.simulate_keystrokes("enter");
            settle(&mut visual, Step::Frame, 2);
            assert!(
                last_error(&mut visual, &view).is_none(),
                "{:?}",
                last_error(&mut visual, &view)
            );
            assert!(
                snapshot(&mut visual, &view)
                    .nodes()
                    .any(|n| n.role == "dialog"),
                "DatePicker must open from its trigger"
            );
        }
        visual.simulate_keystrokes("left");
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            node_by_test_id(&mut visual, &view, "picked").name,
            "2026-10-15",
            "picker={picker}"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn timeline_initial_current_reveal_and_external_selection_target_are_independent(
    cx: &mut TestAppContext,
) {
    let app = r#"
        import "components/timeline" as timeline;
        fn state_schema(){#{fields:#{selected:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"item-0"}]}}}}}
        fn selected(ctx,value){ctx.set_state("selected",value);}
        fn external(ctx,value){ctx.set_state("selected",["item-50"]);}
        fn view(ctx){let items=[];for i in 0..100{items.push(#{key:`item-${i}`,title:`Event ${i}`,status:"success",current:i==99});}
            column([timeline::Timeline(#{key:"events",label:"Events",items:items,height:90,selection_mode:"single",
                selected_keys:ctx.get_state("selected"),on_selection_change:Fn("selected")}),
                row([text("Select B")]).with_key("external").test_id("external").accessibility_role("button").accessibility_label("External selection").on_click(Fn("external")),
                text(`${ctx.get_state("selected")}`).test_id("selected")])
        }
    "#;
    let (mut visual, view) = Mount::components(app)
        .locale("en.rhai", EN_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Frame, 5);
    let list = bounds(&mut visual, &view, "list", "Events");
    let ax = snapshot(&mut visual, &view);
    let current = ax
        .nodes()
        .find(|n| n.current.as_deref() == Some("step"))
        .expect("current item must be realized first")
        .geometry
        .unwrap()
        .visual;
    assert!(
        current.y >= list.y - 1.0 && current.y < list.y + list.height,
        "current={current:?}, list={list:?}"
    );
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_keystrokes("home");
    settle(&mut visual, Step::Frame, 3);
    dispatch(&mut visual, &view, by_test_id("external"), "click").unwrap();
    settle(&mut visual, Step::Frame, 3);
    visual.simulate_keystrokes("down");
    settle(&mut visual, Step::Frame, 3);
    assert!(
        node_by_test_id(&mut visual, &view, "selected")
            .name
            .contains("item-51")
    );
    assert!(last_error(&mut visual, &view).is_none());
}
