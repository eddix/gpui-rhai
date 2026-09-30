#![cfg(test)]
#![allow(unused_imports)]
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use gpui::{Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext, Window, WindowHandle, point, px};
use gpui_rhai::*;
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../..");

struct Host { host: ScriptViewHost, view: ScriptViewHandle }
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), script.to_owned())]);
    for name in ["sortable", "drag_source", "drop_zone", "scroll_area", "dialog"] {
        modules.insert(ModuleId::parse(format!("components/{name}")).unwrap(),
            std::fs::read_to_string(format!("{ROOT}/registry/components/{name}.rhai")).unwrap());
    }
    modules.insert(ModuleId::parse("components/audit_panel").unwrap(), r#"
import "components/sortable" as sortable;
define_component(#{
    metadata:#{id:"components/audit_panel","export":"AuditPanel",version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        prefix:#{schema:#{type:"string"},required:true,sensitive:false},
        on_reorder:#{schema:#{type:"callback"},required:true,sensitive:false}
    },state:#{fields:#{}},events:#{reorder:#{payload:#{type:"ui_value"}}},slots:#{},parts:[]},
    render:Fn("render_panel")
});
fn AuditPanel(props){render_component("components/audit_panel",props)}
fn reordered(ctx,value){ctx.emit("reorder",value);}
fn render_panel(ctx,props){let items=[];for suffix in ["a","b"]{let key=`${props.prefix}-${suffix}`;
    items.push(#{key:key,label:key,content:text(key).with_style(style().height(px(48)).padding(px(8)))});}
    box([sortable::Sortable(#{key:"queue",label:props.prefix,items:items,on_reorder:Fn("reordered")})]).with_key(props.key)}
"#.to_owned());
    let theme = std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap();
    let prepared = EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), &theme)
        .motion_preference(MotionPreference::None).prepare().unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("audit-drag-sort", cx).unwrap();
        let view = prepared.mount(ScriptViewConfig::new("audit-drag-sort"), host.clone(), window, cx).unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host {host, view}
    });
    cx.run_until_parked(); cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}
fn geometry(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> GeometryBounds {
    visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap().find_by_role_and_name(role,name).next().unwrap().geometry.unwrap().visual)
}
fn center(bounds: GeometryBounds) -> gpui::Point<gpui::Pixels> {
    point(px((bounds.x+bounds.width/2.0) as f32), px((bounds.y+bounds.height/2.0) as f32))
}
fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap().nodes().find(|node| node.role=="status").unwrap().name.clone())
}
fn drag(visual: &mut VisualTestContext, source: gpui::Point<gpui::Pixels>, target: gpui::Point<gpui::Pixels>) {
    visual.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
}

fn virtual_scroll_item(visual:&mut VisualTestContext,view:&ScriptViewHandle)->usize {
    visual.update(|_,cx|view.take_performance_snapshot(cx).unwrap().virtual_collections.first().unwrap().scroll_item)
}

#[gpui::test]
fn external_typed_drag_auto_scrolls_virtual_destination(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",value.target_id);}
fn row_target(ctx,payload){drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,
    payload_types:["card"],operations:["move"],on_drop:Fn("dropped"),
    content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        drag_source::DragSource(#{key:"source",label:"External source",source_id:"card",payload_type:"card",payload:#{id:"card"},
            content:text("External card").with_style(style().width(px(150)).height(px(40)))}),
        virtual_collection(#{key:"targets",label:"Virtual lanes",data:data,height:180.0,estimated_height:40.0},Fn("row_target"))
            .accessibility_role("list").accessibility_label("Virtual lanes")
    ]).with_style(style().width(px(280)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","External source"));
    let list=geometry(&mut visual,&view,"list","Virtual lanes");
    let snap=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    for node in snap.nodes().filter(|node|node.role=="group") {eprintln!("virtual lane {} {:?}",node.name,node.geometry);}
    let bottom=point(px((list.x+list.width/2.0)as f32),px((list.y+list.height-5.0)as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    for i in 0..30 {
        visual.simulate_mouse_move(point(bottom.x+px((i%2)as f32),bottom.y),MouseButton::Left,Modifiers::default());
        visual.run_until_parked();
    }
    let scroll=virtual_scroll_item(&mut visual,&view);
    visual.simulate_mouse_up(bottom,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    let result=status(&mut visual,&view);
    eprintln!("external drag virtual destination scroll_item={scroll}, drop={result}, source={source:?}, bottom={bottom:?}, error={:?}",visual.update(|_,cx|view.last_error(cx).unwrap()));
    assert!(result.starts_with("lane-"),"positive control: typed drag/drop is valid");
    assert!(scroll>0,"auto-scroll must follow destination, not require source collection membership");
}

const VIRTUAL_SCRIPT:&str=r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"order",label:"Order",virtual_data:data,height:180.0,on_reorder:Fn("reordered")})])
        .with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;

#[gpui::test]
fn escape_cancels_while_virtual_source_is_still_visible(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window,view)=mount(cx,VIRTUAL_SCRIPT);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    let middle=center(geometry(&mut visual,&view,"list","Order"));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(middle,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    let focused=visual.update(|window,cx|window.focused(cx).is_some());
    visual.simulate_keystrokes("escape");visual.run_until_parked();
    visual.simulate_mouse_up(middle,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    let result=status(&mut visual,&view);eprintln!("visible source Escape focus={focused}, result={result}");
    assert!(focused,"control: initial drag has focus");
    assert_eq!(result,"ready");
}

#[gpui::test]
fn stationary_pointer_scrolls_and_escape_releases_scroll(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window,view)=mount(cx,VIRTUAL_SCRIPT);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    let list=geometry(&mut visual,&view,"list","Order");
    let bottom=point(px((list.x+list.width/2.0)as f32),px((list.y+list.height-5.0)as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(bottom,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let start=virtual_scroll_item(&mut visual,&view);
    for _ in 0..12 {
        cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let scrolled=virtual_scroll_item(&mut visual,&view);
    let before_escape_focus=visual.update(|window,cx|window.focused(cx).is_some());
    visual.simulate_keystrokes("escape");visual.run_until_parked();
    let cancelled=virtual_scroll_item(&mut visual,&view);
    for _ in 0..12 {
        cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let final_scroll=virtual_scroll_item(&mut visual,&view);
    visual.simulate_mouse_up(bottom,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    eprintln!("stationary scroll start={start}, later={scrolled}, focus before Escape={before_escape_focus}, escaped={cancelled}, settled={final_scroll}, commit={}",status(&mut visual,&view));
    assert!(scrolled>start || scrolled>=95,"stationary pointer should keep scrolling before end");
    assert_eq!(cancelled,final_scroll,"cancelled virtual auto-scroll must stop");
    assert_eq!(status(&mut visual,&view),"ready");
}

fn collection_variant(variant:&str)->NativeCollection {
    let mut rows=(0..100).map(|i|BTreeMap::from([
        ("key".to_owned(),UiValue::String(format!("item-{i}"))),
        ("label".to_owned(),UiValue::String(format!("Item {i}"))),
        ("disabled".to_owned(),UiValue::Bool(false)),
    ])).collect::<Vec<_>>();
    match variant {
        "deleted"=>{rows.remove(0);},
        "reordered"=>{rows.swap(0,50);},
        "disabled"=>{rows[0].insert("disabled".to_owned(),UiValue::Bool(true));},
        "replaced"=>{rows[0].insert("label".to_owned(),UiValue::String("Replacement".to_owned()));},
        _=>{},
    }
    NativeCollection::new("key",rows).unwrap()
}

#[gpui::test]
fn changed_offscreen_source_revokes_logical_drag_lease(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=VIRTUAL_SCRIPT.replace("let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}",
        "let data=[];try{data=ctx.get_native_collection(\"rows\");}catch(error){}");
    for variant in ["deleted","reordered","disabled","replaced"] {
        let (window,view)=mount(cx,&script);let mut visual=VisualTestContext::from_window(*window,cx);
        visual.update(|_,cx|view.register_native_collection("rows",collection_variant("initial"),cx).unwrap());
        for _ in 0..4 {cx.background_executor.advance_clock(std::time::Duration::from_millis(16));visual.run_until_parked();}
        let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
        let list=geometry(&mut visual,&view,"list","Order");let middle=center(list);
        visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
        visual.simulate_mouse_move(middle,MouseButton::Left,Modifiers::default());visual.run_until_parked();
        visual.simulate_event(gpui::ScrollWheelEvent{position:middle,delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-400.0))),..Default::default()});
        visual.run_until_parked();
        visual.update(|_,cx|view.replace_native_collection("rows",collection_variant(variant),cx).unwrap());
        for _ in 0..4 {cx.background_executor.advance_clock(std::time::Duration::from_millis(16));visual.run_until_parked();}
        visual.simulate_mouse_move(middle,MouseButton::Left,Modifiers::default());
        visual.simulate_mouse_up(middle,MouseButton::Left,Modifiers::default());visual.run_until_parked();
        let result=status(&mut visual,&view);eprintln!("source {variant}: {result}");
        assert_eq!(result,"ready","business update must revoke old source lease ({variant})");
    }
}
