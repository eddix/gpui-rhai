use gpui::{
    Context, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, Window, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Instant};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
#[gpui::test]
fn morphed_path_hit_follows_presented_endpoint(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let source = r#"
 fn state_schema(){#{fields:#{hit:#{schema:#{type:"string"},"default":#{type:"string",value:"unseen"}}}}}
 fn hit(ctx,e){ctx.set_state("hit",if e.canvas_key==(){"none"}else{e.canvas_key});}
 fn view(ctx){column([
 canvas(canvas_scene([canvas_morph_stroke_path("shape",[path_move(20.0,20.0),path_line(180.0,20.0)],[path_move(20.0,80.0),path_line(180.0,80.0)],10.0,theme_color("accent"))]))
 .with_key("canvas").accessibility_role("button").accessibility_label("Canvas")
 .with_style(style().width(px(200)).height(px(100))).on("pointer_down",Fn("hit"))
 .motion(motion_transition("path_progress",1.0,1.0,#{duration_ms:1000,easing:"linear"})),
 text(ctx.get_state("hit")).with_key("result").accessibility_role("status").accessibility_label(ctx.get_state("hit"))
 ])}
 "#;
    let entry = ModuleId::parse("main").unwrap();
    let theme = std::fs::read_to_string(format!(
        "{}/registry/themes/default_dark.rhai",
        std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()
    ))
    .unwrap();
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
        theme,
    )
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let cap = Rc::new(RefCell::new(None));
    let take = cap.clone();
    let w = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("w", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("v"), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = cap.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*w, cx);
    let b = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Canvas")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    visual.simulate_click(
        point(px((b.x + 100.) as f32), px((b.y + 80.) as f32)),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let end_hit = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "shape")
            .next()
            .is_some()
    });
    let end_missed = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "none")
            .next()
            .is_some()
    });
    visual.simulate_click(
        point(px((b.x + 100.) as f32), px((b.y + 20.) as f32)),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let old_hit = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("status", "shape")
            .next()
            .is_some()
    });
    println!(
        "path_progress=1 painted_y=80: hit_at_y80={end_hit} missed_at_y80={end_missed}; original_y20_still_hits={old_hit}"
    );
    assert!(
        end_hit,
        "the painted endpoint must be interactive, but hit testing still uses the original path"
    );
    assert!(!old_hit, "the old path must stop being interactive");
}

#[gpui::test]
fn timeline_rebinds_a_replaced_child_in_presented_frames(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let source = r#"
    fn state_schema(){#{fields:#{switched:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
    fn replace_target(ctx,event){ctx.set_state("switched",true);}
    fn repaint(ctx,event){}
    fn init(ctx){ctx.register_action("audit.replace",Fn("replace_target"));ctx.register_action("audit.repaint",Fn("repaint"));}
    fn view(ctx){let child=if ctx.get_state("switched"){box([])}else{text("Title")};
      column([child.with_key("title").accessibility_role("button").accessibility_label("Title")
        .with_style(style().width(px(100)).height(px(30)))])
        .with_key("panel").timeline(motion_timeline("intro",motion_track("title",
        motion_transition("width",100.0,200.0,#{duration_ms:1000,easing:"linear"})),#{}))
    }
    "#;
    let entry = ModuleId::parse("main").unwrap();
    let theme = std::fs::read_to_string(format!(
        "{}/registry/themes/default_dark.rhai",
        std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()
    ))
    .unwrap();
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
        theme,
    )
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let cap = Rc::new(RefCell::new(None));
    let take = cap.clone();
    let w = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("w", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("v"), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = cap.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*w, cx);
    let initial = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Title")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
            .width
    });
    let mut widths = vec![initial];
    for id in ["audit.replace", "audit.repaint"] {
        clock.advance(std::time::Duration::from_millis(250));
        visual
            .update(|window, cx| {
                view.automate(
                    AutomationCommand::Action {
                        id: id.to_owned(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        visual.run_until_parked();
        cx.refresh().unwrap();
        visual.run_until_parked();
        widths.push(visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", "Title")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
                .width
        }));
    }
    println!("target replacement presented widths at 0/250/500 ms = {widths:?}");
    assert!(
        widths[2] > 110.0,
        "the current child must keep receiving the declared width track after its retained NodeId changes"
    );
}
