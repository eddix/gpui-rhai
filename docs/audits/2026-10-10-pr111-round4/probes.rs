//! TabBar: document tabs that belong to the panel under them. The selected tab takes the
//! panel's surface and covers the bar's line; a press selects, the close button and a
//! middle press close, the keys move a cursor that Enter selects, a right press or
//! Shift+F10 asks for a menu, tabs reorder by drag or Alt+Arrow, the strip scrolls (also
//! with a vertical wheel) and keeps the selected tab revealed, and an optional menu lists
//! every tab.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render,
    TestAppContext, VisualTestContext, Window, point, px,
};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// A bar `width` wide over `count` tabs (t0..), the selected value in state; `extra` adds
/// TabBar props. The log line records events.
fn script(count: usize, width: u32, extra: &str) -> String {
    format!(
        r#"import "components/tab_bar" as tab_bar;
fn state_schema(){{#{{fields:#{{
    value:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"t1"}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn changed(ctx,value){{ctx.set_state("value",value);note(ctx,`change:${{value}}`);}}
fn closed(ctx,value){{note(ctx,`close:${{value}}`);}}
fn asked(ctx,request){{note(ctx,`ctx:${{request.value}}:${{request.source}}`);}}
fn moved(ctx,request){{note(ctx,`move:${{request.value}}:${{request.placement}}:${{request.anchor}}`);}}
fn tabs(){{let result=[];for index in 0..{count}{{
    result.push(#{{value:`t${{index}}`,label:`Tab number ${{index}}`,closable:index<3,dirty:index==2}});}}result}}
fn view(ctx){{column([
    tab_bar::TabBar(#{{key:"files",label:"Open files",value:ctx.get_state("value"),tabs:tabs(),
        on_change:Fn("changed"),on_close:Fn("closed"),on_context_request:Fn("asked"),on_reorder:Fn("moved")
        {extra}}}).with_style(style().width(px({width}))),
    text(ctx.get_state("log")).test_id("log"),
])}}"#
    )
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .motion_preference(MotionPreference::None)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("tabs", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("tabs"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.test_id.as_deref() == Some("log"))
            .unwrap()
            .name
            .trim()
            .to_owned()
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
            .and_then(|node| node.geometry)
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .visual
    })
}

fn center(bounds: GeometryBounds) -> gpui::Point<gpui::Pixels> {
    #[allow(clippy::cast_possible_truncation)]
    point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    )
}

fn press(visual: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, button: MouseButton) {
    visual.simulate_mouse_move(at, None, Modifiers::none());
    visual.simulate_mouse_down(at, button, Modifiers::none());
    visual.simulate_mouse_up(at, button, Modifiers::none());
    settle(visual);
}



#[gpui::test]
fn empty_tab_bar_mounts_without_error(cx: &mut TestAppContext) {
    let source = script(0, 360, "").replace("value:\"t1\"", "value:\"\"");
    let (mut visual, view) = mount(cx, source);
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    assert!(error.is_none(), "empty TabBar must be valid: {error:?}");
}

#[gpui::test]
fn closing_last_tab_commits_empty_state(cx: &mut TestAppContext) {
    let source = script(1, 360, "")
        .replace("value:\"t1\"", "value:\"t0\"")
        .replace("tabs:tabs()", "tabs:if ctx.get_state(\"value\") == \"\" { [] } else { tabs() }")
        .replace("fn closed(ctx,value){note(ctx,`close:${value}`);}",
          "fn closed(ctx,value){ctx.set_state(\"value\",\"\");note(ctx,`close:${value}`);}");
    assert!(source.contains("ctx.set_state(\"value\",\"\")"));
    let (mut visual, view) = mount(cx, source);
    let close = bounds(&mut visual, &view, "button", "Close Tab number 0");
    press(&mut visual, center(close), MouseButton::Left);
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    assert!(error.is_none(), "closing final tab should commit: {error:?}");
    let count = visual.update(|_,cx| view.accessibility_snapshot(cx).unwrap().nodes().filter(|n| n.role == "tab").count());
    assert_eq!(count,0,"last tab removed");
}

#[gpui::test]
fn reorder_keeps_selected_tab_in_view(cx: &mut TestAppContext) {
    let source = script(12, 360, ",reorderable:true")
        .replace("log:#{schema", "reversed:#{schema:#{type:\"bool\"},\"default\":#{type:\"bool\",value:false}},log:#{schema")
        .replace("fn tabs()", "fn reverse_tabs(ctx,payload){ctx.set_state(\"reversed\",true);}\nfn tabs()")
        .replace("tabs:tabs()", "tabs:ordered_tabs(ctx)")
        .replace("fn view(ctx)", "fn ordered_tabs(ctx){let values=tabs(); if ctx.get_state(\"reversed\") {values.reverse();} values}\nfn view(ctx)")
        .replace("text(ctx.get_state(\"log\")).test_id(\"log\"),", "text(ctx.get_state(\"log\")).test_id(\"log\"), text(\"Reverse\").accessibility_role(\"button\").on_click(Fn(\"reverse_tabs\")),");
    let (mut visual, view) = mount(cx, source);
    let strip = bounds(&mut visual,&view,"tablist","Open files");
    let before = bounds(&mut visual,&view,"tab","Tab number 1");
    assert!(before.x+before.width <= strip.x+strip.width+0.5,"initial selected visible {before:?}");
    let reverse = bounds(&mut visual,&view,"button","Reverse");
    press(&mut visual,center(reverse),MouseButton::Left);
    let selected = bounds(&mut visual,&view,"tab","Tab number 1");
    println!("selection after order change: {selected:?}, strip {strip:?}");
    assert!(selected.x >= strip.x-0.5 && selected.x+selected.width <= strip.x+strip.width+0.5,
      "selected tab must stay revealed after order change: {selected:?} in {strip:?}");
}

#[gpui::test]
fn disabled_tab_cannot_be_closed(cx: &mut TestAppContext) {
    let source=script(3,600,"").replace("closable:index<3", "disabled:index==1,closable:index<3");
    let (mut visual,view)=mount(cx,source);
    let close=bounds(&mut visual,&view,"button","Close Tab number 1");
    press(&mut visual,center(close),MouseButton::Left);
    assert_eq!(log(&mut visual,&view),"");
}

#[test]
fn generated_definitions_use_script_collection_types() {
    let runtime=RuntimeEngine::new();
    let api=gpui_rhai::script_docs::ScriptApi::from_engine(runtime.engine()).unwrap();
    let functions=api.functions.iter().filter(|f|f.name=="actions"||f.name=="date_info"||f.name=="date_month_grid").collect::<Vec<_>>();
    for f in &functions {println!("{} // {}",f.display(),f.signature);}
    println!("definitions: {}",runtime.definition_source().lines().filter(|l|l.contains("Dynamic>")).collect::<Vec<_>>().join("\n"));
    assert!(functions.iter().all(|f| f.return_type=="Array" || f.return_type=="Map"),"wrong script types: {functions:?}");
}

fn list_source(count:usize, selected:bool) -> String {
    format!(r#"import "components/list" as list;
fn view(ctx) {{
    let rows=[];let chosen=[];
    for i in 0..{count} {{let key=`r${{i}}`;rows.push(#{{key:key,title:key}});if {selected}{{chosen.push(key);}}}}
    list::List(#{{key:"rows",label:"Rows",items:rows,height:200.0,
        selection_mode:"multiple",selected_keys:chosen}})
        .with_style(style().width(px(360)))
}}"#)
}
#[gpui::test]
fn list_500_unselected_rows_control(cx:&mut TestAppContext) {
    let (mut visual, view)=mount(cx,list_source(500,false));
    assert!(visual.update(|_,cx|view.last_error(cx).unwrap()).is_none());
}
#[gpui::test]
fn list_500_selected_rows_do_not_expand_into_oversized_arrays(cx:&mut TestAppContext) {
    let (mut visual, view)=mount(cx,list_source(500,true));
    assert!(visual.update(|_,cx|view.last_error(cx).unwrap()).is_none());
}

#[test]
fn installed_skill_recipes_pass_check_as_written() {
    use gpui_rhai_cli::{BundledRegistry,Project};
    let registry=BundledRegistry::load().unwrap();
    let root=std::env::temp_dir().join(format!("gpui-rhai-review-recipes-{}",std::process::id()));
    assert!(!root.exists());
    std::fs::create_dir_all(&root).unwrap();
    let recipe=gpui_rhai_registry::BUNDLED_SKILL_FILES.iter().find(|(name,_)|*name=="gpui-rhai/references/recipes.md").unwrap().1;
    let entry=gpui_rhai_registry::BUNDLED_SKILL_FILES.iter().find(|(name,_)|*name=="gpui-rhai/SKILL.md").unwrap().1;
    let snippets=entry.split("```rhai\n").skip(1).chain(recipe.split("```rhai\n").skip(1));
    let mut failures=Vec::new();
    for (index,chunk) in snippets.enumerate() {
        let source=chunk.split("```").next().unwrap();
        let directory=root.join(format!("recipe-{index}"));
        std::fs::create_dir_all(directory.join("src")).unwrap();
        std::fs::write(directory.join("Cargo.toml"),"[package]\nname=\"skill-probe\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\n[dependencies]\n").unwrap();
        let project=Project::new(&directory);
        project.plan_init().unwrap().apply().unwrap();
        project.plan_skills(std::path::Path::new(".agents/skills")).unwrap().apply().unwrap();
        let imports=source.lines().filter_map(|l|l.trim().strip_prefix("import \"")).filter_map(|l|l.split('"').next()).map(|l|l.rsplit('/').next().unwrap().to_owned()).collect::<Vec<_>>();
        project.plan_add(&registry,&imports).unwrap().apply().unwrap();
        std::fs::write(directory.join("ui/main.rhai"),source).unwrap();
        let result=project.check();
        println!("skill snippet {index}: {result:?}");
        if let Err(error)=result {failures.push(format!("snippet {index}: {error}"));}
    }
    std::fs::remove_dir_all(&root).unwrap();
    assert!(failures.is_empty(),"copied skill recipes failed: {failures:?}");
}

#[gpui::test]
fn list_500_rows_with_20_selected_still_renders(cx:&mut TestAppContext) {
    let source=list_source(500,true).replace("if true{chosen.push(key);}","if i < 20 {chosen.push(key);}");
    assert!(source.contains("i < 20"));
    let (mut visual, view)=mount(cx,source);
    assert!(visual.update(|_,cx|view.last_error(cx).unwrap()).is_none());
}
