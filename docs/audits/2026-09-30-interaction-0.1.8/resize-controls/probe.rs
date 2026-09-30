#![allow(unused_imports, dead_code)]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, WindowHandle, point, px, rgba,
};
use gpui_rhai::*;

const BUTTON: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/button.rhai");
const BADGE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/badge.rhai");
const TABS: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/tabs.rhai");
const INPUT: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/input.rhai");
const SPLIT_PANE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/split_pane.rhai");
const RESIZABLE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/resizable.rhai");
const DRAGGABLE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/draggable.rhai");
const DRAG_SOURCE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/drag_source.rhai");
const DROP_ZONE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/drop_zone.rhai");
const SORTABLE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/sortable.rhai");
const SCROLL_AREA: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/scroll_area.rhai");
const PAN_ZOOM: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/pan_zoom.rhai");
const RANGE_SLIDER: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/range_slider.rhai");
const ROTATABLE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/rotatable.rhai");
const SELECTION_AREA: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/selection_area.rhai");
const TREE: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/tree.rhai");
const INTERACTION_LAB: &str =
    include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/stories/workbench/interaction_lab.rhai");
const ANIMATED_TABS: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/motion/animated_tabs.rhai");
const DEFAULT_DARK: &str = include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/themes/default_dark.rhai");

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    mount_with_overrides(cx, script, name, ThemeTokenOverrides::default())
}

fn mount_with_overrides(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
    overrides: ThemeTokenOverrides,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (ModuleId::parse("components/cell").unwrap(),CELL.to_owned()),
            (ModuleId::parse("components/table").unwrap(),include_str!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/components/table.rhai").to_owned()),
            (
                ModuleId::parse("components/button").unwrap(),
                BUTTON.to_owned(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                BADGE.to_owned(),
            ),
            (ModuleId::parse("components/tabs").unwrap(), TABS.to_owned()),
            (
                ModuleId::parse("components/input").unwrap(),
                INPUT.to_owned(),
            ),
            (
                ModuleId::parse("components/split_pane").unwrap(),
                SPLIT_PANE.to_owned(),
            ),
            (
                ModuleId::parse("components/resizable").unwrap(),
                RESIZABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/draggable").unwrap(),
                DRAGGABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/drag_source").unwrap(),
                DRAG_SOURCE.to_owned(),
            ),
            (
                ModuleId::parse("components/drop_zone").unwrap(),
                DROP_ZONE.to_owned(),
            ),
            (
                ModuleId::parse("components/sortable").unwrap(),
                SORTABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/scroll_area").unwrap(),
                SCROLL_AREA.to_owned(),
            ),
            (
                ModuleId::parse("components/pan_zoom").unwrap(),
                PAN_ZOOM.to_owned(),
            ),
            (
                ModuleId::parse("components/range_slider").unwrap(),
                RANGE_SLIDER.to_owned(),
            ),
            (
                ModuleId::parse("components/rotatable").unwrap(),
                ROTATABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/selection_area").unwrap(),
                SELECTION_AREA.to_owned(),
            ),
            (ModuleId::parse("components/tree").unwrap(), TREE.to_owned()),
            (
                ModuleId::parse("motion/animated_tabs").unwrap(),
                ANIMATED_TABS.to_owned(),
            ),
        ])),
        DEFAULT_DARK,
    )
    .asset_sources([
        ("icons/disclosure_down".to_owned(), AssetData { mime_type: "image/svg+xml".to_owned(), bytes: include_bytes!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/assets/icons/disclosure_down.svg").to_vec() }),
        ("icons/sort_ascending".to_owned(), AssetData { mime_type: "image/svg+xml".to_owned(), bytes: include_bytes!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/assets/icons/sort_ascending.svg").to_vec() }),
        ("icons/sort_descending".to_owned(), AssetData { mime_type: "image/svg+xml".to_owned(), bytes: include_bytes!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/assets/icons/sort_descending.svg").to_vec() }),
        (
            "icons/chevron_down".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/assets/icons/chevron_down.svg").to_vec(),
            },
        ),
        (
            "icons/chevron_right".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("/Users/eddix/Codes/github.com/eddix/gpui-rhai/registry/assets/icons/chevron_right.svg").to_vec(),
            },
        ),
    ])
    .theme_token_overrides(overrides)
    .motion_preference(MotionPreference::None)
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
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap().nodes().find(|n| n.role == "status").unwrap().name.clone())
}
fn bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> GeometryBounds {
    visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap().find_by_role_and_name(role,name).next().unwrap().geometry.unwrap().visual)
}
fn dispatch(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str, event: &str) -> Result<AutomationResult, ScriptViewError> {
    visual.update(|window,cx| view.automate(AutomationCommand::Dispatch { locator:AutomationLocator::RoleName {role:role.to_owned(),name:name.to_owned()}, event:event.to_owned(),payload:None},window,cx))
}
const RANGE_SCRIPT: &str = r#"
import "components/range_slider" as range_slider;
fn state_schema(){#{fields:#{range:#{schema:#{type:"object",allow_unknown:false,fields:#{
    low:#{schema:#{type:"number"},required:true,sensitive:false},
    high:#{schema:#{type:"number"},required:true,sensitive:false}}},
    "default":#{type:"map",value:#{low:#{type:"float",value:20.0},high:#{type:"float",value:80.0}}}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("range",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn external(ctx,payload){ctx.set_state("range",#{low:10.0,high:40.0});}
fn view(ctx){let value=ctx.get_state("range");column([
    text(`${value.low},${value.high},${ctx.get_state("commits")}`).accessibility_role("status"),
    text("External").accessibility_role("button").accessibility_label("External").on_click(Fn("external")),
    range_slider::RangeSlider(#{key:"range",label:"Accepted interval",low_label:"Low bound",
        high_label:"High bound",values:value,min:0.0,max:100.0,step:5.0,minimum_gap:7.0,
        on_change:Fn("changed")})
]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
#[gpui::test]
fn range_gap_preserves_the_global_step_grid(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window,view)=mount(cx,RANGE_SCRIPT,"audit-range-gap");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut visual,&view,"group","Accepted interval");
    let p=point(px((b.x+b.width*0.75) as f32),px((b.y+b.height*0.7) as f32));
    visual.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());
    visual.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());
    visual.run_until_parked();
    let actual=status(&mut visual,&view); println!("range gap result={actual}");
    assert_eq!(actual,"20.0,75.0,1","gap must not shift step origin from global min");
}
#[gpui::test]
fn range_source_replacement_cancels_existing_drag(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=RANGE_SCRIPT.replace("minimum_gap:7.0","minimum_gap:10.0");
    let (window,view)=mount(cx,&script,"audit-range-source");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut visual,&view,"group","Accepted interval");
    let p=point(px((b.x+b.width*0.8) as f32),px((b.y+b.height*0.7) as f32));
    let q=point(px((b.x+b.width*0.7) as f32),p.y);
    visual.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());
    visual.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());
    dispatch(&mut visual,&view,"button","External","click").unwrap();
    visual.run_until_parked();
    println!("before mouseup={}",status(&mut visual,&view));
    visual.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());
    visual.run_until_parked();
    let actual=status(&mut visual,&view); println!("range source result={actual}");
    assert_eq!(actual,"10.0,40.0,0","old gesture must not overwrite new controlled source");
}
const RESIZE_SCRIPT: &str = r#"
import "components/resizable" as resizable;
fn state_schema(){#{fields:#{limit:#{schema:#{type:"number"},"default":#{type:"float",value:360.0}},
 last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn changed(ctx,v){ctx.set_state("last",`${v.width}`);}
fn external(ctx,payload){ctx.set_state("limit",220.0);}
fn view(ctx){column([
 text(ctx.get_state("last")).accessibility_role("status"),
 text("External").accessibility_role("button").accessibility_label("External").on_click(Fn("external")),
 resizable::Resizable(#{key:"card",label:"Demo",rect:#{x:100.0,y:80.0,width:200.0,height:120.0},
 handles:["e"],min_width:80.0,min_height:60.0,max_width:ctx.get_state("limit"),max_height:260.0,
 content:text("Card"),on_resize:Fn("changed")}).with_style(style().width(px(500)).height(px(400)))
])}
"#;
#[gpui::test]
fn resizable_constraint_replacement_cancels_existing_drag(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window,view)=mount(cx,RESIZE_SCRIPT,"audit-resize-source");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut visual,&view,"separator","Demo: e resize handle");
    let p=point(px((b.x+b.width/2.0) as f32),px((b.y+b.height/2.0) as f32));
    let q=point(p.x+px(100.0),p.y);
    visual.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());
    visual.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());
    dispatch(&mut visual,&view,"button","External","click").unwrap();
    visual.run_until_parked();
    visual.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());
    visual.run_until_parked();
    let actual=status(&mut visual,&view); println!("resizable constraint result={actual}");
    assert_eq!(actual,"none","changed constraints must cancel stale gesture");
}
#[gpui::test]
fn resizable_keyboard_is_automatable(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window,view)=mount(cx,RESIZE_SCRIPT,"audit-issue85");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let result=dispatch(&mut visual,&view,"separator","Demo: e resize handle","key:right");
    println!("issue85 dispatch={result:?}");
    assert!(result.is_ok(),"Resizable keyboard has no retained dispatch handler");
    visual.run_until_parked();
    assert_eq!(status(&mut visual,&view),"208.0");
}
#[gpui::test]
fn active_draggable_suspend_is_safe(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=r#"
import "components/draggable" as draggable;
fn view(ctx){draggable::Draggable(#{key:"card",label:"Drag card",position:#{x:20.0,y:20.0},
 handle:text("Handle").with_style(style().width(px(120)).height(px(30))),
 content:text("Body").with_style(style().width(px(120)).height(px(80)))})
 .with_style(style().width(px(500)).height(px(400)))}
"#;
    let (window,view)=mount(cx,script,"audit-drag-suspend");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut visual,&view,"group","Drag card");
    let p=point(px((b.x+20.0) as f32),px((b.y+15.0) as f32));
    let q=point(p.x+px(30.0),p.y+px(20.0));
    visual.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());
    visual.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());
    visual.run_until_parked();
    let result=visual.update(|window,cx| view.suspend(window,cx));
    println!("suspend result={result:?}");
    assert!(result.is_ok());
}
#[gpui::test]
fn aspect_corner_keyboard_vertical_resize_is_effective(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script=RESIZE_SCRIPT.replace("height:120.0","height:100.0").replace("handles:[\"e\"]","handles:[\"se\"],aspect_ratio:2.0");
    let (window,view)=mount(cx,&script,"audit-resize-aspect");
    let mut visual=VisualTestContext::from_window(*window,cx);
    let b=bounds(&mut visual,&view,"separator","Demo: se resize handle");
    let p=point(px((b.x+b.width/2.0) as f32),px((b.y+b.height/2.0) as f32));
    visual.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());
    visual.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());
    visual.run_until_parked();
    visual.simulate_keystrokes("down");
    visual.run_until_parked();
    let actual=status(&mut visual,&view);println!("aspect corner down result={actual}");
    assert_eq!(actual,"216.0","corner Down should grow height by8 and preserve 2:1 aspect");
}
const CELL: &str=r#"/* gpui-rhai
{"id":"components/cell","export":"Cell","version":"0.1.8","runtime_api":{"min_inclusive":2,"max_exclusive":3},"dependencies":[],"capabilities":{}}
*/
define_component(#{metadata:#{id:"components/cell","export":"Cell",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},on_checked:#{schema:#{type:"callback"},required:true,sensitive:false}},
 state:#{fields:#{}},events:#{checked:#{payload:#{type:"bool"}}},slots:#{},parts:[]},render:Fn("render_Cell")});
fn Cell(props){render_component("components/cell",props)}
fn check(ctx,p){let g=ctx.element_bounds("cell_root");ctx.emit("checked",g!=());}
fn render_Cell(ctx,props){text("CheckCell").with_key(props.key).with_ref(element_ref("cell_root"))
 .with_style(style().width(px(120)).height(px(40))).on_click(Fn("check"))
 .accessibility_role("button").accessibility_label("CheckCell")}
"#;
fn cell_script(kind: &str)->String {
    let node="cell::Cell(#{key:\"cell\",on_checked:Fn(\"checked\")})";
    let child=match kind {
        "split"=>format!("split_pane::SplitPane(#{{key:\"split\",label:\"Split\",size:0.5,min_start:0.0,min_end:0.0,start:{node},end:text(\"End\")}}).with_style(style().width(px(400)).height(px(180)))"),
        "drag"=>format!("draggable::Draggable(#{{key:\"drag\",label:\"Drag\",position:#{{x:0.0,y:0.0}},handle:{node},content:text(\"Body\")}}).with_style(style().width(px(400)).height(px(180)))"),
        _=>format!("box([{node}])"),
    };
    format!(r#"
import "components/cell" as cell;import "components/split_pane" as split_pane;import "components/draggable" as draggable;
fn state_schema(){{#{{fields:#{{ok:#{{schema:#{{type:"bool"}},"default":#{{type:"bool",value:false}}}}}}}}}}
fn checked(ctx,value){{ctx.set_state("ok",value);}}
fn view(ctx){{column([text(`${{ctx.get_state("ok")}}`).accessibility_role("status"),{child}])}}
"#)
}
#[gpui::test]
fn formal_component_ref_survives_normal_wrapper(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let(window,view)=mount(cx,&cell_script("plain"),"audit-cell-plain");let mut v=VisualTestContext::from_window(*window,cx);
 let r=dispatch(&mut v,&view,"button","CheckCell","click");println!("plain={r:?}");assert!(r.is_ok());assert_eq!(status(&mut v,&view),"true");
}
#[gpui::test]
fn split_pane_preserves_formal_child_ref(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let(window,view)=mount(cx,&cell_script("split"),"audit-cell-split");let mut v=VisualTestContext::from_window(*window,cx);
 let r=dispatch(&mut v,&view,"button","CheckCell","click");println!("split={r:?}");assert!(r.is_ok(),"SplitPane overwrites caller component ref");assert_eq!(status(&mut v,&view),"true");
}
#[gpui::test]
fn draggable_preserves_formal_handle_ref(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let(window,view)=mount(cx,&cell_script("drag"),"audit-cell-drag");let mut v=VisualTestContext::from_window(*window,cx);
 let r=dispatch(&mut v,&view,"button","CheckCell","click");println!("drag={r:?}");assert!(r.is_ok(),"Draggable overwrites caller component ref");assert_eq!(status(&mut v,&view),"true");
}
#[gpui::test]
fn table_resize_rejected_value_restores_controlled_width(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);
 let script=r#"
import "components/table" as table;
fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn resized(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn view(ctx){column([
text(`${ctx.get_state("count")}`).accessibility_role("status"),
table::Table(#{key:"table",label:"Audit table",row_key:"id",rows:[#{id:"one",left:"Left",right:"Right"}],
 columns:[#{key:"left",title:"Left",width:#{kind:"fixed",value:160.0},max_width:240.0},#{key:"right",title:"Right",width:#{kind:"flex",value:1.0}}],height:140.0,resizable_columns:true,on_column_resize:Fn("resized")})
]).with_style(style().width(px(500)))}
"#;
 let(window,view)=mount(cx,script,"audit-table-reject");let mut v=VisualTestContext::from_window(*window,cx);
 let b=bounds(&mut v,&view,"separator","Resize Left column");
 let p=point(px((b.x+b.width/2.0) as f32),px((b.y+b.height/2.0) as f32));let q=point(p.x+px(40.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let after=bounds(&mut v,&view,"separator","Resize Left column");println!("table rejected preview: before_x={}, after_x={}, callbacks={}",b.x,after.x,status(&mut v,&view));
 assert!((after.x-b.x).abs()<0.1,"rejected width must not remain as native override");
}
