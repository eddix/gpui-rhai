// Executed against an isolated source snapshot of PR99 commit 3209039e.
use std::{cell::RefCell,collections::BTreeMap,rc::Rc};
use gpui::{Context,IntoElement,Render,TestAppContext,VisualTestContext,Window,WindowHandle};
use gpui_rhai::*;
struct Host{host:ScriptViewHost,view:ScriptViewHandle}
impl Render for Host{fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{self.host.container(self.view.element().unwrap())}}
struct Themes(bool);
impl ScriptViewExtension for Themes{
 fn configure_runtime(&self,runtime:&mut UiRuntimeState)->Result<(),String>{
  let dark=runtime.theme.as_ref().unwrap().resolve(None,None,SystemAppearance::Dark).unwrap().variant().clone();
  let mut light=dark.clone();light.name="Light".into();light.mode=ThemeMode::Light;
  let mut manager=ThemeManager::from_variants([dark,light],ThemeSelection::new("Default","Dark")).unwrap();
  if self.0{manager.set_app(ThemePreference::System{family:"Default".into()}).unwrap();}runtime.theme=Some(manager);Ok(())
 }
}
fn mount(cx:&mut TestAppContext,script:&str,name:&str,system:bool)->(WindowHandle<Host>,ScriptViewHandle){
 cx.update(gpui_rhai::install);let entry=ModuleId::parse("main").unwrap();
 let prepared=EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([(entry,script.to_owned())])),include_str!("snapshot-theme.rhai")).extension(Themes(system)).prepare().unwrap();
 let captured=Rc::new(RefCell::new(None));let capture=captured.clone();let name=name.to_owned();
 let window=cx.add_window(move|window,cx|{let host=ScriptViewHost::new(&name,cx).unwrap();let view=prepared.mount(ScriptViewConfig::new(&name),host.clone(),window,cx).unwrap();*capture.borrow_mut()=Some(view.clone());Host{host,view}});
 cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();for _ in 0..4{cx.background_executor.advance_clock(std::time::Duration::from_millis(16));cx.run_until_parked();}let view=captured.borrow().as_ref().unwrap().clone();(window,view)
}
fn state(v:&mut VisualTestContext,view:&ScriptViewHandle)->(String,String){v.update(|_,cx|{let theme=view.theme_snapshot(cx).unwrap();let text=view.accessibility_snapshot(cx).unwrap().nodes().find(|n|n.role=="status").unwrap().name.clone();(theme.variant.name.clone(),text)})}
#[gpui::test]
fn initial_system_light_matches_script_resolved_theme(cx:&mut TestAppContext){
 let(window,view)=mount(cx,r#"fn view(ctx){text(ctx.theme_variant().mode).accessibility_role("status")}"#,"pr99-light",true);
 let mut v=VisualTestContext::from_window(*window,cx);let(theme,text)=state(&mut v,&view);
 println!("Host effective theme={theme},script resolved mode={text}");assert_eq!(theme,"Light");assert_eq!(text,"light","script resolver must learn initial native Light appearance");
}
const EFFECT:&str=r#"
define_component(#{metadata:#{id:"audit/theme_probe","export":"ThemeProbe",version:"0.1.0",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{starts:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},seen:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}},events:#{},slots:#{},parts:[],effects:["observe"]},render:Fn("render_ThemeProbe")});
fn start(ctx,deps){let theme=if deps==(){ctx.theme_variant()}else{deps};ctx.set_state("starts",ctx.get_state("starts")+1);ctx.set_state("seen",theme.mode);}
fn stop(ctx,deps){()}
fn render_ThemeProbe(ctx,props){effect("observe",(),Fn("start"),Fn("stop"));text(`${ctx.get_state("starts")}:${ctx.get_state("seen")}`).accessibility_role("status")}
fn view(ctx){render_component("audit/theme_probe",#{key:"probe"})}
"#;
fn effect_case(cx:&mut TestAppContext,explicit:bool){
 let script=if explicit{EFFECT.replace("effect(\"observe\",(),","effect(\"observe\",ctx.theme_variant(),")}else{EFFECT.to_owned()};
 let(window,view)=mount(cx,&script,if explicit{"pr99-explicit-deps"}else{"pr99-body-read"},false);let mut v=VisualTestContext::from_window(*window,cx);
 let before=state(&mut v,&view);v.update(|_,cx|view.select_theme("Default","Light",cx)).unwrap();v.run_until_parked();for _ in 0..4{cx.background_executor.advance_clock(std::time::Duration::from_millis(16));v.run_until_parked();}let after=state(&mut v,&view);
 println!("explicit_deps={explicit}, before={before:?}, after={after:?}");assert_eq!(after.0,"Light");assert_eq!(after.1,"2:light");
}
#[gpui::test]
fn body_read_restarts_effect_as_pr_docs_claim(cx:&mut TestAppContext){effect_case(cx,false)}
#[gpui::test]
fn explicit_render_dependency_control(cx:&mut TestAppContext){effect_case(cx,true)}
