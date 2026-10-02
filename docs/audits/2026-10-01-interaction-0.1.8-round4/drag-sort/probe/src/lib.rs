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

fn realized_indices(node:&UiNode)->Option<std::collections::BTreeSet<usize>> {
    match node.kind() {
        UiNodeKind::VirtualCollection{spec}=>Some(spec.realized.keys().copied().collect()),
        UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(realized_indices),
        _=>None,
    }
}

fn occluded_unrelated_virtual_list_does_not_scroll(cx:&mut TestAppContext,accept:bool) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn ended(ctx,value){ctx.set_state("status",if value.accepted{value.target_id}else{"rejected"});}
fn lane(ctx,payload){drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,
    payload_types:["card"],operations:["move"],content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    box([
        text(ctx.get_state("status")).accessibility_role("status").with_style(style().absolute().top(px(220))),
        drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
            on_drag_end:Fn("ended"),content:text("Card").with_style(style().width(px(150)).height(px(40)))})
            .with_style(style().absolute().left(px(0)).top(px(30))),
        virtual_collection(#{key:"lanes",label:"Hidden lanes",data:data,height:180.0,estimated_height:40.0},Fn("lane"))
            .accessibility_role("list").accessibility_label("Hidden lanes")
            .with_style(style().absolute().left(px(220)).top(px(0)).width(px(240)).height(px(180))),
        drop_zone::DropZone(#{key:"cover",label:"Foreground target",target_id:"cover",payload_types:["ACCEPTED_TYPE"],operations:["move"],
            content:text("Foreground").with_style(style().width(px(240)).height(px(180)).background(theme_color("surface")))})
            .with_style(style().absolute().left(px(220)).top(px(0)).width(px(240)).height(px(180)).occlude())
    ]).with_style(style().relative().width(px(500)).height(px(280)))}
"#.replace("ACCEPTED_TYPE",if accept{"card"}else{"other"});
    let (window,view)=mount(cx,&script);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let list=geometry(&mut visual,&view,"list","Hidden lanes");
    let bottom=point(px((list.x+list.width/2.0)as f32),px((list.y+list.height-5.0)as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    for i in 0..30 {
        visual.simulate_mouse_move(point(bottom.x+px((i%2)as f32),bottom.y),MouseButton::Left,Modifiers::default());
        visual.run_until_parked();
    }
    let scroll=virtual_scroll_item(&mut visual,&view);
    visual.simulate_mouse_up(bottom,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    let result=status(&mut visual,&view);let expected=if accept{"cover"}else{"rejected"};
    eprintln!("foreground accepts={accept}, native drop={result}, hidden list scroll_item={scroll}, error={:?}",visual.update(|_,cx|view.last_error(cx).unwrap()));
    assert_eq!(result,expected,"positive control: explicit occlusion makes only foreground eligible");
    assert_eq!(scroll,0,"same-view rectangle overlap must not invent scroll ancestry");
}

#[gpui::test]
fn accepting_foreground_does_not_scroll_unrelated_hidden_list(cx:&mut TestAppContext) {
    occluded_unrelated_virtual_list_does_not_scroll(cx,true);
}

#[gpui::test]
fn rejecting_foreground_does_not_activate_hidden_destination_scroll(cx:&mut TestAppContext) {
    occluded_unrelated_virtual_list_does_not_scroll(cx,false);
}

#[gpui::test]
fn newly_realized_dropzone_callback_keeps_formal_owner(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",value.target_id);}
fn lane(ctx,payload){drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,
    payload_types:["card"],operations:["move"],on_drop:Fn("dropped"),
    content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
            content:text("Card").with_style(style().width(px(150)).height(px(40)))}),
        virtual_collection(#{key:"lanes",label:"Lanes",data:data,height:180.0,estimated_height:40.0},Fn("lane"))
            .accessibility_role("list").accessibility_label("Lanes")
    ]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let seed=center(geometry(&mut visual,&view,"group","Lane 1"));
    drag(&mut visual,source,seed);
    assert_eq!(status(&mut visual,&view),"lane-1","control: seed-row callback must work");
    assert_eq!(visual.update(|_,cx|view.last_error(cx).unwrap()),None);
    let list=geometry(&mut visual,&view,"list","Lanes");let middle=center(list);
    for _ in 0..6 {
        visual.simulate_event(gpui::ScrollWheelEvent{position:middle,delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-240.0))),..Default::default()});
        visual.run_until_parked();
        cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    let fresh=snapshot.nodes().filter(|n|n.role=="group").filter_map(|n|{
        let index=n.name.strip_prefix("Lane ")?.parse::<usize>().ok()?;
        let bounds=n.geometry?.visual;
        (index>4 && bounds.y>=list.y+28.0 && bounds.y+bounds.height<=list.y+list.height-28.0).then_some((index,bounds))
    }).next().expect("control: scrolling must realize a fresh non-edge row");
    let expected=format!("lane-{}",fresh.0);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    drag(&mut visual,source,center(fresh.1));
    let actual=status(&mut visual,&view);let error=visual.update(|_,cx|view.last_error(cx).unwrap());
    eprintln!("fresh virtual row expected={expected}, actual={actual}, diagnostic={error:?}");
    assert_eq!(error,None,"later realization must preserve real callback owner");
    assert_eq!(actual,expected);
}

#[gpui::test]
fn still_visible_seed_callback_survives_neighbor_realization(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",value.target_id);}
fn lane(ctx,payload){drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,
    payload_types:["card"],operations:["move"],on_drop:Fn("dropped"),
    content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
            content:text("Card").with_style(style().width(px(150)).height(px(40)))}),
        virtual_collection(#{key:"lanes",label:"Lanes",data:data,height:180.0,estimated_height:40.0},Fn("lane"))
            .accessibility_role("list").accessibility_label("Lanes")
    ]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let seed=center(geometry(&mut visual,&view,"group","Lane 1"));
    drag(&mut visual,source,seed);
    assert_eq!(status(&mut visual,&view),"lane-1","control: seed-row callback must work");
    assert_eq!(visual.update(|_,cx|view.last_error(cx).unwrap()),None);
    let list=geometry(&mut visual,&view,"list","Lanes");let middle=center(list);
    for _ in 0..1 {
        visual.simulate_event(gpui::ScrollWheelEvent{position:middle,delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-45.0))),..Default::default()});
        visual.run_until_parked();
        cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    let fresh=snapshot.nodes().filter(|n|n.role=="group").filter_map(|n|{
        let index=n.name.strip_prefix("Lane ")?.parse::<usize>().ok()?;
        let bounds=n.geometry?.visual;
        (index==2 && bounds.y>=list.y+28.0 && bounds.y+bounds.height<=list.y+list.height-28.0).then_some((index,bounds))
    }).next().expect("control: scrolling must keep seed Lane 2 visible while realizing a neighbor");
    let expected=format!("lane-{}",fresh.0);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    drag(&mut visual,source,center(fresh.1));
    let actual=status(&mut visual,&view);let error=visual.update(|_,cx|view.last_error(cx).unwrap());
    eprintln!("retained seed virtual row expected={expected}, actual={actual}, diagnostic={error:?}");
    assert_eq!(error,None,"later realization must preserve real callback owner");
    assert_eq!(actual,expected);
}

#[gpui::test]
fn latest_realized_formal_dropzone_preserves_parent_callback_owner(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",value.target_id);}
fn lane(ctx,payload){drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,
    payload_types:["card"],operations:["move"],on_drop:Fn("dropped"),
    content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},keyboard_target:"lane-5",
            content:text("Card").with_style(style().width(px(150)).height(px(40)))}),
        virtual_collection(#{key:"lanes",label:"Lanes",data:data,height:180.0,estimated_height:40.0},Fn("lane"))
            .accessibility_role("list").accessibility_label("Lanes")
    ]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let seed=center(geometry(&mut visual,&view,"group","Lane 1"));
    drag(&mut visual,source,seed);
    assert_eq!(status(&mut visual,&view),"lane-1","control: seed-row callback must work");
    assert_eq!(visual.update(|_,cx|view.last_error(cx).unwrap()),None);
    let list=geometry(&mut visual,&view,"list","Lanes");let middle=center(list);
    for _ in 0..1 {
        visual.simulate_event(gpui::ScrollWheelEvent{position:middle,delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-45.0))),..Default::default()});
        visual.run_until_parked();
        cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    assert!(snapshot.find_by_role_and_name("group","Lane 5").next().is_some(),"control: latest row is realized");
    let expected="lane-5";
    let source=center(geometry(&mut visual,&view,"button","Source"));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(source,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    visual.simulate_keystrokes("enter");visual.run_until_parked();
    let actual=status(&mut visual,&view);let error=visual.update(|_,cx|view.last_error(cx).unwrap());
    eprintln!("latest virtual row keyboard expected={expected}, actual={actual}, diagnostic={error:?}");
    assert_eq!(error,None,"later realization must preserve real callback owner");
    assert_eq!(actual,expected);
}

#[gpui::test]
fn delayed_row_forwarded_callback_preserves_root_owner(cx:&mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
define_component(#{metadata:#{id:"test/action","export":"Action",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},label:#{schema:#{type:"string"},required:true,sensitive:false},
        on_action:#{schema:#{type:"callback"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_Action")});
fn Action(props){render_component("test/action",props)}
fn render_Action(ctx,props){text(props.label).on_click(props.on_action).with_key(props.key)
    .accessibility_role("button").accessibility_label(props.label).with_style(style().height(px(40)).width(px(240)))}
fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn clicked(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn row_action(ctx,payload){Action(#{key:payload.key,label:payload.item.label,on_action:Fn("clicked")})}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`action-${index}`,label:`Action ${index}`});}
    column([text(`${ctx.get_state("count")}`).accessibility_role("status"),
        virtual_collection(#{key:"actions",label:"Actions",data:data,height:180.0,estimated_height:40.0},Fn("row_action"))
            .accessibility_role("list").accessibility_label("Actions")]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);let mut visual=VisualTestContext::from_window(*window,cx);
    let seed=center(geometry(&mut visual,&view,"button","Action 1"));
    visual.simulate_mouse_down(seed,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(seed,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    assert_eq!(status(&mut visual,&view),"1","seed forwarded parent callback works");
    let seeded=visual.update(|_,cx|realized_indices(&view.root(cx).unwrap().unwrap()).unwrap());
    let list=geometry(&mut visual,&view,"list","Actions");
    for _ in 0..6 {
        visual.simulate_event(gpui::ScrollWheelEvent{position:center(list),delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-240.0))),..Default::default()});
        visual.run_until_parked();cx.background_executor.advance_clock(std::time::Duration::from_millis(16));
        visual.update(|window,_|window.refresh());visual.run_until_parked();
    }
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    let (index,bounds)=snapshot.nodes().filter(|node|node.role=="button").filter_map(|node|{
        let index=node.name.strip_prefix("Action ")?.parse::<usize>().ok()?;let bounds=node.geometry?.visual;
        (!seeded.contains(&index)&&bounds.y>=list.y&&bounds.y+bounds.height<=list.y+list.height).then_some((index,bounds))
    }).next().expect("control: destination must be a newly realized visible row, absent from seed manifest");
    eprintln!("forwarded callback actual seed indices={seeded:?}; destination index={index}");
    let target=center(bounds);
    assert!(f64::from(target.y)<list.y+list.height,"control: fresh target is visible");
    visual.simulate_mouse_down(target,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(target,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    let actual=status(&mut visual,&view);let error=visual.update(|_,cx|view.last_error(cx).unwrap());
    eprintln!("forwarded callback expected count=2, actual={actual}, diagnostic={error:?}");
    assert_eq!(error,None,"forwarded callback must still belong to root rather than structural scope");
    assert_eq!(actual,"2");
}
