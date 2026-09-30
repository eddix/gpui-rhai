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

#[gpui::test]
fn virtual_sortable_edge_auto_scroll_advances_realization(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"virtual-order",label:"Virtual order",virtual_data:data,
            height:180.0,on_reorder:Fn("reordered")})]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    let list=geometry(&mut visual,&view,"list","Virtual order");
    let bottom=point(px((list.x+list.width/2.0) as f32),px((list.y+list.height-5.0) as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    for step in 0..30 {
        visual.simulate_mouse_move(point(bottom.x+px((step%2) as f32),bottom.y),MouseButton::Left,Modifiers::default());
        visual.run_until_parked();
    }
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    let labels=snapshot.nodes().filter(|n|n.role=="listitem").map(|n|n.name.clone()).collect::<Vec<_>>();
    eprintln!("realized after scroll={labels:?}");
    visual.simulate_mouse_up(bottom,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let actual=status(&mut visual,&view);
    eprintln!("long virtual drag={actual}");
    assert!(labels.iter().any(|label|label.strip_prefix("Item ").and_then(|n|n.parse::<usize>().ok()).is_some_and(|index|index>4)),"edge auto-scroll never left initial rows: {labels:?}");
    assert!(actual.starts_with("item-0:"),"native realization/scroll must preserve active key: {actual}");
}

#[gpui::test]
fn virtual_sortable_preserves_drag_across_actual_wheel_scroll(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"virtual-order",label:"Virtual order",virtual_data:data,
            height:180.0,on_reorder:Fn("reordered")})]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    let list=geometry(&mut visual,&view,"list","Virtual order");
    let middle=point(px((list.x+list.width/2.0) as f32),px((list.y+list.height/2.0) as f32));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_move(middle,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    visual.simulate_event(gpui::ScrollWheelEvent {position:middle,delta:gpui::ScrollDelta::Pixels(point(px(0.0),px(-400.0))),..Default::default()});
    visual.run_until_parked();
    visual.update(|window,_|window.refresh());
    visual.run_until_parked();
    let snapshot=visual.update(|_,cx|view.accessibility_snapshot(cx).unwrap());
    let labels=snapshot.nodes().filter(|n|n.role=="listitem").map(|n|n.name.clone()).collect::<Vec<_>>();
    eprintln!("realized after actual wheel={labels:?}");
    assert!(labels.iter().any(|label|label.strip_prefix("Item ").and_then(|n|n.parse::<usize>().ok()).is_some_and(|index|index>4)),"positive control: wheel must scroll");
    visual.simulate_mouse_move(middle,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(middle,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let actual=status(&mut visual,&view);
    eprintln!("actual wheel drag result={actual}");
    assert!(actual.starts_with("item-0:"),"virtual scrolling must not cancel an existing valid source: {actual}");
}

#[gpui::test]
fn explicit_occlusion_blocks_underlying_drop_target(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",`drop:${value.target_id}`);}
fn cover_clicked(ctx,value){ctx.set_state("status","cover clicked");event_response().stop()}
fn view(ctx){box([
    text(ctx.get_state("status")).accessibility_role("status").with_style(style().absolute().top(px(300))),
    drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
        content:text("Drag me").with_style(style().width(px(100)).height(px(40)))})
        .with_style(style().absolute().left(px(10)).top(px(200))),
    drop_zone::DropZone(#{key:"target",label:"Back target",target_id:"back",payload_types:["card"],operations:["move"],
        content:text("Back target").with_style(style().width(px(180)).height(px(80))),on_drop:Fn("dropped")})
        .with_style(style().absolute().left(px(200)).top(px(50)).width(px(180)).height(px(80))),
    text("Foreground clickable panel").with_key("cover").accessibility_role("button").accessibility_label("Foreground")
        .on_click(Fn("cover_clicked")).with_style(style().absolute().left(px(200)).top(px(50))
            .width(px(180)).height(px(80)).occlude().background(theme_color("surface")))
]).with_style(style().relative().width(px(500)).height(px(400)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let target=center(geometry(&mut visual,&view,"button","Foreground"));
    visual.simulate_mouse_down(target,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(target,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    eprintln!("foreground location={target:?}; error={:?}",visual.update(|_,cx|view.last_error(cx).unwrap()));
    assert_eq!(status(&mut visual,&view),"cover clicked","foreground clickable positive control");
    drag(&mut visual,source,target);
    let actual=status(&mut visual,&view);
    eprintln!("covered target drop={actual}");
    assert_ne!(actual,"drop:back","explicit occlusion must block hidden drop target");
}

#[gpui::test]
fn modal_explicit_keyboard_target_policy_characterization(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
import "components/dialog" as dialog;
fn state_schema(){#{fields:#{drops:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn dropped(ctx,value){ctx.set_state("drops",ctx.get_state("drops")+1);}
fn view(ctx){box([
    text(`${ctx.get_state("drops")}`).accessibility_role("status").with_style(style().absolute().top(px(300))),
    drop_zone::DropZone(#{key:"target",label:"Background target",target_id:"background",payload_types:["card"],operations:["move"],
        content:text("Background target").with_style(style().width(px(80)).height(px(80))),on_drop:Fn("dropped")})
        .with_style(style().absolute().left(px(0)).top(px(0)).width(px(80)).height(px(80))),
    dialog::Dialog(#{key:"modal",open:true,title:"Modal",dismiss_on_outside:false,
        content:drag_source::DragSource(#{key:"source",label:"Modal source",source_id:"card",payload_type:"card",payload:#{id:"card"},
            keyboard_target:"background",content:text("Drag me").with_style(style().width(px(100)).height(px(40)))})})
]).with_style(style().relative().width(px(800)).height(px(600)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Modal source"));
    let target=center(geometry(&mut visual,&view,"group","Background target"));
    let modal=geometry(&mut visual,&view,"dialog","Modal");
    eprintln!("modal={modal:?}; source={source:?}; background={target:?}");
    assert!(f64::from(target.x)<modal.x || f64::from(target.y)<modal.y);
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(source,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    eprintln!("modal background drops after keyboard={}",status(&mut visual,&view));
    drag(&mut visual,source,target);
    let actual=status(&mut visual,&view);
    eprintln!("modal background drops after pointer={actual}");
    assert_eq!(actual,"1","characterization only: explicit keyboard target dispatches; pointer does not");
}

#[gpui::test]
fn same_host_cross_view_drop_is_supported_for_pointer_and_keyboard(cx: &mut TestAppContext) {
    use gpui::{ParentElement, Styled};
    struct Pair {host:ScriptViewHost,left:ScriptViewHandle,right:ScriptViewHandle}
    impl Render for Pair {
        fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement {
            self.host.container(gpui::div().flex().gap(px(20.0))
                .child(gpui::div().w(px(240.0)).h(px(240.0)).child(self.left.element().unwrap()))
                .child(gpui::div().w(px(240.0)).h(px(240.0)).child(self.right.element().unwrap())))
        }
    }
    fn prepared(script:&str)->PreparedScriptView {
        let entry=ModuleId::parse("main").unwrap();
        let mut modules=BTreeMap::from([(entry.clone(),script.to_owned())]);
        for name in ["drag_source","drop_zone"] {
            modules.insert(ModuleId::parse(format!("components/{name}")).unwrap(),std::fs::read_to_string(format!("{ROOT}/registry/components/{name}.rhai")).unwrap());
        }
        let theme=std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap();
        EmbeddedScriptView::new(entry,EmbeddedScriptSource::new(modules),&theme).prepare().unwrap()
    }
    cx.update(gpui_rhai::install);
    let left=prepared(r#"
import "components/drag_source" as drag_source;
fn view(ctx){drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
    keyboard_target:"target",content:text("Source").with_style(style().width(px(150)).height(px(100)))})}
"#);
    let right=prepared(r#"
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn dropped(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn view(ctx){column([text(`${ctx.get_state("count")}`).accessibility_role("status"),
    drop_zone::DropZone(#{key:"target",label:"Target",target_id:"target",payload_types:["card"],operations:["move"],
        content:text("Target").with_style(style().width(px(150)).height(px(100))),on_drop:Fn("dropped")})])}
"#);
    let capture=Rc::new(RefCell::new(None));let captured=capture.clone();
    let window=cx.add_window(move|window,cx|{
        let host=ScriptViewHost::new("pair",cx).unwrap();
        let left=left.mount(ScriptViewConfig::new("left"),host.clone(),window,cx).unwrap();
        let right=right.mount(ScriptViewConfig::new("right"),host.clone(),window,cx).unwrap();
        *captured.borrow_mut()=Some((left.clone(),right.clone()));Pair{host,left,right}
    });
    cx.run_until_parked();cx.refresh().unwrap();
    let (left,right)=capture.borrow().as_ref().unwrap().clone();
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&left,"button","Source"));
    let target=center(geometry(&mut visual,&right,"group","Target"));
    drag(&mut visual,source,target);
    assert_eq!(status(&mut visual,&right),"1");
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(source,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();visual.simulate_keystrokes("enter");visual.run_until_parked();
    assert_eq!(status(&mut visual,&right),"2");
}
