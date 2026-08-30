use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Bounds, ClickEvent, Context, DispatchPhase, Element, ElementId,
    Entity, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Render, ScrollHandle, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled,
    UniformListScrollHandle, Window, canvas, div, img, point, px, relative, rems, rgba,
    uniform_list,
};

use crate::dropdown_element::DropdownSlotRuntime;
use crate::{
    AssetId, ColorResolver, Length, Rgba8, TableAlign, TableNodeSpec, TableSelectionMode,
    TableSort, TableSortDirection, TableState, TextDirection,
};

pub(crate) type TableSortHandler = Rc<dyn Fn(Option<TableSort>, &mut Window, &mut App)>;
pub(crate) type TableSelectionHandler = Rc<dyn Fn(Vec<String>, &mut Window, &mut App)>;
pub(crate) type TableRowHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;

#[derive(Clone, Default)]
pub(crate) struct TableCallbacks {
    pub sort: Option<TableSortHandler>,
    pub selection: Option<TableSelectionHandler>,
    pub row_click: Option<TableRowHandler>,
}

#[derive(Clone, Copy)]
pub(crate) struct TablePalette {
    pub surface: Rgba8,
    pub raised: Rgba8,
    pub hover: Rgba8,
    pub text: Rgba8,
    pub muted: Rgba8,
    pub accent: Rgba8,
    pub on_accent: Rgba8,
    pub border: Rgba8,
}

pub(crate) struct TableEntityElement {
    id: ElementId,
    spec: TableNodeSpec,
    callbacks: TableCallbacks,
    palette: TablePalette,
    runtime: DropdownSlotRuntime,
}

impl TableEntityElement {
    pub(crate) fn new(
        path: &str,
        spec: TableNodeSpec,
        callbacks: TableCallbacks,
        palette: TablePalette,
        runtime: DropdownSlotRuntime,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/table-entity")).into(),
            spec,
            callbacks,
            palette,
            runtime,
        }
    }
}

struct TableElementState {
    view: Entity<TableView>,
}

pub(crate) struct TableFrame {
    element: AnyElement,
    view: Entity<TableView>,
}

impl Element for TableEntityElement {
    type RequestLayoutState = TableFrame;
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
            global_id.expect("Table entity element is keyed"),
            |state, window| {
                let state = state.unwrap_or_else(|| TableElementState {
                    view: cx.new(|_| {
                        TableView::new(
                            self.spec.clone(),
                            self.callbacks.clone(),
                            self.palette,
                            self.runtime.clone(),
                        )
                    }),
                });
                state.view.update(cx, |view, cx| {
                    view.synchronize(
                        self.spec.clone(),
                        self.callbacks.clone(),
                        self.palette,
                        self.runtime.clone(),
                        cx,
                    );
                });
                let mut element = state.view.clone().into_any_element();
                let layout = element.request_layout(window, cx);
                (
                    (
                        layout,
                        TableFrame {
                            element,
                            view: state.view.clone(),
                        },
                    ),
                    state,
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let width = f64::from(bounds.size.width);
        frame.view.update(cx, |view, cx| {
            let height = f64::from(bounds.size.height);
            if (view.viewport_width - width).abs() > f64::EPSILON
                || (view.viewport_height - height).abs() > f64::EPSILON
            {
                view.viewport_width = width;
                view.viewport_height = height;
                view.horizontal_positioned = false;
                cx.notify();
            }
        });
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

impl IntoElement for TableEntityElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct TableView {
    spec: TableNodeSpec,
    state: TableState,
    callbacks: TableCallbacks,
    palette: TablePalette,
    runtime: DropdownSlotRuntime,
    vertical: UniformListScrollHandle,
    horizontal: ScrollHandle,
    viewport_width: f64,
    viewport_height: f64,
    horizontal_positioned: bool,
    horizontal_drag_grab: Option<Pixels>,
}

impl TableView {
    fn new(
        spec: TableNodeSpec,
        callbacks: TableCallbacks,
        palette: TablePalette,
        runtime: DropdownSlotRuntime,
    ) -> Self {
        let state = TableState::new(spec.clone()).expect("validated Table node creates state");
        Self {
            spec,
            state,
            callbacks,
            palette,
            runtime,
            vertical: UniformListScrollHandle::new(),
            horizontal: ScrollHandle::new(),
            viewport_width: 800.0,
            viewport_height: 400.0,
            horizontal_positioned: false,
            horizontal_drag_grab: None,
        }
    }

    fn synchronize(
        &mut self,
        spec: TableNodeSpec,
        callbacks: TableCallbacks,
        palette: TablePalette,
        runtime: DropdownSlotRuntime,
        cx: &mut Context<Self>,
    ) {
        let layout_changed = self.spec.columns != spec.columns
            || self.spec.selection_mode != spec.selection_mode
            || (self.spec.selection_width - spec.selection_width).abs() > f64::EPSILON
            || (self.spec.flex_min_width - spec.flex_min_width).abs() > f64::EPSILON
            || (self.spec.horizontal_scrollbar_height - spec.horizontal_scrollbar_height).abs()
                > f64::EPSILON
            || (self.spec.horizontal_scrollbar_thumb_min_width
                - spec.horizontal_scrollbar_thumb_min_width)
                .abs()
                > f64::EPSILON
            || (self.spec.horizontal_scrollbar_inset - spec.horizontal_scrollbar_inset).abs()
                > f64::EPSILON
            || self.spec.height != spec.height;
        let changed = self
            .state
            .synchronize(spec.clone())
            .expect("validated Table synchronization remains valid");
        let direction_changed = self.runtime.direction != runtime.direction;
        self.spec = spec;
        self.callbacks = callbacks;
        self.palette = palette;
        self.runtime = runtime;
        if direction_changed || layout_changed {
            self.horizontal_positioned = false;
            self.horizontal_drag_grab = None;
        }
        if changed {
            cx.notify();
        }
    }

    fn render_header(&self, widths: &[f64], cx: &mut Context<Self>) -> AnyElement {
        let mut cells = self
            .render_selection_header(cx)
            .into_iter()
            .collect::<Vec<_>>();
        cells.extend(self.spec.columns.iter().zip(widths).map(|(column, width)| {
            let weak_click = cx.entity().downgrade();
            let weak_key = weak_click.clone();
            let key = column.key.clone();
            let indicator_asset = self
                .spec
                .sort
                .as_ref()
                .filter(|sort| sort.key == column.key)
                .map(|sort| match sort.direction {
                    TableSortDirection::Ascending => &self.spec.sort_ascending_asset,
                    TableSortDirection::Descending => &self.spec.sort_descending_asset,
                });
            let indicator = indicator_asset.map_or_else(
                || div().into_any_element(),
                |asset| {
                    table_asset_element(
                        &self.runtime,
                        asset,
                        self.runtime.part_color("header_cell", self.palette.muted),
                        self.spec.row_height * 0.4,
                    )
                },
            );
            let cell = self.runtime.style(
                align_cell(
                    div()
                        .w(px(to_f32(*width)))
                        .h(px(to_f32(self.spec.row_height)))
                        .flex()
                        .items_center()
                        .gap_1()
                        .overflow_hidden()
                        .child(column.title.clone())
                        .child(indicator),
                    column.align,
                    self.runtime.direction,
                ),
                "header_cell",
            );
            if column.sortable {
                let key_for_keyboard = key.clone();
                cell.id(SharedString::from(format!("table-sort-{key}")))
                    .tab_index(0)
                    .tab_stop(true)
                    .on_click(move |event, window, app| {
                        if !matches!(event, ClickEvent::Mouse(_)) {
                            return;
                        }
                        emit_sort(&weak_click, &key, window, app);
                    })
                    .on_key_down(move |event, window, app| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            emit_sort(&weak_key, &key_for_keyboard, window, app);
                            app.stop_propagation();
                        }
                    })
                    .into_any_element()
            } else {
                cell.into_any_element()
            }
        }));
        self.runtime
            .style(
                logical_table_row(div().flex().children(cells), self.runtime.direction),
                "header",
            )
            .into_any_element()
    }

    fn render_selection_header(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        (self.spec.selection_mode == TableSelectionMode::Multiple).then(|| {
            let weak_click = cx.entity().downgrade();
            let weak_key = weak_click.clone();
            let checked =
                !self.spec.rows.is_empty() && self.spec.selected_keys.len() == self.spec.rows.len();
            self.runtime
                .style(
                    div()
                        .w(px(to_f32(self.spec.selection_width)))
                        .h(px(to_f32(self.spec.row_height)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(selection_indicator(
                            &self.runtime,
                            checked,
                            self.palette,
                            &self.spec.check_asset,
                            self.spec.selection_size,
                        )),
                    "selection_header",
                )
                .id("table-select-all")
                .tab_index(0)
                .tab_stop(true)
                .on_click(move |event, window, app| {
                    if matches!(event, ClickEvent::Mouse(_)) {
                        emit_select_all(&weak_click, window, app);
                    }
                })
                .on_key_down(move |event, window, app| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        emit_select_all(&weak_key, window, app);
                        app.stop_propagation();
                    }
                })
                .into_any_element()
        })
    }

    fn render_body(&self, widths: Vec<f64>, cx: &mut Context<Self>) -> AnyElement {
        if self.spec.loading {
            let content = if let Some(slot) = self.spec.loading_slot.as_deref() {
                self.runtime.render(slot, "loading_slot")
            } else if self.spec.loading_rows.is_empty() {
                self.runtime
                    .style(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgba(self.palette.muted.as_rgba_hex()))
                            .child("Loading…"),
                        "loading",
                    )
                    .into_any_element()
            } else {
                let count = self.loading_row_count();
                self.runtime
                    .style(
                        div().size_full().children(
                            self.spec.loading_rows.iter().take(count).enumerate().map(
                                |(index, row)| {
                                    self.runtime.render(row, &format!("loading_row:{index}"))
                                },
                            ),
                        ),
                        "loading",
                    )
                    .into_any_element()
            };
            return self
                .runtime
                .style(div().size_full().child(content), "body")
                .into_any_element();
        }
        if self.spec.rows.is_empty() {
            let content = self.spec.empty_slot.as_deref().map_or_else(
                || {
                    self.runtime
                        .style(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(rgba(self.palette.muted.as_rgba_hex()))
                                .child(self.spec.empty_text.clone()),
                            "empty",
                        )
                        .into_any_element()
                },
                |slot| self.runtime.render(slot, "empty_slot"),
            );
            return self
                .runtime
                .style(div().size_full().child(content), "body")
                .into_any_element();
        }
        let rows = self.spec.rows.len();
        let snapshot = self.clone_for_rows();
        let weak = cx.entity().downgrade();
        let list = uniform_list(
            SharedString::from(format!("table-{}-rows", self.spec.key)),
            rows,
            move |range, _, _| {
                range
                    .map(|row| snapshot.render_row(row, &widths, &weak))
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(self.vertical.clone())
        .size_full()
        .into_any_element();
        self.runtime
            .style(div().size_full().child(list), "body")
            .into_any_element()
    }

    fn clone_for_rows(&self) -> TableRowRenderer {
        TableRowRenderer {
            spec: self.spec.clone(),
            state: self.state.clone(),
            palette: self.palette,
            runtime: self.runtime.clone(),
        }
    }

    fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> (bool, TableKeyEmission) {
        let previous = self.state.focused().map(ToOwned::to_owned);
        match event.keystroke.key.as_str() {
            "up" => self.state.focus_previous(),
            "down" => self.state.focus_next(),
            "home" => self.state.focus_first(),
            "end" => self.state.focus_last(),
            "pageup" => {
                for _ in 0..self.page_rows() {
                    self.state.focus_previous();
                }
            }
            "pagedown" => {
                for _ in 0..self.page_rows() {
                    self.state.focus_next();
                }
            }
            "enter" => {
                return (
                    true,
                    TableKeyEmission {
                        callbacks: self.callbacks.clone(),
                        row_click: self.state.focused().map(ToOwned::to_owned),
                        selection: None,
                    },
                );
            }
            "space" => {
                let selection = self
                    .state
                    .focused()
                    .and_then(|key| self.state.select_row(key).ok())
                    .map(|values| ordered_selection(&self.spec, &values));
                return (
                    true,
                    TableKeyEmission {
                        callbacks: self.callbacks.clone(),
                        row_click: None,
                        selection,
                    },
                );
            }
            _ => return (false, TableKeyEmission::default()),
        }
        if self.state.focused() != previous.as_deref() {
            if let Some(index) = self
                .state
                .focused()
                .and_then(|key| self.spec.rows.iter().position(|row| row.key == key))
            {
                self.vertical.scroll_to_item(index, ScrollStrategy::Center);
            }
            cx.notify();
        }
        (true, TableKeyEmission::default())
    }

    fn page_rows(&self) -> usize {
        let body_height = self.body_viewport_height();
        (body_height / self.spec.row_height)
            .floor()
            .to_string()
            .parse()
            .unwrap_or(1)
    }

    fn loading_row_count(&self) -> usize {
        loading_row_count(
            self.body_viewport_height(),
            self.spec.row_height,
            self.spec.loading_rows.len(),
        )
    }

    fn body_viewport_height(&self) -> f64 {
        let scrollbar_height = if self.has_horizontal_overflow() {
            self.spec.horizontal_scrollbar_height
        } else {
            0.0
        };
        (self.viewport_height - self.spec.row_height - scrollbar_height).max(self.spec.row_height)
    }

    fn has_horizontal_overflow(&self) -> bool {
        let selection = if self.spec.selection_mode == TableSelectionMode::None {
            0.0
        } else {
            self.spec.selection_width
        };
        self.spec
            .resolve_widths((self.viewport_width - selection).max(0.0))
            .is_ok_and(|layout| {
                layout.content_width + selection > self.viewport_width + f64::EPSILON
            })
    }

    fn render_horizontal_scrollbar(
        &self,
        content_width: f64,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let geometry = horizontal_scrollbar_geometry(
            self.viewport_width,
            content_width,
            f64::from(self.horizontal.offset().x),
            self.spec.horizontal_scrollbar_thumb_min_width,
            self.spec.horizontal_scrollbar_inset,
        )?;
        let thumb_height =
            self.spec.horizontal_scrollbar_height - self.spec.horizontal_scrollbar_inset * 2.0;
        let entity = cx.entity();
        let entity_down = entity.clone();
        let entity_up = entity.clone();
        let entity_move = entity.clone();
        let inset = self.spec.horizontal_scrollbar_inset;
        let track_selector = format!("table-{}-horizontal-scrollbar", self.spec.key);
        let thumb_selector = format!("table-{}-horizontal-scrollbar-thumb", self.spec.key);
        let thumb = self
            .runtime
            .style(
                div()
                    .absolute()
                    .left(px(to_f32(geometry.thumb_left)))
                    .top(px(to_f32(inset)))
                    .w(px(to_f32(geometry.thumb_width)))
                    .h(px(to_f32(thumb_height)))
                    .bg(rgba(self.palette.border.as_rgba_hex())),
                "horizontal_scrollbar_thumb",
            )
            .id(SharedString::from(thumb_selector.clone()))
            .debug_selector(move || thumb_selector)
            .hover(|thumb| thumb.bg(rgba(self.palette.muted.as_rgba_hex())))
            .child(
                canvas(
                    |_, _, _| (),
                    move |thumb_bounds, (), window, _| {
                        register_horizontal_scrollbar_mouse_handlers(
                            thumb_bounds,
                            entity_down.clone(),
                            entity_up.clone(),
                            entity_move.clone(),
                            geometry,
                            inset,
                            window,
                        );
                    },
                )
                .size_full(),
            );
        Some(
            self.runtime
                .style(
                    div()
                        .relative()
                        .w_full()
                        .h(px(to_f32(self.spec.horizontal_scrollbar_height)))
                        .bg(rgba(self.palette.raised.as_rgba_hex()))
                        .child(thumb),
                    "horizontal_scrollbar",
                )
                .id(SharedString::from(track_selector.clone()))
                .debug_selector(move || track_selector)
                .into_any_element(),
        )
    }
}

#[derive(Clone)]
struct TableRowRenderer {
    spec: TableNodeSpec,
    state: TableState,
    palette: TablePalette,
    runtime: DropdownSlotRuntime,
}

impl TableRowRenderer {
    fn render_row(
        &self,
        row_index: usize,
        widths: &[f64],
        weak: &gpui::WeakEntity<TableView>,
    ) -> AnyElement {
        let row = &self.spec.rows[row_index];
        let row_key = row.key.clone();
        let selected = self.spec.selected_keys.contains(&row.key);
        let focused = self.state.focused() == Some(row.key.as_str());
        let mut cells = self
            .render_selection_cell(row, selected, weak)
            .into_iter()
            .collect::<Vec<_>>();
        cells.extend(self.spec.columns.iter().zip(widths).enumerate().map(
            |(column_index, (column, width))| {
                let content = column
                    .custom_cells
                    .as_ref()
                    .and_then(|cells| cells.get(row_index))
                    .map_or_else(
                        || {
                            self.spec
                                .display_cell(row_index, column_index)
                                .unwrap_or_else(|error| error.to_string())
                                .into_any_element()
                        },
                        |node| {
                            self.runtime
                                .render(node, &format!("cell:{row_key}:{}", column.key))
                        },
                    );
                self.runtime
                    .style(
                        align_cell(
                            div()
                                .w(px(to_f32(*width)))
                                .h(px(to_f32(self.spec.row_height)))
                                .flex()
                                .items_center()
                                .overflow_hidden()
                                .child(content),
                            column.align,
                            self.runtime.direction,
                        ),
                        "cell",
                    )
                    .into_any_element()
            },
        ));
        let weak_row = weak.clone();
        let selection_mode = self.spec.selection_mode;
        self.runtime
            .style(
                logical_table_row(
                    div()
                        .flex()
                        .when(self.spec.striped && row_index % 2 == 1, |row| {
                            row.bg(rgba(self.palette.raised.as_rgba_hex()))
                        })
                        .when(focused, |row| {
                            row.bg(rgba(self.palette.hover.as_rgba_hex()))
                        })
                        .when(selected, |row| {
                            row.text_color(rgba(self.palette.accent.as_rgba_hex()))
                        })
                        .children(cells),
                    self.runtime.direction,
                ),
                "row",
            )
            .id(SharedString::from(format!("table-row-{row_key}")))
            .on_click(move |_, window, app| {
                let _ = weak_row.update(app, |view, cx| {
                    if view.state.focus_key(&row_key).unwrap_or(false) {
                        cx.notify();
                    }
                });
                if selection_mode == TableSelectionMode::Single {
                    emit_selection(&weak_row, &row_key, window, app);
                }
                if let Ok(Some(handler)) =
                    weak_row.read_with(app, |view, _| view.callbacks.row_click.clone())
                {
                    handler(row_key.clone(), window, app);
                }
            })
            .into_any_element()
    }

    fn render_selection_cell(
        &self,
        row: &crate::TableRowSpec,
        selected: bool,
        weak: &gpui::WeakEntity<TableView>,
    ) -> Option<AnyElement> {
        (self.spec.selection_mode != TableSelectionMode::None).then(|| {
            let weak_selection = weak.clone();
            let selection_key = row.key.clone();
            self.runtime
                .style(
                    div()
                        .w(px(to_f32(self.spec.selection_width)))
                        .h(px(to_f32(self.spec.row_height)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(selection_indicator(
                            &self.runtime,
                            selected,
                            self.palette,
                            &self.spec.check_asset,
                            self.spec.selection_size,
                        )),
                    "selection_cell",
                )
                .id(SharedString::from(format!("table-select-{selection_key}")))
                .on_click(move |_, window, app| {
                    emit_selection(&weak_selection, &selection_key, window, app);
                    app.stop_propagation();
                })
                .into_any_element()
        })
    }
}

impl Render for TableView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = if self.spec.selection_mode == TableSelectionMode::None {
            0.0
        } else {
            self.spec.selection_width
        };
        let layout = self
            .spec
            .resolve_widths((self.viewport_width - selection).max(0.0))
            .expect("validated Table resolves measured viewport");
        let content_width = layout.content_width + selection;
        if !self.horizontal_positioned {
            let overflow = (content_width - self.viewport_width).max(0.0);
            let offset = if self.runtime.direction == TextDirection::RightToLeft {
                -to_f32(overflow)
            } else {
                0.0
            };
            self.horizontal.set_offset(point(px(offset), px(0.0)));
            self.horizontal_positioned = true;
        }
        let header = self.render_header(&layout.widths, cx);
        let body = self.render_body(layout.widths, cx);
        let content_selector = format!("table-{}-horizontal-content", self.spec.key);
        let content = div()
            .w(px(to_f32(content_width)))
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .debug_selector(move || content_selector)
            .child(header)
            .child(div().flex_1().min_h_0().overflow_hidden().child(body));
        let scrollbar = self.render_horizontal_scrollbar(content_width, cx);
        let weak = cx.entity().downgrade();
        let mut horizontal_scroll = div()
            .id("table-horizontal-scroll")
            .w_full()
            .flex_1()
            .min_h_0()
            .overflow_x_scroll()
            .track_scroll(&self.horizontal)
            .child(content);
        horizontal_scroll.style().restrict_scroll_to_axis = Some(true);
        let table = div()
            .id(SharedString::from(format!("table-root-{}", self.spec.key)))
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .border_1()
            .border_color(rgba(self.palette.border.as_rgba_hex()))
            .bg(rgba(self.palette.surface.as_rgba_hex()))
            .text_color(rgba(self.palette.text.as_rgba_hex()))
            .tab_index(0)
            .tab_stop(true)
            .on_key_down(move |event, window, app| {
                if let Ok((handled, emission)) =
                    weak.update(app, |view, cx| view.handle_key(event, cx))
                {
                    emission.emit_with_window(window, app);
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .child(horizontal_scroll)
            .when_some(scrollbar, ParentElement::child);
        apply_height(table, self.runtime.colors.resolve_length(self.spec.height))
    }
}

#[derive(Clone, Default)]
struct TableKeyEmission {
    callbacks: TableCallbacks,
    row_click: Option<String>,
    selection: Option<Vec<String>>,
}

impl TableKeyEmission {
    fn emit_with_window(self, window: &mut Window, app: &mut App) {
        if let (Some(handler), Some(key)) = (self.callbacks.row_click, self.row_click) {
            handler(key, window, app);
        }
        if let (Some(handler), Some(values)) = (self.callbacks.selection, self.selection) {
            handler(values, window, app);
        }
    }
}

fn emit_sort(weak: &gpui::WeakEntity<TableView>, key: &str, window: &mut Window, app: &mut App) {
    if let Ok(Some((handler, sort))) = weak.update(app, |view, _| {
        view.state
            .toggle_sort(key)
            .ok()
            .map(|sort| (view.callbacks.sort.clone(), sort))
    }) && let Some(handler) = handler
    {
        handler(sort, window, app);
    }
}

fn emit_select_all(weak: &gpui::WeakEntity<TableView>, window: &mut Window, app: &mut App) {
    if let Ok(Some((handler, values))) = weak.update(app, |view, _| {
        (!view.spec.rows.is_empty())
            .then(|| view.state.select_all_current().ok())
            .flatten()
            .filter(|values| values != &view.spec.selected_keys)
            .map(|values| {
                (
                    view.callbacks.selection.clone(),
                    ordered_selection(&view.spec, &values),
                )
            })
    }) && let Some(handler) = handler
    {
        handler(values, window, app);
    }
}

fn selection_indicator(
    runtime: &DropdownSlotRuntime,
    checked: bool,
    palette: TablePalette,
    check_asset: &AssetId,
    size: f64,
) -> gpui::Div {
    let indicator = div()
        .w(px(to_f32(size)))
        .h(px(to_f32(size)))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(to_f32(size * 0.25)))
        .border_1()
        .border_color(rgba(if checked {
            palette.accent.as_rgba_hex()
        } else {
            palette.border.as_rgba_hex()
        }))
        .when(checked, |indicator| {
            indicator
                .bg(rgba(palette.accent.as_rgba_hex()))
                .child(table_asset_element(
                    runtime,
                    check_asset,
                    runtime.part_color("selection_indicator", palette.on_accent),
                    size * 0.75,
                ))
        });
    runtime.style(indicator, "selection_indicator")
}

fn table_asset_element(
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

fn emit_selection(
    weak: &gpui::WeakEntity<TableView>,
    key: &str,
    window: &mut Window,
    app: &mut App,
) {
    if let Ok(Some((handler, values))) = weak.update(app, |view, cx| {
        if view.state.focus_key(key).unwrap_or(false) {
            cx.notify();
        }
        view.state
            .select_row(key)
            .ok()
            .filter(|values| values != &view.spec.selected_keys)
            .map(|values| {
                (
                    view.callbacks.selection.clone(),
                    ordered_selection(&view.spec, &values),
                )
            })
    }) && let Some(handler) = handler
    {
        handler(values, window, app);
    }
}

fn align_cell(cell: gpui::Div, align: TableAlign, direction: TextDirection) -> gpui::Div {
    match (align, direction) {
        (TableAlign::Start, TextDirection::LeftToRight)
        | (TableAlign::End, TextDirection::RightToLeft) => cell.justify_start(),
        (TableAlign::Center, _) => cell.justify_center(),
        (TableAlign::End, TextDirection::LeftToRight)
        | (TableAlign::Start, TextDirection::RightToLeft) => cell.justify_end(),
    }
}

fn logical_table_row(row: gpui::Div, direction: TextDirection) -> gpui::Div {
    match direction {
        TextDirection::LeftToRight => row.flex_row(),
        TextDirection::RightToLeft => row.flex_row_reverse(),
    }
}

fn ordered_selection(
    spec: &TableNodeSpec,
    selected: &std::collections::BTreeSet<String>,
) -> Vec<String> {
    spec.rows
        .iter()
        .filter(|row| selected.contains(&row.key))
        .map(|row| row.key.clone())
        .collect()
}

fn apply_height(
    element: gpui::Stateful<gpui::Div>,
    height: Option<Length>,
) -> gpui::Stateful<gpui::Div> {
    match height {
        Some(Length::Pixels(value)) => element.h(px(to_f32(value))),
        Some(Length::Rems(value)) => element.h(rems(to_f32(value))),
        Some(Length::Relative(value)) => element.h(relative(to_f32(value))),
        Some(Length::ThemeSpacing(_) | Length::ThemeRadius(_)) | None => element.h_full(),
    }
}

fn to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or(f32::MAX)
}

fn loading_row_count(body_height: f64, row_height: f64, available: usize) -> usize {
    (body_height / row_height)
        .ceil()
        .to_string()
        .parse::<usize>()
        .unwrap_or(1)
        .clamp(1, available.max(1))
        .min(available)
}

fn register_horizontal_scrollbar_mouse_handlers(
    thumb_bounds: Bounds<Pixels>,
    entity_down: Entity<TableView>,
    entity_up: Entity<TableView>,
    entity_move: Entity<TableView>,
    geometry: HorizontalScrollbarGeometry,
    inset: f64,
    window: &mut Window,
) {
    window.on_mouse_event(move |event: &MouseDownEvent, phase, _, app| {
        if phase == DispatchPhase::Bubble
            && event.button == MouseButton::Left
            && thumb_bounds.contains(&event.position)
        {
            entity_down.update(app, |view, cx| {
                view.horizontal_drag_grab = Some(event.position.x - thumb_bounds.origin.x);
                cx.notify();
            });
            app.stop_propagation();
        }
    });
    window.on_mouse_event(move |event: &MouseUpEvent, phase, _, app| {
        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
            entity_up.update(app, |view, cx| {
                if view.horizontal_drag_grab.take().is_some() {
                    cx.notify();
                }
            });
        }
    });
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, app| {
        if phase != DispatchPhase::Capture || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        entity_move.update(app, |view, cx| {
            let Some(grab) = view.horizontal_drag_grab else {
                return;
            };
            let track_bounds = view.horizontal.bounds();
            let position = f64::from(event.position.x - track_bounds.origin.x - grab) - inset;
            let fraction = if geometry.travel > f64::EPSILON {
                position.clamp(0.0, geometry.travel) / geometry.travel
            } else {
                0.0
            };
            let current = view.horizontal.offset();
            view.horizontal
                .set_offset(point(px(to_f32(-geometry.overflow * fraction)), current.y));
            cx.notify();
        });
        app.stop_propagation();
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HorizontalScrollbarGeometry {
    overflow: f64,
    thumb_width: f64,
    thumb_left: f64,
    travel: f64,
}

fn horizontal_scrollbar_geometry(
    viewport_width: f64,
    content_width: f64,
    offset_x: f64,
    thumb_min_width: f64,
    inset: f64,
) -> Option<HorizontalScrollbarGeometry> {
    let overflow = content_width - viewport_width;
    if !viewport_width.is_finite()
        || !content_width.is_finite()
        || !offset_x.is_finite()
        || overflow <= f64::EPSILON
    {
        return None;
    }
    let track_width = (viewport_width - inset * 2.0).max(0.0);
    let thumb_width = (track_width * viewport_width / content_width)
        .max(thumb_min_width)
        .min(track_width);
    let travel = (track_width - thumb_width).max(0.0);
    let fraction = (-offset_x / overflow).clamp(0.0, 1.0);
    Some(HorizontalScrollbarGeometry {
        overflow,
        thumb_width,
        thumb_left: inset + travel * fraction,
        travel,
    })
}

#[cfg(test)]
mod tests {
    use super::{horizontal_scrollbar_geometry, loading_row_count};

    #[test]
    fn loading_rows_cover_measured_body_without_exceeding_prebuilt_nodes() {
        assert_eq!(loading_row_count(426.0, 32.0, 100), 14);
        assert_eq!(loading_row_count(2000.0, 24.0, 32), 32);
        assert_eq!(loading_row_count(0.0, 32.0, 0), 0);
    }

    #[test]
    fn horizontal_scrollbar_geometry_tracks_both_ltr_and_rtl_offsets() {
        assert!(horizontal_scrollbar_geometry(400.0, 400.0, 0.0, 64.0, 4.0).is_none());
        let start = horizontal_scrollbar_geometry(400.0, 800.0, 0.0, 64.0, 4.0).unwrap();
        assert!((start.thumb_width - 196.0).abs() < f64::EPSILON);
        assert!((start.thumb_left - 4.0).abs() < f64::EPSILON);
        let middle = horizontal_scrollbar_geometry(400.0, 800.0, -200.0, 64.0, 4.0).unwrap();
        assert!((middle.thumb_left - 102.0).abs() < f64::EPSILON);
        let rtl_end = horizontal_scrollbar_geometry(400.0, 800.0, -400.0, 64.0, 4.0).unwrap();
        assert!((rtl_end.thumb_left - 200.0).abs() < f64::EPSILON);
    }
}
