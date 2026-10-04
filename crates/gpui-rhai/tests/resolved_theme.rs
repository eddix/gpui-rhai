use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

fn manager() -> ThemeManager {
    let engine = RuntimeEngine::new();
    let dark = load_theme_source(
        engine.engine(),
        "dark",
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .unwrap();
    let light = load_theme_source(
        engine.engine(),
        "light",
        include_str!("../../../registry/themes/default_light.rhai"),
    )
    .unwrap();
    ThemeManager::from_variants([dark, light], ThemeSelection::new("Default", "Dark")).unwrap()
}
fn context(
    runtime: Rc<RefCell<UiRuntimeState>>,
    component: ComponentInstancePath,
    window: Option<&str>,
    phase: ExecutionPhase,
) -> UiContext {
    UiContext::new(
        runtime,
        component,
        window.map(str::to_owned),
        phase,
        BTreeMap::new(),
    )
}
fn info(name: &str, mode: ThemeMode) -> ThemeVariantInfo {
    ThemeVariantInfo {
        family: "Default".into(),
        name: name.into(),
        mode,
    }
}

#[test]
fn no_theme_and_unavailable_borrow_return_none_without_panicking() {
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let ctx = context(
        runtime.clone(),
        ComponentInstancePath::root("App", "none"),
        None,
        ExecutionPhase::Render,
    );
    assert_eq!(ctx.resolved_theme_variant(), None);
    assert_eq!(ctx.resolved_theme_selection(), None);
    assert_eq!(ctx.motion_tokens(), ThemeMotion::default());
    let mut engine = RuntimeEngine::new();
    let source = engine
        .compile(r#"fn view(ctx){text(if ctx.theme_variant()==(){"none"}else{"unexpected"})}"#)
        .unwrap();
    let result = engine.render_with_context(&source, ctx.clone()).unwrap();
    let UiNodeKind::Text { text } = result.kind() else {
        panic!("expected text")
    };
    assert_eq!(text.as_str(), "none");
    runtime.borrow_mut().theme = Some(manager());
    let borrow = runtime.borrow();
    assert_eq!(ctx.resolved_theme_variant(), None);
    assert_eq!(ctx.resolved_theme_selection(), None);
    drop(borrow);
    assert_eq!(
        ctx.resolved_theme_variant(),
        Some(info("Dark", ThemeMode::Dark))
    );
}

#[test]
fn resolved_identity_is_not_system_preference_and_headless_fallback_is_explicit() {
    let mut themes = manager();
    themes
        .set_app(ThemePreference::System {
            family: "Default".into(),
        })
        .unwrap();
    let mut state = UiRuntimeState::new();
    state.theme = Some(themes);
    let runtime = Rc::new(RefCell::new(state));
    let ctx = context(
        runtime.clone(),
        ComponentInstancePath::root("App", "headless"),
        Some("not-mounted"),
        ExecutionPhase::Event,
    );
    assert_eq!(
        ctx.resolved_theme_variant(),
        Some(info("Dark", ThemeMode::Dark))
    );
    assert_eq!(
        ctx.resolved_theme_selection(),
        Some(ThemeSelection::new("Default", "Dark"))
    );
    assert!(
        matches!(runtime.borrow().theme.as_ref().unwrap().app_preference(),ThemePreference::System{family} if family=="Default")
    );
    assert_eq!(
        runtime
            .borrow()
            .theme
            .as_ref()
            .unwrap()
            .resolve(Some("not-mounted"), None, SystemAppearance::Light)
            .unwrap()
            .variant()
            .mode,
        ThemeMode::Light
    );
    // This is the documented pre-native fallback, not a simulated Dark window.
}

#[test]
fn window_nearest_local_and_sibling_resolution_share_one_identity_projection() {
    let root = ComponentInstancePath::root("App", "scopes");
    let local = root.child("Panel", "local");
    let nested = local.child("Reader", "nested");
    let sibling = root.child("Panel", "sibling");
    let mut themes = manager();
    themes
        .set_window(
            "first",
            ThemePreference::Fixed {
                selection: ThemeSelection::new("Default", "Light"),
            },
        )
        .unwrap();
    themes
        .set_scope(
            local.clone(),
            ThemePreference::Fixed {
                selection: ThemeSelection::new("Default", "Dark"),
            },
        )
        .unwrap();
    let mut state = UiRuntimeState::new();
    state.theme = Some(themes);
    let runtime = Rc::new(RefCell::new(state));
    for (path, window, expected) in [
        (root.clone(), "first", info("Light", ThemeMode::Light)),
        (local.clone(), "first", info("Dark", ThemeMode::Dark)),
        (nested, "first", info("Dark", ThemeMode::Dark)),
        (sibling.clone(), "first", info("Light", ThemeMode::Light)),
        (sibling, "second", info("Dark", ThemeMode::Dark)),
    ] {
        let ctx = context(runtime.clone(), path, Some(window), ExecutionPhase::Render);
        assert_eq!(
            ctx.resolved_theme_selection(),
            Some(ThemeSelection::new(&expected.family, &expected.name))
        );
        assert_eq!(ctx.resolved_theme_variant(), Some(expected));
    }
    let ctx = context(runtime.clone(), local, Some("first"), ExecutionPhase::Event);
    ctx.set_local_theme("Default", "Light").unwrap();
    assert_eq!(
        ctx.resolved_theme_variant(),
        Some(info("Light", ThemeMode::Light))
    );
}

#[test]
fn only_render_reads_track_theme_and_rhai_returns_exact_metadata_fields() {
    for phase in [
        ExecutionPhase::Init,
        ExecutionPhase::Event,
        ExecutionPhase::Render,
    ] {
        let root = ComponentInstancePath::root("App", format!("{phase:?}"));
        let mut state = UiRuntimeState::new();
        state.theme = Some(manager());
        let runtime = Rc::new(RefCell::new(state));
        let ctx = context(runtime.clone(), root.clone(), Some("window"), phase);
        assert_eq!(
            ctx.resolved_theme_variant(),
            Some(info("Dark", ThemeMode::Dark))
        );
        runtime
            .borrow_mut()
            .select_theme_from_host("Default", "Light")
            .unwrap();
        assert_eq!(
            runtime.borrow().dirty_components().contains(&root),
            phase == ExecutionPhase::Render
        );
        assert_eq!(
            ctx.resolved_theme_variant(),
            Some(info("Light", ThemeMode::Light))
        );
    }
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(r#"fn view(ctx){text("ok")}fn metadata(ctx,payload){ctx.theme_variant()}"#)
        .unwrap();
    let mut state = UiRuntimeState::new();
    state.theme = Some(manager());
    let ctx = context(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "metadata"),
        None,
        ExecutionPhase::Event,
    );
    engine.render_with_context(&compiled, ctx.clone()).unwrap();
    let callback = engine.callback(&compiled, "metadata").unwrap();
    let metadata = engine
        .invoke_callback(&compiled, &callback, (ctx, ()))
        .unwrap()
        .cast::<rhai::Map>();
    assert_eq!(metadata.len(), 3);
    for (key, value) in [("family", "Default"), ("name", "Dark"), ("mode", "dark")] {
        assert_eq!(
            metadata[key].clone_cast::<rhai::ImmutableString>().as_str(),
            value
        );
    }
}

#[test]
fn token_and_motion_updates_do_not_infer_a_different_identity_from_colors() {
    let mut themes = manager();
    let mut variant = themes
        .resolve(None, None, SystemAppearance::Dark)
        .unwrap()
        .variant()
        .clone();
    let tokens = std::sync::Arc::make_mut(&mut variant.tokens);
    tokens
        .colors
        .insert("background".into(), Rgba8::from_rgb_hex(0x00ff_ffff));
    tokens.motion.durations_ms.insert("normal".into(), 42);
    themes.replace_variant(variant.clone()).unwrap();
    let mut state = UiRuntimeState::new();
    state.theme = Some(themes);
    let ctx = context(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "tokens"),
        None,
        ExecutionPhase::Render,
    );
    assert_eq!(
        ctx.resolved_theme_variant(),
        Some(info("Dark", ThemeMode::Dark))
    );
    assert_eq!(ctx.motion_tokens(), variant.tokens.motion);
}
