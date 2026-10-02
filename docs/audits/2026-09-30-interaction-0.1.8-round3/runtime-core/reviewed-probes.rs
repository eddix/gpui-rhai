use std::{cell::{Cell,RefCell},collections::BTreeMap,rc::Rc,time::Instant};
use gpui::{App,Context,IntoElement,Modifiers,MouseButton,Render,TestAppContext,VisualTestContext,
    Window,WindowHandle,point,px};
use gpui_rhai::*;

struct Host{host:ScriptViewHost,view:ScriptViewHandle}
impl Render for Host{
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{
        self.host.container(if self.view.state()==ScriptViewState::Active{
            self.view.element().unwrap()
        }else{gpui::div().into_any_element()})
    }
}
struct FaultHandler(Rc<Cell<bool>>);
impl PrimitiveHandler for FaultHandler{
    fn render(&mut self,_:&PrimitiveInstance,_:&PrimitiveContext,_:&PrimitiveTheme,
        _:&mut Window,_:&mut App)->Result<gpui::AnyElement,String>{Ok(gpui::div().into_any_element())}
    fn suspend(&mut self,_:&PrimitiveInstanceId,_:&mut App){
        assert!(!self.0.replace(false),"injected late primitive suspend failure");
    }
}
struct FaultExtension(Rc<Cell<bool>>);
impl ScriptViewExtension for FaultExtension{
    fn configure_engine(&self,engine:&mut RuntimeEngine)->Result<(),String>{
        engine.register_primitive(PrimitiveDescriptor{
            id:PrimitiveId::parse("zz_audit.fault").unwrap(),export:"Fault".to_owned(),
            props:BTreeMap::new(),events:BTreeMap::new(),state:ComponentStateSchema::default(),
            lifecycle:true,effect:None,
        },FaultHandler(self.0.clone())).map_err(|e|e.to_string())
    }
}
fn mount(cx:&mut TestAppContext,script:&str,fault:Rc<Cell<bool>>)->(WindowHandle<Host>,ScriptViewHandle){
    cx.update(gpui_rhai::install);
    let entry=ModuleId::parse("main").unwrap();
    let prepared=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([
        (entry,script.to_owned()),
        (ModuleId::parse("components/draggable").unwrap(),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/draggable.rhai")).to_owned()),
        (ModuleId::parse("components/pan_zoom").unwrap(),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/pan_zoom.rhai")).to_owned()),
    ])),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .extension(FaultExtension(fault))
        .runtime_clock(ManualRuntimeClock::new(Instant::now()).clock()).prepare().unwrap();
    let capture=Rc::new(RefCell::new(None));let c=capture.clone();
    let window=cx.add_window(move |window,cx|{
        let host=ScriptViewHost::new("round3-host",cx).unwrap();
        let view=prepared.mount(ScriptViewConfig::new("round3"),host.clone(),window,cx).unwrap();
        *c.borrow_mut()=Some(view.clone());Host{host,view}
    });
    cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    let view=capture.borrow().as_ref().unwrap().clone();(window,view)
}
fn texts_of(node:&UiNode,result:&mut Vec<String>){match node.kind(){
    UiNodeKind::Text{text}=>result.push(text.to_string()),
    UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().for_each(|n|texts_of(n,result)),
    _=>{}
}}
fn texts(v:&mut VisualTestContext,view:&ScriptViewHandle)->Vec<String>{v.update(|_,cx|{
    let mut result=vec![];texts_of(&view.root(cx).unwrap().unwrap(),&mut result);result
})}
fn bounds(v:&mut VisualTestContext,view:&ScriptViewHandle,role:&str,name:&str)->AutomationBounds{
    let result=v.update(|window,cx|view.automate(AutomationCommand::Query{
        locator:AutomationLocator::RoleName{role:role.to_owned(),name:name.to_owned()},
    },window,cx)).unwrap();let AutomationResult::Node{node}=result else{panic!("node")};node.bounds.unwrap()
}
const DRAG_SCRIPT:&str=r#"
import "components/draggable" as draggable;
fn state_schema(){#{fields:#{
axes:#{schema:#{type:"string"},"default":#{type:"string",value:"horizontal"}},
last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}
}}}
fn moved(ctx,v){ctx.set_state("last",`${v.x},${v.y}`);}
fn change_axes(ctx,v){ctx.set_state("axes","vertical");}
fn failing_delivery(ctx,v){throw "independent timer failure";}
fn view(ctx){
 timeout("a-change",100,false,Fn("change_axes"),());
 FAILING_TIMER
 column([
  text(`${ctx.get_state("axes")}:${ctx.get_state("last")}`),
  draggable::Draggable(#{key:"drag",label:"Move",position:#{x:20.0,y:20.0},axes:ctx.get_state("axes"),
   handle:text("Handle").with_style(style().width(px(120)).height(px(30))),
   content:text("Body").with_style(style().width(px(120)).height(px(80))),on_move:Fn("moved")})
    .with_style(style().width(px(500)).height(px(400)))])}
"#;
fn batch_probe(cx:&mut TestAppContext,failing:bool){
    let timers=if failing{"timeout(\"z-fail\",100,false,Fn(\"failing_delivery\"),());"}else{""};
    let (window,view)=mount(cx,&DRAG_SCRIPT.replace("FAILING_TIMER",timers),Rc::new(Cell::new(false)));
    let mut v=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut v,&view,"group","Move");
    let p=point(px((b.x+20.0)as f32),px((b.y+15.0)as f32));let q=point(p.x+px(40.0),p.y);
    v.simulate_mouse_down(p,MouseButton::Left,Modifiers::default());
    v.simulate_mouse_move(q,MouseButton::Left,Modifiers::default());
    let result=v.update(|window,cx|view.automate(AutomationCommand::AdvanceTime{millis:100},window,cx));
    println!("batch failing={failing} advance={result:?}");
    assert_eq!(result.is_err(),failing);
    v.run_until_parked();
    let before=texts(&mut v,&view);println!("batch before release={before:?}");
    assert!(before.contains(&"vertical:none".to_owned()),"successful first delivery must be committed");
    v.simulate_mouse_up(q,MouseButton::Left,Modifiers::default());v.run_until_parked();
    let after=texts(&mut v,&view);println!("batch after release={after:?}");
    assert!(after.contains(&"vertical:none".to_owned()),"unrelated failed delivery must not erase committed invalidation");
}
#[gpui::test]fn single_timer_contract_control(cx:&mut TestAppContext){batch_probe(cx,false);}
#[gpui::test]fn failed_sibling_delivery_preserves_successful_invalidation(cx:&mut TestAppContext){batch_probe(cx,true);}

const PAN_SCRIPT:&str=r#"
import "components/pan_zoom" as panzoom;
fn state_schema(){#{fields:#{x:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,v){ctx.set_state("x",v.x);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([
panzoom::PanZoom(#{key:"pan",label:"Pan",transform:#{x:ctx.get_state("x"),y:0.0,scale:1.0},
 content:text("Canvas contents"),on_transform_change:Fn("changed")}).with_style(style().width(px(400)).height(px(250))),
zz_audit::Fault(#{key:"fault"}),text(`${ctx.get_state("x")},${ctx.get_state("commits")}`)
])}
"#;
fn pan_after_lifecycle(cx:&mut TestAppContext,injected:bool){
    let fail=Rc::new(Cell::new(injected));let(window,view)=mount(cx,PAN_SCRIPT,fail);
    let mut v=VisualTestContext::from_window(*window,cx);
    let result=v.update(|window,cx|view.suspend(window,cx));
    println!("suspend injected={injected}: {result:?}; state={:?}",view.state());
    if injected{assert!(result.is_err());assert_eq!(view.state(),ScriptViewState::Active);
        assert!(v.update(|window,cx|view.suspend(window,cx)).unwrap());
    }else{assert!(result.unwrap());}
    assert!(v.update(|_,cx|view.resume(cx)).unwrap());v.run_until_parked();
    // The Host conditionally removes suspended views; explicitly redraw its
    // composition when reattaching the resumed view before testing input.
    window.update(cx, |_, _, host_cx| host_cx.notify()).unwrap();
    cx.refresh().unwrap();v.run_until_parked();
    v.simulate_mouse_down(point(px(30.0),px(30.0)),MouseButton::Left,Modifiers::default());
    v.simulate_mouse_move(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());
    v.simulate_mouse_up(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());v.run_until_parked();
    let error=v.update(|_,cx|view.last_error(cx).unwrap());let actual=texts(&mut v,&view);
    println!("post lifecycle injected={injected}: {actual:?}, error={error:?}");
    assert!(error.is_none());assert!(actual.contains(&"50.0,1".to_owned()));
}
#[gpui::test]fn pan_zoom_suspend_resume_then_interact(cx:&mut TestAppContext){pan_after_lifecycle(cx,false);}
#[gpui::test]fn pan_zoom_compensate_retry_resume_then_interact(cx:&mut TestAppContext){pan_after_lifecycle(cx,true);}
