#![allow(dead_code)]
// Real geometry and input regressions from the independent PR101 review.
use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
struct Rows;
impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        let rows = (0..1000).map(|i| {
            BTreeMap::from([
                ("id".into(), UiValue::String(format!("row-{i}"))),
                ("a".into(), UiValue::String("Alpha body".into())),
                ("b".into(), UiValue::String("Beta body".into())),
                (
                    "c".into(),
                    UiValue::String("Wide status value with many characters".into()),
                ),
            ])
        });
        r.native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|e| e.to_string())
    }
}
fn script(rtl: bool, native: bool, single: bool) -> String {
    let rows = if native {
        "ctx.get_native_collection(\"rows\")"
    } else {
        "[#{id:\"r\",a:\"Alpha body\",b:\"Beta body\",c:\"Wide status value with many characters\"}]"
    };
    let cols = if single {
        r#"[#{key:"c",title:"C",width:#{kind:"fixed",value:w.c},sortable:true,max_width:400}]"#
    } else {
        r#"[#{key:"a",title:"A",width:#{kind:"fixed",value:w.a},sortable:true,max_width:400},#{key:"b",title:"B",width:#{kind:"fixed",value:w.b},sortable:true,max_width:400},#{key:"c",title:"C",width:#{kind:"fixed",value:w.c},sortable:true,max_width:400}]"#
    };
    format!(
        r#"
import "components/table" as table;
fn state_schema(){{#{{fields:#{{widths:#{{schema:#{{type:"map",values:#{{type:"number"}}}},"default":#{{type:"map",value:#{{a:#{{type:"float",value:120.0}},b:#{{type:"float",value:140.0}},c:#{{type:"float",value:160.0}}}}}}}},event:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}},sorted:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}}}}}}}}
fn init(ctx){{ctx.set_locale("{}");}}
fn resized(ctx,p){{let w=ctx.get_state("widths");w[p.key]=p.width.value;ctx.set_state("widths",w);ctx.set_state("event",`${{p.key}}:${{p.width.value}}`);}}
fn sorted(ctx,p){{ctx.set_state("sorted",p.key);}}
fn view(ctx){{let w=ctx.get_state("widths");column([text(`${{ctx.get_state("event")}}|${{ctx.get_state("sorted")}}`).accessibility_role("status"),table::Table(#{{key:"table",label:"Table",row_key:"id",rows:{rows},columns:{cols},height:180.0,resizable_columns:true,on_column_resize:Fn("resized"),on_sort_change:Fn("sorted")}})]).with_style(style().width(px(600)))}}
"#,
        if rtl { "ar" } else { "en" }
    )
}
fn mount(
    cx: &mut TestAppContext,
    script: String,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script),
            (
                ModuleId::parse("components/table").unwrap(),
                include_str!("../../../../registry/components/table.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                include_str!("../../../../registry/components/badge.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../registry/themes/default_dark.rhai"),
    )
    .locale_sources([
        (
            "en.rhai".into(),
            include_str!("../../../../registry/locales/en.rhai").to_owned(),
        ),
        (
            "ar.rhai".into(),
            include_str!("../../../../registry/locales/ar.rhai").to_owned(),
        ),
    ])
    .asset_sources([
        (
            "icons/disclosure_down".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../../registry/assets/icons/disclosure_down.svg")
                    .to_vec(),
            },
        ),
        (
            "icons/chevron_right".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../../registry/assets/icons/chevron_right.svg").to_vec(),
            },
        ),
        (
            "icons/sort_ascending".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../../registry/assets/icons/sort_ascending.svg").to_vec(),
            },
        ),
        (
            "icons/sort_descending".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: include_bytes!("../../../../registry/assets/icons/sort_descending.svg")
                    .to_vec(),
            },
        ),
    ])
    .extension(Rows)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}
fn bounds(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    })
}
fn status(v: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.role == "status")
            .unwrap()
            .name
            .clone()
    })
}
fn drag(v: &mut VisualTestContext, p: gpui::Point<gpui::Pixels>, dx: f32) {
    let q = point(p.x + px(dx), p.y);
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_move(q, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_up(q, MouseButton::Left, Modifiers::none());
    v.run_until_parked();
}
fn state_script(rtl:bool,fill:bool)->String{
 let mut source=script(rtl,true,false)
 .replace("fields:#{widths:","fields:#{mode:#{schema:#{type:\"integer\"},\"default\":#{type:\"integer\",value:0}},widths:")
 .replace("fn view(ctx)","fn set_mode(ctx,v){ctx.set_state(\"mode\",v);ctx.set_state(\"sorted\",\"none\");}\nfn view(ctx)")
  .replace("let w=ctx.get_state(\"widths\");column([", "let w=ctx.get_state(\"widths\");let mode=ctx.get_state(\"mode\");column([row([text(\"Data\").accessibility_role(\"button\").accessibility_label(\"Mode0\").on_click_value(Fn(\"set_mode\"),0),text(\"Loading\").accessibility_role(\"button\").accessibility_label(\"Mode1\").on_click_value(Fn(\"set_mode\"),1),text(\"Empty\").accessibility_role(\"button\").accessibility_label(\"Mode2\").on_click_value(Fn(\"set_mode\"),2),text(\"Filtered\").accessibility_role(\"button\").accessibility_label(\"Mode3\").on_click_value(Fn(\"set_mode\"),3)]).with_style(style().height(px(24))),")
 .replace("rows:ctx.get_native_collection(\"rows\")","loading:mode==1,query:if mode==3{\"unmatched\"}else{\"\"},search_fields:[\"a\",\"b\",\"c\"],rows:if mode==2{[]}else{ctx.get_native_collection(\"rows\")}")
 .replace("style().width(px(600))","style().width(px(300))");
 if fill{source=source.replace("height:180.0,resizable_columns","fill_height:true,resizable_columns").replace("style().width(px(300))","style().width(px(300)).height(px(360))");}
 source
}
fn command(v:&mut VisualTestContext,view:&ScriptViewHandle,role:&str,name:&str,event:&str){
 v.update(|window,cx|view.automate(AutomationCommand::Dispatch{locator:AutomationLocator::RoleName{role:role.into(),name:name.into()},event:event.into(),payload:None},window,cx)).unwrap();v.run_until_parked();
}
fn click_at(v:&mut VisualTestContext,p:gpui::Point<gpui::Pixels>){v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();}
fn horizontal_wheel(v:&mut VisualTestContext,p:gpui::Point<gpui::Pixels>,delta:f32){
 v.simulate_mouse_move(p,None,Modifiers::none());v.simulate_event(gpui::ScrollWheelEvent{position:p,delta:gpui::ScrollDelta::Pixels(point(px(delta),px(0.0))),touch_phase:gpui::TouchPhase::Moved,modifiers:Modifiers::none()});v.run_until_parked();v.update(|window,_|window.refresh());v.run_until_parked();
}
#[gpui::test]
fn round7_live_states_preserve_height_scroll_and_controlled_width(cx:&mut TestAppContext){
 let(window,view)=mount(cx,state_script(false,false),"round7-live-states");let mut v=VisualTestContext::from_window(*window,cx);
 command(&mut v,&view,"separator","Resize A column","key:right");assert_eq!(status(&mut v,&view),"a:128.0|none");let initial=bounds(&mut v,&view,"table","Table");
 let direct=v.update(|_,cx|{let tree=view.accessibility_snapshot(cx).unwrap();let table=tree.find_by_role_and_name("table","Table").next().unwrap();tree.nodes().filter(|n|n.parent==Some(table.id)).map(|n|(n.role.clone(),n.geometry.map(|g|g.layout))).collect::<Vec<_>>()});println!("scroll root direct semantic children={direct:?}");
 let a=bounds(&mut v,&view,"columnheader","A");horizontal_wheel(&mut v,point(px((initial.x+180.0)as f32),px((initial.y+15.0)as f32)),-80.0);let scrolled=bounds(&mut v,&view,"columnheader","A");println!("initial horizontal scroll A.x {} -> {}",a.x,scrolled.x);let scroll_moved=scrolled.x<a.x-50.0;
 for mode in [1,2,3,0]{
  command(&mut v,&view,"button",&format!("Mode{mode}"),"click");let table=bounds(&mut v,&view,"table","Table");let a=bounds(&mut v,&view,"columnheader","A");let c=bounds(&mut v,&view,"columnheader","C");println!("mode={mode} table={table:?}, A={a:?}, C={c:?}");assert!((table.height-initial.height).abs()<0.5);assert!((a.width-128.0).abs()<0.5);assert!((a.x-scrolled.x).abs()<0.5,"state change must not lose the shared viewport scroll");
  let outside=point(px((c.x+c.width-12.0)as f32),px((c.y+c.height/2.0)as f32));assert!(f64::from(outside.x)>table.x+table.width);click_at(&mut v,outside);assert_eq!(status(&mut v,&view),"a:128.0|none","out-of-viewport header must not sort");
  click_at(&mut v,point(px((c.x+16.0)as f32),outside.y));assert_eq!(status(&mut v,&view),"a:128.0|c","visible header remains interactive");
 }
 assert!(scroll_moved,"wide Table must expose a usable horizontal scroll range");
}
#[gpui::test]
fn round7_fill_height_live_transitions_keep_viewport(cx:&mut TestAppContext){
 let(window,view)=mount(cx,state_script(false,true),"round7-fill-states");let mut v=VisualTestContext::from_window(*window,cx);let initial=bounds(&mut v,&view,"table","Table");assert!(initial.height>250.0,"fixture must exercise remaining-height growth");
 for mode in [1,2,3,0]{command(&mut v,&view,"button",&format!("Mode{mode}"),"click");let table=bounds(&mut v,&view,"table","Table");println!("fill mode={mode}: initial={},actual={}",initial.height,table.height);assert!((table.height-initial.height).abs()<0.5);}
}
#[gpui::test]
fn round7_rtl_shared_viewport_can_reveal_overflowing_last_column(cx:&mut TestAppContext){
 let(window,view)=mount(cx,state_script(true,false),"round7-rtl-overflow");let mut v=VisualTestContext::from_window(*window,cx);
 let mut all_revealed=true;
 for mode in [0,1,2,3]{if mode!=0{command(&mut v,&view,"button",&format!("Mode{mode}"),"click");}let table=bounds(&mut v,&view,"table","Table");let before=bounds(&mut v,&view,"columnheader","C");let p=point(px((table.x+100.0)as f32),px((table.y+15.0)as f32));
  horizontal_wheel(&mut v,p,160.0);let positive=bounds(&mut v,&view,"columnheader","C");horizontal_wheel(&mut v,p,-160.0);let negative=bounds(&mut v,&view,"columnheader","C");println!("RTL mode={mode}: root.x={}, C before.x={},after+160.x={},after-160.x={}",table.x,before.x,positive.x,negative.x);all_revealed&=positive.x>=table.x-0.5||negative.x>=table.x-0.5;
 }
 assert!(all_revealed,"logical end content must be reachable by horizontal scrolling in every state");
}
#[gpui::test]
fn round7_wheel_driver_control_scrolls_a_wide_direct_child(cx:&mut TestAppContext){
 let source=r#"fn view(ctx){box([text("Marker").with_style(style().width(px(500)).height(px(40)))]).with_key("scroll").with_style(style().width(px(300)).height(px(80)).overflow_x_scroll())}"#.to_owned();
 let(window,view)=mount(cx,source,"round7-wheel-control");let mut v=VisualTestContext::from_window(*window,cx);let before=bounds(&mut v,&view,"text","Marker");horizontal_wheel(&mut v,point(px(150.0),px(20.0)),-80.0);let after=bounds(&mut v,&view,"text","Marker");println!("wheel driver positive control: before.x={},after.x={}",before.x,after.x);assert!((after.x-before.x+80.0).abs()<0.5);
}
