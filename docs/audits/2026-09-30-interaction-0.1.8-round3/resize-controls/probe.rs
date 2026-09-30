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
                if name.ends_with("identity-control") { RANGE_SLIDER.replace("RangeSliderPrimitive(#{key:props.key,", "RangeSliderPrimitive(#{key:`${props.key}-range`,") } else { RANGE_SLIDER.to_owned() },
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
fn state_schema(){#{fields:#{last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn changed(ctx,v){ctx.set_state("last",`${v.low},${v.high}`);}
fn view(ctx){column([text(ctx.get_state("last")).accessibility_role("status"),
range_slider::RangeSlider(#{key:"range",label:"Range",values:#{low:20.0,high:80.0},
min:0.0,max:97.0,step:10.0,minimum_gap:5.0,on_change:Fn("changed")})])}
"#;
#[gpui::test]
fn range_pointer_agrees_with_the_accepted_off_grid_endpoint(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let script=RANGE_SCRIPT.replace("high:80.0","high:97.0");let(window,view)=mount(cx,&script,"r3-range-end-pointer");let mut v=VisualTestContext::from_window(*window,cx);
 let b=bounds(&mut v,&view,"group","Range");let p=point(px((b.x+b.width-0.1) as f32),px((b.y+b.height*0.7) as f32));
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let actual=status(&mut v,&view);println!("off-grid pointer endpoint={actual}");assert_eq!(actual,"20.0,97.0","pointer solver must preserve the same accepted endpoint as pair normalization");
}
#[gpui::test]
fn range_aligned_endpoint_control(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let script=RANGE_SCRIPT.replace("max:97.0","max:100.0");let(window,view)=mount(cx,&script,"r3-range-end-control");let mut v=VisualTestContext::from_window(*window,cx);
 let b=bounds(&mut v,&view,"group","Range");let p=point(px((b.x+b.width-0.1) as f32),px((b.y+b.height*0.7) as f32));
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();assert_eq!(status(&mut v,&view),"20.0,100.0");
}
#[gpui::test]
fn table_uncontrolled_cancel_restores_last_accepted_width(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);let script=TABLE_SCRIPT.replace(",on_column_resize:Fn(\"resized\")","");
 let(window,view)=mount(cx,&script,"r3-table-uncontrolled-cancel");let mut v=VisualTestContext::from_window(*window,cx);
 v.update(|window,cx|view.focus(window,cx)).unwrap();
 let b=bounds(&mut v,&view,"separator","Resize Left column");let p=point(px((b.x+b.width/2.0) as f32),px((b.y+b.height/2.0) as f32));let q=point(p.x+px(40.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(q,MouseButton::Left,Modifiers::none());v.run_until_parked();
 let accepted=bounds(&mut v,&view,"separator","Resize Left column");println!("uncontrolled accepted initial_x={},accepted_x={}",b.x,accepted.x);assert!((accepted.x-b.x-40.0).abs()<0.1);
 let p=point(px((accepted.x+accepted.width/2.0) as f32),px((accepted.y+accepted.height/2.0) as f32));let q=point(p.x+px(20.0),p.y);
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_move(q,MouseButton::Left,Modifiers::none());v.simulate_keystrokes("escape");v.run_until_parked();
 let cancelled=bounds(&mut v,&view,"separator","Resize Left column");println!("uncontrolled after cancel_x={}",cancelled.x);
 assert!((cancelled.x-accepted.x).abs()<0.1,"cancel must preserve the last locally accepted width, not erase it back to initial props");
}
#[gpui::test]
fn resizable_keyboard_uses_current_boundary_after_window_resize(cx:&mut TestAppContext){
 cx.update(gpui_rhai::install);
 let script=r#"
import "components/resizable" as resizable;
fn state_schema(){#{fields:#{last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
fn resized(ctx,v){ctx.set_state("last",`${v.width}`);}
fn view(ctx){box([
 text(ctx.get_state("last")).accessibility_role("status"),
 resizable::Resizable(#{key:"card",label:"Card",rect:#{x:100.0,y:80.0,width:200.0,height:120.0},handles:["e"],min_width:80.0,min_height:60.0,max_width:1000.0,max_height:1000.0,content:text("Body"),on_resize:Fn("resized")})
 .with_style(style().width(relative(1.0)).height(relative(1.0)))
]).with_style(style().width(relative(1.0)).height(relative(1.0)))}
"#;
 let(window,view)=mount(cx,script,"r3-resize-boundary");let mut v=VisualTestContext::from_window(*window,cx);
 v.simulate_resize(gpui::size(px(500.0),px(400.0)));v.run_until_parked();
 v.simulate_resize(gpui::size(px(305.0),px(400.0)));v.run_until_parked();
 dispatch(&mut v,&view,"separator","Card: e resize handle","key:right").unwrap();v.run_until_parked();
 let actual=status(&mut v,&view);println!("resizable narrowed boundary proposal={actual}");assert_eq!(actual,"205.0","keyboard must clamp to current 305px boundary minus100px x");
}
fn range_keyboard_focus_sequence(cx:&mut TestAppContext,name:&str)->String{
 cx.update(gpui_rhai::install);let script=RANGE_SCRIPT.replace("max:97.0","max:100.0");let(window,view)=mount(cx,&script,name);let mut v=VisualTestContext::from_window(*window,cx);
 for frame in 0..3 {
  let before=v.update(|window,cx| {window.focus_next(cx);window.focused(cx).map(|focus|format!("{focus:?}"))});
  v.run_until_parked();v.update(|window,_|window.refresh());v.run_until_parked();
  let after=v.update(|window,cx|window.focused(cx).map(|focus|format!("{focus:?}")));
  println!("{name}: focus frame {frame} before={before:?} after={after:?}");
 }

 let b=bounds(&mut v,&view,"group","Range");let p=point(px((b.x+b.width*0.8) as f32),px((b.y+b.height*0.7) as f32));
 v.simulate_mouse_down(p,MouseButton::Left,Modifiers::none());v.simulate_mouse_up(p,MouseButton::Left,Modifiers::none());v.run_until_parked();
 println!("{name}: before-key {}",status(&mut v,&view));
 v.simulate_event(gpui::KeyDownEvent{keystroke:gpui::Keystroke::parse("right").unwrap(),is_held:false,prefer_character_input:false});v.run_until_parked();
 let actual=status(&mut v,&view);println!("{name}: after-key {actual}");actual
}
#[gpui::test]
fn range_keyboard_focus_survives_native_repaint(cx:&mut TestAppContext){
 assert_eq!(range_keyboard_focus_sequence(cx,"r3-range-identity-product"),"20.0,90.0","public RangeSlider must preserve the focused high thumb after painting");
}
#[gpui::test]
fn range_equal_native_and_retained_key_diagnostic_control(cx:&mut TestAppContext){
 assert_eq!(range_keyboard_focus_sequence(cx,"r3-range-identity-control"),"20.0,90.0","probe-only equal-key substitution isolates native identity mismatch");
}
