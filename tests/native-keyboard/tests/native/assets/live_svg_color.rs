//! One retained image source must follow native text refinements without recreation.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Context, ImageSource, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    RenderImage, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window,
};
use gpui_rhai::{AssetData, AssetId, AssetRegistry, InMemoryAssetProvider};

const NORMAL: u32 = 0x1234_ab80;
const HOVER: u32 = 0x28bc_31ff;
const FOCUS: u32 = 0x2c31_4e00;

struct LiveColorImage {
    source: ImageSource,
}

impl Render for LiveColorImage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .id("color-source")
            .w(gpui::px(80.0))
            .h(gpui::px(40.0))
            .text_color(gpui::rgba(NORMAL))
            .hover(|style| style.text_color(gpui::rgba(HOVER)))
            .focusable()
            .tab_index(0)
            .focus(|style| style.text_color(gpui::rgba(FOCUS)))
            .child(gpui::img(self.source.clone()).size_full())
    }
}

type ReadyImages = Rc<RefCell<BTreeMap<u32, Arc<RenderImage>>>>;

fn settle_until(
    visual: &mut VisualTestContext,
    ready: &ReadyImages,
    color: u32,
) -> Arc<RenderImage> {
    for _ in 0..100 {
        visual.run_until_parked();
        visual
            .background_executor
            .advance_clock(Duration::from_millis(5));
        std::thread::sleep(Duration::from_millis(2));
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
        if let Some(image) = ready.borrow().get(&color) {
            return image.clone();
        }
    }
    panic!("SVG variant for {color:#x} never became ready");
}

fn observe(source: ImageSource, ready: &ReadyImages) -> ImageSource {
    let ImageSource::Custom(loader) = source else {
        panic!("inherited SVG is a native loader");
    };
    let ready = ready.clone();
    ImageSource::from(move |window: &mut Window, cx: &mut gpui::App| {
        // Record the ready image by the same byte colors authored by the root;
        // this deliberately does not reproduce the SDK's native-color conversion.
        let color = window.text_style().color;
        let key = [NORMAL, HOVER, FOCUS]
            .into_iter()
            .find(|bytes| color == gpui::Hsla::from(gpui::rgba(*bytes)))
            .unwrap_or_else(|| panic!("unexpected native foreground {color:?}"));
        let image = loader(window, cx);
        if let Some(Ok(image)) = &image {
            ready.borrow_mut().insert(key, image.clone());
        }
        image
    })
}

#[gpui::test]
fn one_svg_source_tracks_hover_and_restores_the_cached_color(cx: &mut TestAppContext) {
    let registry = AssetRegistry::new();
    registry.register("app",InMemoryAssetProvider::new(BTreeMap::from([("mixed".to_owned(),AssetData {
        mime_type:"image/svg+xml".to_owned(),
        bytes:br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="1" height="1" fill="currentColor"/><rect x="1" width="1" height="1" fill="#ff0000"/></svg>"##.to_vec(),
    })]))).unwrap();
    let handle = registry
        .load_image(&AssetId::parse("app/mixed").unwrap())
        .unwrap();
    let ready = Rc::new(RefCell::new(BTreeMap::new()));
    let source = observe(
        registry
            .image_source_inheriting_text_color(handle.opaque())
            .unwrap(),
        &ready,
    );
    let window = cx.add_window(move |_, _| LiveColorImage { source });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_mouse_move(
        gpui::point(gpui::px(200.0), gpui::px(200.0)),
        None,
        Modifiers::none(),
    );
    let normal = settle_until(&mut visual, &ready, NORMAL);
    assert_eq!(
        normal.as_bytes(0).unwrap(),
        [0xab, 0x34, 0x12, 0x80, 0, 0, 0xff, 0xff]
    );
    visual.simulate_mouse_move(
        gpui::point(gpui::px(20.0), gpui::px(20.0)),
        None,
        Modifiers::none(),
    );
    let hover = settle_until(&mut visual, &ready, HOVER);
    assert_eq!(
        hover.as_bytes(0).unwrap(),
        [0x31, 0xbc, 0x28, 0xff, 0, 0, 0xff, 0xff]
    );
    let misses = registry.svg_cache_stats().unwrap().misses;
    ready.borrow_mut().remove(&NORMAL);
    visual.simulate_mouse_move(
        gpui::point(gpui::px(200.0), gpui::px(200.0)),
        None,
        Modifiers::none(),
    );
    let restored = settle_until(&mut visual, &ready, NORMAL);
    assert!(
        Arc::ptr_eq(&restored, &normal),
        "returning to normal reuses the ready bitmap"
    );
    let stats = registry.svg_cache_stats().unwrap();
    assert_eq!(stats.misses, misses);
    assert_eq!(stats.entries, 2);
    assert!(stats.bytes <= stats.max_bytes);
    visual.update(|window, cx| window.focus_next(cx));
    let focused = settle_until(&mut visual, &ready, FOCUS);
    assert_eq!(
        focused.as_bytes(0).unwrap(),
        [0, 0, 0, 0, 0, 0, 0xff, 0xff],
        "transparent currentColor must not erase the explicit red sibling"
    );
    assert_eq!(registry.svg_cache_stats().unwrap().entries, 3);
}
