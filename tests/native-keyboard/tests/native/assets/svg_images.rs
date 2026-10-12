//! SVG images through GPUI's image pipeline: tinted SVG bytes follow GPUI's BGRA
//! contract and the raster adapter uses GPUI's system font environment.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Context, ImageSource, IntoElement, Render, TestAppContext, VisualTestContext, Window, img,
};
use gpui_rhai::{AssetData, AssetId, AssetRegistry, InMemoryAssetProvider, Rgba8};

struct ImageSourceProbe {
    source: ImageSource,
}

impl Render for ImageSourceProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        img(self.source.clone())
    }
}

fn render_image_source(cx: &mut TestAppContext, source: ImageSource) -> Vec<u8> {
    match source {
        ImageSource::Image(image) => cx.update(|app| {
            image
                .to_image_data(app.svg_renderer())
                .unwrap()
                .as_bytes(0)
                .unwrap()
                .to_vec()
        }),
        ImageSource::Render(image) => image.as_bytes(0).unwrap().to_vec(),
        ImageSource::Resource(_) => panic!("test source must be memory-backed"),
        ImageSource::Custom(loader) => {
            let ready = Rc::new(RefCell::new(None::<Arc<gpui::RenderImage>>));
            let ready_for_loader = Rc::clone(&ready);
            let calls = Rc::new(Cell::new(0_u32));
            let calls_for_loader = Rc::clone(&calls);
            let source = ImageSource::from(move |window: &mut Window, app: &mut gpui::App| {
                calls_for_loader.set(calls_for_loader.get().saturating_add(1));
                let result = loader(window, app);
                if let Some(Ok(image)) = &result {
                    *ready_for_loader.borrow_mut() = Some(Arc::clone(image));
                }
                result
            });
            let window = cx.add_window(move |_, _| ImageSourceProbe {
                source: source.clone(),
            });
            cx.run_until_parked();
            cx.refresh().unwrap();
            let mut visual = VisualTestContext::from_window(*window, cx);
            for _ in 0..100 {
                visual.run_until_parked();
                cx.background_executor
                    .advance_clock(Duration::from_millis(5));
                std::thread::sleep(Duration::from_millis(2));
                cx.run_until_parked();
                visual.update(|window, _| window.refresh());
                cx.refresh().unwrap();
                if let Some(image) = ready.borrow().as_ref() {
                    return image.as_bytes(0).unwrap().to_vec();
                }
            }
            panic!(
                "custom image source did not finish rendering after {} loader calls",
                calls.get()
            );
        }
    }
}

#[gpui::test]
fn tinted_svg_image_bytes_match_gpui_bgra_contract(cx: &mut TestAppContext) {
    let mut pixel = |body: &str, color: Option<Rgba8>| {
        let registry = AssetRegistry::new();
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" {body}/></svg>"#
        );
        let svg_len = svg.len();
        registry
            .register(
                "app",
                InMemoryAssetProvider::new(std::collections::BTreeMap::from([(
                    "pixel".to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: svg.into_bytes(),
                    },
                )])),
            )
            .unwrap();
        let handle = registry
            .load_image(&AssetId::parse("app/pixel").unwrap())
            .unwrap();
        let source = color
            .map_or_else(
                || registry.image_source(handle.opaque()),
                |color| registry.image_source_tinted(handle.opaque(), Some(color)),
            )
            .unwrap();
        let is_async_variant = matches!(&source, ImageSource::Custom(_));
        let pixels = render_image_source(cx, source);
        if is_async_variant {
            let stats = registry.svg_cache_stats().unwrap();
            assert_eq!(stats.entries, 1);
            assert_eq!(stats.misses, 1);
            assert!(stats.hits >= 1);
            assert!(stats.bytes > svg_len);
            assert!(stats.bytes <= stats.max_bytes);
        }
        pixels
    };

    assert_eq!(
        pixel(
            "fill=\"currentColor\"",
            Some(Rgba8::from_rgb_hex(0x0012_34ab))
        ),
        [0xab, 0x34, 0x12, 0xff]
    );
    assert_eq!(pixel("fill=\"#ff0000\"", None), [0x00, 0x00, 0xff, 0xff]);
    assert_eq!(
        pixel(
            "color=\"#ff0000\" fill=\"currentColor\"",
            Some(Rgba8::from_rgb_hex(0x0000_ff00))
        ),
        [0x00, 0x00, 0xff, 0xff]
    );
    assert_eq!(
        pixel(
            "fill=\"currentColor\"",
            Some(Rgba8::from_rgba_hex(0x1234_ab00))
        )[3],
        0
    );
    assert_eq!(
        pixel(
            "fill=\"currentColor\"",
            Some(Rgba8::from_rgba_hex(0x1234_ab80))
        ),
        [0xab, 0x34, 0x12, 0x80]
    );
}

#[gpui::test]
fn svg_raster_adapter_uses_the_gpui_system_font_environment(cx: &mut TestAppContext) {
    let registry = AssetRegistry::new();
    registry
        .register(
            "app",
            InMemoryAssetProvider::new(std::collections::BTreeMap::from([(
                "text".to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: br##"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="40"><path fill="#ff0000" d="M0 0h1v1H0z"/><text x="3" y="28" font-family="Definitely Missing, sans-serif" font-size="24" fill="black">Readable</text></svg>"##.to_vec(),
                },
            )])),
        )
        .unwrap();
    let handle = registry
        .load_image(&AssetId::parse("app/text").unwrap())
        .unwrap();
    let pixels = render_image_source(cx, registry.image_source(handle.opaque()).unwrap());
    assert!(
        pixels.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 100,
        "system font glyphs must survive the SVG raster adapter"
    );
}
