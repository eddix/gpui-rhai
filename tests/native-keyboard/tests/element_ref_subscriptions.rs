#[cfg(test)]
mod tests {
    use gpui::{
        Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
        Window, WindowHandle, point, px,
    };
    use gpui_rhai::*;
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
    const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    struct Host {
        host: ScriptViewHost,
        view: ScriptViewHandle,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.host
                .container(if self.view.state() == ScriptViewState::Suspended {
                    gpui::div().into_any_element()
                } else {
                    self.view.element().unwrap()
                })
        }
    }
    fn script(initial: i64, replace: bool) -> String {
        let src = r#"
define_component(#{metadata:#{id:"tests/producer","export":"Producer",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},target:#{schema:#{type:"ref"},required:true,sensitive:false}},state:#{fields:#{phase:#{schema:#{type:"integer"},"default":#{type:"integer",value:INITIAL}}}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Producer")});
fn advance(ctx,value){ctx.set_state("phase",ctx.get_state("phase")+1);}
fn render_Producer(ctx,props){let phase=ctx.get_state("phase");let children=[text("Advance").with_key("advance").accessibility_role("button").accessibility_label("Advance").with_style(style().width(px(120)).height(px(30))).on_click(Fn("advance"))];if phase>0&&phase<3{let node=if REPLACE&&phase==2{box([])}else{text("target")};children.push(node.with_key("target").with_ref(props.target).with_style(style().width(px(if phase==1{120}else{180})).height(px(20))).accessibility_role("image").accessibility_label("target"));}column(children)}
define_component(#{metadata:#{id:"tests/reader","export":"Reader",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},target:#{schema:#{type:"ref"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Reader")});
fn read_text(ctx,target){let facts=ctx.element_bounds(target);text(if facts==(){"pending"}else{`${facts.layout.width}`}).accessibility_role("status")}
fn render_Reader(ctx,props){read_text(ctx,props.target)}
define_component(#{metadata:#{id:"tests/provider","export":"Provider",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Provider")});
fn render_Provider(ctx,props){let target=element_ref("field");let producer=render_component("tests/producer",#{key:"producer",target:target});let reader=render_component("tests/reader",#{key:"reader",target:target});column([producer,reader])}
fn view(ctx){render_component("tests/provider",#{key:"provider"})}
"#;
        src.replace("INITIAL", &initial.to_string())
            .replace("REPLACE", if replace { "true" } else { "false" })
    }
    fn mount(
        cx: &mut TestAppContext,
        source: &str,
        name: &str,
    ) -> (WindowHandle<Host>, ScriptViewHandle) {
        cx.update(gpui_rhai::install);
        let entry = ModuleId::parse("main").unwrap();
        let prepared = EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
            std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap(),
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .prepare()
        .unwrap();
        let capture = Rc::new(RefCell::new(None));
        let save = capture.clone();
        let name = name.to_owned();
        let window = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new(&name, cx).unwrap();
            let view = prepared
                .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
                .unwrap();
            *save.borrow_mut() = Some(view.clone());
            Host { host, view }
        });
        cx.run_until_parked();
        cx.refresh().unwrap();
        let view = capture.borrow().clone().unwrap();
        (window, view)
    }
    fn settle(v: &mut VisualTestContext) {
        for _ in 0..4 {
            v.background_executor
                .advance_clock(std::time::Duration::from_millis(32));
            v.run_until_parked();
            v.update(|window, _| window.refresh());
        }
    }
    fn readout(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
        v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .filter(|n| n.role == "status")
                .map(|n| n.name.clone())
                .collect()
        })
    }
    fn advance(v: &mut VisualTestContext, view: &ScriptViewHandle) {
        click(v, view, "Advance");
    }

    fn click(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
        let b = v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("button", name)
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
        });
        let p = point(px((b.x + 30.) as f32), px((b.y + 15.) as f32));
        v.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
        v.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
        settle(v);
    }
    fn target_width(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<f64> {
        v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("image", "target")
                .next()
                .map(|n| n.geometry.unwrap().visual.width)
        })
    }

    fn target_id(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<u64> {
        v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("image", "target")
                .next()
                .map(|node| node.id.get())
        })
    }
    #[gpui::test]
    fn round7_pending_ref_wakes_formal_reader_when_producer_appears(cx: &mut TestAppContext) {
        let (w, view) = mount(cx, &script(0, false), "pending-formal");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["pending"]);
        advance(&mut v, &view);
        let status = readout(&mut v, &view);
        println!(
            "appearance target={:?}, reader={status:?}",
            target_width(&mut v, &view)
        );
        assert_eq!(status, vec!["120.0"]);
    }
    #[gpui::test]
    fn round7_ref_rebinding_to_new_node_preserves_reader(cx: &mut TestAppContext) {
        let (w, view) = mount(cx, &script(1, true), "ref-rebind");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        let before_id = target_id(&mut v, &view);
        advance(&mut v, &view);
        let after_id = target_id(&mut v, &view);
        println!("ref target identity changed {before_id:?} -> {after_id:?}");
        assert_ne!(before_id, after_id);
        let status = readout(&mut v, &view);
        println!(
            "replacement target={:?}, reader={status:?}",
            target_width(&mut v, &view)
        );
        assert_eq!(status, vec!["180.0"]);
    }
    #[gpui::test]
    fn round7_ref_removal_notifies_reader(cx: &mut TestAppContext) {
        let (w, view) = mount(cx, &script(1, false), "ref-remove");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        advance(&mut v, &view);
        assert_eq!(readout(&mut v, &view), vec!["180.0"]);
        advance(&mut v, &view);
        let status = readout(&mut v, &view);
        println!(
            "removed target={:?}, reader={status:?}",
            target_width(&mut v, &view)
        );
        assert_eq!(target_width(&mut v, &view), None);
        assert_eq!(status, vec!["pending"]);
    }
    #[gpui::test]
    fn round7_existing_node_geometry_change_control(cx: &mut TestAppContext) {
        let (w, view) = mount(cx, &script(1, false), "same-node-control");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        advance(&mut v, &view);
        println!(
            "same-node target={:?}, reader={:?}",
            target_width(&mut v, &view),
            readout(&mut v, &view)
        );
        assert_eq!(readout(&mut v, &view), vec!["180.0"]);
    }

    #[gpui::test]
    fn reader_retargets_ref_without_retaining_old_node_subscription(cx: &mut TestAppContext) {
        let source = script(1, true)
            .replace(
                "fn render_Reader(ctx,props){read_text(ctx,props.target)}",
                r#"fn choose(ctx,value){ctx.set_state("choice",!ctx.get_state("choice"));}
fn render_Reader(ctx,props){column([
 text("Switch").with_key("switch").accessibility_role("button").accessibility_label("Switch").with_style(style().width(px(120)).height(px(30))).on_click(Fn("choose")),
 read_text(ctx,if ctx.get_state("choice"){props.other}else{props.target})])}"#,
            )
            .replace(
                r#"state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Reader")"#,
                r#"state:#{fields:#{choice:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Reader")"#,
            )
            .replace(
                r#"target:#{schema:#{type:"ref"},required:true,sensitive:false}},state:"#,
                r#"target:#{schema:#{type:"ref"},required:true,sensitive:false},other:#{schema:#{type:"ref"},required:false,sensitive:false}},state:"#,
            )
            .replace(
                r#"let reader=render_component("tests/reader",#{key:"reader",target:target});column([producer,reader])"#,
                r#"let other=element_ref("other");let reader=render_component("tests/reader",#{key:"reader",target:target,other:other});column([producer,box([]).with_key("other").with_ref(other).with_style(style().width(px(200)).height(px(20))),reader])"#,
            );
        let (w, view) = mount(cx, &source, "ref-retarget");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        click(&mut v, &view, "Switch");
        assert_eq!(readout(&mut v, &view), vec!["200.0"]);
        advance(&mut v, &view);
        assert_eq!(target_width(&mut v, &view), Some(180.0));
        assert_eq!(readout(&mut v, &view), vec!["200.0"]);
        advance(&mut v, &view);
        assert_eq!(target_width(&mut v, &view), None);
        assert_eq!(readout(&mut v, &view), vec!["200.0"]);
        click(&mut v, &view, "Switch");
        assert_eq!(readout(&mut v, &view), vec!["pending"]);
    }

    #[gpui::test]
    fn failed_effect_commit_restores_ref_binding_and_native_subscription(cx: &mut TestAppContext) {
        let source = script(1, true)
            .replace(
                r#"parts:["root"]},render:Fn("render_Producer")"#,
                r#"parts:["root"],effects:["candidate"]},render:Fn("render_Producer")"#,
            )
            .replace(
                r#"let phase=ctx.get_state("phase");"#,
                r#"let phase=ctx.get_state("phase");effect("candidate",phase,Fn("candidate_start"),Fn("candidate_cleanup"));"#,
            )
            .replace(
                "fn view(ctx)",
                r#"fn candidate_start(ctx,value){if value==2{throw "ref-candidate-failure";}}
fn candidate_cleanup(ctx,value){}
fn view(ctx)"#,
            );
        let (w, view) = mount(cx, &source, "ref-rollback");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        let before = target_id(&mut v, &view);
        for _ in 0..2 {
            // Capture the failed transaction at its synchronous boundary.
            // A later successful independent geometry reader may clear the
            // view's last-error diagnostic without undoing that failure.
            let error = v
                .update(|window, cx| {
                    view.automate(
                        AutomationCommand::Dispatch {
                            locator: AutomationLocator::RoleName {
                                role: "button".into(),
                                name: "Advance".into(),
                            },
                            event: "click".into(),
                            payload: None,
                        },
                        window,
                        cx,
                    )
                })
                .unwrap_err();
            assert!(
                error.to_string().contains("ref-candidate-failure"),
                "{error}"
            );
            settle(&mut v);
            assert_eq!(target_id(&mut v, &view), before);
            assert_eq!(target_width(&mut v, &view), Some(120.0));
            assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        }
    }

    #[gpui::test]
    fn suspend_resume_preserves_logical_ref_readers(cx: &mut TestAppContext) {
        let (w, view) = mount(cx, &script(1, true), "ref-resume");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        assert!(v.update(|window, cx| view.suspend(window, cx)).unwrap());
        settle(&mut v);
        assert_eq!(view.state(), ScriptViewState::Suspended);
        assert!(v.update(|_, cx| view.resume(cx)).unwrap());
        settle(&mut v);
        assert_eq!(view.state(), ScriptViewState::Active);
        assert_eq!(readout(&mut v, &view), vec!["120.0"]);
        advance(&mut v, &view);
        assert_eq!(readout(&mut v, &view), vec!["180.0"]);
    }

    #[gpui::test]
    fn round7_formal_virtual_row_unknown_geometry_has_executable_owner_control(
        cx: &mut TestAppContext,
    ) {
        let source = r#"
define_component(#{metadata:#{id:"tests/row_geometry","export":"GeometryRow",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_GeometryRow")});
fn render_GeometryRow(ctx,props){let target=element_ref("self");let facts=ctx.element_bounds(target);text(if facts==(){"pending"}else{`${facts.layout.width}`}).with_key("self").with_ref(target).with_style(style().width(px(120)).height(px(24))).accessibility_role("status")}
fn row(ctx,p){render_component("tests/row_geometry",#{key:p.key})}
fn view(ctx){virtual_collection(#{key:"rows",label:"Rows",data:[#{key:"one"}],height:40,estimated_height:24,overdraw_pixels:0},Fn("row"))}
"#;
        let (w, view) = mount(cx, source, "virtual-owner-control");
        let mut v = VisualTestContext::from_window(*w, cx);
        settle(&mut v);
        let result = readout(&mut v, &view);
        println!("formal virtual row initial ref→geometry readout={result:?}");
        assert_eq!(result, vec!["120.0"]);
    }
}
