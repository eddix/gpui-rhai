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

#[path="round7.rs"] mod round7_replay;
fn followup(v:&mut VisualTestContext){
 v.executor().advance_clock(std::time::Duration::from_millis(32));v.run_until_parked();
 for _ in 0..3 {v.update(|window,cx|window.simulate_next_frame(cx));v.run_until_parked();}
}
fn width_pair(v:&mut VisualTestContext,view:&ScriptViewHandle,title:&str,text:&str)->(f64,f64){
 (bounds(v,view,"columnheader",title).width,bounds(v,view,"gridcell",text).width)
}
#[gpui::test]
fn middle_percentage_native_resize_remains_fixed_after_viewport_and_state_changes(cx:&mut TestAppContext){
 for (rtl,native) in [(false,false),(true,true)] {
  let mut source=state_script(rtl,false)
    .replace("fields:#{mode:","fields:#{viewport:#{schema:#{type:\"number\"},\"default\":#{type:\"float\",value:500.0}},mode:")
    .replace("fn set_mode(ctx,v)","fn grow(ctx,v){ctx.set_state(\"viewport\",700.0);}\nfn set_mode(ctx,v)")
    .replace("row([text(\"Data\")", "row([text(\"Grow\").accessibility_role(\"button\").accessibility_label(\"Grow\").on_click(Fn(\"grow\")),text(\"Data\")")
    .replace("key:\"b\",title:\"B\",width:#{kind:\"fixed\",value:w.b}","key:\"b\",title:\"B\",width:#{kind:\"percent\",value:50.0}")
    .replace("key:\"c\",title:\"C\",width:#{kind:\"fixed\",value:w.c}","key:\"c\",title:\"C\",width:#{kind:\"flex\",value:1.0}")
    .replace(",on_column_resize:Fn(\"resized\")","")
    .replace("style().width(px(300))","style().width(px(ctx.get_state(\"viewport\")))");
  if !native {source=source.replace("ctx.get_native_collection(\"rows\")","[#{id:\"r\",a:\"Alpha body\",b:\"Beta body\",c:\"Wide status value with many characters\"}]");}
  let(window,view)=mount(cx,source,&format!("r8-mid-percent-{rtl}-{native}"));let mut v=VisualTestContext::from_window(*window,cx);followup(&mut v);
  let before=width_pair(&mut v,&view,"B","Beta body");assert!((before.0-249.0).abs()<0.6&&(before.0-before.1).abs()<0.6);
  let h=bounds(&mut v,&view,"separator","Resize B column");let p=point(px((h.x+h.width/2.0)as f32),px((h.y+h.height/2.0)as f32));let q=point(p.x+px(if rtl{-40.0}else{40.0}),p.y);
  v.update(|_,cx|view.take_performance_snapshot(cx).unwrap());v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
  let preview=width_pair(&mut v,&view,"B","Beta body");assert!((preview.0-289.0).abs()<0.6&&(preview.0-preview.1).abs()<0.6);let ops=v.update(|_,cx|view.take_performance_snapshot(cx).unwrap().timings.iter().map(|t|t.operations).sum::<u64>());assert_eq!(ops,0);
  v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();command(&mut v,&view,"button","Grow","click");followup(&mut v);
  let after=width_pair(&mut v,&view,"B","Beta body");let c=width_pair(&mut v,&view,"C","Wide status value with many characters");println!("middle percent rtl={rtl} native={native}: before={before:?},preview={preview:?},after={after:?},flex={c:?}");assert!((after.0-289.0).abs()<0.6&&(after.0-after.1).abs()<0.6);assert!((c.0-289.0).abs()<0.6&&(c.0-c.1).abs()<0.6);
  for mode in [1,2,3,0]{command(&mut v,&view,"button",&format!("Mode{mode}"),"click");followup(&mut v);let b=bounds(&mut v,&view,"columnheader","B").width;let c=bounds(&mut v,&view,"columnheader","C").width;assert!((b-289.0).abs()<0.6&&(c-289.0).abs()<0.6);}
 }
}
#[gpui::test]
fn changing_direction_preserves_distance_from_logical_start(cx:&mut TestAppContext){
 let mut outcomes=Vec::new();
 for rtl in [false,true]{let(window,view)=mount(cx,state_script(rtl,false),&format!("r8-direction-{rtl}"));let mut v=VisualTestContext::from_window(*window,cx);followup(&mut v);let table=bounds(&mut v,&view,"table","Table");horizontal_wheel(&mut v,point(px((table.x+150.0)as f32),px((table.y+15.0)as f32)),if rtl{80.0}else{-80.0});let a=bounds(&mut v,&view,"columnheader","A");let before=if rtl{a.x+a.width-(table.x+table.width-1.0)}else{table.x+1.0-a.x};assert!((before-80.0).abs()<0.6);
  v.update(|_,cx|view.select_locale(if rtl{"en"}else{"ar"},cx)).unwrap();followup(&mut v);let a=bounds(&mut v,&view,"columnheader","A");let after=if !rtl{a.x+a.width-(table.x+table.width-1.0)}else{table.x+1.0-a.x};println!("locale direction initial_rtl={rtl}: logical before={before},after={after}");outcomes.push((rtl,before,after));}
 assert!(outcomes.iter().all(|(_,before,after)|(before-after).abs()<0.6),"direction transitions must use one symmetric logical-offset policy: {outcomes:?}");
}
