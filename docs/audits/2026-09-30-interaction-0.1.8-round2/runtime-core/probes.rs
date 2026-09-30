use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use gpui::{Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext,
    VisualTestContext, Window, WindowHandle, point, px};
use gpui_rhai::*;

struct Host { host: ScriptViewHost, view: ScriptViewHandle }
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let content = if self.view.state() == ScriptViewState::Active {
            self.view.element().unwrap()
        } else { gpui::div().into_any_element() };
        self.host.container(content)
    }
}
struct ReplaceExtension;
impl ScriptViewExtension for ReplaceExtension {
    fn configure_engine(&self, engine:&mut RuntimeEngine) -> Result<(),String> {
        engine.register_native_handler(NativeHandlerDescriptor::new(
            NativeHandlerId::parse("audit.replace").unwrap(),
            BTreeMap::from([("click".to_owned(),ValueSchema::UiValue)]),
        ).unwrap(), |_,runtime,_,_| {
            runtime.set_component_state_from_host(&ComponentInstancePath::root("View","round2"),
                "x",UiValue::Float(300.0)).map_err(|e|e.to_string())?;
            Ok(EventResponse::new().stop())
        }).map_err(|e|e.to_string())
    }
}
fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([
        (entry,script.to_owned()),
        (ModuleId::parse("components/pan_zoom").unwrap(),
            include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/components/pan_zoom.rhai")).to_owned()),
    ])),include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .extension(ReplaceExtension).prepare().unwrap();
    let capture = Rc::new(RefCell::new(None));
    let capture2 = capture.clone();
    let window = cx.add_window(move |window,cx| {
        let host=ScriptViewHost::new("round2-host",cx).unwrap();
        let view=prepared.mount(ScriptViewConfig::new("round2"),host.clone(),window,cx).unwrap();
        *capture2.borrow_mut()=Some(view.clone()); Host{host,view}
    });
    cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    let view=capture.borrow().as_ref().unwrap().clone();
    (window,view)
}
const SCRIPT:&str=r#"
import "components/pan_zoom" as panzoom;
fn state_schema(){#{fields:#{
 x:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
 commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn external(ctx,payload){ctx.set_state("x",300.0);}
fn changed(ctx,payload){ctx.set_state("commits",ctx.get_state("commits")+1);ctx.set_state("x",payload.x);}
fn view(ctx){column([
 panzoom::PanZoom(#{key:"pan",label:"Pan surface",transform:#{x:ctx.get_state("x"),y:0.0,scale:1.0},
    content:box([text("Canvas contents")]),on_transform_change:Fn("changed")})
    .with_style(style().width(px(400)).height(px(250))),
 text("External").test_id("external").on("click",EXTERNAL_HANDLER),
 text(`${ctx.get_state("x")},${ctx.get_state("commits")}`)
])}
"#;
fn current_error(visual:&mut VisualTestContext,view:&ScriptViewHandle)->Option<String>{
    visual.update(|_,cx|view.last_error(cx).unwrap())
}
fn text_values(node:&UiNode,output:&mut Vec<String>){
    match node.kind(){
        UiNodeKind::Text{text}=>output.push(text.to_string()),
        UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>
            children.iter().for_each(|n|text_values(n,output)),
        _=>{}
    }
}
fn texts(visual:&mut VisualTestContext,view:&ScriptViewHandle)->Vec<String>{
    visual.update(|_,cx|{
        let mut values=vec![];
        text_values(&view.root(cx).unwrap().unwrap(),&mut values);values
    })
}
fn dispatch_external(visual:&mut VisualTestContext,view:&ScriptViewHandle){
    visual.update(|window,cx|view.automate(AutomationCommand::Dispatch{
        locator:AutomationLocator::TestId{id:"external".to_owned()},event:"click".to_owned(),payload:None,
    },window,cx)).unwrap();
    visual.run_until_parked();
}
#[gpui::test]
fn idle_pan_zoom_suspend_is_safe(cx:&mut TestAppContext){
    let (window,view)=mount(cx,&SCRIPT.replace("EXTERNAL_HANDLER","Fn(\"external\")"));
    let mut visual=VisualTestContext::from_window(*window,cx);
    assert!(current_error(&mut visual,&view).is_none());
    let result=visual.update(|window,cx|view.suspend(window,cx));
    println!("idle PanZoom suspend: {result:?}, state={:?}",view.state());
    assert!(result.is_ok(),"idle PanZoom must support normal suspend without reentrant entity updates");
}
fn replace_during_pan(cx:&mut TestAppContext,native:bool){
    let handler=if native{"native_handler(\"audit.replace\")"}else{"Fn(\"external\")"};
    let (window,view)=mount(cx,&SCRIPT.replace("EXTERNAL_HANDLER",handler));
    let mut visual=VisualTestContext::from_window(*window,cx);
    visual.simulate_mouse_down(point(px(30.0),px(30.0)),MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    assert!(current_error(&mut visual,&view).is_none());
    dispatch_external(&mut visual,&view);
    let error=current_error(&mut visual,&view);
    println!("external native={native} error after replacement: {error:?}");
    assert!(error.is_none(),"source replacement must cancel old gesture without PanZoom Entity reentry");
    visual.simulate_mouse_up(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    assert!(current_error(&mut visual,&view).is_none());
    let actual=texts(&mut visual,&view);
    println!("external native={native} final content: {actual:?}");
    assert!(actual.contains(&"300.0,0".to_owned()),"replaced source must win and old gesture must not emit a new proposal");
}
#[gpui::test]
fn script_source_replacement_control(cx:&mut TestAppContext){replace_during_pan(cx,false);}
#[gpui::test]
fn native_source_replacement_is_equivalent(cx:&mut TestAppContext){replace_during_pan(cx,true);}

#[gpui::test]
fn pan_without_replacement_control(cx:&mut TestAppContext){
    let (window,view)=mount(cx,&SCRIPT.replace("EXTERNAL_HANDLER","Fn(\"external\")"));
    let mut visual=VisualTestContext::from_window(*window,cx);
    visual.simulate_mouse_down(point(px(30.0),px(30.0)),MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(point(px(80.0),px(60.0)),MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let actual=texts(&mut visual,&view);
    println!("normal pan final content: {actual:?}");
    assert!(actual.contains(&"50.0,1".to_owned()));
}
