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

fn gap_script(block:bool)->String {
    r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn ended(ctx,value){ctx.set_state("status",if value.cancelled{"cancelled"}else if value.accepted{value.target_id}else{"rejected"});}
fn lane(ctx,payload){let children=[
    drop_zone::DropZone(#{key:payload.key,label:payload.item.label,target_id:payload.key,payload_types:["card"],operations:["move"],
        content:text(payload.item.label).with_style(style().height(px(40)).width(px(240)))})
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).height(px(40)))
    ];
    if BLOCK_GAP {children.push(drop_zone::DropZone(#{key:`blocked-${payload.key}`,label:`Blocked ${payload.item.label}`,target_id:`blocked-${payload.key}`,
        payload_types:["other"],operations:["move"],content:text("Not a destination").with_style(style().height(px(20)).width(px(240)))})
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(40)).height(px(20)).occlude()));}
    box(children).with_style(style().relative().height(px(60)).width(px(240)))
}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`lane-${index}`,label:`Lane ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},on_drag_end:Fn("ended"),
            content:text("Card").with_style(style().width(px(150)).height(px(40)))}),
        virtual_collection(#{key:"lanes",label:"Lanes",data:data,height:180.0,estimated_height:60.0},Fn("lane"))
            .accessibility_role("list").accessibility_label("Lanes")
    ]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#.replace("BLOCK_GAP",if block{"true"}else{"false"})
}
fn scroll_position(visual:&mut VisualTestContext,view:&ScriptViewHandle)->(usize,f64) {
    visual.update(|_,cx|{let snapshot=view.take_performance_snapshot(cx).unwrap();let m=&snapshot.virtual_collections[0];(m.scroll_item,m.scroll_offset)})
}
fn tick(cx:&mut TestAppContext,visual:&mut VisualTestContext,n:usize){
    for _ in 0..n {cx.background_executor.advance_clock(std::time::Duration::from_millis(16));visual.run_until_parked();}
}
fn row_gap_probe(cx:&mut TestAppContext,block:bool){
    cx.update(gpui_rhai::install);let (window,view)=mount(cx,&gap_script(block));
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let accepted=center(geometry(&mut visual,&view,"group","Lane 1"));
    drag(&mut visual,source,accepted);assert_eq!(status(&mut visual,&view),"lane-1","control: valid row accepts typed drag");
    let list=geometry(&mut visual,&view,"list","Lanes");let gap=point(px((list.x+list.width/2.0)as f32),px((list.y+list.height-5.0)as f32));
    if block {
        let blocked=geometry(&mut visual,&view,"group","Blocked Lane 2");
        assert!(f64::from(gap.x)>=blocked.x && f64::from(gap.x)<blocked.x+blocked.width
            && f64::from(gap.y)>=blocked.y && f64::from(gap.y)<blocked.y+blocked.height,"control: pointer lies inside occluding rejecting child");
        drag(&mut visual,source,gap);
        let direct=scroll_position(&mut visual,&view);
        eprintln!("direct blocked target without accepted history: end={}, scroll={direct:?}",status(&mut visual,&view));
        assert_eq!(status(&mut visual,&view),"rejected","control: blocked child rejects native drop");
        assert_eq!(direct,(0,0.0),"control: rejected destination alone does not scroll");
    }
    let before=scroll_position(&mut visual,&view);
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(accepted,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    visual.simulate_mouse_move(gap,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    tick(cx,&mut visual,12);
    let after=scroll_position(&mut visual,&view);visual.simulate_keystrokes("escape");visual.run_until_parked();
    visual.simulate_mouse_up(gap,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    eprintln!("row gap block={block}, before={before:?}, after={after:?}, end={}, diagnostic={:?}",status(&mut visual,&view),visual.update(|_,cx|view.last_error(cx).unwrap()));
    if block {assert_eq!(before,after,"occluding rejecting child must stop gap continuation");}
    else {assert_ne!(before,after,"control: native ticks should continue through genuine open row gap");}
}

#[gpui::test]
fn open_row_gap_continues_with_stationary_pointer(cx:&mut TestAppContext){row_gap_probe(cx,false);}
#[gpui::test]
fn occluding_rejecting_child_blocks_row_gap_continuation(cx:&mut TestAppContext){row_gap_probe(cx,true);}

#[gpui::test]
fn canceled_tick_cannot_transfer_previous_destination_to_new_drag(cx:&mut TestAppContext){
    cx.update(gpui_rhai::install);let(window,view)=mount(cx,&gap_script(false));let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));let accepted=center(geometry(&mut visual,&view,"group","Lane 1"));
    let list=geometry(&mut visual,&view,"list","Lanes");let gap=point(px((list.x+list.width/2.0)as f32),px((list.y+list.height-5.0)as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());visual.simulate_mouse_move(accepted,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    visual.simulate_keystrokes("escape");visual.run_until_parked();visual.simulate_mouse_up(accepted,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    assert_eq!(status(&mut visual,&view),"cancelled");let before=scroll_position(&mut visual,&view);
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());visual.simulate_mouse_move(gap,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    tick(cx,&mut visual,12);let after=scroll_position(&mut visual,&view);
    visual.simulate_mouse_up(gap,MouseButton::Left,Modifiers::default());visual.run_until_parked();
    eprintln!("new drag direct gap before={before:?}, after={after:?}, outcome={}",status(&mut visual,&view));
    assert_eq!(before,after);assert_eq!(status(&mut visual,&view),"rejected");
}
