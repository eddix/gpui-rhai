//! Exercise the unmodified CLI starter, without the test host's bundled theme/fonts.

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::ScriptView;
use gpui_rhai_cli::{GpuiBackend, Project};

use crate::support::{Host, Step, bounds, last_error, settle};

#[gpui::test]
fn installed_starter_applies_the_public_body_recipe_in_all_four_designs(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname = \"installed-starter\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\n",
    ).unwrap();
    let backend = if cfg!(feature = "gpui-fast") {
        GpuiBackend::Fast
    } else {
        GpuiBackend::Pre
    };
    let project = Project::new(directory.path());
    project
        .plan_init_for(None, backend)
        .unwrap()
        .apply()
        .unwrap();
    assert_eq!(project.check().unwrap().components, 0);

    cx.update(gpui_rhai::install);
    let prepared = ScriptView::files(directory.path().join("ui/main.rhai"))
        .prepare()
        .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("installed-starter", "installed-starter"),
        false,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(640.0), px(360.0)));
    for (family, variant) in [
        ("Modern", "Dark"),
        ("Modern", "Light"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, variant, cx))
            .unwrap();
        settle(&mut visual, Step::Refresh, 3);
        let title = bounds(&mut visual, &view, "text", "GPUI Rhai");
        assert!(
            (title.height - 20.0).abs() < 0.5,
            "{family}/{variant}: starter text must inherit the 20px body line, {title:?}"
        );
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        assert_eq!(root.style().base.typography.as_deref(), Some("body"));
        assert!(
            root.style().base.text_color.is_some(),
            "plain starter text must not use a system-default foreground"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
