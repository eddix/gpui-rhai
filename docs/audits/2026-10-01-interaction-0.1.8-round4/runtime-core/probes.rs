use std::{cell::{Cell,RefCell},collections::BTreeMap,rc::Rc,time::Duration};
use gpui::prelude::*;
use gpui::{Context,FocusHandle,Render,ScrollDelta,ScrollWheelEvent,TestAppContext,
    VisualTestContext,Window,WindowHandle,point,px};
use gpui_rhai::*;

struct Host { host:ScriptViewHost, view:ScriptViewHandle, focus:FocusHandle, escapes:Rc<Cell<usize>> }
impl Render for Host {
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
        let escapes=self.escapes.clone();
        gpui::div().size_full().track_focus(&self.focus).on_key_down(move |event,_,_|{
            if event.keystroke.key=="escape" {escapes.set(escapes.get()+1);}
        }).child(self.host.container(if self.view.state()==ScriptViewState::Active{
            self.view.element().unwrap()
        }else{gpui::div().into_any_element()}))
    }
}
fn mount(cx:&mut TestAppContext,name:&str)->(WindowHandle<Host>,ScriptViewHandle,Rc<Cell<usize>>){
    cx.update(gpui_rhai::install);
    let entry=ModuleId::parse("main").unwrap();
    let script=r#"
import "components/pan_zoom" as pan;
fn view(ctx){pan::PanZoom(#{key:"pan",label:"Pan surface",transform:#{x:0.0,y:0.0,scale:1.0},
 wheel_zoom:"always",content:canvas(canvas_scene([
 canvas_rect("square",20.0,20.0,30.0,30.0,theme_color("accent"))
 ]))}).with_style(style().width(px(300)).height(px(180)))}
"#;
    let prepared=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([
        (entry,script.to_owned()),(ModuleId::parse("components/pan_zoom").unwrap(),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/pan_zoom.rhai")).to_owned()),
    ])),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .motion_preference(MotionPreference::None).prepare().unwrap();
    let capture=Rc::new(RefCell::new(None));let c=capture.clone();let escapes=Rc::new(Cell::new(0));let e=escapes.clone();
    let name=name.to_owned();let w=cx.add_window(move |window,cx|{
        let host=ScriptViewHost::new(format!("{name}-host"),cx).unwrap();
        let view=prepared.mount(ScriptViewConfig::new(name),host.clone(),window,cx).unwrap();
        *c.borrow_mut()=Some(view.clone());let focus=cx.focus_handle();focus.focus(window,cx);
        Host{host,view,focus,escapes:e}
    });
    cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    let view=capture.borrow().as_ref().unwrap().clone();(w,view,escapes)
}
fn signal(node:&UiNode)->Option<NativeSignal>{
    if let Some((_,s))=node.signal_bindings().find(|(p,_)|*p==SignalProperty::ScaleX){return Some(s.clone());}
    match node.kind(){UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(signal),_=>None}
}
fn scale(v:&mut VisualTestContext,view:&ScriptViewHandle)->f64{
    v.update(|_,cx|{
        let root=view.root(cx).unwrap().unwrap();let s=signal(&root).unwrap();
        match view.read_signal(&s,cx).unwrap(){SignalValue::Float(v)=>v,_=>panic!("float")}
    })
}
fn wheel(v:&mut VisualTestContext,phase:gpui::TouchPhase,y:f32){
    v.simulate_event(ScrollWheelEvent{position:point(px(100.0),px(100.0)),
        delta:ScrollDelta::Pixels(point(px(0.0),px(y))),touch_phase:phase,..Default::default()});
    v.run_until_parked();
}
fn after_completed_wheel(cx:&mut TestAppContext,explicit:bool){
    let(w,view,escapes)=mount(cx,if explicit{"explicit"}else{"debounced"});
    let mut v=VisualTestContext::from_window(*w,cx);
    wheel(&mut v,if explicit{gpui::TouchPhase::Started}else{gpui::TouchPhase::Moved},-40.0);
    let active=scale(&mut v,&view);assert!(active>1.0,"wheel must actually activate");
    if explicit{wheel(&mut v,gpui::TouchPhase::Ended,0.0);}else{
        cx.background_executor.advance_clock(Duration::from_millis(120));v.run_until_parked();
    }
    let completed=scale(&mut v,&view);println!("explicit={explicit} active={active} completed={completed}");
    assert_eq!(completed,1.0,"completed rejected/unobserved controlled proposal restores source");
    v.simulate_keystrokes("escape");v.run_until_parked();
    let first=escapes.get();
    v.simulate_keystrokes("escape");v.run_until_parked();
    println!("explicit={explicit} Escape delivered first={first}, after second={}",escapes.get());
    assert_eq!(first,1,"completed wheel must release interceptor ownership");
    assert_eq!(escapes.get(),2);
}
#[gpui::test]fn idle_escape_reaches_host(cx:&mut TestAppContext){
    let(w,_,escapes)=mount(cx,"idle");let mut v=VisualTestContext::from_window(*w,cx);
    v.simulate_keystrokes("escape");assert_eq!(escapes.get(),1);
}
#[gpui::test]fn explicitly_completed_wheel_releases_escape(cx:&mut TestAppContext){after_completed_wheel(cx,true);}
#[gpui::test]fn debounced_wheel_releases_escape(cx:&mut TestAppContext){after_completed_wheel(cx,false);}
#[gpui::test]fn other_window_escape_does_not_cancel_owner(cx:&mut TestAppContext){
    let(wa,va,ea)=mount(cx,"window-a");let(wb,_,eb)=mount(cx,"window-b");
    let mut a=VisualTestContext::from_window(*wa,cx);let mut b=VisualTestContext::from_window(*wb,cx);
    wheel(&mut a,gpui::TouchPhase::Started,-40.0);let zoomed=scale(&mut a,&va);assert!(zoomed>1.0);
    b.simulate_keystrokes("escape");b.run_until_parked();
    assert_eq!(eb.get(),1);assert_eq!(scale(&mut a,&va),zoomed,"other window cannot cancel owner");
    a.simulate_keystrokes("escape");a.run_until_parked();
    assert_eq!(scale(&mut a,&va),1.0);assert_eq!(ea.get(),0,"active owner consumes its cancellation");
}

#[gpui::test]fn disposing_view_releases_window_escape_owner(cx:&mut TestAppContext){
    let(w,view,escapes)=mount(cx,"dispose-owner");let mut v=VisualTestContext::from_window(*w,cx);
    wheel(&mut v,gpui::TouchPhase::Started,-40.0);assert!(scale(&mut v,&view)>1.0);
    v.update(|_,cx|view.dispose(cx)).unwrap();v.run_until_parked();
    assert_eq!(view.state(),ScriptViewState::Disposed);
    v.simulate_keystrokes("escape");v.run_until_parked();
    assert_eq!(escapes.get(),1,"disposed owner must not consume its former window's Escape");
}
