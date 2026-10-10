//! Direct-manipulation and text-field contracts of the native primitives: a no-op
//! interaction emits nothing, horizontal sorting follows RTL, and a field without a
//! submit handler leaves Enter to its parent.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
};
use gpui_rhai::*;

const MODULES: [(&str, &str); 6] = [
    (
        "components/split_pane",
        include_str!("../../../registry/components/split_pane.rhai"),
    ),
    (
        "components/selection_area",
        include_str!("../../../registry/components/selection_area.rhai"),
    ),
    (
        "components/sortable",
        include_str!("../../../registry/components/sortable.rhai"),
    ),
    (
        "components/input",
        include_str!("../../../registry/components/input.rhai"),
    ),
    (
        "components/slider",
        include_str!("../../../registry/components/slider.rhai"),
    ),
    (
        "components/range_slider",
        include_str!("../../../registry/components/range_slider.rhai"),
    ),
];

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if self.view.state() == gpui_rhai::ScriptViewState::Active {
            self.host.container(self.view.element().unwrap())
        } else {
            self.host.container(gpui::div())
        }
    }
}

fn mount(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut sources = BTreeMap::from([(entry.clone(), script.to_owned())]);
    for (id, source) in MODULES {
        sources.insert(ModuleId::parse(id).unwrap(), source.to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(sources),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .locale_sources([
        (
            "en.rhai".to_owned(),
            include_str!("../../../registry/locales/en.rhai").to_owned(),
        ),
        (
            "ar.rhai".to_owned(),
            include_str!("../../../registry/locales/ar.rhai").to_owned(),
        ),
    ])
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
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    })
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} named {name}"))
            .geometry
            .unwrap()
            .visual
    })
}

#[allow(clippy::cast_possible_truncation)]
fn at(bounds: GeometryBounds, x: f64, y: f64) -> gpui::Point<gpui::Pixels> {
    point(
        px((bounds.x + bounds.width * x) as f32),
        px((bounds.y + bounds.height * y) as f32),
    )
}

fn click(visual: &mut VisualTestContext, position: gpui::Point<gpui::Pixels>) {
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
}

fn dispatch_key(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
    key: &str,
) -> Result<AutomationResult, ScriptViewError> {
    let result = visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::RoleName {
                    role: role.to_owned(),
                    name: name.to_owned(),
                },
                event: format!("key:{key}"),
                payload: None,
            },
            window,
            cx,
        )
    });
    visual.run_until_parked();
    result
}

#[gpui::test]
fn split_pane_key_step_against_a_limit_proposes_nothing(cx: &mut TestAppContext) {
    let script = r#"
import "components/split_pane" as split_pane;
fn state_schema(){#{fields:#{size:#{schema:#{type:"number",min:0.0,max:1.0},
    "default":#{type:"float",value:0.0}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn resized(ctx,value){ctx.set_state("size",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([
    text(`${ctx.get_state("commits")}`).accessibility_role("status"),
    split_pane::SplitPane(#{key:"layout",label:"Resize panels",size:ctx.get_state("size"),
        min_start:80.0,min_end:80.0,start:text("Start"),end:text("End"),on_resize:Fn("resized")})
        .with_style(style().width(px(420)).height(px(180))),
    split_pane::SplitPane(#{key:"locked",label:"Locked panels",size:0.5,disabled:true,
        start:text("Start"),end:text("End"),on_resize:Fn("resized")})
        .with_style(style().width(px(420)).height(px(180)))
])}
"#;
    let (window, view) = mount(cx, script, "split-pane-no-op");
    let mut visual = VisualTestContext::from_window(*window, cx);
    // The start pane already sits at `min_start`, so a step towards the start changes nothing.
    dispatch_key(&mut visual, &view, "separator", "Resize panels", "left").unwrap();
    assert_eq!(status(&mut visual, &view), "0");
    // A step away from the limit still proposes.
    dispatch_key(&mut visual, &view, "separator", "Resize panels", "right").unwrap();
    assert_eq!(status(&mut visual, &view), "1");
    // A disabled separator takes no key steps.
    assert!(dispatch_key(&mut visual, &view, "separator", "Locked panels", "right").is_err());
    let locked = bounds(&mut visual, &view, "separator", "Locked panels");
    click(&mut visual, at(locked, 0.5, 0.5));
    visual.simulate_keystrokes("right left");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "1");
}

#[gpui::test]
fn split_pane_drag_that_ends_where_it_started_proposes_nothing(cx: &mut TestAppContext) {
    let script = r#"
import "components/split_pane" as split_pane;
fn state_schema(){#{fields:#{commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn resized(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([
    text(`${ctx.get_state("commits")}`).accessibility_role("status"),
    split_pane::SplitPane(#{key:"layout",label:"Resize panels",size:0.5,
        min_start:80.0,min_end:80.0,start:text("Start"),end:text("End"),on_resize:Fn("resized")})
        .with_style(style().width(px(420)).height(px(180)))
])}
"#;
    let (window, view) = mount(cx, script, "split-pane-drag-no-op");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let separator = bounds(&mut visual, &view, "separator", "Resize panels");
    let start = at(separator, 0.5, 0.5);
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(
        point(start.x + px(40.0), start.y),
        MouseButton::Left,
        Modifiers::default(),
    );
    visual.simulate_mouse_move(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(start, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "0");
}

#[gpui::test]
fn selection_area_click_on_the_only_selected_target_proposes_nothing(cx: &mut TestAppContext) {
    let script = r#"
import "components/selection_area" as selection_area;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    anchor:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,value){ctx.set_state("selected",value.selected_keys);ctx.set_state("active",value.active_key);
    ctx.set_state("anchor",value.anchor_key);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){let targets=[#{key:"a",x:20.0,y:20.0,width:50.0,height:40.0},
    #{key:"b",x:100.0,y:20.0,width:50.0,height:40.0}];
    column([text(`${ctx.get_state("commits")}`).accessibility_role("status"),
        selection_area::SelectionArea(#{key:"area",label:"Node selection",targets:targets,
            selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),anchor_key:ctx.get_state("anchor"),
            content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,50.0,40.0,theme_color("accent")),
                canvas_rect("b",100.0,20.0,50.0,40.0,theme_color("warning"))])).with_key("canvas"),
            on_selection_change:Fn("changed")}).with_style(style().width(px(200)).height(px(100)))
    ]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "selection-area-no-op");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let area = bounds(&mut visual, &view, "grid", "Node selection");
    let a = point(px((area.x + 45.0) as f32), px((area.y + 40.0) as f32));
    click(&mut visual, a);
    assert_eq!(status(&mut visual, &view), "1");
    click(&mut visual, a);
    assert_eq!(
        status(&mut visual, &view),
        "1",
        "a repeat click changes nothing"
    );
    let b = point(px((area.x + 125.0) as f32), px((area.y + 40.0) as f32));
    click(&mut visual, b);
    assert_eq!(status(&mut visual, &view), "2");
}

const RTL_SORTABLE: &str = r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{
    order:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[
        #{type:"string",value:"alpha"},#{type:"string",value:"bravo"},
        #{type:"string",value:"charlie"},#{type:"string",value:"delta"}]}},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}
}}}
fn init(ctx){ctx.set_locale("ar");}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let items=[];for key in ctx.get_state("order"){
    items.push(#{key:key,label:key,content:text(key).with_style(style().width(px(64)).height(px(40)).items_center())});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"queue",label:"Queue",items:items,direction:"horizontal",on_reorder:Fn("reordered")})
    ]).with_style(style().width(px(560)).padding(px(12)).gap(px(8)))}
"#;

#[gpui::test]
fn rtl_horizontal_sortable_drops_on_the_side_the_pointer_is_on(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, RTL_SORTABLE, "rtl-sortable-pointer");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let alpha = bounds(&mut visual, &view, "listitem", "alpha");
    let bravo = bounds(&mut visual, &view, "listitem", "bravo");
    assert!(
        alpha.x > bravo.x,
        "the row runs right to left: {alpha:?} {bravo:?}"
    );
    // Drag delta (the leftmost row) onto bravo's right half: between alpha and bravo,
    // that is before bravo in the logical order.
    let source = at(
        bounds(&mut visual, &view, "button", "Reorder delta"),
        0.5,
        0.5,
    );
    let target = at(bravo, 0.85, 0.5);
    visual.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "delta:before:bravo");
}

#[gpui::test]
fn rtl_horizontal_sortable_alt_arrows_move_the_way_they_point(cx: &mut TestAppContext) {
    let (window, view) = mount(cx, RTL_SORTABLE, "rtl-sortable-keys");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let target = bounds(&mut visual, &view, "button", "Reorder alpha");
    click(&mut visual, at(target, 0.5, 0.5));
    // Alpha is the rightmost row; Alt+Left moves it left, past bravo.
    visual.simulate_keystrokes("alt-left");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "alpha:after:bravo");
}

#[gpui::test]
fn input_without_on_submit_leaves_enter_to_its_parent(cx: &mut TestAppContext) {
    let script = r#"
import "components/input" as input;
fn state_schema(){#{fields:#{log:#{schema:#{type:"string"},"default":#{type:"string",value:"log"}}}}}
fn changed(ctx,value){}
fn parent_enter(ctx,value){ctx.set_state("log",`${ctx.get_state("log")} parent:${value}`);}
fn submitted(ctx,value){ctx.set_state("log",`${ctx.get_state("log")} submit`);}
fn view(ctx){column([
    text(ctx.get_state("log")).accessibility_role("status"),
    column([input::Input(#{key:"plain",label:"Plain field",value:"",on_change:Fn("changed")})])
        .on_key_value("enter",Fn("parent_enter"),"plain"),
    column([input::Input(#{key:"submitting",label:"Submitting field",value:"",
        on_change:Fn("changed"),on_submit:Fn("submitted")})])
        .on_key_value("enter",Fn("parent_enter"),"submitting")
]).with_style(style().width(px(320)).padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "input-enter");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let target = bounds(&mut visual, &view, "text_field", "Plain field");
    click(&mut visual, at(target, 0.5, 0.5));
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "log parent:plain");
    let target = bounds(&mut visual, &view, "text_field", "Submitting field");
    click(&mut visual, at(target, 0.5, 0.5));
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "log parent:plain submit");
}

#[gpui::test]
fn slider_keys_and_clicks_that_keep_the_value_propose_nothing(cx: &mut TestAppContext) {
    let script = r#"
import "components/slider" as slider;
fn state_schema(){#{fields:#{value:#{schema:#{type:"number"},"default":#{type:"float",value:100.0}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("value",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([
    text(`${ctx.get_state("value")},${ctx.get_state("commits")}`).accessibility_role("status"),
    slider::Slider(#{key:"volume",label:"Volume",value:ctx.get_state("value"),min:0.0,max:100.0,
        step:5.0,on_change:Fn("changed")})
]).with_style(style().width(px(320)).padding(px(20)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "slider-no-op");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let track = bounds(&mut visual, &view, "slider", "Volume");
    // A click on the thumb's own value leaves it there.
    click(&mut visual, at(track, 0.999, 0.5));
    assert_eq!(status(&mut visual, &view), "100.0,0");
    visual.simulate_keystrokes("end right up");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "100.0,0");
    visual.simulate_keystrokes("left");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "95.0,1");
}

#[gpui::test]
fn range_slider_click_on_a_thumb_proposes_nothing(cx: &mut TestAppContext) {
    let script = r#"
import "components/range_slider" as range_slider;
fn state_schema(){#{fields:#{range:#{schema:#{type:"object",allow_unknown:false,fields:#{
    low:#{schema:#{type:"number"},required:true,sensitive:false},
    high:#{schema:#{type:"number"},required:true,sensitive:false}}},
    "default":#{type:"map",value:#{low:#{type:"float",value:20.0},high:#{type:"float",value:100.0}}}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("range",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){let value=ctx.get_state("range");column([
    text(`${value.low},${value.high},${ctx.get_state("commits")}`).accessibility_role("status"),
    range_slider::RangeSlider(#{key:"range",label:"Accepted interval",low_label:"Low bound",
        high_label:"High bound",values:value,min:0.0,max:100.0,step:5.0,minimum_gap:10.0,
        on_change:Fn("changed")})
]).with_style(style().width(px(320)).padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "range-slider-no-op");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let control = bounds(&mut visual, &view, "group", "Accepted interval");
    click(&mut visual, at(control, 0.999, 0.7));
    assert_eq!(status(&mut visual, &view), "20.0,100.0,0");
    visual.simulate_keystrokes("end");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "20.0,100.0,0");
}
