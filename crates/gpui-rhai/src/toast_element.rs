use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, SharedString,
    StatefulInteractiveElement, Styled, Window, deferred, div, px, rgba,
};

use crate::overlay_element::WindowOverlayCoordinator;
use crate::renderer::{OwnedColorResolver, apply_style_override};
use crate::{
    OverlayBounds, OverlayId, Rgba8, Style, TextDirection, ToastHostSpec, ToastItemSpec,
    ToastRegion, ToastVariant,
};

pub(crate) type ToastDismissHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;

#[derive(Clone, Copy)]
pub(crate) struct ToastPalette {
    pub surface: Rgba8,
    pub text: Rgba8,
    pub muted: Rgba8,
    pub border: Rgba8,
    pub success: Rgba8,
    pub warning: Rgba8,
    pub danger: Rgba8,
}

#[derive(Clone)]
pub(crate) struct ToastPartStyles {
    pub styles: BTreeMap<String, Style>,
    pub colors: OwnedColorResolver,
    pub direction: TextDirection,
}

pub(crate) struct ToastHostElement {
    id: ElementId,
    spec: ToastHostSpec,
    palette: ToastPalette,
    coordinator: WindowOverlayCoordinator,
    dismiss: ToastDismissHandler,
    part_styles: ToastPartStyles,
    view_id: String,
}

impl ToastHostElement {
    pub(crate) fn new(
        path: &str,
        spec: ToastHostSpec,
        palette: ToastPalette,
        coordinator: WindowOverlayCoordinator,
        dismiss: ToastDismissHandler,
        part_styles: ToastPartStyles,
        view_id: impl Into<String>,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/toast-host")).into(),
            spec,
            palette,
            coordinator,
            dismiss,
            part_styles,
            view_id: view_id.into(),
        }
    }

    fn style(&self, element: gpui::Div, part: &str) -> gpui::Div {
        if let Some(style) = self.part_styles.styles.get(part) {
            apply_style_override(
                element,
                style,
                &self.part_styles.colors,
                self.part_styles.direction,
            )
        } else {
            element
        }
    }

    fn synchronize(&self, window: &mut Window, cx: &mut App) {
        let now = cx.background_executor().now();
        let source_ids = self
            .spec
            .items
            .iter()
            .take(self.spec.max_visible)
            .map(|item| {
                WindowOverlayCoordinator::scoped_id(&self.view_id, &OverlayId::new(&item.id))
            })
            .collect::<BTreeSet<_>>();
        for region in toast_regions() {
            for id in self.coordinator.toast_visible(region) {
                if id.as_str().starts_with(&format!("{}::", self.view_id))
                    && !source_ids.contains(&id)
                {
                    let _ = self.coordinator.toast_dismiss(&id);
                }
            }
        }
        for item in self.spec.items.iter().take(self.spec.max_visible) {
            let id = WindowOverlayCoordinator::scoped_id(
                &self.view_id,
                &OverlayId::new(item.id.clone()),
            );
            if !self.coordinator.toast_contains(&id)
                && let Ok(evicted) = self.coordinator.toast_enqueue(
                    id.clone(),
                    item.id.clone(),
                    self.dismiss.clone(),
                    item.region,
                    Duration::from_millis(item.duration_ms),
                    now,
                )
            {
                self.coordinator.toast_dispatch(evicted, window, cx);
                schedule_dismiss(
                    Duration::from_millis(item.duration_ms),
                    self.coordinator.clone(),
                    window,
                    cx,
                );
            }
            if item.paused {
                self.coordinator.toast_pause(&id, now);
            } else if self.coordinator.toast_resume(&id, now)
                && let Some(remaining) = self.coordinator.toast_remaining(&id, now)
            {
                schedule_dismiss(remaining, self.coordinator.clone(), window, cx);
            }
        }
    }

    fn layer(&self, viewport: OverlayBounds) -> AnyElement {
        let columns = toast_regions().into_iter().filter_map(|region| {
            let entries = self
                .coordinator
                .toast_visible(region)
                .into_iter()
                .filter_map(|id| {
                    self.spec
                        .items
                        .iter()
                        .find(|item| {
                            WindowOverlayCoordinator::scoped_id(
                                &self.view_id,
                                &OverlayId::new(item.id.clone()),
                            ) == id
                        })
                        .map(|item| render_toast(item, &id, self.palette, &self.coordinator, self))
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then(|| toast_column(region, entries, self))
        });
        deferred(
            div()
                .absolute()
                .left(pixel_from_f64(viewport.x))
                .top(pixel_from_f64(viewport.y))
                .w(pixel_from_f64(viewport.width))
                .h(pixel_from_f64(viewport.height))
                .children(columns),
        )
        .with_priority(9_000)
        .into_any_element()
    }

    fn register_host_elements(&self) {
        for region in toast_regions() {
            for id in self.coordinator.toast_visible(region) {
                if let Some(item) = self.spec.items.iter().find(|item| {
                    WindowOverlayCoordinator::scoped_id(
                        &self.view_id,
                        &OverlayId::new(item.id.clone()),
                    ) == id
                }) {
                    let element = render_toast(item, &id, self.palette, &self.coordinator, self);
                    self.coordinator.register_toast_element(id, region, element);
                }
            }
        }
    }
}

impl Element for ToastHostElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.synchronize(window, cx);
        let mut layer = if self.coordinator.host_managed() {
            self.register_host_elements();
            div().into_any_element()
        } else {
            self.layer(self.coordinator.viewport_or_window(window.viewport_size()))
        };
        let layout = layer.request_layout(window, cx);
        (layout, layer)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        layer: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        layer.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        layer: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        layer.paint(window, cx);
    }
}

impl IntoElement for ToastHostElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn render_toast(
    item: &ToastItemSpec,
    id: &OverlayId,
    palette: ToastPalette,
    coordinator: &WindowOverlayCoordinator,
    styles: &ToastHostElement,
) -> AnyElement {
    let hover_id = id.clone();
    let hover_coordinator = coordinator.clone();
    let close_id = id.clone();
    let close_coordinator = coordinator.clone();
    let accent = match item.variant {
        ToastVariant::Neutral => palette.border,
        ToastVariant::Success => palette.success,
        ToastVariant::Warning => palette.warning,
        ToastVariant::Danger => palette.danger,
    };
    let close = item.dismissible.then(|| {
        styles
            .style(div(), "close")
            .id(SharedString::from(format!("toast-close-{}", item.id)))
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                if let Some((local_id, callback)) = close_coordinator.toast_dismiss(&close_id) {
                    callback(local_id, window, cx);
                    window.refresh();
                }
            })
            .child("×")
    });
    styles
        .style(
            div()
                .w(px(320.0))
                .p_3()
                .rounded(px(8.0))
                .border_1()
                .border_color(rgba(accent.as_rgba_hex()))
                .bg(rgba(palette.surface.as_rgba_hex()))
                .text_color(rgba(palette.text.as_rgba_hex())),
            "toast",
        )
        .id(SharedString::from(format!("toast-{}", item.id)))
        .on_hover(move |hovered, window, cx| {
            let now = cx.background_executor().now();
            if *hovered {
                hover_coordinator.toast_pause(&hover_id, now);
            } else if hover_coordinator.toast_resume(&hover_id, now)
                && let Some(remaining) = hover_coordinator.toast_remaining(&hover_id, now)
            {
                schedule_dismiss(remaining, hover_coordinator.clone(), window, cx);
            }
        })
        .child(
            div()
                .flex()
                .justify_between()
                .child(styles.style(div().child(item.title.clone()), "title"))
                .children(close),
        )
        .when(!item.message.is_empty(), |toast| {
            toast.child(
                styles.style(
                    div()
                        .pt_1()
                        .text_color(rgba(palette.muted.as_rgba_hex()))
                        .child(item.message.clone()),
                    "message",
                ),
            )
        })
        .into_any_element()
}

fn toast_column(
    region: ToastRegion,
    entries: Vec<AnyElement>,
    styles: &ToastHostElement,
) -> AnyElement {
    let column = styles
        .style(
            div()
                .w(px(320.0))
                .flex()
                .flex_col()
                .gap_2()
                .children(entries),
            "region",
        )
        .absolute();
    match region {
        ToastRegion::TopLeft => column.top(px(12.0)).left(px(12.0)),
        ToastRegion::TopRight => column.top(px(12.0)).right(px(12.0)),
        ToastRegion::BottomLeft => column.bottom(px(12.0)).left(px(12.0)),
        ToastRegion::BottomRight => column.bottom(px(12.0)).right(px(12.0)),
    }
    .into_any_element()
}

fn schedule_dismiss(
    delay: Duration,
    coordinator: WindowOverlayCoordinator,
    window: &mut Window,
    cx: &mut App,
) {
    let timer = cx.background_executor().timer(delay);
    window
        .spawn(cx, async move |cx| {
            timer.await;
            let _ = cx.update(|window, cx| {
                let now = cx.background_executor().now();
                coordinator.toast_tick(now, window, cx);
                window.refresh();
            });
        })
        .detach();
}

fn toast_regions() -> [ToastRegion; 4] {
    [
        ToastRegion::TopLeft,
        ToastRegion::TopRight,
        ToastRegion::BottomLeft,
        ToastRegion::BottomRight,
    ]
}

#[allow(clippy::cast_possible_truncation)]
fn pixel_from_f64(value: f64) -> Pixels {
    px(value as f32)
}
