use std::collections::BTreeMap;
use gpui::{Context,IntoElement,Render,TestAppContext,VisualTestContext,Window};
use gpui_rhai::*;

struct Host{host:ScriptViewHost,views:Vec<ScriptViewHandle>}
impl Render for Host{
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{
        use gpui::ParentElement;
        self.host.container(gpui::div().children(self.views.iter().filter_map(|v|v.element().ok())))
    }
}
fn peer_drain_after_source_dispose(cx:&mut TestAppContext,operation:&str){
    cx.update(gpui_rhai::install);
    let source=r#"
fn init(ctx){}
fn open_child(ctx,v){ctx.open_window("child","Child",400,300,false);}
fn queued(ctx,v){QUEUED_OPERATION}
fn fail(ctx,v){throw "independent delivery failure";}
fn late(ctx,v){ctx.open_window("late","Late",400,300,false);}
fn view(ctx){
 if ctx.window_id()=="domain" {
  timeout("a-queue",100,false,Fn("queued"),());
  timeout("z-fail",100,false,Fn("fail"),());
 }
 text("Open child").test_id("open-child").on_click(Fn("open_child")).on_key_value("n",Fn("late"),())
}
"#.replace("QUEUED_OPERATION",if operation=="close"{"ctx.close_window(\"child\");"}else{"ctx.open_window(\"late\",\"Late\",400,300,false);"});
    let entry=ModuleId::parse("main").unwrap();
    let prepared=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([(entry,source)])),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .runtime_clock(ManualRuntimeClock::new(std::time::Instant::now()).clock()).prepare().unwrap();
    let w=cx.add_window(move|window,cx|{
        let host=ScriptViewHost::new("domain",cx).unwrap();
        let view=prepared.mount_window(ScriptViewConfig::new("owner"),host.clone(),window,cx).unwrap();
        Host{host,views:vec![view]}
    });cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    w.update(cx,|root,window,cx|root.views[0].automate(AutomationCommand::Dispatch{
        locator:AutomationLocator::TestId{id:"open-child".into()},event:"click".into(),payload:None,
    },window,cx)).unwrap().unwrap();cx.run_until_parked();
    assert_eq!(cx.windows().len(),2,"setup must create a real secondary window");
    let child=cx.windows().into_iter().find(|handle|*handle!=*w).unwrap();
    w.update(cx,|root,window,cx|{
        let owner=&root.views[0];
        let result=owner.automate(AutomationCommand::AdvanceTime{millis:100},window,cx);
        println!("queued {operation} then failing sibling: {result:?}");
        assert!(result.is_err());owner.dispose(cx).unwrap();root.views.clear();cx.notify();
    }).unwrap();
    cx.run_until_parked();
    cx.background_executor.advance_clock(std::time::Duration::from_millis(32));cx.run_until_parked();
    let windows=cx.windows();println!("revoked source queued {operation}: native_count={}, child_survives={}",windows.len(),windows.contains(&child));
    assert_eq!(windows.len(),2,"another owned view must not execute a command from a disposed source");
    assert!(windows.contains(&child));
    if operation=="open" {
        let mut child_visual=VisualTestContext::from_window(child,cx);
        child_visual.update(|window,cx|window.focus_next(cx));
        child_visual.simulate_keystrokes("n");child_visual.run_until_parked();
        assert_eq!(cx.windows().len(),3,"live child must reuse the cancelled late reservation");
        println!("cancelled Open reservation reused by live child: 3 windows");
    }
}
#[gpui::test]fn disposed_source_close_is_not_reauthorized_by_secondary_pump(cx:&mut TestAppContext){peer_drain_after_source_dispose(cx,"close");}
#[gpui::test]fn disposed_source_open_is_not_reauthorized_by_secondary_pump(cx:&mut TestAppContext){peer_drain_after_source_dispose(cx,"open");}

fn builder(source:&str)->EmbeddedScriptView{
    let entry=ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([(entry,source.to_owned())])),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
}
fn mounted(cx:&mut TestAppContext,prepared:PreparedScriptView)->gpui::WindowHandle<Host>{
    cx.update(gpui_rhai::install);let w=cx.add_window(move|window,cx|{
        let host=ScriptViewHost::new("domain",cx).unwrap();
        let view=prepared.mount_window(ScriptViewConfig::new("owner"),host.clone(),window,cx).unwrap();
        Host{host,views:vec![view]}
    });cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();w
}
fn dispatch(w:gpui::WindowHandle<Host>,cx:&mut TestAppContext,id:&str)->Result<AutomationResult,ScriptViewError>{
    w.update(cx,|root,window,cx|root.views[0].automate(AutomationCommand::Dispatch{
        locator:AutomationLocator::TestId{id:id.into()},event:"click".into(),payload:None,
    },window,cx)).unwrap()
}
#[gpui::test]fn open_focus_close_chain_uses_one_reservation(cx:&mut TestAppContext){
    let source=r#"
fn run(ctx,v){ctx.open_window("transient","Transient",400,300,false);ctx.focus_window("transient");ctx.close_window("transient");}
fn view(ctx){text("Run").test_id("run").on_click(Fn("run"))}
"#;
    let w=mounted(cx,builder(source).prepare().unwrap());
    for _ in 0..2 {dispatch(w,cx,"run").unwrap();cx.run_until_parked();assert_eq!(cx.windows().len(),1);}
    let error=w.update(cx,|root,_,cx|root.views[0].last_error(cx).unwrap()).unwrap();
    println!("Open->Focus->Close repeated with same id: native windows=1, error={error:?}");assert!(error.is_none());
}

#[derive(Clone)]struct Hooks{seen:std::rc::Rc<std::cell::RefCell<Vec<(u64,(usize,usize))>>>,system:bool}
impl ScriptViewExtension for Hooks{
    fn configure_engine(&self,engine:&mut RuntimeEngine)->Result<(),String>{
        self.seen.borrow_mut().push((engine.operation_limit(),engine.expression_depth_limits()));
        engine.register_native_handler(NativeHandlerDescriptor::new(NativeHandlerId::parse("audit.host_close").unwrap(),
            BTreeMap::from([("click".to_owned(),ValueSchema::UiValue)])).unwrap(),|_,runtime,_,_|{
                runtime.windows.request_close("child").map_err(|e|e.to_string())?;Ok(EventResponse::new().stop())
            }).map_err(|e|e.to_string())
    }
    fn configure_runtime(&self,runtime:&mut UiRuntimeState)->Result<(),String>{
        if self.system{runtime.theme.as_mut().unwrap().set_app(ThemePreference::System{family:"Default".into()}).map_err(|e|e.to_string())?;}
        Ok(())
    }
}
#[gpui::test]fn trusted_host_origin_survives_transport_view_dispose(cx:&mut TestAppContext){
    let source=r#"
fn open(ctx,v){ctx.open_window("child","Child",400,300,false);}
fn view(ctx){column([text("Open").test_id("open").on_click(Fn("open")),
 text("Host close").test_id("host-close").on("click",native_handler("audit.host_close"))])}
"#;
    let hooks=Hooks{seen:Default::default(),system:false};let w=mounted(cx,builder(source).extension(hooks).prepare().unwrap());
    dispatch(w,cx,"open").unwrap();cx.run_until_parked();assert_eq!(cx.windows().len(),2);
    w.update(cx,|root,window,cx|{
        root.views[0].automate(AutomationCommand::Dispatch{locator:AutomationLocator::TestId{id:"host-close".into()},event:"click".into(),payload:None},window,cx).unwrap();
        root.views[0].dispose(cx).unwrap();root.views.clear();cx.notify();
    }).unwrap();cx.run_until_parked();
    println!("explicit trusted Host close after transport dispose: native windows={}",cx.windows().len());
    assert_eq!(cx.windows().len(),1);assert!(cx.windows().contains(&*w));
}
#[gpui::test]fn secondary_init_self_close_preserves_native_qualification_theme_and_policy(cx:&mut TestAppContext){
    let source=r#"
fn init(ctx){if ctx.theme_variant().mode!="light"{throw "init did not receive native Light";}
 if ctx.window_id()=="child"{ctx.focus_window("child");ctx.close_window("child");}}
fn open(ctx,v){ctx.open_window("child","Child",400,300,false);}
fn view(ctx){text("Open").test_id("open").on_click(Fn("open"))}
"#;
    let hooks=Hooks{seen:Default::default(),system:true};let seen=hooks.seen.clone();
    let prepared=builder(source).theme_sources([("light".into(),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_light.rhai")).into())])
        .operation_limit(300_000).expression_depth_limits(96,48).extension(hooks).prepare().unwrap();
    let w=mounted(cx,prepared);dispatch(w,cx,"open").unwrap();cx.run_until_parked();
    assert_eq!(cx.windows().len(),1,"init close must retain its original qualified reservation");
    let policies=seen.borrow().clone();println!("primary and secondary policies: {policies:?}; child init native-Light self-close complete");
    assert_eq!(policies,vec![(300_000,(96,48)),(300_000,(96,48))]);
}
