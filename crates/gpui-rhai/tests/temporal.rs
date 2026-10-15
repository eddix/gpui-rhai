mod common;

use common::{mount, new_runtime};
use gpui_rhai::{LocaleManager, UiValue, WallClock, load_locale_source};
use gpui_rhai_registry as registry;

fn compile_official(name: &str, source: &str) -> (gpui_rhai::RuntimeEngine, gpui_rhai::CompiledUi) {
    // Read source-owned modules at execution time so edit/verify loops test the exact
    // current copies, including Gallery modules which are not runtime constructors.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry");
    let modules = common::official_modules()
        .map(|(id, _)| {
            (
                id.to_owned(),
                std::fs::read_to_string(root.join(format!("{id}.rhai"))).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut engine = common::engine_with(
        modules
            .iter()
            .map(|(id, source)| (id.as_str(), source.as_str())),
    );
    let compiled = common::compile(&mut engine, name, source);
    (engine, compiled)
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
