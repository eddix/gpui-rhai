mod common;

use common::{compile_official, mount, new_runtime};

#[test]
fn column_browser_mounts_controlled_columns_and_native_list_composition() {
    let (mut engine, compiled) = compile_official(
        "browser",
        r#"
        import "components/column_browser" as browser;
        fn view(ctx) {
            browser::ColumnBrowser(#{ key: "config", label: "Configuration", height: 320,
                columns: [
                    #{ key: "tenant", title: "Tenant", items: [#{key:"payments",title:"Payments",has_children:true}], selected:"payments" },
                    #{ key: "directory", title: "Directory", items: [], loading:true },
                ], detail: text("Choose an item") })
        }
    "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    assert!(format!("{:?}", lifecycle.retained()).contains("Configuration"));
}

#[test]
fn browser_logical_handlers_are_not_double_mirrored_in_rtl() {
    let (mut engine, compiled) = compile_official(
        "rtl-browser",
        r#"
        import "components/column_browser" as browser;
        fn init(ctx) { ctx.set_locale("ar"); }
        fn view(ctx) { browser::ColumnBrowser(#{key:"config",label:"Configuration",height:240,show_path:false,
            columns:[#{key:"root",title:"Root",selected:"a",items:[#{key:"a",title:"Alpha",has_children:true}]},
                #{key:"child",title:"Child",items:[#{key:"x",title:"Child"}]}]}) }
    "#,
    );
    let bundles = [
        ("en.rhai", gpui_rhai_registry::EN_LOCALE),
        ("ar.rhai", gpui_rhai_registry::AR_LOCALE),
    ]
    .into_iter()
    .map(|(name, source)| gpui_rhai::load_locale_source(engine.engine(), name, source).unwrap());
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(gpui_rhai::LocaleManager::new(bundles, "en", "en").unwrap());
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let root = lifecycle.root().unwrap();
    assert!(
        root.handlers().contains_key("key:right"),
        "{:?}",
        root.handlers().keys().collect::<Vec<_>>()
    );
    assert!(!root.handlers().contains_key("key:left"));
}
