//! One native image owns its loading lease, diagnostic and non-interactive cue.

use std::{
    rc::{Rc, Weak},
    sync::Arc,
};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, Image, ImageSource,
    InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels, SharedString, Styled,
    StyledImage, Window, div, img,
};

use crate::presentation_diagnostic::PresentationSink;
use crate::renderer::{OwnedColorResolver, apply_style_override};
use crate::{ImageSourceSpec, InlineSvg, NodeId, ScriptGeneration, Style, TextDirection};

#[derive(Clone)]
pub(crate) enum ImagePresentationIdentity {
    Image {
        source: ImageSourceSpec,
        backing: Option<Arc<Image>>,
    },
    Svg(InlineSvg),
}

impl PartialEq for ImagePresentationIdentity {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Image {
                    source: left,
                    backing: a,
                },
                Self::Image {
                    source: right,
                    backing: b,
                },
            ) => {
                left == right
                    && match (a, b) {
                        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (Self::Svg(left), Self::Svg(right)) => left == right,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ImageFallbackText {
    pub text: String,
    pub style: Style,
    pub colors: OwnedColorResolver,
    pub direction: TextDirection,
}

impl ImageFallbackText {
    fn element(&self) -> AnyElement {
        apply_style_override(
            div().flex().size_full().items_center().justify_center(),
            &self.style,
            &self.colors,
            self.direction,
        )
        .child(self.text.clone())
        .into_any_element()
    }
}

#[derive(Clone)]
pub(crate) struct ImagePresentationReport {
    pub sink: Option<PresentationSink>,
    pub path: String,
    pub node: Option<NodeId>,
    pub asset: Option<String>,
}

impl ImagePresentationReport {
    fn result(&self, error: Option<String>) {
        if let Some(sink) = &self.sink {
            sink.image_result(&self.path, self.node, self.asset.as_deref(), error);
        }
    }
}

struct ImagePresentationState {
    identity: ImagePresentationIdentity,
    generation: ScriptGeneration,
    lease: Rc<()>,
}

pub(crate) struct ImagePresentationElement {
    pub id: ElementId,
    pub identity: ImagePresentationIdentity,
    pub generation: ScriptGeneration,
    pub source: Result<ImageSource, String>,
    pub fallback: Option<ImageFallbackText>,
    pub report: ImagePresentationReport,
    pub fill_box: bool,
}

#[cfg(feature = "vendor-gpui")]
impl IntoElement for ImagePresentationElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

fn observe_source(
    source: ImageSource,
    lease: Weak<()>,
    report: ImagePresentationReport,
) -> ImageSource {
    match source {
        ImageSource::Custom(loader) => {
            ImageSource::from(move |window: &mut Window, cx: &mut App| {
                let result = loader(window, cx);
                if lease.upgrade().is_some()
                    && let Some(result) = &result
                {
                    report.result(result.as_ref().err().map(ToString::to_string));
                }
                result
            })
        }
        source => source,
    }
}

pub(crate) fn sized_image(source: impl Into<ImageSource>, fill_box: bool) -> gpui::Img {
    let image = img(source);
    if fill_box { image.size_full() } else { image }
}

impl Element for ImagePresentationElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        window.with_element_state(
            id.expect("image presentation is keyed"),
            |previous: Option<ImagePresentationState>, window| {
                let changed = previous.as_ref().is_none_or(|previous| {
                    previous.identity != self.identity || previous.generation != self.generation
                });
                let state = if changed {
                    ImagePresentationState {
                        identity: self.identity.clone(),
                        generation: self.generation,
                        lease: Rc::new(()),
                    }
                } else {
                    previous.expect("unchanged image has state")
                };
                let lease = Rc::downgrade(&state.lease);
                let mut element = match &self.source {
                    Ok(source) => {
                        // A pending frame is not successful recovery. Clear once
                        // for a new content lease, not on every repaint/lookup.
                        if changed {
                            self.report.result(None);
                        }
                        let custom = matches!(source, ImageSource::Custom(_));
                        let source =
                            observe_source(source.clone(), lease.clone(), self.report.clone());
                        let fallback = self.fallback.clone();
                        let report = self.report.clone();
                        let image = sized_image(source, self.fill_box).with_fallback(move || {
                            if !custom && lease.upgrade().is_some() {
                                report.result(Some("native image loading failed".into()));
                            }
                            fallback.as_ref().map_or_else(
                                || div().into_any_element(),
                                ImageFallbackText::element,
                            )
                        });
                        image.into_any_element()
                    }
                    Err(error) => {
                        self.report.result(Some(error.clone()));
                        self.fallback.as_ref().map_or_else(
                            || {
                                div()
                                    .child(format!("Image error: {error}"))
                                    .into_any_element()
                            },
                            ImageFallbackText::element,
                        )
                    }
                };
                let layout = element.request_layout(window, cx);
                ((layout, element), state)
            },
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

pub(crate) fn presentation_id(path: &str) -> ElementId {
    SharedString::from(format!("{path}/image-presentation")).into()
}

#[cfg(all(test, any(feature = "offscreen-pre", feature = "offscreen-fast")))]
mod tests {
    use super::*;
    use crate::presentation_diagnostic::PresentationDiagnostics;
    use crate::{AssetId, ColorValue, Rgba8};
    use gpui::{Context, Render, TestAppContext, VisualTestContext};
    use std::cell::Cell;

    struct Probe {
        source: ImageSource,
        diagnostics: PresentationDiagnostics,
        resource: Rc<Cell<bool>>,
    }
    struct EmptyHost;
    impl Render for EmptyHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .w(gpui::px(40.))
                .h(gpui::px(40.))
                .child(ImagePresentationElement {
                    id: presentation_id("probe"),
                    identity: ImagePresentationIdentity::Image {
                        source: ImageSourceSpec::Asset(
                            AssetId::parse(if self.resource.get() {
                                "app/second"
                            } else {
                                "app/first"
                            })
                            .unwrap(),
                        ),
                        backing: None,
                    },
                    generation: ScriptGeneration::default(),
                    source: Ok(self.source.clone()),
                    fill_box: true,
                    fallback: Some(ImageFallbackText {
                        text: "A".into(),
                        style: Style::new()
                            .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0000_ff00))),
                        colors: OwnedColorResolver::default(),
                        direction: TextDirection::LeftToRight,
                    }),
                    report: ImagePresentationReport {
                        sink: Some(self.diagnostics.sink(ScriptGeneration::default())),
                        path: "probe".into(),
                        node: None,
                        asset: None,
                    },
                })
        }
    }
    fn cue(visual: &mut VisualTestContext) -> usize {
        visual.update(|window, _| {
            window
                .painted_quads()
                .iter()
                .filter(|quad| quad.background == gpui::Background::from(gpui::rgb(0x0000_ff00)))
                .count()
        })
    }
    fn frame(visual: &mut VisualTestContext) {
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
    }

    #[gpui::test]
    fn native_custom_pending_failure_recovery_and_replacement_have_one_error_lease(
        cx: &mut TestAppContext,
    ) {
        let phase = Rc::new(Cell::new(0));
        let selected = phase.clone();
        let image = Arc::new(gpui::RenderImage::new(vec![image::Frame::new(
            image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 255, 255])),
        )]));
        let source = ImageSource::from(move |_: &mut Window, _: &mut App| match selected.get() {
            0 => None,
            1 => Some(Err(gpui::ImageCacheError::Asset("specific failure".into()))),
            _ => Some(Ok(image.clone())),
        });
        let diagnostics = PresentationDiagnostics::default();
        let reports = diagnostics.clone();
        let resource = Rc::new(Cell::new(false));
        let key = resource.clone();
        let window = cx.add_window(move |_, _| Probe {
            source,
            diagnostics,
            resource,
        });
        let mut visual = VisualTestContext::from_window(*window, cx);
        frame(&mut visual);
        assert_eq!(cue(&mut visual), 0);
        assert_eq!(reports.drain(), Vec::new());
        phase.set(1);
        frame(&mut visual);
        assert_eq!(cue(&mut visual), 1);
        let first = reports.drain();
        assert_eq!(first.len(), 1);
        assert!(first[0].message.contains("specific failure"));
        for _ in 0..4 {
            frame(&mut visual);
            assert_eq!(reports.drain(), Vec::new());
        }
        phase.set(0);
        frame(&mut visual);
        assert_eq!(cue(&mut visual), 0);
        assert_eq!(reports.drain(), Vec::new());
        phase.set(1);
        frame(&mut visual);
        assert_eq!(reports.drain(), Vec::new(), "pending is not recovery");
        phase.set(2);
        frame(&mut visual);
        assert_eq!(cue(&mut visual), 0);
        phase.set(1);
        frame(&mut visual);
        assert_eq!(
            reports.drain().len(),
            1,
            "real ready result permits a later failure"
        );
        key.set(true);
        window.update(&mut visual, |_, _, cx| cx.notify()).unwrap();
        frame(&mut visual);
        assert_eq!(reports.drain().len(), 1, "new source gets its own lease");
    }

    #[gpui::test]
    fn obsolete_native_loader_cannot_clear_or_replace_current_diagnostic(cx: &mut TestAppContext) {
        let reports = PresentationDiagnostics::default();
        let sink = reports.sink(ScriptGeneration::default());
        let lease = Rc::new(());
        let image = Arc::new(gpui::RenderImage::new(vec![image::Frame::new(
            image::RgbaImage::new(1, 1),
        )]));
        let old = observe_source(
            ImageSource::from(move |_: &mut Window, _: &mut App| Some(Ok(image.clone()))),
            Rc::downgrade(&lease),
            ImagePresentationReport {
                sink: Some(sink.clone()),
                path: "same".into(),
                node: None,
                asset: None,
            },
        );
        let old_error = observe_source(
            ImageSource::from(|_: &mut Window, _: &mut App| {
                Some(Err(gpui::ImageCacheError::Asset("obsolete failure".into())))
            }),
            Rc::downgrade(&lease),
            ImagePresentationReport {
                sink: Some(sink.clone()),
                path: "same".into(),
                node: None,
                asset: None,
            },
        );
        drop(lease);
        sink.image_result("same", None, None, Some("current failure".into()));
        let window = cx.add_window(|_, _| EmptyHost);
        window
            .update(cx, |_, window, cx| {
                let ImageSource::Custom(loader) = old else {
                    panic!()
                };
                assert!(loader(window, cx).unwrap().is_ok());
                let ImageSource::Custom(loader) = old_error else {
                    panic!()
                };
                assert!(loader(window, cx).unwrap().is_err());
            })
            .unwrap();
        assert_eq!(reports.drain()[0].message, "current failure");
    }
}
