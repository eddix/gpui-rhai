mod common;

use common::{compile_official, mount, new_runtime};
use gpui_rhai::{LocaleManager, UiValue, WallClock, load_locale_source};
use gpui_rhai_registry as registry;

fn settle_lifecycle(
    lifecycle: &mut gpui_rhai::ScriptLifecycle,
    engine: &mut gpui_rhai::RuntimeEngine,
) {
    for _ in 0..16 {
        if !lifecycle.render_dirty(engine).unwrap() {
            return;
        }
    }
    panic!("effect state did not settle after 16 formal renders");
}

fn handler_node<'a>(node: &'a gpui_rhai::UiNode, event: &str) -> Option<&'a gpui_rhai::UiNode> {
    if node.handler(event).is_some() {
        return Some(node);
    }
    match node.kind() {
        gpui_rhai::UiNodeKind::Box { children } | gpui_rhai::UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| handler_node(child, event))
        }
        gpui_rhai::UiNodeKind::Overlay {
            trigger, content, ..
        } => handler_node(trigger, event).or_else(|| handler_node(content, event)),
        _ => None,
    }
}

#[test]
fn formal_effect_replacement_commits_owned_state_and_reaches_a_fixed_point() {
    let (mut engine, compiled) = compile_official(
        "effect_replace.rhai",
        r#"
        define_component(#{metadata:#{id:"test/effect_copy","export":"EffectCopy",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},value:#{schema:#{type:"integer"},required:true,sensitive:false}},
                state:#{fields:#{copied:#{schema:#{type:"integer"},"default":#{type:"integer",value:-1}}}},events:#{},slots:#{},parts:[],effects:["copy"]},render:Fn("render_copy")});
        fn state_schema(){#{fields:#{value:#{schema:#{type:"integer"},"default":#{type:"integer",value:1}}}}}
        fn copy_value(ctx,value){ctx.set_state("copied",value);}
        fn copy_cleanup(ctx,value){ctx.get_state("copied");}
        fn render_copy(ctx,props){effect("copy",props.value,Fn("copy_value"),Fn("copy_cleanup"));text(`${ctx.get_state("copied")}`)}
        fn external_replacement(ctx,payload){ctx.set_state("value",2);}
        fn view(ctx){render_component("test/effect_copy",#{key:"copy",value:ctx.get_state("value")})}
    "#,
    );
    let replace = engine.callback(&compiled, "external_replacement").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    settle_lifecycle(&mut lifecycle, &mut engine);
    let owner = lifecycle.root().unwrap().component_root().unwrap().clone();
    assert_eq!(
        runtime.borrow().component_state.get(&owner, "copied"),
        Some(&UiValue::Integer(1))
    );
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &replace, UiValue::Null)
        .unwrap();
    settle_lifecycle(&mut lifecycle, &mut engine);
    assert_eq!(
        runtime.borrow().component_state.get(&owner, "copied"),
        Some(&UiValue::Integer(2)),
        "state={:?}, effects={:?}",
        runtime.borrow().component_state.inspect(),
        runtime.borrow().effects.iter().collect::<Vec<_>>()
    );
}

#[test]
fn open_range_rebase_effect_commits_the_new_identity_before_apply() {
    let (mut engine, compiled) = compile_official(
        "range_effect_rebase.rhai",
        r#"
        import "components/time_range_picker" as picker;
        fn state_schema(){#{fields:#{value:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{kind:#{type:"string",value:"absolute"},start:#{type:"integer",value:1791947400000},end:#{type:"integer",value:1791952200000}}}},
            zone:#{schema:#{type:"string"},"default":#{type:"string",value:"UTC"}},precision:#{schema:#{type:"string"},"default":#{type:"string",value:"minute"}},revision:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
        fn external_replacement(ctx,payload){ctx.set_state("value",#{kind:"absolute",start:1792033800000,end:1792038600000});ctx.set_state("zone","offset:60");ctx.set_state("precision","second");ctx.set_state("revision",1);}
        fn view(ctx){picker::TimeRangePicker(#{key:"range",label:"Range",value:ctx.get_state("value"),time_zone:ctx.get_state("zone"),precision:ctx.get_state("precision"),revision:ctx.get_state("revision")})}
    "#,
    );
    let replace = engine.callback(&compiled, "external_replacement").unwrap();
    let runtime = new_runtime();
    let locale = load_locale_source(engine.engine(), "en.rhai", registry::EN_LOCALE).unwrap();
    runtime.borrow_mut().locale = Some(LocaleManager::new([locale], "en", "en").unwrap());
    runtime.borrow_mut().wall_clock = WallClock::fixed(1_791_952_200_000);
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let owner = lifecycle.root().unwrap().component_root().unwrap().clone();
    let open = handler_node(lifecycle.root().unwrap(), "open_change")
        .unwrap()
        .handler("open_change")
        .unwrap()
        .as_script()
        .unwrap()
        .clone();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &open, UiValue::Bool(true))
        .unwrap();
    settle_lifecycle(&mut lifecycle, &mut engine);
    assert_eq!(
        runtime.borrow().component_state.get(&owner, "open"),
        Some(&UiValue::Bool(true))
    );
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &replace, UiValue::Null)
        .unwrap();
    settle_lifecycle(&mut lifecycle, &mut engine);
    let base = runtime
        .borrow()
        .component_state
        .get(&owner, "draft_base")
        .cloned();
    let Some(UiValue::Map(base)) = base else {
        panic!("missing typed draft identity");
    };
    assert_eq!(
        base.get("revision"),
        Some(&UiValue::Integer(1)),
        "state={:?}, effects={:?}",
        runtime.borrow().component_state.inspect(),
        runtime.borrow().effects.iter().collect::<Vec<_>>()
    );
    assert_eq!(base.get("zone"), Some(&UiValue::String("offset:60".into())));
    assert_eq!(
        runtime.borrow().component_state.get(&owner, "end_time"),
        Some(&UiValue::String("05:30:00 AM".into()))
    );
}
#[test]
fn temporal_components_evaluate_with_pinned_clock_and_each_locale() {
    let source = r#"
        import "components/calendar" as calendar;
        import "components/date_picker" as date_picker;
        import "components/time_input" as time_input;
        import "components/time_range_picker" as time_range_picker;
        import "components/timeline" as timeline;
        fn noop(ctx,value) {}
        fn view(ctx) { column([
            calendar::Calendar(#{key:"calendar",label:"Calendar",value:"2026-10-14",months:2}),
            date_picker::DatePicker(#{key:"date",label:"Date",value:"2026-10-14"}),
            time_input::TimeInput(#{key:"clock",label:"Time",value:ctx.format_time(ctx.now(),"short","UTC"),on_change:Fn("noop")}),
            time_range_picker::TimeRangePicker(#{key:"range",label:"Range",value:#{kind:"relative",duration_ms:900000},time_zone:"UTC",on_change:Fn("noop")}),
            timeline::Timeline(#{key:"timeline",label:"Timeline",items:[
                #{key:"stage",title:"Stage",status:"success",start:1791947400000,end:1791947500000,
                    children:[#{key:"step",title:"Step",status:"success",start:1791947400000}]},
                #{key:"current",title:"Current",status:"running",start:1791947500000,current:true}],
                expanded:["stage"],details:#{current:text("Rich detail")},selection_mode:"single",time_display:"relative",time_zone:"UTC",height:180})
        ]) }
    "#;
    for locale_source in [
        registry::EN_LOCALE,
        registry::ZH_CN_LOCALE,
        registry::AR_LOCALE,
    ] {
        let (mut engine, compiled) = compile_official("temporal.rhai", source);
        let locale = load_locale_source(engine.engine(), "locale.rhai", locale_source).unwrap();
        let id = locale.locale.clone();
        let runtime = new_runtime();
        runtime.borrow_mut().locale = Some(LocaleManager::new([locale], &id, &id).unwrap());
        runtime.borrow_mut().wall_clock = WallClock::fixed(1_791_947_600_000);
        let lifecycle = mount(&mut engine, compiled, &runtime);
        assert!(lifecycle.root().is_some());
    }
}

#[test]
fn context_time_uses_locale_digits_and_unambiguous_integer_ranges() {
    let (mut engine, compiled) = compile_official(
        "clock.rhai",
        r#"
        fn view(ctx) {
            if ctx.now()!=1791947400000 { throw "clock was not pinned"; }
            let resolved=time_range_resolve(#{kind:"relative",duration_ms:900000},ctx.now());
            if resolved.end-resolved.start!=900000 { throw "relative range changed"; }
            if ctx.parse_time(ctx.format_time(ctx.now(),"medium","UTC"),"second")==() { throw "localized round trip failed"; }
            text(ctx.format_relative(ctx.now()-60000)+" "+ctx.format_duration(3661000))
        }
    "#,
    );
    let locale = load_locale_source(engine.engine(), "ar.rhai", registry::AR_LOCALE).unwrap();
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(LocaleManager::new([locale], "ar", "ar").unwrap());
    runtime.borrow_mut().wall_clock = WallClock::fixed(1_791_947_400_000);
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let gpui_rhai::UiNodeKind::Text { text } = lifecycle.root().unwrap().kind() else {
        panic!("expected text")
    };
    assert!(text.contains('١'));
    assert!(text.contains("منذ"));
    assert_eq!(
        UiValue::Integer(runtime.borrow().wall_clock.now_ms()),
        UiValue::Integer(1_791_947_400_000)
    );
}

#[test]
fn range_draft_handles_dst_choices_invalid_text_bounds_and_future() {
    let (mut engine, compiled) = compile_official(
        "draft.rhai",
        r#"
        import "components/time_range_picker" as picker;
        fn view(ctx){
            let identity=#{value:#{kind:"absolute",start:1000,end:2000},zone:"UTC",precision:"minute",revision:0,locale_key:"locale"};
            if !picker::same_identity(identity,identity){throw "matching draft identity was rejected";}
            let replacement=identity;replacement.value.end=3000;
            if picker::same_identity(identity,replacement){throw "changed range must invalidate draft identity";}
            let revision=identity;revision.revision=1;
            if picker::same_identity(identity,revision){throw "revision must invalidate draft identity";}
            let fold=picker::parse_draft(ctx,"2026-11-01","01:30 AM","2026-11-01","02:30 AM","America/New_York","minute",(),(),(),(),());
            if fold.error!=ctx.t("time.fold")||fold.start_candidates.len!=2{throw "fold must require a choice";}
            let chosen=picker::parse_draft(ctx,"2026-11-01","01:30 AM","2026-11-01","02:30 AM","America/New_York","minute",fold.start_candidates[1],(),(),(),());
            if chosen.value==()||chosen.value.start!=fold.start_candidates[1]{throw "explicit fold instant not preserved";}
            let gap=picker::parse_draft(ctx,"2026-03-08","02:30 AM","2026-03-08","03:30 AM","America/New_York","minute",(),(),(),(),());
            if gap.error!=ctx.t("time.gap"){throw "gap must be inline error";}
            let invalid=picker::parse_draft(ctx,"2026-1","01:30 AM","2026-11-01","02:30 AM","UTC","minute",(),(),(),(),());
            if invalid.error!=ctx.t("time.invalid"){throw "partial date must remain invalid draft";}
            let future=picker::parse_draft(ctx,"2030-01-01","01:30 AM","2030-01-01","02:30 AM","UTC","minute",(),(),(),(),());
            if future.value==(){throw "future time is allowed by default";}
            let capped=picker::parse_draft(ctx,"2030-01-01","01:30 AM","2030-01-01","02:30 AM","UTC","minute",(),(),(),ctx.now(),());
            if capped.error!=ctx.t("time.bounds"){throw "caller max forbids future";}
            text("validated")
        }
    "#,
    );
    let locale = load_locale_source(engine.engine(), "en.rhai", registry::EN_LOCALE).unwrap();
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(LocaleManager::new([locale], "en", "en").unwrap());
    runtime.borrow_mut().wall_clock = WallClock::fixed(1_791_947_400_000);
    let lifecycle = mount(&mut engine, compiled, &runtime);
    assert!(lifecycle.root().is_some());
}

#[test]
fn timeline_missing_skew_and_running_time_modes_are_stable() {
    let (mut engine, compiled) = compile_official(
        "timeline_time.rhai",
        r#"
        import "components/timeline" as timeline;
        fn view(ctx){let now=ctx.now();
            if timeline::timeline_time(ctx,#{status:"pending"},"relative","UTC",now)!="—"{throw "missing timestamp";}
            if timeline::timeline_time(ctx,#{start:now-3600000,status:"running"},"duration","UTC",now)!=ctx.format_duration(3600000){throw "running duration";}
            if timeline::timeline_time(ctx,#{start:now+1000,end:now+2000,status:"success"},"duration","UTC",now)!=ctx.format_time(now+1000,"short","UTC"){throw "future timestamp must be absolute";}
            if timeline::timeline_time(ctx,#{start:now-1000,end:now-2000,status:"success"},"relative","UTC",now)!=ctx.format_time(now-1000,"short","UTC"){throw "skew must be absolute";}
            text("valid")
        }
    "#,
    );
    let locale = load_locale_source(engine.engine(), "en.rhai", registry::EN_LOCALE).unwrap();
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(LocaleManager::new([locale], "en", "en").unwrap());
    runtime.borrow_mut().wall_clock = WallClock::fixed(1_791_947_400_000);
    assert!(mount(&mut engine, compiled, &runtime).root().is_some());
}
