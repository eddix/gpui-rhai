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
    for name in ["sortable", "drag_source", "drop_zone", "scroll_area"] {
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
fn clipped_drop_target_still_receives_drop(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",`drop:${value.target_id}`);}
fn view(ctx){box([
    text(ctx.get_state("status")).accessibility_role("status").with_style(style().absolute().top(px(350))),
    drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
        content:text("Drag me").with_style(style().width(px(100)).height(px(40)))})
        .with_style(style().absolute().left(px(10)).top(px(250))),
    box([drop_zone::DropZone(#{key:"target",label:"Clipped target",target_id:"hidden",payload_types:["card"],operations:["move"],
        content:text("Clipped target").with_style(style().width(px(180)).height(px(60))),on_drop:Fn("dropped")})
        .with_style(style().absolute().top(px(120)).left(px(0)).width(px(180)).height(px(60)))])
        .with_style(style().absolute().left(px(200)).top(px(0)).width(px(180)).height(px(80)).overflow_hidden())
]).with_style(style().relative().width(px(500)).height(px(400)))}
"#;
    let (window, view) = mount(cx, script);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let source = center(geometry(&mut visual,&view,"button","Source"));
    let target = center(geometry(&mut visual,&view,"group","Clipped target"));
    eprintln!("clipped target at {target:?}, clip y=0..80");
    drag(&mut visual,source,target);
    let actual = status(&mut visual,&view);
    eprintln!("clipped target result = {actual}");
    assert_eq!(actual,"ready","off-clip target must not accept pointer drop");
}

#[gpui::test]
fn overlapping_targets_follow_paint_order_not_target_name(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn dropped(ctx,value){ctx.set_state("status",`drop:${value.target_id}`);}
fn zone(id){drop_zone::DropZone(#{key:id,label:id,target_id:id,payload_types:["card"],operations:["move"],
    content:text(id).with_style(style().width(px(180)).height(px(80)).background(theme_color("surface"))),on_drop:Fn("dropped")})
    .with_style(style().absolute().left(px(200)).top(px(50)).width(px(180)).height(px(80)))}
fn view(ctx){box([
    text(ctx.get_state("status")).accessibility_role("status").with_style(style().absolute().top(px(300))),
    drag_source::DragSource(#{key:"source",label:"Source",source_id:"card",payload_type:"card",payload:#{id:"card"},
        content:text("Drag me").with_style(style().width(px(100)).height(px(40)))})
        .with_style(style().absolute().left(px(10)).top(px(200))),
    zone("zz-back"),zone("aa-front")
]).with_style(style().relative().width(px(500)).height(px(400)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Source"));
    let target=center(geometry(&mut visual,&view,"group","aa-front"));
    drag(&mut visual,source,target);
    let actual=status(&mut visual,&view);
    eprintln!("overlap result={actual}");
    assert_eq!(actual,"drop:aa-front","painted foreground target must beat hidden sibling by default");
}

#[gpui::test]
fn virtual_sortable_end_works_without_realizing_last_row(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"virtual-order",label:"Virtual order",virtual_data:data,
            height:180.0,on_reorder:Fn("reordered")})]).with_style(style().width(px(300)).padding(px(12)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(source,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let focused=visual.update(|window,cx|window.focused(cx).map(|handle|format!("{handle:?}")));
    eprintln!("virtual source position={source:?}; focused={focused:?}");
    visual.simulate_keystrokes("alt-down");
    visual.run_until_parked();
    eprintln!("virtual Alt+Down result={}",status(&mut visual,&view));
    visual.simulate_keystrokes("alt-end");
    visual.run_until_parked();
    let actual=status(&mut visual,&view);
    eprintln!("virtual Alt+End result={actual}");
    let bounds=geometry(&mut visual,&view,"button","Reorder Item 2");
    let target=point(px((bounds.x+bounds.width/2.0)as f32),px((bounds.y+bounds.height*0.75)as f32));
    drag(&mut visual,source,target);
    eprintln!("virtual pointer control={}",status(&mut visual,&view));
    assert_eq!(actual,"item-0:after:item-99","virtual keyboard move must not depend on destination realization");
}

#[gpui::test]
fn bounded_sortable_keyboard_control(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..3{data.push(#{key:`item-${index}`,label:`Item ${index}`,
    content:text(`Item ${index}`).with_style(style().height(px(40)))});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"bounded-order",label:"Bounded order",items:data,on_reorder:Fn("reordered")})])
        .with_style(style().width(px(300)).padding(px(12)))}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder Item 0"));
    visual.simulate_mouse_down(source,MouseButton::Left,Modifiers::default());
    visual.simulate_mouse_up(source,MouseButton::Left,Modifiers::default());
    visual.run_until_parked();
    let focused=visual.update(|window,cx|window.focused(cx).is_some());
    eprintln!("bounded source focused={focused}");
    assert!(focused);
    visual.simulate_keystrokes("alt-down");
    visual.run_until_parked();
    assert_eq!(status(&mut visual,&view),"item-0:after:item-1");
    visual.simulate_keystrokes("alt-end");
    visual.run_until_parked();
    assert_eq!(status(&mut visual,&view),"item-0:after:item-2");
}

#[gpui::test]
fn independent_sortables_with_same_local_key_reject_foreign_source(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/audit_panel" as audit_panel;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){column([text(ctx.get_state("status")).accessibility_role("status"),
    row([
        audit_panel::AuditPanel(#{key:"left",prefix:"left",on_reorder:Fn("reordered")}).with_style(style().width(px(200))),
        audit_panel::AuditPanel(#{key:"right",prefix:"right",on_reorder:Fn("reordered")}).with_style(style().width(px(200)))
    ]).with_style(style().gap(px(20)))])}
"#;
    let (window,view)=mount(cx,script);
    let mut visual=VisualTestContext::from_window(*window,cx);
    let source=center(geometry(&mut visual,&view,"button","Reorder left-a"));
    let bounds=geometry(&mut visual,&view,"listitem","right-b");
    let target=point(px((bounds.x+bounds.width/2.0)as f32),px((bounds.y+bounds.height*0.75)as f32));
    drag(&mut visual,source,target);
    let actual=status(&mut visual,&view);
    eprintln!("independent list result={actual}");
    assert_eq!(actual,"ready","component-local keys must not create cross-list drop channels");
}
