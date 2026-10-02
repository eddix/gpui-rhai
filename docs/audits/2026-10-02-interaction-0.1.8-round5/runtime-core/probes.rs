use std::{cell::{Cell,RefCell},collections::BTreeMap,rc::Rc,time::Duration};
use gpui::prelude::*;
use gpui::{Context,FocusHandle,Modifiers,MouseButton,Render,ScrollDelta,ScrollWheelEvent,TestAppContext,
    VisualTestContext,Window,WindowHandle,point,px};
use gpui_rhai::*;
struct Host{host:ScriptViewHost,view:ScriptViewHandle,focus:FocusHandle,escapes:Rc<Cell<usize>>}
impl Render for Host{
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{
        let e=self.escapes.clone();gpui::div().size_full().track_focus(&self.focus)
            .on_key_down(move|ev,_,_|{if ev.keystroke.key=="escape"{e.set(e.get()+1);}})
            .child(self.host.container(if self.view.state()==ScriptViewState::Active{
                self.view.element().unwrap()
            }else{gpui::div().into_any_element()}))
    }
}
struct ApplicationPolicy;
impl ScriptViewExtension for ApplicationPolicy{
    fn configure_window(&self,id:&str,runtime:&mut UiRuntimeState)->Result<(),String>{
        // This is an explicit main-API emulation of PR100's only behavioral
        // effect: the logical window permission becomes ApplicationOwned.
        // It does NOT pretend the private constructor is public on main.
        assert!(runtime.windows.remove(id));
        runtime.windows.register_open_for_view(id,WindowCommandPolicy::ApplicationOwned,"view")
            .map_err(|e|e.to_string())
    }
}
fn mount(cx:&mut TestAppContext,source:&str,allow:bool)->(WindowHandle<Host>,ScriptViewHandle,Rc<Cell<usize>>){
    cx.update(gpui_rhai::install);let entry=ModuleId::parse("main").unwrap();
    let mut builder=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([
        (entry,source.to_owned()),(ModuleId::parse("components/pan_zoom").unwrap(),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/pan_zoom.rhai")).to_owned()),
        (ModuleId::parse("components/rotatable").unwrap(),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/rotatable.rhai")).to_owned()),
    ])),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .motion_preference(MotionPreference::None);
    if allow{builder=builder.extension(ApplicationPolicy);}
    let prepared=builder.prepare().unwrap();let capture=Rc::new(RefCell::new(None));let c=capture.clone();
    let escapes=Rc::new(Cell::new(0));let e=escapes.clone();let w=cx.add_window(move|window,cx|{
        let host=ScriptViewHost::new("embedded-window",cx).unwrap();
        let view=prepared.mount(ScriptViewConfig::new("view"),host.clone(),window,cx).unwrap();
        *c.borrow_mut()=Some(view.clone());let focus=cx.focus_handle();focus.focus(window,cx);
        Host{host,view,focus,escapes:e}
    });cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    let view=capture.borrow().as_ref().unwrap().clone();(w,view,escapes)
}
fn signal(node:&UiNode,property:SignalProperty)->Option<NativeSignal>{
    if let Some((_,s))=node.signal_bindings().find(|(p,_)|*p==property){return Some(s.clone());}
    match node.kind(){UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(|n|signal(n,property)),_=>None}
}
fn value(v:&mut VisualTestContext,view:&ScriptViewHandle,property:SignalProperty)->f64{v.update(|_,cx|{
    let s=signal(&view.root(cx).unwrap().unwrap(),property).unwrap();match view.read_signal(&s,cx).unwrap(){
        SignalValue::Float(value)=>value,_=>panic!("float")}
})}
fn scale(v:&mut VisualTestContext,view:&ScriptViewHandle)->f64{value(v,view,SignalProperty::ScaleX)}
fn wheel(v:&mut VisualTestContext,phase:gpui::TouchPhase){
    v.simulate_event(ScrollWheelEvent{position:point(px(100.0),px(100.0)),
        delta:ScrollDelta::Pixels(point(px(0.0),px(-40.0))),touch_phase:phase,..Default::default()});v.run_until_parked();
}
const PAN:&str=r#"
import "components/pan_zoom" as pan;
fn view(ctx){pan::PanZoom(#{key:"pan",label:"Pan",transform:#{x:0.0,y:0.0,scale:1.0},wheel_zoom:"always",
 content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))]))})
 .with_style(style().width(px(300)).height(px(180)))}
"#;
#[gpui::test]fn stale_debounce_cannot_release_new_wheel_owner(cx:&mut TestAppContext){
    let(w,view,escapes)=mount(cx,PAN,false);let mut v=VisualTestContext::from_window(*w,cx);
    wheel(&mut v,gpui::TouchPhase::Moved);cx.background_executor.advance_clock(Duration::from_millis(40));v.run_until_parked();
    v.simulate_keystrokes("escape");assert_eq!(scale(&mut v,&view),1.0);
    wheel(&mut v,gpui::TouchPhase::Started);let next=scale(&mut v,&view);assert!(next>1.0);
    cx.background_executor.advance_clock(Duration::from_millis(120));v.run_until_parked();
    assert_eq!(scale(&mut v,&view),next,"old timeout must not end new explicit wheel");
    v.simulate_keystrokes("escape");assert_eq!(scale(&mut v,&view),1.0);assert_eq!(escapes.get(),0);
    v.simulate_keystrokes("escape");assert_eq!(escapes.get(),1);
    println!("stale debounce/new explicit generation: retained then cancelled correctly");
}
#[gpui::test]fn suspended_debounce_cannot_steal_resumed_escape(cx:&mut TestAppContext){
    let(w,view,escapes)=mount(cx,PAN,false);let mut v=VisualTestContext::from_window(*w,cx);
    wheel(&mut v,gpui::TouchPhase::Moved);assert!(scale(&mut v,&view)>1.0);
    assert!(v.update(|window,cx|view.suspend(window,cx)).unwrap());
    cx.background_executor.advance_clock(Duration::from_millis(120));v.run_until_parked();
    assert!(v.update(|_,cx|view.resume(cx)).unwrap());
    w.update(cx,|_,_,host_cx|host_cx.notify()).unwrap();cx.refresh().unwrap();v.run_until_parked();
    assert_eq!(scale(&mut v,&view),1.0);v.simulate_keystrokes("escape");assert_eq!(escapes.get(),1);
    assert!(v.update(|_,cx|view.last_error(cx).unwrap()).is_none());
    println!("suspend/pending debounce/resume: source restored and Escape released");
}
#[gpui::test]fn active_rotation_suspend_reads_canvas_geometry_inside_lifecycle(cx:&mut TestAppContext){
    let source=r#"
import "components/rotatable" as r;
fn view(ctx){r::Rotatable(#{key:"r",label:"Rotate",angle:0.0,pivot:#{x:100.0,y:60.0},
 content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))]))})
 .with_style(style().width(px(200)).height(px(120)))}
"#;
    let(w,view,_)=mount(cx,source,false);let mut v=VisualTestContext::from_window(*w,cx);
    v.simulate_mouse_down(point(px(180.0),px(60.0)),MouseButton::Left,Modifiers::default());
    v.simulate_mouse_move(point(px(100.0),px(100.0)),MouseButton::Left,Modifiers::default());v.run_until_parked();
    let active=value(&mut v,&view,SignalProperty::Rotate);assert!(active.abs()>5.0,"rotation must activate");
    let suspended=v.update(|window,cx|view.suspend(window,cx));
    println!("active rotation angle={active}, suspend={suspended:?}");assert!(suspended.unwrap());
    assert!(v.update(|_,cx|view.resume(cx)).unwrap());w.update(cx,|_,_,c|c.notify()).unwrap();cx.refresh().unwrap();v.run_until_parked();
    assert_eq!(value(&mut v,&view,SignalProperty::Rotate),0.0);
    assert!(v.update(|_,cx|view.last_error(cx).unwrap()).is_none());
}
const CLOSE:&str=r#"
fn close(ctx,event){ctx.close_window(ctx.window_id());}
fn focus(ctx,event){ctx.focus_window(ctx.window_id());}
fn view(ctx){column([text("Close").test_id("close").on_click(Fn("close")),
text("Focus").test_id("focus").on_click(Fn("focus"))])}
"#;
fn dispatch(v:&mut VisualTestContext,view:&ScriptViewHandle,id:&str)->Result<AutomationResult,ScriptViewError>{
    v.update(|window,cx|view.automate(AutomationCommand::Dispatch{
        locator:AutomationLocator::TestId{id:id.to_owned()},event:"click".to_owned(),payload:None,
    },window,cx))
}
#[gpui::test]fn embedded_disabled_policy_control(cx:&mut TestAppContext){
    let(w,view,_)=mount(cx,CLOSE,false);let mut v=VisualTestContext::from_window(*w,cx);
    let result=dispatch(&mut v,&view,"close");println!("Disabled close: {result:?}");
    assert!(result.unwrap_err().to_string().contains("unavailable for embedded view"));assert!(cx.windows().contains(&*w));
}
#[gpui::test]fn pr100_application_policy_needs_real_native_window_registration(cx:&mut TestAppContext){
    let(w,view,_)=mount(cx,CLOSE,true);let mut v=VisualTestContext::from_window(*w,cx);
    let close=dispatch(&mut v,&view,"close");let focus=dispatch(&mut v,&view,"focus");
    v.run_until_parked();let remains=cx.windows().contains(&*w);
    println!("ApplicationOwned logical policy close: {close:?}; focus: {focus:?}; window_remains={remains}");
    assert!(close.is_ok(),"PR100 intended embedded close requires a native handle, not just policy permission");
    assert!(!remains);
}
