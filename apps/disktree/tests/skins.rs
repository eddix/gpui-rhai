//! Prepare every shipped skin through gpui-rhai's real loaders. Runs the same
//! validation a launch does — module resolution, component schemas,
//! runtime_api, token and asset coverage — without opening a window, so a
//! registry or runtime change that breaks a skin fails `cargo test`.
//! The mounted integration case uses GPUI's test platform to exercise init and
//! live switching; it is not proof of physical OS input or Metal appearance.

use std::path::{Path, PathBuf};

use disktree_rhai::{capability::DisktreeExtension, embedded};
use gpui_rhai::ScriptView;

struct SkinWindow {
    host: gpui_rhai::ScriptViewHost,
    view: gpui_rhai::ScriptViewHandle,
}

impl gpui::Render for SkinWindow {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        if self.view.state() == gpui_rhai::ScriptViewState::Active {
            self.host.container(self.view.element().unwrap())
        } else {
            self.host.container(gpui::div())
        }
    }
}

/// The skin's deep view functions exceed the runtime's 32-deep function
/// default; this mirrors src/main.rs.
const EXPRESSION_DEPTH_LIMITS: (usize, usize) = (64, 128);

#[test]
fn minimal_slice_and_dice_conserves_weighted_area() {
    // Evaluate the shipped skin implementation, not a Rust replica of
    // its arithmetic. Include the uneven weights from the display fixture.
    let mut engine = rhai::Engine::new();
    engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
    engine.set_max_expr_depths(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1);
    let core = include_str!("../skins/minimal/core.rhai");
    let checks = r#"
        for area in [rect(12.0,7.0,1100.0,740.0),rect(-3.0,4.0,340.0,870.0)] {
            for weights in [[212,160,80],[1,1,1,1],[5,0,1,0,2],[3],[1000000,6700,2]] {
                let slices=squarify(weights,area);
                if slices.len!=weights.len {throw "tile index was lost";}
                let total=0.0;
                for weight in weights { total+=weight.to_float(); }
                let painted=0.0;
                for i in 0..slices.len {
                    let tile=slices[i];
                    let amount=tile.w*tile.h;
                    let expect=area.w*area.h*weights[i].to_float()/total;
                    if abs(amount-expect)>0.0001 {
                        throw `tile ${i}: expected ${expect}, got ${amount}`;
                    }
                    if tile.x<area.x-1e-6 || tile.y<area.y-1e-6 ||
                       tile.x+tile.w>area.x+area.w+1e-6 || tile.y+tile.h>area.y+area.h+1e-6 {
                        throw "tile escapes the viewport";
                    }
                    for j in 0..i {
                        let other=slices[j];
                        let w=min(tile.x+tile.w,other.x+other.w)-max(tile.x,other.x);
                        let h=min(tile.y+tile.h,other.y+other.h)-max(tile.y,other.y);
                        if w>1e-6 && h>1e-6 {throw "tiles overlap";}
                    }
                    painted+=amount;
                }
                if abs(painted-area.w*area.h)>0.0001 {throw "treemap does not fill its rectangle";}
            }
        }
        if !squarify([0,0],rect(0.0,0.0,100.0,100.0)).is_empty() {throw "no positive weights";}
    "#;
    engine.run(&format!("{core}\n{checks}")).unwrap();
}

/// Both skins intentionally ship registry copies. The minimal skin is a UI
/// directory rather than a Cargo project, so the outer project's CLI update
/// plan alone cannot catch a stale component or primary preset inside it.
#[test]
fn maintained_skin_sources_and_fonts_match_the_current_registry() {
    use gpui_rhai_registry as registry;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (directory, primary) in [
        ("ui", "modern_dark.rhai"),
        ("skins/minimal", "clean_dark.rhai"),
    ] {
        let skin = root.join(directory);
        assert!(
            std::fs::read_to_string(skin.join("tokens.rhai")).unwrap()
                == registry::TOKEN_BASE_SOURCE,
            "{directory}/tokens.rhai"
        );
        for &(id, expected) in registry::BUNDLED_COMPONENT_SOURCES_BY_ID
            .iter()
            .chain(registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
            .chain(registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        {
            let path = skin.join(format!("{id}.rhai"));
            if path.exists() {
                assert!(
                    std::fs::read_to_string(&path).unwrap() == expected,
                    "{} is a stale registry copy",
                    path.display()
                );
            }
        }
        for &(name, expected) in registry::BUNDLED_DESIGN_PRESET_SOURCES {
            let path = if name == primary {
                skin.join("theme.rhai")
            } else {
                skin.join("themes").join(name)
            };
            assert!(
                std::fs::read_to_string(&path).unwrap() == expected,
                "{} is a stale full preset",
                path.display()
            );
        }
        for &(name, expected) in registry::BUNDLED_FONT_SOURCES {
            assert!(
                std::fs::read(skin.join("fonts").join(name)).unwrap() == expected,
                "{directory}/{name}"
            );
        }
        assert!(
            std::fs::read_to_string(skin.join("licenses/Inter-OFL-1.1.txt")).unwrap()
                == registry::INTER_FONT_LICENSE,
            "{directory}: bundled font license differs"
        );
    }
}

fn extension() -> DisktreeExtension {
    DisktreeExtension::new().with_args(PathBuf::from("/tmp"), false, false, 3, "size", "dark")
}

fn prepare_file_skin(entry: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(entry);
    if let Err(error) = ScriptView::files(path)
        .extension(extension())
        .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
        .prepare()
    {
        panic!("{entry}: {error}");
    }
}

fn mosaic_fills(node: &gpui_rhai::UiNode) -> Option<Vec<gpui_rhai::PaintColor>> {
    use gpui_rhai::{CanvasCommand, PrimitiveValue, UiNodeKind};
    if node
        .key()
        .is_some_and(|key| key.as_str() == "mosaic_canvas")
        && let UiNodeKind::Canvas { scene } = node.kind()
    {
        return Some(
            scene
                .commands()
                .iter()
                .filter_map(|command| match command {
                    CanvasCommand::Rect { key, fill, .. } if key.starts_with("f:") => {
                        Some(fill.clone())
                    }
                    _ => None,
                })
                .collect(),
        );
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(mosaic_fills)
        }
        UiNodeKind::Custom { primitive } => {
            primitive.props.iter().find_map(|(_, value)| match value {
                PrimitiveValue::Node(child) => mosaic_fills(child),
                PrimitiveValue::Nodes(children) => children.iter().find_map(mosaic_fills),
                _ => None,
            })
        }
        UiNodeKind::Layer { content, .. } => mosaic_fills(content),
        UiNodeKind::Overlay {
            trigger, content, ..
        } => mosaic_fills(trigger).or_else(|| mosaic_fills(content)),
        UiNodeKind::ErrorBoundary { child, .. } => mosaic_fills(child),
        _ => None,
    }
}

fn presented_fills(
    visual: &mut gpui::VisualTestContext,
    view: &gpui_rhai::ScriptViewHandle,
) -> Vec<gpui_rhai::PaintColor> {
    visual
        .update(|_, cx| view.root(cx).unwrap().and_then(|root| mosaic_fills(&root)))
        .unwrap_or_default()
}

/// The default skin as files (what --dev and --skin load).
#[test]
fn the_default_skin_prepares_from_files() {
    prepare_file_skin("ui/main.rhai");
}

/// The minimal example skin as files.
#[test]
fn the_minimal_skin_prepares_from_files() {
    prepare_file_skin("skins/minimal/main.rhai");
}

/// The embedded packaging path: the ui/ tree build.rs embedded, the exact
/// assembly src/main.rs ships.
#[test]
fn the_embedded_skin_prepares() {
    if let Err(error) = ScriptView::embedded(&embedded::UI)
        .extension(extension())
        .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
        .prepare()
    {
        panic!("embedded: {error}");
    }
}

/// Reflow a populated, focused application through all four presets. Verify
/// both the painted data and the directory navigation exposed to the user.
#[gpui::test]
fn both_skins_mount_and_switch_complete_designs_without_runtime_errors(
    cx: &mut gpui::TestAppContext,
) {
    use gpui_rhai::{ScriptViewConfig, ScriptViewHandle, ScriptViewHost};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    // This mounts the real filesystem subscription, whose std-thread delivery
    // legitimately wakes the foreground pump. GPUI's mixed I/O mode permits
    // those external wakes; this test does not assert their scheduling order.
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    for (skin, family) in [("ui", "Modern"), ("skins/minimal", "Clean")] {
        let fixture = tempfile::tempdir().unwrap();
        let scan_root = fixture
            .path()
            .join("earlier-ancestor-with-a-name-wide-enough-to-overflow-the-unconstrained-title")
            .join("another-long-display-only-ancestor-that-must-yield-before-the-current-name")
            .join("mounted-breadcrumb-root");
        std::fs::create_dir_all(&scan_root).unwrap();
        let nested = scan_root.join("mounted-breadcrumb-child");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("nested.txt"), vec![b'x'; 2048]).unwrap();
        let extension =
            DisktreeExtension::new().with_args(scan_root, false, false, 3, "size", "dark");
        let prepared = ScriptView::files(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(skin)
                .join("main.rhai"),
        )
        .extension(extension)
        .operation_limit(10_000_000)
        .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
        .motion_preference(gpui_rhai::MotionPreference::None)
        .prepare()
        .unwrap();
        let mounted: Rc<RefCell<Option<ScriptViewHandle>>> = Rc::default();
        let capture = Rc::clone(&mounted);
        let window = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new("skin-test", cx).unwrap();
            let view = prepared
                .mount(
                    ScriptViewConfig::new("skin").window_drag_areas(true),
                    host.clone(),
                    window,
                    cx,
                )
                .unwrap();
            *capture.borrow_mut() = Some(view.clone());
            SkinWindow { host, view }
        });
        let view = mounted.borrow().clone().unwrap();
        let mut visual = gpui::VisualTestContext::from_window(*window, cx);
        visual.simulate_resize(gpui::size(gpui::px(1280.), gpui::px(800.)));
        let initial = visual.update(|_, cx| view.theme_snapshot(cx)).unwrap();
        assert_eq!(initial.variant.family, family);
        let tick = |visual: &mut gpui::VisualTestContext| {
            visual.run_until_parked();
            visual.executor().advance_clock(Duration::from_millis(32));
            visual.update(|window, cx| window.simulate_next_frame(cx));
            visual.run_until_parked();
        };
        let settle = |visual: &mut gpui::VisualTestContext| {
            for _ in 0..8 {
                tick(visual);
            }
        };
        let click = |visual: &mut gpui::VisualTestContext, point| {
            visual.simulate_mouse_move(point, None, gpui::Modifiers::none());
            visual.simulate_click(point, gpui::Modifiers::none());
            settle(visual);
        };
        let assert_current = |visual: &mut gpui::VisualTestContext, step: &str, name: &str| {
            let expected = if skin == "ui" {
                format!("Go to {name}")
            } else {
                name.to_owned()
            };
            let (actual, error) = visual.update(|_, cx| {
                let tree = view.accessibility_snapshot(cx).unwrap();
                (
                    tree.nodes()
                        .filter(|node| node.current.as_deref() == Some("page"))
                        .map(|node| node.name.clone())
                        .collect::<Vec<_>>(),
                    view.last_error(cx).unwrap(),
                )
            });
            assert!(error.is_none(), "{skin} / {step}: {error:?}");
            assert_eq!(
                actual,
                [expected],
                "{skin} / {step}: unique current directory"
            );
        };
        // Metadata-only assertions missed a stale literal Canvas paint cache.
        // Wait for the real filesystem scan/layout before taking its dark
        // witness, so a late scan cannot accidentally rebuild it in Light.
        let mut populated = false;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(10));
            tick(&mut visual);
            if !presented_fills(&mut visual, &view).is_empty() {
                populated = true;
                break;
            }
        }
        assert!(
            populated,
            "{skin}: the native mosaic must actually contain scan data"
        );
        assert_current(&mut visual, "scan", "mounted-breadcrumb-root");
        let mosaic_center = visual.update(|window, _| {
            window.activate_window();
            let size = window.viewport_size();
            gpui::point(size.width / 2., size.height / 2.)
        });
        tick(&mut visual);
        click(&mut visual, mosaic_center);
        visual.simulate_keystrokes("enter");
        settle(&mut visual);
        assert_current(&mut visual, "Enter", "mounted-breadcrumb-child");
        let dark_fills = presented_fills(&mut visual, &view);
        assert!(!dark_fills.is_empty(), "{skin}: child mosaic has scan data");
        for (family, appearance) in [
            ("Modern", "Light"),
            ("Modern", "Dark"),
            ("Clean", "Light"),
            ("Clean", "Dark"),
        ] {
            visual
                .update(|_, cx| view.select_theme(family, appearance, cx))
                .unwrap();
            settle(&mut visual);
            let (theme, error, semantic_count) = visual.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap(),
                    view.last_error(cx).unwrap(),
                    view.accessibility_snapshot(cx).unwrap().nodes().count(),
                )
            });
            assert_eq!(theme.variant.family, family);
            assert_eq!(theme.variant.name, appearance);
            assert_eq!(
                theme.variant.tokens.typography.family.as_deref(),
                Some("Inter")
            );
            assert!(!theme.variant.tokens.component_styles.is_empty());
            let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
            assert_eq!(
                root.style().base.typography.as_deref(),
                Some("body"),
                "{skin} must apply the public body recipe, not only declare Inter in tokens"
            );
            assert!(root.style().base.text_color.is_some());
            assert!(error.is_none(), "{skin} {family} {appearance}: {error:?}");
            let fills = presented_fills(&mut visual, &view);
            assert!(
                !fills.is_empty(),
                "{skin}: theme switching must not blank the mosaic"
            );
            if appearance == "Light" {
                assert_ne!(
                    fills, dark_fills,
                    "{skin}: cached fills still use Dark while labels use Light"
                );
            } else {
                assert_eq!(
                    fills, dark_fills,
                    "{skin}: returning to Dark must restore the actual cached fills"
                );
            }
            assert!(
                semantic_count > 10,
                "the real application chrome must be mounted"
            );
            assert_current(&mut visual, family, "mounted-breadcrumb-child");
        }
        if skin == "ui" {
            // The absolute path prefix has no click targets. A long prefix
            // must not take space from the current name when the name fits.
            // Use this same label at a wide size as the natural-width control,
            // independent of theme padding or the chosen text system.
            let current_geometry = |visual: &mut gpui::VisualTestContext| {
                visual.update(|_, cx| {
                    view.accessibility_snapshot(cx)
                        .unwrap()
                        .nodes()
                        .find(|node| node.current.as_deref() == Some("page"))
                        .and_then(|node| node.geometry)
                        .unwrap()
                })
            };
            visual.simulate_resize(gpui::size(gpui::px(2200.), gpui::px(800.)));
            settle(&mut visual);
            let wide = current_geometry(&mut visual).visual;
            visual.simulate_resize(gpui::size(gpui::px(960.), gpui::px(800.)));
            settle(&mut visual);
            let narrow = current_geometry(&mut visual);
            assert!(
                wide.width > 48. && narrow.visual.width >= wide.width - 0.5,
                "the current directory must keep its natural width: wide={wide:?}, narrow={narrow:?}"
            );
            assert!(
                narrow.clip.is_none_or(|clip| {
                    narrow.visual.x >= clip.x
                        && narrow.visual.x + narrow.visual.width <= clip.x + clip.width + 0.5
                }),
                "keeping the width must not just clip the current directory: {narrow:?}"
            );
        }
        // There is no pointer refocus after changing themes. A working key
        // here proves that the switched layout still routes the mosaic's input.
        visual.simulate_keystrokes("backspace");
        settle(&mut visual);
        assert_current(&mut visual, "Backspace", "mounted-breadcrumb-root");
        visual.simulate_resize(gpui::size(gpui::px(960.), gpui::px(800.)));
        settle(&mut visual);
        visual.simulate_keystrokes("enter");
        settle(&mut visual);
        assert_current(&mut visual, "second Enter", "mounted-breadcrumb-child");
        let geometry = visual.update(|_, cx| {
            let parent_name = if skin == "ui" {
                "Go to mounted-breadcrumb-root"
            } else {
                "mounted-breadcrumb-root"
            };
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "button" && node.name == parent_name)
                .and_then(|node| node.geometry)
                .unwrap()
        });
        let parent = geometry.visual;
        assert!(parent.width > 0. && parent.height > 0.);
        let x = parent.x + parent.width / 2.;
        let y = parent.y + parent.height / 2.;
        assert!(
            geometry.clip.is_none_or(|clip| {
                x >= clip.x && x < clip.x + clip.width && y >= clip.y && y < clip.y + clip.height
            }),
            "{skin}: parent breadcrumb center must be reachable: {geometry:?}",
        );
        let point = gpui::point(gpui::px(x as f32), gpui::px(y as f32));
        click(&mut visual, point);
        assert_current(&mut visual, "breadcrumb click", "mounted-breadcrumb-root");
        visual.update(|_, cx| view.dispose(cx)).unwrap();
        visual.run_until_parked();
    }
}
