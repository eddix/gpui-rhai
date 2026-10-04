use std::{cell::Cell,rc::Rc};
use gpui::{Context,IntoElement,KeyDownEvent,Keystroke,Modifiers,Render,TestAppContext,VisualTestContext,Window};
use gpui_rhai::{GpuiNodeRenderer,UiNode,HostCallback,EventResponse};
include!("pr97_validation.rs");
struct KeyHost{node:UiNode}
impl Render for KeyHost{
    fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{
        GpuiNodeRenderer::render(&self.node)
    }
}
fn deliver(cx:&mut TestAppContext,key:&str,events:Vec<Keystroke>)->usize{
    let count=Rc::new(Cell::new(0));let c=count.clone();
    let node=UiNode::text("Focused key target").with_handler(format!("key:{key}"),
        HostCallback::new("key-probe",move|_,_,_|{c.set(c.get()+1);EventResponse::new().stop()}));
    let w=cx.add_window(move|_,_|KeyHost{node});cx.run_until_parked();cx.refresh().unwrap();
    let mut v=VisualTestContext::from_window(*w,cx);v.update(|window,cx|window.focus_next(cx));
    for keystroke in events{v.simulate_event(KeyDownEvent{keystroke,is_held:false,prefer_character_input:false});v.run_until_parked();}
    count.get()
}
#[test]fn pr97_validators_should_reject_chord_and_multi_symbol_names(){
    for value in ["cmd-s","shift-/","??"]{
        println!("PR97 name {value:?}: sugar={}, generic-key={}",is_valid_key_handler_name(value),is_printable_key_character(value));
    }
    assert!(!is_valid_key_handler_name("cmd-s"),"node key names do not represent chords");
}
#[gpui::test]fn literal_question_mark_dispatches(cx:&mut TestAppContext){
    let count=deliver(cx,"?",vec![Keystroke::parse("?").unwrap()]);
    println!("key=? delivered={count}");assert_eq!(count,1);
}
#[gpui::test]fn uppercase_accepted_by_pr_does_not_dispatch(cx:&mut TestAppContext){
    assert!(is_printable_key_character("Escape"),"PR generic key path now accepts this spelling");
    let upper=deliver(cx,"Escape",vec![Keystroke::parse("escape").unwrap()]);
    let lower=deliver(cx,"escape",vec![Keystroke::parse("escape").unwrap()]);
    println!("generic key:Escape delivered={upper}; key:escape delivered={lower}");
    assert_eq!(lower,1);assert_eq!(upper,1,"accepted key spelling should be normalized or rejected, not silently inert");
}
#[gpui::test]fn characterize_raw_key_and_modifiers(cx:&mut TestAppContext){
    let modified=deliver(cx,"/",vec![Keystroke{key:"/".to_owned(),key_char:None,
        modifiers:Modifiers{control:true,..Default::default()}}]);
    let character_only=deliver(cx,"?",vec![Keystroke{key:"q".to_owned(),key_char:Some("?".to_owned()),
        modifiers:Modifiers{alt:true,..Default::default()}}]);
    let shifted_slash=deliver(cx,"?",vec![Keystroke::parse("shift-/").unwrap()]);
    println!("ctrl-/ -> slash count={modified}; key=q,key_char=? -> question count={character_only}; synthetic shift-/ -> question count={shifted_slash}");
    assert_eq!(modified,1);assert_eq!(character_only,0);assert_eq!(shifted_slash,0);
}
