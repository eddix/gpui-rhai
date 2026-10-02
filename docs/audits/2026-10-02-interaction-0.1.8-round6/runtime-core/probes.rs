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
const SOURCE:&str=r#"
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn requested(ctx,payload){ctx.set_state("status","requested");}
fn init(ctx){ctx.set_close_handler(Fn("requested"));}
fn close(ctx,payload){ctx.close_window(ctx.window_id());}
fn view(ctx){column([text(ctx.get_state("status")).accessibility_role("status"),
 text("Close").test_id("close").on_click(Fn("close"))])}
"#;
fn prepared()->PreparedScriptView{
    let entry=ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(entry.clone(),EmbeddedScriptSource::new(BTreeMap::from([(entry,SOURCE.to_owned())])),
        include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"),"/registry/themes/default_dark.rhai")))
        .prepare().unwrap()
}
fn queue_close(view:&ScriptViewHandle,window:&mut Window,cx:&mut gpui::App){
    view.automate(AutomationCommand::Dispatch{locator:AutomationLocator::TestId{id:"close".into()},
        event:"click".into(),payload:None},window,cx).unwrap();
}
fn mounted(cx:&mut TestAppContext)->gpui::WindowHandle<Host>{
    cx.update(gpui_rhai::install);
    let w=cx.add_window(|window,cx|{
        let host=ScriptViewHost::new("domain",cx).unwrap();
        let view=prepared().mount_window(ScriptViewConfig::new("owner"),host.clone(),window,cx).unwrap();
        Host{host,views:vec![view]}
    });
    cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();w
}
fn queued_close_after_replacement(cx:&mut TestAppContext,explicit_dispose:bool){
    let w=mounted(cx);
    let replacement=w.update(cx,|root,window,cx|{
        // There is exactly one public Handle, held in this rendered Host.
        let owner=root.views.pop().unwrap();
        queue_close(&owner,window,cx);
        if explicit_dispose{owner.dispose(cx).unwrap();}
        drop(owner);
        let new_host=ScriptViewHost::new("replacement-domain",cx).unwrap();
        let replacement=prepared().mount_window(ScriptViewConfig::new("replacement"),new_host.clone(),window,cx).unwrap();
        root.host=new_host;root.views.push(replacement.clone());cx.notify();replacement
    }).unwrap();
    cx.run_until_parked();
    let remains=cx.windows().contains(&*w);
    println!("explicit_dispose={explicit_dispose}: window_remains={remains}, replacement_state={:?}",replacement.state());
    assert!(remains,"revoked owner must not close a window that has been delegated to a replacement Host");
    assert_eq!(replacement.state(),ScriptViewState::Active);
    let mut visual=VisualTestContext::from_window(*w,cx);
    assert!(!visual.simulate_close(),"replacement close confirmation must remain installed");
}
#[gpui::test]fn explicit_dispose_revokes_queued_authority_control(cx:&mut TestAppContext){queued_close_after_replacement(cx,true);}
#[gpui::test]fn dropping_last_handle_revokes_queued_authority(cx:&mut TestAppContext){queued_close_after_replacement(cx,false);}

#[gpui::test]fn late_old_view_cleanup_cannot_unregister_same_id_replacement(cx:&mut TestAppContext){
    let w=mounted(cx);
    let replacement=w.update(cx,|root,window,cx|{
        let old=root.views.pop().unwrap();drop(old);
        let replacement=prepared().mount_window(ScriptViewConfig::new("owner"),root.host.clone(),window,cx).unwrap();
        root.views.push(replacement.clone());cx.notify();replacement
    }).unwrap();
    cx.run_until_parked();cx.refresh().unwrap();cx.run_until_parked();
    let result=w.update(cx,|root,window,cx|prepared().mount_window(
        ScriptViewConfig::new("intruder"),root.host.clone(),window,cx)).unwrap();
    println!("late old rendered view cleanup: replacement_state={:?}, competing_owner_result={:?}",replacement.state(),result.as_ref().map(|_|"granted").map_err(ToString::to_string));
    assert!(matches!(result,Err(ScriptViewError::WindowCommandOwner{..})),"late cleanup for old same-id mount must not release replacement's ownership lease");
    assert_eq!(replacement.state(),ScriptViewState::Active);
}
