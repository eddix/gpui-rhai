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
    OverlayId, Rgba8, Style, TextDirection, ToastHostSpec, ToastItemSpec, ToastRegion, ToastVariant,
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
}

impl ToastHostElement {
    pub(crate) fn new(
        path: &str,
        spec: ToastHostSpec,
        palette: ToastPalette,
        coordinator: WindowOverlayCoordinator,
        dismiss: ToastDismissHandler,
        part_styles: ToastPartStyles,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/toast-host")).into(),
            spec,
            palette,
            coordinator,
            dismiss,
            part_styles,
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
        self.coordinator
            .toast_set_max_visible(self.spec.max_visible);
        let source_ids = self
            .spec
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();
        for region in toast_regions() {
            for id in self.coordinator.toast_visible(region) {
                if !source_ids.contains(id.as_str()) {
                    self.coordinator.toast_dismiss(&id);
                }
            }
        }
        for item in &self.spec.items {
            let id = OverlayId::new(item.id.clone());
            if !self.coordinator.toast_contains(&id)
                && let Ok(evicted) = self.coordinator.toast_enqueue(
                    id.clone(),
                    item.region,
                    Duration::from_millis(item.duration_ms),
                    now,
                )
            {
                for evicted in evicted {
                    schedule_dismiss(
                        Duration::ZERO,
                        self.coordinator.clone(),
                        self.dismiss.clone(),
                        Some(evicted),
                        window,
                        cx,
                    );
                }
                schedule_dismiss(
                    Duration::from_millis(item.duration_ms),
                    self.coordinator.clone(),
                    self.dismiss.clone(),
                    None,
                    window,
                    cx,
                );
            }
            if item.paused {
                self.coordinator.toast_pause(&id, now);
            } else if self.coordinator.toast_resume(&id, now)
                && let Some(remaining) = self.coordinator.toast_remaining(&id, now)
            {
                schedule_dismiss(
                    remaining,
                    self.coordinator.clone(),
                    self.dismiss.clone(),
                    None,
                    window,
                    cx,
                );
            }
        }
    }

    fn layer(&self, viewport: gpui::Size<Pixels>) -> AnyElement {
        let columns = toast_regions().into_iter().filter_map(|region| {
            let entries = self
                .coordinator
                .toast_visible(region)
                .into_iter()
                .filter_map(|id| {
                    self.spec
                        .items
                        .iter()
                        .find(|item| item.id == id.as_str())
                        .map(|item| {
                            render_toast(item, self.palette, &self.coordinator, &self.dismiss, self)
                        })
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then(|| toast_column(region, entries, self))
        });
        deferred(
            div()
                .absolute()
                .w(viewport.width)
                .h(viewport.height)
                .children(columns),
        )
        .with_priority(9_000)
        .into_any_element()
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
        let mut layer = self.layer(window.viewport_size());
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
    palette: ToastPalette,
    coordinator: &WindowOverlayCoordinator,
    dismiss: &ToastDismissHandler,
    styles: &ToastHostElement,
) -> AnyElement {
    let id = OverlayId::new(item.id.clone());
    let hover_id = id.clone();
    let hover_coordinator = coordinator.clone();
    let hover_dismiss = dismiss.clone();
    let close_id = id.clone();
    let close_coordinator = coordinator.clone();
    let close_dismiss = dismiss.clone();
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
                if close_coordinator.toast_dismiss(&close_id) {
                    close_dismiss(close_id.as_str().to_owned(), window, cx);
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
                schedule_dismiss(
                    remaining,
                    hover_coordinator.clone(),
                    hover_dismiss.clone(),
                    None,
                    window,
                    cx,
                );
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
    dismiss: ToastDismissHandler,
    immediate: Option<OverlayId>,
    window: &mut Window,
    cx: &mut App,
) {
    let timer = cx.background_executor().timer(delay);
    window
        .spawn(cx, async move |cx| {
            timer.await;
            let _ = cx.update(|window, cx| {
                let now = cx.background_executor().now();
                let expired = immediate.map_or_else(|| coordinator.toast_tick(now), |id| vec![id]);
                for id in expired {
                    dismiss(id.as_str().to_owned(), window, cx);
                }
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
