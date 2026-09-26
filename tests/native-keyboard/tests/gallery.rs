use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{Context, IntoElement, Render, TestAppContext, Window, WindowHandle};
use gpui_rhai::{
    AutomationCommand, AutomationLocator, AutomationResult, ScriptViewHandle, ScriptViewHost,
    UiNodeKind,
};
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare, stories};
use gpui_rhai_registry::BUNDLED_THEME_SOURCES;

struct GalleryHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for GalleryHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let content = if self.view.state() == gpui_rhai::ScriptViewState::Active {
            self.view.flex_item().unwrap()
        } else {
            gpui::div().into_any_element()
        };
        self.host.container(
            gpui::div()
                .size_full()
                .child(content),
        )
    }
}

fn node_texts(node: &gpui_rhai::UiNode, output: &mut Vec<String>) {
    match node.kind() {
        UiNodeKind::Text { text, .. } => output.push(text.to_string()),
        UiNodeKind::RichText { spans, .. } => {
            output.push(spans.iter().map(|span| span.text()).collect::<String>());
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                node_texts(child, output);
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            node_texts(trigger, output);
            node_texts(content, output);
        }
        UiNodeKind::Layer { content, .. } => node_texts(content, output),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            node_texts(child, output);
            node_texts(fallback, output);
        }
        UiNodeKind::VirtualCollection { spec } => {
            for item in spec.realized.values() {
                node_texts(item, output);
            }
        }
        UiNodeKind::Custom { .. }
        | UiNodeKind::Canvas { .. }
        | UiNodeKind::Svg { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. } => {}
    }
}

fn dispatch(visual: &mut gpui::VisualTestContext, view: &ScriptViewHandle, id: &str) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId { id: id.to_owned() },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
}

fn mount_story(
    cx: &mut TestAppContext,
    launch: GalleryLaunch,
    identity: &str,
) -> (WindowHandle<GalleryHost>, ScriptViewHandle) {
    let prepared = prepare(&launch).unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let host_identity = format!("{identity}-host");
    let view_identity = format!("{identity}-view");
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(host_identity, cx).unwrap();
        let view = prepared
            .mount(
                gpui_rhai::ScriptViewConfig::new(view_identity),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        GalleryHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

fn rendered_texts(
    visual: &mut gpui::VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<String> {
    let mut texts = Vec::new();
    node_texts(
        &visual.update(|_, cx| view.root(cx).unwrap().unwrap()),
        &mut texts,
    );
    texts
}

fn wait_for_text(
    visual: &mut gpui::VisualTestContext,
    view: &ScriptViewHandle,
    expected: &str,
) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        visual.run_until_parked();
        let texts = rendered_texts(visual, view);
        if texts.iter().any(|text| text.contains(expected)) {
            return texts;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected:?}; rendered {texts:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[gpui::test]
fn every_gallery_story_case_mounts_draws_and_presents_semantics(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    for story in stories() {
        for case in story.cases {
            let launch = GalleryLaunch {
                story: story.id.to_owned(),
                case: case.id.to_owned(),
                ..GalleryLaunch::default()
            };
            let prepared = prepare(&launch)
                .unwrap_or_else(|error| panic!("{}/{} prepare: {error}", story.id, case.id));
            let captured = Rc::new(RefCell::new(None));
            let captured_for_window = Rc::clone(&captured);
            let identity = format!("{}-{}", story.id.replace('/', "-"), case.id);
            let host_identity = format!("gallery-host-{identity}");
            let view_identity = format!("gallery-view-{identity}");
            let window = cx.add_window(move |window, cx| {
                let host = ScriptViewHost::new(host_identity, cx).unwrap();
                let view = prepared
                    .mount(
                        gpui_rhai::ScriptViewConfig::new(view_identity),
                        host.clone(),
                        window,
                        cx,
                    )
                    .unwrap();
                *captured_for_window.borrow_mut() = Some(view.clone());
                GalleryHost { host, view }
            });
            cx.run_until_parked();
            cx.refresh().unwrap();
            cx.run_until_parked();

            let view = captured.borrow().as_ref().unwrap().clone();
            let mut visual = gpui::VisualTestContext::from_window(*window, cx);
            assert!(
                visual
                    .update(|_, cx| view.last_error(cx).unwrap())
                    .is_none(),
                "{}/{} recorded an error after draw",
                story.id,
                case.id
            );
            let result = visual
                .update(|window, cx| view.automate(AutomationCommand::Snapshot, window, cx))
                .unwrap_or_else(|error| {
                    panic!("{}/{} semantic snapshot: {error}", story.id, case.id)
                });
            let AutomationResult::Snapshot { snapshot } = result else {
                panic!("{}/{} returned the wrong automation result", story.id, case.id);
            };
            assert!(
                !snapshot.roots.is_empty(),
                "{}/{} presented no semantic roots",
                story.id,
                case.id
            );
            assert!(
                snapshot.nodes.iter().any(|node| node
                    .bounds
                    .is_some_and(|bounds| bounds.width > 0.0 && bounds.height > 0.0)),
                "{}/{} presented no laid-out semantic node",
                story.id,
                case.id
            );
        }
    }
}

#[gpui::test]
fn operations_workbench_completes_and_cancels_the_deployment_boundary(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "config-diff".to_owned(),
            ..GalleryLaunch::default()
        },
        "operations-normal",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    visual.update(|_, cx| {
        assert!(view.select_theme("Default", "Light", cx).unwrap());
        assert!(view.select_locale("zh-CN", cx).unwrap());
        assert!(
            view.set_motion_preference(gpui_rhai::MotionPreference::Reduced, cx)
                .unwrap()
        );
    });
    visual.run_until_parked();
    dispatch(&mut visual, &view, "stage-deploy");
    dispatch(&mut visual, &view, "cancel-deploy");
    let mut texts = rendered_texts(&mut visual, &view);
    assert!(
        !texts
            .iter()
            .any(|text| text.contains("Configuration verified"))
    );

    dispatch(&mut visual, &view, "stage-deploy");
    dispatch(&mut visual, &view, "confirm-deploy");
    texts = wait_for_text(&mut visual, &view, "Deployment complete");
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Configuration verified")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "Deployment complete"),
        "{texts:?}"
    );
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
}

#[gpui::test]
fn workbench_consumes_host_owned_events_and_configuration_documents(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (dashboard_window, dashboard) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            ..GalleryLaunch::default()
        },
        "operations-host-events",
    );
    let mut dashboard_visual =
        gpui::VisualTestContext::from_window(*dashboard_window, cx);
    assert!(dashboard_visual.update(|_, cx| {
        dashboard
            .accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("table", "Recent operations events")
            .next()
            .is_some()
    }));
    drop(dashboard_visual);

    let (config_window, config) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "config-diff".to_owned(),
            ..GalleryLaunch::default()
        },
        "operations-host-config",
    );
    let mut config_visual = gpui::VisualTestContext::from_window(*config_window, cx);
    let invalidated = config_visual
        .update(|_, cx| {
            config.replace_native_text_document(
                "operations_config_right",
                gpui_rhai::NativeTextDocument::new(
                    "operations_config_right",
                    2,
                    "server {\n  host = \"edge-02\"\n  channel = \"canary\"\n}\n",
                )
                .unwrap(),
                cx,
            )
        })
        .unwrap();
    assert!(invalidated, "configuration document reader was not tracked");
    config_visual.run_until_parked();
    assert!(
        config_visual
            .update(|_, cx| config.last_error(cx).unwrap())
            .is_none()
    );
}

#[gpui::test]
fn operations_workbench_failure_keeps_the_previous_configuration(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "failure".to_owned(),
            ..GalleryLaunch::default()
        },
        "operations-failure",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    dispatch(&mut visual, &view, "stage-deploy");
    dispatch(&mut visual, &view, "confirm-deploy");
    let texts = wait_for_text(&mut visual, &view, "Deployment failed");
    assert!(
        texts
            .iter()
            .any(|text| text.contains("previous Host configuration remains active")),
        "{texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|text| text.contains("Configuration verified")),
        "{texts:?}"
    );
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
}

#[gpui::test]
fn operations_fixture_cases_are_observable_and_streaming_is_bounded(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    for (case, expected) in [
        ("loading", "Loading host health and metrics"),
        ("empty", "No hosts yet"),
        ("large", "1,000 Rust-owned rows"),
    ] {
        let (window, view) = mount_story(
            cx,
            GalleryLaunch {
                story: "apps/operations".to_owned(),
                case: case.to_owned(),
                ..GalleryLaunch::default()
            },
            &format!("operations-{case}"),
        );
        let mut visual = gpui::VisualTestContext::from_window(*window, cx);
        let texts = rendered_texts(&mut visual, &view);
        assert!(
            texts.iter().any(|text| text.contains(expected)),
            "{case}: {texts:?}"
        );
        assert!(
            visual
                .update(|_, cx| view.last_error(cx).unwrap())
                .is_none(),
            "{case} recorded an error"
        );
    }

    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "streaming".to_owned(),
            ..GalleryLaunch::default()
        },
        "operations-streaming",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let texts = wait_for_text(&mut visual, &view, "Streaming revision 4");
    assert!(
        texts
            .iter()
            .any(|text| text.contains("without Rhai polling")),
        "{texts:?}"
    );
}

#[gpui::test]
fn streaming_workbench_suspend_cancels_late_delivery_and_resume_restarts(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let prepared = prepare(&GalleryLaunch {
        story: "apps/operations".to_owned(),
        case: "streaming".to_owned(),
        ..GalleryLaunch::default()
    })
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("operations-stream-lifecycle-host", cx).unwrap();
        let view = prepared
            .mount(
                gpui_rhai::ScriptViewConfig::new("operations-stream-lifecycle-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        view.suspend(window, cx).unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        GalleryHost { host, view }
    });
    let view = captured.borrow().as_ref().unwrap().clone();
    assert_eq!(view.state(), gpui_rhai::ScriptViewState::Suspended);
    cx.executor().allow_parking();
    std::thread::sleep(Duration::from_millis(260));
    cx.run_until_parked();
    cx.refresh().unwrap();
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    assert_eq!(view.state(), gpui_rhai::ScriptViewState::Suspended);
    let resumed_initial = visual.update(|_, cx| {
        view.resume(cx).unwrap();
        let root = view.root(cx).unwrap().unwrap();
        let mut texts = Vec::new();
        node_texts(&root, &mut texts);
        texts
    });
    assert!(
        !resumed_initial
            .iter()
            .any(|text| text == "Streaming revision 4"),
        "late suspended delivery advanced state: {resumed_initial:?}"
    );
    let resumed = wait_for_text(&mut visual, &view, "Streaming revision 4");
    assert!(
        resumed
            .iter()
            .any(|text| text == "Streaming revision 4")
    );
}

#[gpui::test]
fn every_bundled_theme_hot_switches_the_shared_design_story(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "components/catalog".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-theme-matrix",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let engine = gpui_rhai::RuntimeEngine::new();
    for (theme_file, source) in BUNDLED_THEME_SOURCES {
        let theme = gpui_rhai::load_theme_source(engine.engine(), theme_file, source).unwrap();
        visual.update(|_, cx| {
            view.select_theme(&theme.family, &theme.name, cx).unwrap();
        });
        visual.run_until_parked();
        let snapshot = visual.update(|_, cx| view.theme_snapshot(cx).unwrap());
        assert_eq!(snapshot.variant.family, theme.family, "{theme_file}");
        assert_eq!(snapshot.variant.name, theme.name, "{theme_file}");
        assert!(
            visual
                .update(|_, cx| view.last_error(cx).unwrap())
                .is_none(),
            "{theme_file} recorded an error"
        );
    }
}

#[gpui::test]
fn critical_story_viewport_and_locale_matrix_mounts_real_frames(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, catalog) = mount_story(
        cx,
        GalleryLaunch {
            story: "components/catalog".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-responsive-matrix",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    for (width, expected) in [(520.0, "compact"), (800.0, "regular"), (1_120.0, "wide")] {
        visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(760.0)));
        visual.run_until_parked();
        let texts = rendered_texts(&mut visual, &catalog);
        assert!(
            texts.iter().any(|text| text == expected),
            "{width} did not produce {expected}: {texts:?}"
        );
    }
    drop(visual);

    for locale in ["zh-CN", "ar"] {
        let (window, view) = mount_story(
            cx,
            GalleryLaunch {
                story: "components/input".to_owned(),
                locale: locale.to_owned(),
                ..GalleryLaunch::default()
            },
            &format!("gallery-locale-{locale}"),
        );
        let mut visual = gpui::VisualTestContext::from_window(*window, cx);
        visual.run_until_parked();
        assert!(
            visual
                .update(|_, cx| view.last_error(cx).unwrap())
                .is_none(),
            "{locale} input story recorded an error"
        );
        assert!(visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("text_field", "Host name")
                .next()
                .is_some()
        }));
    }
}

#[gpui::test]
fn chart_diagnostic_story_exposes_last_good_invalid_semantics(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "charts/interaction".to_owned(),
            case: "diagnostics".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-chart-diagnostic",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let dispatch_error = visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId {
                        id: "break-chart".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .expect_err("breaking the chart must reject the event transaction");
    assert!(dispatch_error.to_string().contains("piee"));
    visual.run_until_parked();
    let error = visual
        .update(|_, cx| view.last_error(cx).unwrap())
        .expect("broken spec must reach the view error channel");
    assert!(error.contains("piee"), "{error}");
    assert!(visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Diagnostic chart")
            .next()
            .is_some_and(|node| node.invalid && node.description.contains("piee"))
    }));
}

#[gpui::test]
fn content_bearing_tabs_stretch_table_and_chart_panels(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "components/tabs".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-tabs-fill",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let table_width = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("table", "Hosts in Tabs")
            .next()
            .and_then(|node| node.geometry)
            .map(|geometry| geometry.visual.width)
            .unwrap()
    });
    assert!(table_width > 500.0, "table width was {table_width}");

    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "tab".to_owned(),
                        name: "Latency".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    let chart_width = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Latency")
            .next()
            .and_then(|node| node.geometry)
            .map(|geometry| geometry.visual.width)
            .unwrap()
    });
    assert!(chart_width > 500.0, "chart width was {chart_width}");
}

#[gpui::test]
fn host_theme_overrides_survive_live_theme_switches(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "theme-overrides".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-theme-overrides",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let assert_radii = |snapshot: gpui_rhai::ThemeSnapshot| {
        assert_eq!(
            snapshot.variant.tokens.radii["sm"],
            gpui_rhai::Length::Pixels(4.0)
        );
        assert_eq!(
            snapshot.variant.tokens.radii["md"],
            gpui_rhai::Length::Pixels(7.0)
        );
        assert_eq!(
            snapshot.variant.tokens.radii["lg"],
            gpui_rhai::Length::Pixels(10.0)
        );
    };
    assert_radii(visual.update(|_, cx| view.theme_snapshot(cx).unwrap()));
    visual.update(|_, cx| {
        assert!(view.select_theme("Nord", "Dark", cx).unwrap());
    });
    visual.run_until_parked();
    let snapshot = visual.update(|_, cx| view.theme_snapshot(cx).unwrap());
    assert_eq!(snapshot.variant.family, "Nord");
    assert_radii(snapshot);
    let texts = rendered_texts(&mut visual, &view);
    assert!(texts.iter().any(|text| text == "Selected hosts: 1"));
}

#[gpui::test]
fn host_embedding_story_types_through_the_real_nested_view_boundary(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let launch = GalleryLaunch {
        story: "apps/host-embedding".to_owned(),
        ..GalleryLaunch::default()
    };
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let resident_host = ScriptViewHost::new("gallery-resident-host", cx).unwrap();
        let resident = gpui_rhai_cli::gallery::host_resident_view(&launch)
            .unwrap()
            .prepare()
            .unwrap()
            .mount(
                gpui_rhai::ScriptViewConfig::new("gallery-resident-view"),
                resident_host,
                window,
                cx,
            )
            .unwrap();
        let slots = gpui_rhai::HostSlotRegistry::new()
            .with_script_view("resident-form", resident.clone())
            .unwrap();
        let shell_host = ScriptViewHost::new("gallery-host-shell", cx).unwrap();
        let shell = gpui_rhai_cli::gallery::view_with_host_slots(&launch, slots)
            .unwrap()
            .prepare()
            .unwrap()
            .mount(
                gpui_rhai::ScriptViewConfig::new("gallery-host-shell-view"),
                shell_host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some((shell.clone(), resident));
        GalleryHost {
            host: shell_host,
            view: shell,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    cx.simulate_input(*window, "中文");

    let (shell, resident) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let texts = wait_for_text(&mut visual, &resident, "resident:中文");
    assert!(texts.iter().any(|text| text == "resident:中文"));
    assert!(visual.update(|_, cx| shell.last_error(cx).unwrap()).is_none());
    assert!(
        visual
            .update(|_, cx| resident.last_error(cx).unwrap())
            .is_none()
    );
}

#[gpui::test]
fn workbench_sheet_and_command_dialog_share_session_navigation(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "large".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-workbench-overlays",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    dispatch(&mut visual, &view, "open-host-sheet");
    assert!(visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("dialog", "Host details")
            .next()
            .is_some()
    }));
    dispatch(&mut visual, &view, "compare-from-sheet");
    let texts = rendered_texts(&mut visual, &view);
    assert!(texts.iter().any(|text| text == "Configurations"));

    dispatch(&mut visual, &view, "open-command");
    assert!(visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("dialog", "Navigate")
            .next()
            .is_some()
    }));
}

#[gpui::test]
fn workbench_large_hosts_filter_in_rust_without_materializing_rows_in_rhai(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount_story(
        cx,
        GalleryLaunch {
            story: "apps/operations".to_owned(),
            case: "large".to_owned(),
            ..GalleryLaunch::default()
        },
        "gallery-workbench-search",
    );
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    let result = visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "button".to_owned(),
                        name: "Next page".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    let AutomationResult::Dispatch { report } = result else {
        panic!("pagination dispatch returned the wrong automation result");
    };
    assert!(report.invoked > 0);
    visual.run_until_parked();
    let page_two = rendered_texts(&mut visual, &view);
    assert!(page_two.iter().any(|text| text.contains("Page 2 / 40")));
    drop(visual);

    cx.simulate_input(*window, "edge-1000");
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let texts = rendered_texts(&mut visual, &view);
    assert!(texts.iter().any(|text| text == "edge-1000"), "{texts:?}");
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
}
