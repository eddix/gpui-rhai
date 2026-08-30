use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Element, ElementId, Entity, GlobalElementId,
    InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId, ParentElement,
    Pixels, Render, SharedString, StatefulInteractiveElement, Styled, Window, div, img, px, rgba,
};

use crate::dropdown_element::DropdownSlotRuntime;
use crate::overlay_element::{
    OpenChangeHandler, PanelKeyHandler, ScriptOverlayElement, WindowOverlayCoordinator,
};
use crate::{
    AssetId, DatePickerCellState, DatePickerKey, DatePickerNodeSpec, DatePickerOutcome,
    DatePickerState, GregorianDate, OverlayDismissPolicy, OverlayId, OverlayKind, OverlayNodeSpec,
    Rgba8, TextDirection,
};

pub(crate) type DateChangeHandler = Rc<dyn Fn(Option<String>, &mut Window, &mut App)>;

#[derive(Clone, Default)]
pub(crate) struct DatePickerCallbacks {
    pub change: Option<DateChangeHandler>,
}

#[derive(Clone, Copy)]
pub(crate) struct DatePickerPalette {
    pub surface: Rgba8,
    pub hover: Rgba8,
    pub text: Rgba8,
    pub muted: Rgba8,
    pub accent: Rgba8,
    pub on_accent: Rgba8,
    pub focus_ring: Rgba8,
}

pub(crate) struct DatePickerEntityElement {
    id: ElementId,
    spec: DatePickerNodeSpec,
    callbacks: DatePickerCallbacks,
    palette: DatePickerPalette,
    coordinator: WindowOverlayCoordinator,
    runtime: DropdownSlotRuntime,
}

impl DatePickerEntityElement {
    pub(crate) fn new(
        path: &str,
        spec: DatePickerNodeSpec,
        callbacks: DatePickerCallbacks,
        palette: DatePickerPalette,
        coordinator: WindowOverlayCoordinator,
        runtime: DropdownSlotRuntime,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/date-picker-entity")).into(),
            spec,
            callbacks,
            palette,
            coordinator,
            runtime,
        }
    }
}

struct DatePickerElementState {
    view: Entity<DatePickerView>,
}

pub(crate) struct DatePickerFrame {
    element: AnyElement,
}

impl Element for DatePickerEntityElement {
    type RequestLayoutState = DatePickerFrame;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            global_id.expect("DatePicker entity element is keyed"),
            |state, window| {
                let state = state.unwrap_or_else(|| DatePickerElementState {
                    view: cx.new(|_| {
                        DatePickerView::new(
                            self.spec.clone(),
                            self.callbacks.clone(),
                            self.palette,
                            self.coordinator.clone(),
                            self.runtime.clone(),
                        )
                    }),
                });
                state.view.update(cx, |view, cx| {
                    view.synchronize(
                        self.spec.clone(),
                        self.callbacks.clone(),
                        self.palette,
                        self.coordinator.clone(),
                        self.runtime.clone(),
                        cx,
                    );
                });
                let mut element = state.view.clone().into_any_element();
                let layout = element.request_layout(window, cx);
                ((layout, DatePickerFrame { element }), state)
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.paint(window, cx);
    }
}

impl IntoElement for DatePickerEntityElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct DatePickerView {
    spec: DatePickerNodeSpec,
    state: DatePickerState,
    callbacks: DatePickerCallbacks,
    palette: DatePickerPalette,
    coordinator: WindowOverlayCoordinator,
    runtime: DropdownSlotRuntime,
}

impl DatePickerView {
    fn new(
        spec: DatePickerNodeSpec,
        callbacks: DatePickerCallbacks,
        palette: DatePickerPalette,
        coordinator: WindowOverlayCoordinator,
        runtime: DropdownSlotRuntime,
    ) -> Self {
        let state = DatePickerState::new(spec.clone())
            .expect("validated DatePicker node creates valid native state");
        Self {
            spec,
            state,
            callbacks,
            palette,
            coordinator,
            runtime,
        }
    }

    fn synchronize(
        &mut self,
        spec: DatePickerNodeSpec,
        callbacks: DatePickerCallbacks,
        palette: DatePickerPalette,
        coordinator: WindowOverlayCoordinator,
        runtime: DropdownSlotRuntime,
        cx: &mut Context<Self>,
    ) {
        let changed = self
            .state
            .synchronize(spec.clone())
            .expect("validated DatePicker synchronization remains valid");
        self.spec = spec;
        self.callbacks = callbacks;
        self.palette = palette;
        self.coordinator = coordinator;
        self.runtime = runtime;
        if changed {
            cx.notify();
        }
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.state.set_open(open) {
            cx.notify();
        }
    }

    fn activate(&mut self, date: GregorianDate, cx: &mut Context<Self>) -> DateEmission {
        let outcome = self.state.select(date);
        self.emission(outcome, cx)
    }

    fn clear(&mut self, cx: &mut Context<Self>) -> DateEmission {
        let outcome = self.state.clear();
        self.emission(outcome, cx)
    }

    fn emission(&self, outcome: DatePickerOutcome, cx: &mut Context<Self>) -> DateEmission {
        if outcome.open_changed || outcome.value_changed {
            cx.notify();
        }
        DateEmission {
            callbacks: self.callbacks.clone(),
            dismiss: (outcome.open_changed && !self.state.is_open())
                .then(|| (self.coordinator.clone(), self.overlay_id())),
            value: if outcome.value_changed {
                DateValueEmission::Changed(self.state.value().map(GregorianDate::to_iso))
            } else {
                DateValueEmission::None
            },
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> (bool, DateEmission) {
        let key = match event.keystroke.key.as_str() {
            "left" if self.runtime.direction == TextDirection::RightToLeft => {
                Some(DatePickerKey::NextDay)
            }
            "left" => Some(DatePickerKey::PreviousDay),
            "right" if self.runtime.direction == TextDirection::RightToLeft => {
                Some(DatePickerKey::PreviousDay)
            }
            "right" => Some(DatePickerKey::NextDay),
            "up" => Some(DatePickerKey::PreviousWeek),
            "down" => Some(DatePickerKey::NextWeek),
            "home" => Some(DatePickerKey::WeekStart),
            "end" => Some(DatePickerKey::WeekEnd),
            "pageup" if event.keystroke.modifiers.shift => Some(DatePickerKey::PreviousYear),
            "pageup" => Some(DatePickerKey::PreviousMonth),
            "pagedown" if event.keystroke.modifiers.shift => Some(DatePickerKey::NextYear),
            "pagedown" => Some(DatePickerKey::NextMonth),
            "enter" | "space" => Some(DatePickerKey::Commit),
            "escape" => Some(DatePickerKey::Escape),
            _ => None,
        };
        let Some(key) = key else {
            return (false, DateEmission::default());
        };
        let outcome = self.state.handle_key(key).unwrap_or_default();
        cx.notify();
        (true, self.emission(outcome, cx))
    }

    fn change_month(&mut self, months: i32, cx: &mut Context<Self>) {
        if self.state.move_visible_month(months).unwrap_or(false) {
            cx.notify();
        }
    }

    fn render_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak_previous = cx.entity().downgrade();
        let weak_next = weak_previous.clone();
        let previous_enabled = self.state.can_move_visible_month(-1);
        let next_enabled = self.state.can_move_visible_month(1);
        let title = month_title(self.state.visible_month(), &self.spec);
        let previous = self
            .runtime
            .style(
                div().child(asset_element(
                    &self.runtime,
                    &self.spec.previous_asset,
                    self.runtime.part_color("previous", self.palette.text),
                    self.spec.cell_size * 0.5,
                )),
                "previous",
            )
            .id("date-picker-previous")
            .when(previous_enabled, |button| {
                button.on_click(move |_, _, app| {
                    let _ = weak_previous.update(app, |view, cx| view.change_month(-1, cx));
                })
            })
            .opacity(if previous_enabled { 1.0 } else { 0.45 });
        let next = self
            .runtime
            .style(
                div().child(asset_element(
                    &self.runtime,
                    &self.spec.next_asset,
                    self.runtime.part_color("next", self.palette.text),
                    self.spec.cell_size * 0.5,
                )),
                "next",
            )
            .id("date-picker-next")
            .when(next_enabled, |button| {
                button.on_click(move |_, _, app| {
                    let _ = weak_next.update(app, |view, cx| view.change_month(1, cx));
                })
            })
            .opacity(if next_enabled { 1.0 } else { 0.45 });
        let header = self.runtime.style(
            div()
                .h(px(to_f32(self.spec.cell_size)))
                .flex()
                .items_center()
                .justify_between()
                .child(previous)
                .child(title)
                .child(next),
            "header",
        );
        let weekdays = self.render_weekdays();
        let grid = self.render_grid(cx);
        let footer = self.render_footer(cx);
        self.runtime
            .style(
                div()
                    .w(px(to_f32(self.spec.panel_width)))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(header)
                    .child(weekdays)
                    .child(grid)
                    .children(footer),
                "panel",
            )
            .into_any_element()
    }

    fn render_weekdays(&self) -> AnyElement {
        let first = self.spec.calendar.first_weekday.sunday_index();
        let labels = (0..7).map(|offset| {
            let index = (first + offset) % 7;
            self.runtime.style(
                div()
                    .w(px(to_f32(self.spec.cell_size)))
                    .h(px(to_f32(self.spec.cell_size)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(self.palette.muted.as_rgba_hex()))
                    .child(self.spec.calendar.weekdays.short[index].clone()),
                "weekday",
            )
        });
        self.runtime
            .style(div().flex().children(labels), "weekdays")
            .into_any_element()
    }

    fn render_grid(&self, cx: &mut Context<Self>) -> AnyElement {
        let cells = self.state.cells();
        let rows = cells.chunks(7).enumerate().map(|(week_index, week)| {
            div()
                .flex()
                .children(week.iter().enumerate().map(|(day_index, cell)| {
                    let date = cell.date;
                    let weak = cx.entity().downgrade();
                    let disabled = cell.has(DatePickerCellState::Disabled);
                    let selected = cell.has(DatePickerCellState::Selected);
                    let today = cell.has(DatePickerCellState::Today);
                    let outside = cell.has(DatePickerCellState::OutsideMonth);
                    let part = if disabled {
                        "day_disabled"
                    } else if selected {
                        "day_selected"
                    } else if today {
                        "day_today"
                    } else if outside {
                        "day_outside"
                    } else {
                        "day"
                    };
                    let label = date.map_or_else(String::new, |date| {
                        localize_day(date.day(), &self.spec.number)
                    });
                    self.runtime
                        .style(
                            div()
                                .w(px(to_f32(self.spec.cell_size)))
                                .h(px(to_f32(self.spec.cell_size)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(rgba(if disabled || outside {
                                    self.palette.muted.as_rgba_hex()
                                } else if selected {
                                    self.palette.on_accent.as_rgba_hex()
                                } else {
                                    self.palette.text.as_rgba_hex()
                                }))
                                .when(selected, |day| {
                                    day.bg(rgba(self.palette.accent.as_rgba_hex()))
                                })
                                .when(
                                    date == Some(self.state.focused()) && !selected && !disabled,
                                    |day| day.bg(rgba(self.palette.hover.as_rgba_hex())),
                                )
                                .child(label),
                            part,
                        )
                        .id(SharedString::from(format!(
                            "date-picker-day-{week_index}-{day_index}"
                        )))
                        .when_some(date.filter(|_| !disabled), |day, date| {
                            day.on_click(move |_, window, app| {
                                if let Ok(emission) =
                                    weak.update(app, |view, cx| view.activate(date, cx))
                                {
                                    emission.emit(window, app);
                                }
                            })
                        })
                        .into_any_element()
                }))
        });
        self.runtime
            .style(div().flex().flex_col().children(rows), "grid")
            .into_any_element()
    }

    fn render_footer(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut footer = Vec::new();
        if !self.state.presets().is_empty() {
            let presets = self
                .state
                .presets()
                .iter()
                .enumerate()
                .map(|(index, preset)| {
                    let weak = cx.entity().downgrade();
                    let date = preset.value;
                    self.runtime
                        .style(
                            div()
                                .opacity(if preset.disabled { 0.45 } else { 1.0 })
                                .child(preset.label.clone()),
                            "preset",
                        )
                        .id(("date-picker-preset", index))
                        .when(!preset.disabled, |button| {
                            button.on_click(move |_, window, app| {
                                if let Ok(emission) =
                                    weak.update(app, |view, cx| view.activate(date, cx))
                                {
                                    emission.emit(window, app);
                                }
                            })
                        })
                        .into_any_element()
                });
            footer.push(
                self.runtime
                    .style(div().flex().gap_1().children(presets), "footer")
                    .into_any_element(),
            );
        }
        footer
    }

    fn overlay_spec(&self) -> OverlayNodeSpec {
        let id = self.overlay_id();
        let parent = self.spec.parent_overlay.as_ref().map(|parent| {
            WindowOverlayCoordinator::scoped_id(
                &self.runtime.view_id,
                &OverlayId::new(parent.clone()),
            )
        });
        OverlayNodeSpec {
            id,
            parent,
            kind: OverlayKind::Dropdown,
            placement: self.spec.placement,
            open: self.state.is_open() && !self.spec.disabled,
            gap: self.spec.overlay_gap,
            modal: false,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
        }
    }

    fn overlay_id(&self) -> OverlayId {
        WindowOverlayCoordinator::scoped_id(
            &self.runtime.view_id,
            &OverlayId::new(self.spec.id.clone()),
        )
    }
}

impl Render for DatePickerView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.spec.display_value.is_empty() {
            self.spec.placeholder.clone()
        } else {
            self.spec.display_value.clone()
        };
        let trigger_value = self.runtime.style(div().child(label), "trigger_value");
        let clearable = self.spec.clearable && self.spec.value.is_some();
        let weak_clear = cx.entity().downgrade();
        let trigger = self.runtime.style(
            div()
                .w(px(to_f32(self.spec.panel_width)))
                .h(px(to_f32(self.spec.trigger_height)))
                .flex()
                .items_center()
                .justify_between()
                .child(trigger_value)
                .when(clearable, |trigger| {
                    trigger.child(
                        self.runtime
                            .style(
                                div().child(asset_element(
                                    &self.runtime,
                                    &self.spec.clear_asset,
                                    self.runtime.part_color("clear", self.palette.muted),
                                    self.spec.trigger_height * 0.45,
                                )),
                                "clear",
                            )
                            .id("date-picker-clear")
                            .on_click(move |_, window, app| {
                                if let Ok(emission) = weak_clear.update(app, DatePickerView::clear)
                                {
                                    emission.emit(window, app);
                                }
                                app.stop_propagation();
                            }),
                    )
                })
                .child(self.runtime.style(
                    div().child(asset_element(
                        &self.runtime,
                        &self.spec.trigger_asset,
                        self.runtime.part_color("trigger_icon", self.palette.muted),
                        self.spec.trigger_height * 0.5,
                    )),
                    "trigger_icon",
                )),
            "trigger",
        );
        let content = self.render_panel(cx);
        let weak_open = cx.entity().downgrade();
        let open_change = (!self.spec.disabled).then(|| {
            Rc::new(move |open, _: &mut Window, app: &mut App| {
                let _ = weak_open.update(app, |view, cx| view.set_open(open, cx));
            }) as OpenChangeHandler
        });
        let weak_key = cx.entity().downgrade();
        let panel_key = Rc::new(
            move |event: &KeyDownEvent, window: &mut Window, app: &mut App| match weak_key
                .update(app, |view, cx| view.handle_key(event, cx))
            {
                Ok((handled, emission)) => {
                    emission.emit(window, app);
                    handled
                }
                Err(_) => false,
            },
        ) as PanelKeyHandler;
        ScriptOverlayElement::new(
            &format!("date-picker/{}", self.spec.id),
            trigger.into_any_element(),
            content,
            self.overlay_spec(),
            open_change,
            Some(panel_key),
            self.coordinator.clone(),
        )
        .with_focus_ring(self.palette.focus_ring)
        .with_focus_surface(self.palette.surface)
        .with_open_key("down")
        .restore_focus_on_close(true)
    }
}

#[derive(Clone, Default)]
struct DateEmission {
    callbacks: DatePickerCallbacks,
    dismiss: Option<(WindowOverlayCoordinator, OverlayId)>,
    value: DateValueEmission,
}

#[derive(Clone, Default)]
enum DateValueEmission {
    #[default]
    None,
    Changed(Option<String>),
}

impl DateEmission {
    fn emit(self, window: &mut Window, cx: &mut App) {
        if let Some((coordinator, id)) = self.dismiss {
            let _ = coordinator.dismiss(&id, window, cx);
            window.refresh();
        }
        if let (Some(change), DateValueEmission::Changed(value)) =
            (self.callbacks.change, self.value)
        {
            change(value, window, cx);
        }
    }
}

fn month_title(date: GregorianDate, spec: &DatePickerNodeSpec) -> String {
    crate::locale::format_month_year_with_metadata(date, &spec.calendar, &spec.number)
        .unwrap_or_else(|error| error.to_string())
}

fn localize_day(day: u8, number: &crate::NumberMetadata) -> String {
    localize_ascii_digits(&day.to_string(), &number.digits)
}

fn localize_ascii_digits(value: &str, digits: &[String]) -> String {
    value
        .chars()
        .map(|character| {
            character.to_digit(10).map_or_else(
                || character.to_string(),
                |digit| digits[usize::try_from(digit).unwrap_or(0)].clone(),
            )
        })
        .collect()
}

fn asset_element(
    runtime: &DropdownSlotRuntime,
    asset: &AssetId,
    tint: Rgba8,
    size: f64,
) -> AnyElement {
    runtime.assets.cached_image(asset).map_or_else(
        |error| {
            div()
                .child(format!("Asset error: {error}"))
                .into_any_element()
        },
        |handle| {
            runtime
                .assets
                .image_source_tinted(handle.opaque(), Some(tint))
                .map_or_else(
                    |error| {
                        div()
                            .child(format!("Image error: {error}"))
                            .into_any_element()
                    },
                    |source| {
                        img(source)
                            .w(px(to_f32(size)))
                            .h(px(to_f32(size)))
                            .into_any_element()
                    },
                )
        },
    )
}

fn to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(f32::MAX)
}
