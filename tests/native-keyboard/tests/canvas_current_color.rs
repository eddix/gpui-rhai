//! Actual Metal paint, independent of native mock-scene instrumentation.

use gpui_rhai::offscreen::{ShotPlan, run_plan};
use gpui_rhai::{EmbeddedScriptSource, ModuleId, ScriptView};
use std::{collections::BTreeMap, path::Path};

const SPINNERS: &str = r#"
    import "components/button" as button;
    import "components/icon_button" as icon_button;
    import "components/spinner" as spinner;
    fn view(ctx){column([
        button::Button(#{key:"inherited",text:"Busy",loading:true,prefix:text("x"),
            style:style().width(px(180)).height(px(48)).background(rgb(0xffffff)).text_color(rgb(0xff0000)),
            part_styles:#{label:style().opacity(0.0),spinner:style().width(px(24)).height(px(24))}}),
        button::Button(#{key:"local",text:"Busy",loading:true,prefix:text("x"),
            style:style().width(px(180)).height(px(48)).background(rgb(0xffffff)).text_color(rgb(0xff0000)),
            part_styles:#{label:style().opacity(0.0),spinner:style().width(px(24)).height(px(24)).text_color(rgb(0x0000ff))}}),
        icon_button::IconButton(#{key:"icon",label:"Icon activity",loading:true,icon:text("x"),
            style:style().width(px(180)).height(px(48)).background(rgb(0xffffff)).text_color(rgb(0x00ff00)),
            part_styles:#{spinner:style().width(px(24)).height(px(24))}}),
        spinner::Spinner(#{key:"standalone",label:"Standalone activity",
            style:style().text_color(rgb(0xff0000)),
            part_styles:#{indicator:style().width(px(24)).height(px(24)).text_color(rgb(0x0000ff))}})
    ]).with_style(style().width(px(200)).height(px(256)).gap(px(8)).background(rgb(0xffffff)))}
"#;

fn spinner_overrides(directory: &Path) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let output = directory.join(format!("spinner-{name}.png"));
        let plan = ShotPlan::parse([
            "--size".into(),
            "200x256".into(),
            "--out".into(),
            output.display().to_string(),
        ])
        .unwrap();
        run_plan(&plan, || {
            let main = ModuleId::parse("main").unwrap();
            let mut modules = BTreeMap::from([(main.clone(), SPINNERS.to_owned())]);
            for id in [
                "components/button",
                "components/icon_button",
                "components/spinner",
            ] {
                let source = gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
                    .iter()
                    .find(|(candidate, _)| *candidate == id)
                    .unwrap()
                    .1;
                modules.insert(ModuleId::parse(id).unwrap(), source.to_owned());
            }
            ScriptView::sources(main, EmbeddedScriptSource::new(modules), *theme)
                .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
                .motion_preference(gpui_rhai::MotionPreference::None)
                .prepare()
        })
        .unwrap();
        let image = image::open(&output).unwrap().to_rgba8();
        let scale = image.width() / 200;
        for (channel, top) in [(0, 0), (2, 56), (1, 112), (2, 168)] {
            let colored = image
                .enumerate_pixels()
                .filter(|(_, y, _)| *y >= top * scale && *y < (top + 48) * scale)
                .filter(|(_, _, pixel)| {
                    let rgb = pixel.0;
                    rgb[channel] > 100
                        && (0..3)
                            .filter(|other| *other != channel)
                            .all(|other| i16::from(rgb[channel]) - i16::from(rgb[other]) > 20)
                })
                .count();
            assert!(
                colored > 20,
                "{name}: spinner channel {channel}, colored pixels {colored}"
            );
        }
    }
}

// Independent analytic coverage, not the implementation's Bezier paths. Stay
// clear of antialiased boundaries; check ring, quarter-arc, round end caps and
// transparent center, including two separately composited translucent paints.
fn standalone_spinners(directory: &Path) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let output = directory.join(format!("standalone-{name}.png"));
        let plan = ShotPlan::parse([
            "--size".into(),
            "128x128".into(),
            "--out".into(),
            output.display().to_string(),
        ])
        .unwrap();
        run_plan(&plan, || {
            let main = ModuleId::parse("main").unwrap();
            let source = gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
                .iter()
                .find(|(id, _)| *id == "components/spinner")
                .unwrap()
                .1;
            ScriptView::sources(
                main.clone(),
                EmbeddedScriptSource::new(BTreeMap::from([
                    (
                        main,
                        r#"import "components/spinner" as spinner;
                    fn view(ctx){spinner::Spinner(#{key:"activity",label:"Activity",
                        style:style().text_color(rgba(0x1d75b380)).background(rgb(0xffffff)),
                        part_styles:#{indicator:style().width(px(128)).height(px(128))}})}"#
                            .to_owned(),
                    ),
                    (
                        ModuleId::parse("components/spinner").unwrap(),
                        source.to_owned(),
                    ),
                ])),
                *theme,
            )
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .motion_preference(gpui_rhai::MotionPreference::None)
            .prepare()
        })
        .unwrap();
        let image = image::open(&output).unwrap().to_rgba8();
        assert!([128, 256].contains(&image.width()));
        let scale = f64::from(image.width()) / 16.0;
        let (width, track_alpha): (f64, f64) = if name.starts_with("clean_") {
            (1.8, 0.18)
        } else {
            (2.0, 0.2)
        };
        let mut coverage = [0_usize; 4];
        for (x, y, pixel) in image.enumerate_pixels() {
            let x = (f64::from(x) + 0.5) / scale;
            let y = (f64::from(y) + 0.5) / scale;
            let radial = (x - 8.0).hypot(y - 8.0);
            let ring = (radial - 6.0).abs() - width / 2.0;
            // The SDK holds a looping source at its visible midpoint under
            // None/Reduced: 180 degrees, not the initial zero-degree pose.
            let (arc_x, arc_y) = (16.0 - x, 16.0 - y);
            let arc = if arc_x >= 8.0 && arc_y <= 8.0 {
                (radial - 6.0).abs()
            } else {
                (arc_x - 8.0)
                    .hypot(arc_y - 2.0)
                    .min((arc_x - 14.0).hypot(arc_y - 8.0))
            } - width / 2.0;
            if ring.abs() < 0.15 || arc.abs() < 0.15 {
                continue;
            }
            let on_ring = ring < 0.0;
            let on_arc = arc < 0.0;
            coverage[usize::from(on_ring) + 2 * usize::from(on_arc)] += 1;
            let track = if on_ring {
                (128.0 * track_alpha).round() / 255.0
            } else {
                0.0
            };
            let alpha = if on_arc {
                128.0 / 255.0 + track * (1.0 - 128.0 / 255.0)
            } else {
                track
            };
            for (channel, foreground) in [29.0, 117.0, 179.0].into_iter().enumerate() {
                let expected = foreground * alpha + 255.0 * (1.0 - alpha);
                assert!(
                    (f64::from(pixel.0[channel]) - expected).abs() < 2.0,
                    "{name}: source({x},{y}) channel {channel}, got {:?}, expected {expected}, ring={ring}, arc={arc}",
                    pixel.0
                );
            }
            assert_eq!(pixel.0[3], 255);
        }
        assert!(
            coverage[0] > 1000 && coverage[1] > 1000 && coverage[3] > 500,
            "{name}: {coverage:?}"
        );
    }
}

fn pixel(image: &image::RgbaImage, point: (u32, u32)) -> image::Rgba<u8> {
    let scale = image.width() / 480;
    *image.get_pixel(point.0 * scale, point.1 * scale)
}

fn check(path: &Path, reference: (u32, u32), half: (u32, u32)) {
    let image = image::open(path).unwrap().to_rgba8();
    assert!([480, 960].contains(&image.width()));
    let expected = pixel(&image, reference);
    for x in [8, 28, 48, 72, 96, 120] {
        assert_eq!(
            pixel(&image, (10 + x, 18)),
            expected,
            "{}: current color at x={x}",
            path.display()
        );
    }
    assert_eq!(
        pixel(&image, (154, 18)),
        pixel(&image, half),
        "{}: multiplied alpha",
        path.display()
    );
    assert_eq!(
        pixel(&image, (178, 18)).0,
        [255, 0, 0, 255],
        "explicit sibling must not be tinted"
    );
}

fn main() {
    if !cfg!(target_os = "macos") {
        println!("canvas_current_color: real renderer requires macOS");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = |name: &str| directory.path().join(name).display().to_string();
    let args = vec![
        "--size".to_owned(),
        "480x220".to_owned(),
        "--click".into(),
        "Outside".into(),
        "--click".into(),
        "Outside".into(),
        "--hover".into(),
        "Outside".into(),
        "--hover".into(),
        "Outside".into(),
        "--out".into(),
        path("normal.png"),
        "--hover".into(),
        "Parent".into(),
        "--out".into(),
        path("parent.png"),
        "--hover".into(),
        "Canvas".into(),
        "--out".into(),
        path("own.png"),
        "--key".into(),
        "shift-tab".into(),
        "--out".into(),
        path("focus.png"),
        "--key".into(),
        "tab".into(),
        "--hover".into(),
        "Outside".into(),
        "--out".into(),
        path("restored.png"),
    ];
    let plan = ShotPlan::parse(args).unwrap();
    run_plan(&plan, || {
        let main = ModuleId::parse("main").unwrap();
        ScriptView::sources(
            main.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(
                main,
                include_str!("fixtures/canvas_current_color.rhai").to_owned(),
            )])),
            gpui_rhai_registry::DEFAULT_THEME,
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .motion_preference(gpui_rhai::MotionPreference::None)
        .prepare()
    })
    .unwrap();
    for (name, reference, half) in [
        ("normal.png", (18, 50), (42, 50)),
        ("parent.png", (66, 50), (90, 50)),
        ("own.png", (114, 50), (138, 50)),
        ("focus.png", (162, 50), (162, 50)),
        ("restored.png", (18, 50), (42, 50)),
    ] {
        check(&directory.path().join(name), reference, half);
    }
    assert_eq!(
        image::open(directory.path().join("normal.png"))
            .unwrap()
            .to_rgba8(),
        image::open(directory.path().join("restored.png"))
            .unwrap()
            .to_rgba8(),
        "exact frame restoration"
    );
    spinner_overrides(directory.path());
    standalone_spinners(directory.path());
    println!("canvas_current_color: all shapes, native hover/focus, alpha and restoration pass");
}
