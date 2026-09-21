use gpui::{
    Context, ImageSource, IntoElement, Render, TestAppContext, VisualTestContext, Window,
    WindowHandle,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
fn repo() -> String {
    std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap()
}
fn builder(src: &str) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    let mut scripts = BTreeMap::from([(entry.clone(), src.to_owned())]);
    for dir in ["components", "motion"] {
        for e in std::fs::read_dir(format!("{}/registry/{dir}", repo())).unwrap() {
            let p = e.unwrap().path();
            if p.extension().and_then(|x| x.to_str()) == Some("rhai") {
                scripts.insert(
                    ModuleId::parse(format!(
                        "{dir}/{}",
                        p.file_stem().unwrap().to_str().unwrap()
                    ))
                    .unwrap(),
                    std::fs::read_to_string(p).unwrap(),
                );
            }
        }
    }
    let mut assets = vec![];
    for e in std::fs::read_dir(format!("{}/registry/assets/icons", repo())).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) == Some("svg") {
            assets.push((
                format!("icons/{}", p.file_stem().unwrap().to_str().unwrap()),
                AssetData {
                    mime_type: "image/svg+xml".into(),
                    bytes: std::fs::read(p).unwrap(),
                },
            ));
        }
    }
    EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(scripts),
        std::fs::read_to_string(format!("{}/registry/themes/default_dark.rhai", repo())).unwrap(),
    )
    .asset_sources(assets)
}
fn mount(
    cx: &mut TestAppContext,
    p: PreparedScriptView,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle, ScriptViewHost) {
    let cap = Rc::new(RefCell::new(None));
    let take = cap.clone();
    let name = name.to_owned();
    let w = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = p
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some((view.clone(), host.clone()));
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let (v, h) = cap.borrow().as_ref().unwrap().clone();
    (w, v, h)
}
fn pump(cx: &mut TestAppContext, v: &mut VisualTestContext) {
    for _ in 0..8 {
        cx.background_executor
            .advance_clock(Duration::from_millis(20));
        v.run_until_parked();
        cx.refresh().unwrap();
        v.run_until_parked();
    }
}
fn pixel(cx: &mut TestAppContext, body: &str, color: Option<Rgba8>) -> Vec<u8> {
    let r = AssetRegistry::new();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" {body}/></svg>"#
    );
    r.register(
        "app",
        InMemoryAssetProvider::new(BTreeMap::from([(
            "pixel".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: svg.into_bytes(),
            },
        )])),
    )
    .unwrap();
    let h = r.load_image(&AssetId::parse("app/pixel").unwrap()).unwrap();
    let ImageSource::Image(i) = r.image_source_tinted(h.opaque(), color).unwrap() else {
        panic!()
    };
    cx.update(|cx| {
        i.to_image_data(cx.svg_renderer())
            .unwrap()
            .as_bytes(0)
            .unwrap()
            .to_vec()
    })
}
#[gpui::test]
fn svg_document_color_overrides_inherited_tint(cx: &mut TestAppContext) {
    let body = "color=\"#ff0000\" fill=\"currentColor\"";
    let untinted = pixel(cx, body, None);
    let tinted = pixel(cx, body, Some(Rgba8::from_rgba_hex(0x00ff00ff)));
    println!("SVG own color red: no ambient={untinted:?}; ambient green={tinted:?}");
    assert_eq!(untinted, vec![0, 0, 255, 255]);
    assert_eq!(
        tinted, untinted,
        "ambient color must not override an explicit SVG color property"
    );
}
#[gpui::test]
fn svg_text_survives_raster_adapter(cx: &mut TestAppContext) {
    let svg=br#"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="40"><text x="3" y="28" font-family="Arial" font-size="24" fill="black">Readable</text></svg>"#.to_vec();
    let direct = std::sync::Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Svg, svg.clone()));
    let original = cx.update(|cx| direct.to_image_data(cx.svg_renderer()).unwrap());
    let before = original
        .as_bytes(0)
        .unwrap()
        .chunks_exact(4)
        .filter(|p| p[3] > 0)
        .count();
    let r = AssetRegistry::new();
    r.register(
        "app",
        InMemoryAssetProvider::new(BTreeMap::from([(
            "text".into(),
            AssetData {
                mime_type: "image/svg+xml".into(),
                bytes: svg,
            },
        )])),
    )
    .unwrap();
    let h = r.load_image(&AssetId::parse("app/text").unwrap()).unwrap();
    let ImageSource::Image(i) = r.image_source(h.opaque()).unwrap() else {
        panic!()
    };
    let converted = cx.update(|cx| i.to_image_data(cx.svg_renderer()).unwrap());
    let after = converted
        .as_bytes(0)
        .unwrap()
        .chunks_exact(4)
        .filter(|p| p[3] > 0)
        .count();
    println!("SVG text nontransparent pixels: pinned GPUI={before}; PNG adapter={after}");
    assert!(
        before > 100,
        "control: system font renderer must produce glyphs"
    );
    assert!(after > 100, "SVG text disappeared in the adapter");
}
fn groups(node: &UiNode, out: &mut Vec<String>) {
    if let Some(UiValue::String(g)) = node.attributes().get("shared_layout_group") {
        out.push(g.clone());
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for c in children {
                groups(c, out)
            }
        }
        UiNodeKind::VirtualCollection { spec } => {
            for c in spec.realized.values() {
                groups(c, out)
            }
        }
        _ => {}
    }
}
#[gpui::test]
fn changing_inherited_group_does_not_keep_old_presentation(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"
 define_component(#{metadata:#{id:"components/group_edge","export":"GroupEdge",version:"0.1.0",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{},slots:#{},parts:[]},render:Fn("render_probe")});
 fn state_schema(){#{fields:#{g:#{schema:#{type:"string"},"default":#{type:"string",value:"a"}}}}}
 fn swap(ctx,p){ctx.set_state("g","b");}fn bump(ctx,p){ctx.set_state("n",ctx.get_state("n")+1);}fn init(ctx){ctx.register_action("probe.swap",Fn("swap"));}
 fn render_probe(ctx,props){text(ctx.get_state("n").to_string()).with_key("child").test_id("child").on_click(Fn("bump")).shared_layout("item").layout_motion(100,"linear")}
 fn view(ctx){motion_group(ctx.get_state("g"),[render_component("components/group_edge",#{key:"probe"})])}
 "#;
    let (w, view, _) = mount(cx, builder(src).prepare().unwrap(), "group-switch");
    let mut v = VisualTestContext::from_window(*w, cx);
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: "probe.swap".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    pump(cx, &mut v);
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::TestId { id: "child".into() },
                event: "click".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    let mut g = vec![];
    v.update(|_, cx| groups(&view.root(cx).unwrap().unwrap(), &mut g));
    println!("changed formal component groups={g:?}");
    assert_eq!(g, vec!["b"]);
}
#[gpui::test]
fn virtual_items_keep_current_inherited_group(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"
 fn state_schema(){#{fields:#{g:#{schema:#{type:"string"},"default":#{type:"string",value:"a"}}}}}
 fn swap(ctx,p){ctx.set_state("g","b");}fn init(ctx){ctx.register_action("probe.swap",Fn("swap"));}
 fn row_item(ctx,p){text(p.key).with_key(p.key).shared_layout(p.key).layout_motion(100,"linear").with_style(style().height(px(20)))}
 fn view(ctx){let data=[];for i in 0..60{data.push(#{key:`row-${i}`});}motion_group(ctx.get_state("g"),[virtual_collection(#{key:"rows",data:data,height:120,estimated_height:20,overdraw_pixels:20},Fn("row_item"))])}
 "#;
    let (w, view, _) = mount(cx, builder(src).prepare().unwrap(), "group-virtual");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    assert!(v.update(|_, cx| view.last_error(cx).unwrap()).is_none());
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: "probe.swap".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    pump(cx, &mut v);
    let mut g = vec![];
    v.update(|_, cx| groups(&view.root(cx).unwrap().unwrap(), &mut g));
    println!(
        "changed virtual item groups={g:?}, error={:?}",
        v.update(|_, cx| view.last_error(cx).unwrap())
    );
    assert!(!g.is_empty());
    assert!(g.iter().all(|g| g == "b"));
}
