#[cfg(test)]
mod tests {
    use gpui::{Context, IntoElement, Render, TestAppContext, Window};
    use gpui_rhai::*;
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    struct Host {
        host: ScriptViewHost,
        view: ScriptViewHandle,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.host.container(self.view.element().unwrap())
        }
    }
    const SCRIPT: &str = r#"
import "components/table" as table;
fn state_schema(){#{fields:#{viewport:#{schema:#{type:"number"},"default":#{type:"float",value:300.0}}}}}
fn wider(ctx,value){ctx.set_state("viewport",500.0);}
fn view(ctx){column([
    text("Wider").test_id("wider").on_click(Fn("wider")),
    table::Table(#{key:"table",label:"Probe",row_key:"id",rows:[],loading:true,height:100.0,
        columns:[#{key:"a",title:"A",width:#{kind:"fixed",value:120.0}},
                 #{key:"b",title:"B",width:#{kind:"percent",value:50.0}},
                 #{key:"c",title:"C",width:#{kind:"flex",value:1.0}}]})
]).with_style(style().width(px(ctx.get_state("viewport"))))}
"#;

    fn mount(cx: &mut TestAppContext) -> (gpui::WindowHandle<Host>, ScriptViewHandle) {
        cx.update(gpui_rhai::install);
        let entry = ModuleId::parse("main").unwrap();
        let modules = BTreeMap::from([
            (entry.clone(), SCRIPT.into()),
            (
                ModuleId::parse("components/table").unwrap(),
                std::fs::read_to_string(format!("{REPO}/registry/components/table.rhai")).unwrap(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                std::fs::read_to_string(format!("{REPO}/registry/components/badge.rhai")).unwrap(),
            ),
        ]);
        let prepared = EmbeddedScriptView::new(
            entry,
            EmbeddedScriptSource::new(modules),
            std::fs::read_to_string(format!("{REPO}/registry/themes/default_dark.rhai")).unwrap(),
        )
        .asset_sources(
            [
                "disclosure_down",
                "chevron_right",
                "sort_ascending",
                "sort_descending",
            ]
            .into_iter()
            .map(|name| {
                (
                    format!("icons/{name}"),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: std::fs::read(format!("{REPO}/registry/assets/icons/{name}.svg"))
                            .unwrap(),
                    },
                )
            }),
        )
        .prepare()
        .unwrap();
        let saved = Rc::new(RefCell::new(None));
        let capture = saved.clone();
        let window = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new("frame-probe", cx).unwrap();
            let view = prepared
                .mount(
                    ScriptViewConfig::new("frame-probe"),
                    host.clone(),
                    window,
                    cx,
                )
                .unwrap();
            *capture.borrow_mut() = Some(view.clone());
            Host { host, view }
        });
        cx.run_until_parked();
        let view = saved.borrow().clone().unwrap();
        (window, view)
    }

    fn widths(cx: &mut TestAppContext, view: &ScriptViewHandle) -> Vec<f64> {
        cx.update(|cx| {
            let tree = view.accessibility_snapshot(cx).unwrap();
            ["A", "B", "C"]
                .into_iter()
                .map(|name| {
                    tree.find_by_role_and_name("columnheader", name)
                        .next()
                        .unwrap()
                        .geometry
                        .unwrap()
                        .visual
                        .width
                })
                .collect()
        })
    }

    fn table_width(cx: &mut TestAppContext, view: &ScriptViewHandle) -> f64 {
        cx.update(|cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("table", "Probe")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
                .width
        })
    }

    fn clear_script_timings(cx: &mut TestAppContext, view: &ScriptViewHandle) {
        cx.update(|cx| {
            view.take_performance_snapshot(cx).unwrap();
        });
    }

    fn assert_native_idle(cx: &mut TestAppContext, view: &ScriptViewHandle, scheduled: &[usize]) {
        assert_eq!(
            scheduled.last(),
            Some(&0),
            "measurement must not keep requesting frames"
        );
        let operations = cx.update(|cx| {
            view.take_performance_snapshot(cx)
                .unwrap()
                .timings
                .iter()
                .map(|timing| timing.operations)
                .sum::<u64>()
        });
        assert_eq!(
            operations, 0,
            "measurement follow-up frames must not execute Rhai"
        );
    }

    #[gpui::test]
    fn measured_viewport_schedules_its_own_followup_frame(cx: &mut TestAppContext) {
        let (window, view) = mount(cx);
        // GPUI open_window already paints the initial frame. Do not add a
        // caller-supplied refresh here: it would hide the missing request.
        cx.run_until_parked();
        clear_script_timings(cx, &view);
        cx.background_executor
            .advance_clock(Duration::from_millis(128));
        cx.run_until_parked();
        let mut scheduled = Vec::new();
        for _ in 0..3 {
            scheduled.push(
                window
                    .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                    .unwrap(),
            );
            cx.run_until_parked();
        }
        let actual = widths(cx, &view);
        assert_native_idle(cx, &view, &scheduled);
        println!(
            "after initial frame + normal clock, scheduled={scheduled:?}, no manual followup: {actual:?}"
        );
        cx.refresh().unwrap();
        cx.update(|_| ());
        cx.run_until_parked();
        println!(
            "explicit extra refresh driver control: {:?}",
            widths(cx, &view)
        );
        assert!(
            (actual[1] - 149.0).abs() < 0.6,
            "percent column must use measured viewport"
        );
        assert!(
            (actual[2] - 29.0).abs() < 0.6,
            "flex column must use measured viewport"
        );
    }

    #[gpui::test]
    fn viewport_change_schedules_its_own_followup_frame(cx: &mut TestAppContext) {
        let (window, view) = mount(cx);
        // Establish the previous correct frame as a driver positive control.
        for _ in 0..3 {
            cx.refresh().unwrap();
            cx.update(|_| ());
            cx.run_until_parked();
        }
        assert!((widths(cx, &view)[1] - 149.0).abs() < 0.6);
        window
            .update(cx, |_, window, cx| {
                view.automate(
                    AutomationCommand::Dispatch {
                        locator: AutomationLocator::TestId { id: "wider".into() },
                        event: "click".into(),
                        payload: None,
                    },
                    window,
                    cx,
                )
            })
            .unwrap()
            .unwrap();
        cx.run_until_parked();
        // The ordinary state transaction already dirties and paints the view.
        clear_script_timings(cx, &view);
        cx.background_executor
            .advance_clock(Duration::from_millis(128));
        cx.run_until_parked();
        let mut scheduled = Vec::new();
        for _ in 0..3 {
            scheduled.push(
                window
                    .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                    .unwrap(),
            );
            cx.run_until_parked();
        }
        let actual = widths(cx, &view);
        assert_native_idle(cx, &view, &scheduled);
        let presented_viewport = table_width(cx, &view);
        println!(
            "after resize frame + normal clock, scheduled={scheduled:?}, viewport={presented_viewport}, no manual followup: {actual:?}"
        );
        assert!(
            (presented_viewport - 500.0).abs() < 0.6,
            "driver must present the viewport change"
        );
        cx.refresh().unwrap();
        cx.update(|_| ());
        cx.run_until_parked();
        let control = widths(cx, &view);
        println!("explicit extra refresh driver control: {control:?}");
        assert!((control[1] - 249.0).abs() < 0.6);
        assert!((control[2] - 129.0).abs() < 0.6);
        assert!(
            (actual[1] - 249.0).abs() < 0.6,
            "percent column must use new measured viewport"
        );
        assert!(
            (actual[2] - 129.0).abs() < 0.6,
            "flex column must use new measured viewport"
        );
    }
}
