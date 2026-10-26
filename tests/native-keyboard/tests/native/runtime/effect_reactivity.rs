//! Native foreground delivery must settle without a timer and stop rejected cascades.
use std::collections::BTreeMap;
use std::{cell::Cell, rc::Rc};

use crate::support::{
    Mount, ReplaceOnce, Step, by_test_id, dispatch, last_error, node_by_test_id, settle,
};
use gpui::TestAppContext;
use gpui_rhai::{
    NativeCollection, NativeTextDocument, ScriptViewExtension, UiRuntimeState, UiValue,
};

const CASCADE: &str = r#"
    define_component(#{metadata:#{id:"test/native_cascade","export":"NativeCascade",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},enabled:#{schema:#{type:"bool"},required:true,sensitive:false},parent_count:#{schema:#{type:"integer"},required:true,sensitive:false},semantic:#{schema:#{type:"bool"},required:true,sensitive:false},on_tick:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false}},
            state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{tick:#{payload:#{type:"ui_value"}}},slots:#{},parts:[],effects:["step"]},render:Fn("render_native_cascade")});
    fn state_schema(){#{fields:#{enabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
    fn step(ctx,deps){if deps.enabled {if deps.semantic {ctx.emit("tick",());} else {ctx.set_state("count",ctx.get_state("count")+1);}}}
    fn cleanup(ctx,deps){}
    fn render_native_cascade(ctx,props){
        effect("step",#{enabled:props.enabled,semantic:props.semantic,parent_count:props.parent_count,count:ctx.get_state("count")},Fn("step"),Fn("cleanup"));
        text(`${ctx.get_state("count")+props.parent_count}`).test_id("count")
    }
    fn tick(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
    fn enable(ctx,payload){ctx.set_state("enabled",true);}
    fn disable(ctx,payload){ctx.set_state("enabled",false);}
    fn failed_recovery(ctx,payload){ctx.set_state("enabled",false);throw "rejected recovery";}
    fn view(ctx){column([
        render_component("test/native_cascade",#{key:"cascade",enabled:ctx.get_state("enabled"),parent_count:ctx.get_state("count"),semantic:false,on_tick:Fn("tick")}),
        text("Enable").test_id("enable").on_click(Fn("enable")),
        text("Disable").test_id("disable").on_click(Fn("disable")),
        text("Rejected recovery").test_id("reject").on_click(Fn("failed_recovery")),
        text(`${ctx.get_state("enabled")}`).test_id("enabled")
    ])}
"#;

#[derive(Clone)]
struct DispatchCalls(Rc<Cell<usize>>);

impl ScriptViewExtension for DispatchCalls {
    fn configure_rhai_engine(&self, engine: &mut gpui_rhai::rhai::Engine) -> Result<(), String> {
        let calls = Rc::clone(&self.0);
        engine.register_fn("record_dispatch", move || calls.set(calls.get() + 1));
        Ok(())
    }
}

#[gpui::test]
fn pure_semantic_loop_is_quarantined_across_old_wakes_clock_polls_and_external_stop(
    cx: &mut TestAppContext,
) {
    let calls = Rc::new(Cell::new(0));
    let audit = DispatchCalls(Rc::clone(&calls));
    let (mut visual, view) = Mount::new(r#"
        define_component(#{metadata:#{id:"test/pure_dispatch","export":"PureDispatch",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},enabled:#{schema:#{type:"bool"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[],effects:["dispatch"]},render:Fn("render_pure_dispatch")});
        fn state_schema(){#{fields:#{enabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
        fn init(ctx){ctx.register_action("test.loop",Fn("loop_action"));}
        fn loop_action(ctx,payload){record_dispatch();ctx.dispatch_action("test.loop",());}
        fn start_dispatch(ctx,enabled){if enabled {ctx.dispatch_action("test.loop",());}}
        fn cleanup_dispatch(ctx,enabled){}
        fn disable(ctx,payload){ctx.set_state("enabled",false);}
        fn render_pure_dispatch(ctx,props){effect("dispatch",props.enabled,Fn("start_dispatch"),Fn("cleanup_dispatch"));text("stable")}
        fn view(ctx){column([render_component("test/pure_dispatch",#{key:"loop",enabled:ctx.get_state("enabled")}),
            text("Stop").test_id("stop").on_click(Fn("disable")),text(`${ctx.get_state("enabled")}`).test_id("enabled")])}
    "#).with(move |view| view.extension(audit)).mount(cx);
    settle(&mut visual, Step::Frame, 4);
    assert!(
        last_error(&mut visual, &view)
            .unwrap()
            .contains("64-callback budget")
    );
    let stopped = calls.get();
    assert!(
        stopped > 0 && stopped <= 64,
        "pure dispatch ran {stopped} callbacks"
    );
    settle(&mut visual, Step::Frame, 4);
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(
        calls.get(),
        stopped,
        "rollback or old wakes/polls restarted rejected pending work"
    );
    dispatch(&mut visual, &view, by_test_id("stop"), "click").unwrap();
    settle(&mut visual, Step::Frame, 4);
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(node_by_test_id(&mut visual, &view, "enabled").name, "false");
    assert_eq!(
        calls.get(),
        stopped,
        "external recovery revived an old effect derivation"
    );
}

#[gpui::test]
fn state_and_semantic_effect_cascades_stop_and_external_input_recovers_without_ticks(
    cx: &mut TestAppContext,
) {
    for semantic in [false, true] {
        let source = CASCADE.replace_once("semantic:false", &format!("semantic:{semantic}"));
        let (mut visual, view) = Mount::new(source)
            .id(&format!("cascade-{semantic}"))
            .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(node_by_test_id(&mut visual, &view, "count").name, "0");
        dispatch(&mut visual, &view, by_test_id("enable"), "click").unwrap();
        // run_until_parked must terminate even though each successful render
        // immediately wakes the next turn. Frame never advances the clock.
        settle(&mut visual, Step::Frame, 4);
        assert!(
            last_error(&mut visual, &view)
                .unwrap()
                .contains("64-callback budget")
        );
        let rejected_count = node_by_test_id(&mut visual, &view, "count").name;
        let count = rejected_count.parse::<usize>().unwrap();
        assert!(
            count > 0 && count <= 32,
            "unexpected rejected count {count}"
        );
        assert!(dispatch(&mut visual, &view, by_test_id("reject"), "click").is_err());
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(node_by_test_id(&mut visual, &view, "enabled").name, "true");
        assert_eq!(
            node_by_test_id(&mut visual, &view, "count").name,
            rejected_count
        );
        dispatch(&mut visual, &view, by_test_id("disable"), "click").unwrap();
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(node_by_test_id(&mut visual, &view, "enabled").name, "false");
        let recovered = node_by_test_id(&mut visual, &view, "count").name;
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(node_by_test_id(&mut visual, &view, "count").name, recovered);
    }
}

#[derive(Clone)]
struct HostInputs;

fn rows(count: usize) -> NativeCollection {
    NativeCollection::new(
        "id",
        (0..count)
            .map(|index| BTreeMap::from([("id".to_owned(), UiValue::String(index.to_string()))])),
    )
    .unwrap()
}

impl ScriptViewExtension for HostInputs {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .native_documents
            .register(
                "source",
                NativeTextDocument::new("source", 1, "first").unwrap(),
            )
            .map_err(|error| error.to_string())?;
        runtime
            .native_collections
            .register("rows", rows(1))
            .map_err(|error| error.to_string())
    }
}

#[gpui::test]
fn direct_host_document_and_collection_mutations_render_without_a_clock_tick(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(r#"
        fn view(ctx){
            let revision = "missing";
            try {revision = `${ctx.get_native_text_document("source").revision}`;} catch(error) {}
            column([text(revision).test_id("revision"),text(`${ctx.get_native_collection("rows").len}`).test_id("rows")])
        }
    "#).with(|view| view.extension(HostInputs)).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "revision").name, "1");
    assert_eq!(node_by_test_id(&mut visual, &view, "rows").name, "1");
    assert!(
        visual
            .update(|_, cx| view.replace_native_text_document(
                "source",
                NativeTextDocument::new("source", 1, "stale").unwrap(),
                cx
            ))
            .is_err()
    );
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(node_by_test_id(&mut visual, &view, "revision").name, "1");
    visual.update(|_, cx| {
        assert!(
            view.replace_native_text_document(
                "source",
                NativeTextDocument::new("source", 2, "second").unwrap(),
                cx
            )
            .unwrap()
        );
        assert!(view.replace_native_collection("rows", rows(2), cx).unwrap());
    });
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "revision").name, "2");
    assert_eq!(node_by_test_id(&mut visual, &view, "rows").name, "2");
    assert!(
        visual
            .update(|_, cx| view.release_native_text_document("source", cx))
            .unwrap()
    );
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        node_by_test_id(&mut visual, &view, "revision").name,
        "missing"
    );
    visual
        .update(|_, cx| {
            view.register_native_text_document(
                "source",
                NativeTextDocument::new("source", 1, "registered again").unwrap(),
                cx,
            )
        })
        .unwrap();
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(node_by_test_id(&mut visual, &view, "revision").name, "1");
    assert!(last_error(&mut visual, &view).is_none());
}
