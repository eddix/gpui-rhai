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
    .runtime_clock(ManualRuntimeClock::new(std::time::Instant::now()).clock())
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
const TABLE_SCRIPT:&str=r#"
import "components/table" as table;
fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn resized(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn view(ctx){column([
text(`${ctx.get_state("count")}`).accessibility_role("status"),
table::Table(#{key:"table",label:"Audit table",row_key:"id",rows:[#{id:"one",left:"Left",right:"Right"}],
 columns:[#{key:"left",title:"Left",width:#{kind:"fixed",value:160.0},max_width:240.0},#{key:"right",title:"Right",width:#{kind:"flex",value:1.0}}],height:140.0,resizable_columns:true,on_column_resize:Fn("resized")})
]).with_style(style().width(px(500)))}
"#;
const RANGE_SCRIPT:&str=r#"
import "components/range_slider" as range_slider;
fn noop(ctx,value){()}
fn view(ctx){range_slider::RangeSlider(#{key:"range",label:"Range",values:#{low:20.0,high:80.0},
 min:0.0,max:100.0,step:10.0,minimum_gap:5.0})}
"#;
fn thumb_quads(v:&mut VisualTestContext)->Vec<f32>{
 let mut x=v.update(|window,_|window.painted_quads().into_iter().filter(|q|q.border_widths.left.0>0.0 && (q.bounds.size.width.0-q.bounds.size.height.0).abs()<0.1).map(|q|q.bounds.origin.x.0).collect::<Vec<_>>());x.sort_by(f32::total_cmp);x
}
fn range_keyboard_rejection(cx:&mut TestAppContext,callback:bool){
 cx.update(gpui_rhai::install);let script=if callback{RANGE_SCRIPT.replace("minimum_gap:5.0","minimum_gap:5.0,on_change:Fn(\"noop\")")}else{RANGE_SCRIPT.to_owned()};let(window,view)=mount(cx,&script,if callback{"r4-range-noop"}else{"r4-range-unobserved"});let mut v=VisualTestContext::from_window(*window,cx);
 let b=bounds(&mut v,&view,"group","Range");let p=point(px((b.x+b.width*0.8) as f32),px((b.y+b.height*0.7) as f32));
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let before=thumb_quads(&mut v);assert_eq!(before.len(),2,"exactly two painted thumb border quads required");
 v.simulate_event(gpui::KeyDownEvent{keystroke:gpui::Keystroke::parse("right").unwrap(),is_held:false,prefer_character_input:false});v.run_until_parked();
 let after=thumb_quads(&mut v);println!("callback={callback} thumb x before={before:?},after={after:?}");assert_eq!(after,before,"unaccepted controlled keyboard proposal must not become a persistent native value");
}
#[gpui::test]
fn range_without_callback_stays_controlled_on_keyboard(cx:&mut TestAppContext){range_keyboard_rejection(cx,false)}
#[gpui::test]
fn range_noop_callback_control(cx:&mut TestAppContext){range_keyboard_rejection(cx,true)}
const TABLE_CHANGE_SCRIPT:&str=r#"
import "components/table" as table;
fn state_schema(){#{fields:#{width:#{schema:#{type:"number"},"default":#{type:"float",value:160.0}}}}}
fn replace(ctx,p){ctx.set_state("width",180.0);}
fn view(ctx){column([
text(`${ctx.get_state("width")}`).accessibility_role("status"),
text("Replace").accessibility_role("button").accessibility_label("Replace").on_click(Fn("replace")),
table::Table(#{key:"table",label:"Table",row_key:"id",rows:[#{id:"row",left:"Left",right:"Right"}],
 columns:[#{key:"left",title:"Left",width:#{kind:"fixed",value:ctx.get_state("width")},max_width:300.0},#{key:"right",title:"Right",width:#{kind:"flex",value:1.0}}],height:140.0,resizable_columns:true})
]).with_style(style().width(px(500)))}
"#;
#[gpui::test]
fn new_table_source_wins_over_prior_uncontrolled_override_on_cancel(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let(window,view)=mount(cx,TABLE_CHANGE_SCRIPT,"r4-table-new-source");let mut v=VisualTestContext::from_window(*window,cx);
 let before=bounds(&mut v,&view,"separator","Resize Left column");let p=point(px((before.x+before.width/2.0) as f32),px((before.y+before.height/2.0) as f32));let q=point(p.x+px(40.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let accepted=bounds(&mut v,&view,"separator","Resize Left column");assert!((accepted.x-before.x-40.0).abs()<0.1);
 let p=point(px((accepted.x+accepted.width/2.0) as f32),px((accepted.y+accepted.height/2.0) as f32));let q=point(p.x+px(30.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());
 dispatch(&mut v,&view,"button","Replace","click").unwrap();v.run_until_parked();v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let after=bounds(&mut v,&view,"separator","Resize Left column");println!("table source180 after old override200: initial_x={},final_x={},source={}",before.x,after.x,status(&mut v,&view));assert!((after.x-before.x-20.0).abs()<0.1,"new source180 must win over restored old local override200");
}
fn range_end_key(cx:&mut TestAppContext,vertical:bool){
 cx.update(gpui_rhai::install);
 let orientation=if vertical{"vertical"}else{"horizontal"};
 let script=format!(r#"
import "components/range_slider" as range_slider;
fn state_schema(){{#{{fields:#{{last:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"none"}}}}}}}}}}
fn changed(ctx,v){{ctx.set_state("last",`${{v.low}},${{v.high}}`);}}
fn view(ctx){{column([text(ctx.get_state("last")).accessibility_role("status"),range_slider::RangeSlider(#{{key:"range",label:"Range",values:#{{low:20.0,high:80.0}},min:0.0,max:97.0,step:10.0,minimum_gap:5.0,orientation:"{orientation}",on_change:Fn("changed")}})])}}
"#);
 let(window,view)=mount(cx,&script,if vertical{"r4-range-end-vertical"}else{"r4-range-end-horizontal"});let mut v=VisualTestContext::from_window(*window,cx);
 let b=bounds(&mut v,&view,"group","Range");let p=if vertical{point(px((b.x+b.width*0.7) as f32),px((b.y+b.height*(1.0-80.0/97.0)) as f32))}else{point(px((b.x+b.width*80.0/97.0) as f32),px((b.y+b.height*0.7) as f32))};
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();
 v.simulate_event(gpui::KeyDownEvent{keystroke:gpui::Keystroke::parse("end").unwrap(),is_held:false,prefer_character_input:false});v.run_until_parked();
 let actual=status(&mut v,&view);println!("RangeSlider {orientation} End={actual}");assert_eq!(actual,"20.0,97.0");
}
#[gpui::test]
fn range_horizontal_end_uses_same_endpoint_domain(cx:&mut TestAppContext){range_end_key(cx,false)}
#[gpui::test]
fn range_vertical_end_uses_same_endpoint_domain(cx:&mut TestAppContext){range_end_key(cx,true)}
#[gpui::test]
fn narrowed_table_constraint_wins_over_previous_native_override(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);
 let script=TABLE_CHANGE_SCRIPT.replace("value:160.0}}}}}","value:300.0}}}}}").replace("value:ctx.get_state(\"width\")","value:160.0").replace("max_width:300.0","max_width:ctx.get_state(\"width\")");
 let(window,view)=mount(cx,&script,"r4-table-new-constraint");let mut v=VisualTestContext::from_window(*window,cx);
 let before=bounds(&mut v,&view,"separator","Resize Left column");let p=point(px((before.x+before.width/2.0) as f32),px((before.y+before.height/2.0) as f32));let q=point(p.x+px(40.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let accepted=bounds(&mut v,&view,"separator","Resize Left column");assert!((accepted.x-before.x-40.0).abs()<0.1);
 let p=point(px((accepted.x+accepted.width/2.0) as f32),px((accepted.y+accepted.height/2.0) as f32));let q=point(p.x+px(30.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());dispatch(&mut v,&view,"button","Replace","click").unwrap();v.run_until_parked();v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let after=bounds(&mut v,&view,"separator","Resize Left column");println!("Table max180 overrides old accepted200: initial_x={},final_x={},max={}",before.x,after.x,status(&mut v,&view));assert!(after.x-before.x<=20.1,"previous native200 must not survive the new180 maximum");
}
