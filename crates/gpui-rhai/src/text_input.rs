//! Native text input behavior adapted from GPUI's Apache-2.0 `examples/input.rs`.

use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    IntoElement, KeyBinding, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, Render, ShapedLine, SharedString, Style as GpuiStyle,
    TextRun, UTF16Selection, UnderlineStyle, Window, actions, div, fill, point, prelude::*, px,
    relative, rgba, size,
};

pub use crate::text_edit::TextBuffer;

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveValue, Rgba8, UiValue, ValueSchema,
};

actions!(
    gpui_rhai_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Submit,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
    ]
);

pub fn init_text_input(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("GPUIRhaiTextInput")),
        KeyBinding::new("delete", Delete, Some("GPUIRhaiTextInput")),
        KeyBinding::new("left", Left, Some("GPUIRhaiTextInput")),
        KeyBinding::new("right", Right, Some("GPUIRhaiTextInput")),
        KeyBinding::new("shift-left", SelectLeft, Some("GPUIRhaiTextInput")),
        KeyBinding::new("shift-right", SelectRight, Some("GPUIRhaiTextInput")),
        KeyBinding::new("cmd-a", SelectAll, Some("GPUIRhaiTextInput")),
        KeyBinding::new("cmd-v", Paste, Some("GPUIRhaiTextInput")),
        KeyBinding::new("cmd-c", Copy, Some("GPUIRhaiTextInput")),
        KeyBinding::new("cmd-x", Cut, Some("GPUIRhaiTextInput")),
        KeyBinding::new("cmd-z", Undo, Some("GPUIRhaiTextInput")),
        KeyBinding::new("shift-cmd-z", Redo, Some("GPUIRhaiTextInput")),
        KeyBinding::new("home", Home, Some("GPUIRhaiTextInput")),
        KeyBinding::new("end", End, Some("GPUIRhaiTextInput")),
        KeyBinding::new("enter", Submit, Some("GPUIRhaiTextInput")),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some("GPUIRhaiTextInput"),
        ),
    ]);
}

pub(crate) type TextValueHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub(crate) type TextSignalHandler = Rc<dyn Fn(&mut Window, &mut App)>;
pub(crate) type TextTabHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(Clone, Default)]
pub(crate) struct TextInputCallbacks {
    pub change: Option<TextValueHandler>,
    pub submit: Option<TextValueHandler>,
    pub focus: Option<TextSignalHandler>,
    pub blur: Option<TextSignalHandler>,
    pub tab: Option<TextTabHandler>,
}

pub(crate) struct TextInputEntity {
    focus: FocusHandle,
    buffer: TextBuffer,
    placeholder: SharedString,
    disabled: bool,
    read_only: bool,
    selection_color: Rgba8,
    caret_color: Rgba8,
    callbacks: TextInputCallbacks,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    selecting: bool,
}

#[derive(Clone)]
struct TextInputConfig {
    value: String,
    placeholder: String,
    disabled: bool,
    read_only: bool,
    selection_color: Rgba8,
    caret_color: Rgba8,
}

impl TextInputEntity {
    fn new(config: TextInputConfig, callbacks: TextInputCallbacks, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(!config.disabled),
            buffer: TextBuffer::new(&config.value),
            placeholder: config.placeholder.into(),
            disabled: config.disabled,
            read_only: config.read_only,
            selection_color: config.selection_color,
            caret_color: config.caret_color,
            callbacks,
            last_layout: None,
            last_bounds: None,
            scroll_x: px(0.0),
            selecting: false,
        }
    }

    fn update_props(
        &mut self,
        config: &TextInputConfig,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) {
        self.buffer.set_controlled(&config.value);
        self.focus = self.focus.clone().tab_stop(!config.disabled);
        self.placeholder = config.placeholder.clone().into();
        self.disabled = config.disabled;
        self.read_only = config.read_only;
        self.selection_color = config.selection_color;
        self.caret_color = config.caret_color;
        self.callbacks = callbacks;
        cx.notify();
    }

    fn emit_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(change) = self.callbacks.change.clone() {
            let value = self.buffer.content.clone();
            window.defer(cx, move |window, cx| change(value, window, cx));
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.previous_boundary(self.buffer.cursor_offset())
        } else {
            self.buffer.selected.start
        };
        self.buffer.move_to(offset);
        cx.notify();
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.next_boundary(self.buffer.selected.end)
        } else {
            self.buffer.selected.end
        };
        self.buffer.move_to(offset);
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.buffer.previous_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.buffer.next_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        cx.notify();
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_to(0);
        self.buffer.select_to(self.buffer.content.len());
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_to(0);
        cx.notify();
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_to(self.buffer.content.len());
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if self.buffer.selected.is_empty() {
            let previous = self.buffer.previous_boundary(self.buffer.cursor_offset());
            self.buffer.select_to(previous);
        }
        self.buffer.replace(None, "");
        self.emit_change(window, cx);
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if self.buffer.selected.is_empty() {
            let next = self.buffer.next_boundary(self.buffer.cursor_offset());
            self.buffer.select_to(next);
        }
        self.buffer.replace(None, "");
        self.emit_change(window, cx);
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.buffer.selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.buffer.content[self.buffer.selected.clone()].to_owned(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only || self.buffer.selected.is_empty() {
            return;
        }
        self.copy(&Copy, window, cx);
        self.buffer.replace(None, "");
        self.emit_change(window, cx);
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.buffer.replace(None, &text.replace('\n', " "));
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled && !self.read_only && self.buffer.undo() {
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled && !self.read_only && self.buffer.redo() {
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if !self.disabled
            && let Some(submit) = self.callbacks.submit.clone()
        {
            let value = self.buffer.content.clone();
            window.defer(cx, move |window, cx| submit(value, window, cx));
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        if !self.disabled {
            window.show_character_palette();
        }
    }

    fn index_for_mouse(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(line)) = (&self.last_bounds, &self.last_layout) else {
            return 0;
        };
        if position.y < bounds.top() {
            0
        } else if position.y > bounds.bottom() {
            self.buffer.content.len()
        } else {
            line.closest_index_for_x(position.x - bounds.left() + self.scroll_x)
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.focus.focus(window);
        self.selecting = true;
        let index = self.index_for_mouse(event.position);
        if event.modifiers.shift {
            self.buffer.select_to(index);
        } else {
            self.buffer.move_to(index);
        }
        cx.notify();
    }

    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            let index = self.index_for_mouse(event.position);
            self.buffer.select_to(index);
            cx.notify();
        }
    }
}

impl EntityInputHandler for TextInputEntity {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.buffer.range_from_utf16(&range_utf16);
        actual_range.replace(self.buffer.range_to_utf16(&range));
        self.buffer.content.get(range).map(ToOwned::to_owned)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.buffer.range_to_utf16(&self.buffer.selected),
            reversed: self.buffer.selection_reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.buffer
            .marked
            .as_ref()
            .map(|range| self.buffer.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.buffer.unmark();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }
        self.buffer.replace(range_utf16.as_ref(), text);
        self.emit_change(window, cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }
        self.buffer
            .replace_and_mark(range_utf16.as_ref(), text, selected_utf16);
        self.emit_change(window, cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let line = self.last_layout.as_ref()?;
        let range = self.buffer.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + line.x_for_index(range.start) - self.scroll_x,
                bounds.top(),
            ),
            point(
                bounds.left() + line.x_for_index(range.end) - self.scroll_x,
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        if !bounds.contains(&point) {
            return None;
        }
        let line = self.last_layout.as_ref()?;
        let index = line.index_for_x(point.x - bounds.left() + self.scroll_x)?;
        Some(self.buffer.offset_to_utf16(index))
    }
}

struct TextElement {
    input: Entity<TextInputEntity>,
}

struct TextPrepaint {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    origin: Point<Pixels>,
    scroll_x: Pixels,
}

fn horizontal_scroll_for_cursor(
    line_width: Pixels,
    cursor_x: Pixels,
    viewport_width: Pixels,
    current: Pixels,
) -> Pixels {
    let viewport_width = viewport_width.max(px(0.0));
    let max_scroll = (line_width - viewport_width).max(px(0.0));
    let mut scroll_x = current.min(max_scroll);
    if cursor_x < scroll_x {
        scroll_x = cursor_x;
    } else if cursor_x > scroll_x + viewport_width - px(2.0) {
        scroll_x = (cursor_x - viewport_width + px(2.0)).min(max_scroll);
    }
    scroll_x.max(px(0.0))
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = GpuiStyle::default();
        style.size.width = relative(1.0).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> TextPrepaint {
        let input = self.input.read(cx);
        let content: SharedString = input.buffer.content.clone().into();
        let selected = input.buffer.selected.clone();
        let cursor = input.buffer.cursor_offset();
        let text_style = window.text_style();
        let (display, color) = if content.is_empty() {
            (input.placeholder.clone(), text_style.color.opacity(0.55))
        } else {
            (content, text_style.color)
        };
        let run = TextRun {
            len: display.len(),
            font: text_style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = input.buffer.marked.as_ref() {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.end - marked.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display, font_size, &runs, None);
        let cursor_x = line.x_for_index(cursor);
        let scroll_x =
            horizontal_scroll_for_cursor(line.width, cursor_x, bounds.size.width, input.scroll_x);
        let origin = point(bounds.left() - scroll_x, bounds.top());
        let (selection, cursor) = if selected.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(origin.x + cursor_x, bounds.top()),
                        size(px(1.5), bounds.size.height),
                    ),
                    rgba(input.caret_color.as_rgba_hex()),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(origin.x + line.x_for_index(selected.start), bounds.top()),
                        point(origin.x + line.x_for_index(selected.end), bounds.bottom()),
                    ),
                    rgba(input.selection_color.as_rgba_hex()),
                )),
                None,
            )
        };
        TextPrepaint {
            line: Some(line),
            cursor,
            selection,
            origin,
            scroll_x,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        state: &mut TextPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let Some(line) = state.line.take() else {
            return;
        };
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(selection) = state.selection.take() {
                window.paint_quad(selection);
            }
            let _ = line.paint(state.origin, window.line_height(), window, cx);
            if focus.is_focused(window)
                && let Some(cursor) = state.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.scroll_x = state.scroll_x;
        });
    }
}

impl Render for TextInputEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.callbacks.tab.clone();
        let submit_enabled = self.callbacks.submit.is_some();
        div()
            .flex()
            .size_full()
            .items_center()
            .key_context("GPUIRhaiTextInput")
            .track_focus(&self.focus)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    if let Some(tab) = &tab {
                        let tab = tab.clone();
                        let shift = event.keystroke.modifiers.shift;
                        window.defer(cx, move |window, cx| tab(shift, window, cx));
                    } else if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                }
            })
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .when(submit_enabled, |input| {
                input.on_action(cx.listener(Self::submit))
            })
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .line_height(px(16.0))
            .text_size(px(12.0))
            .opacity(if self.disabled { 0.55 } else { 1.0 })
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInputEntity {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

#[derive(Default)]
pub struct TextInputPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<TextInputEntity>>,
}

impl PrimitiveHandler for TextInputPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &crate::PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<gpui::AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "TextInputPrimitive requires a stable key".to_owned())?;
        let value = string_prop(&instance.node.props, "value").unwrap_or_default();
        let placeholder = string_prop(&instance.node.props, "placeholder").unwrap_or_default();
        let disabled = bool_prop(&instance.node.props, "disabled").unwrap_or(false);
        let read_only = bool_prop(&instance.node.props, "read_only").unwrap_or(false);
        let selection_color = theme
            .color("selection")
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0x292e_42ff));
        let caret_color = theme
            .color("accent")
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0x7aa2_f7ff));
        let config = TextInputConfig {
            value,
            placeholder,
            disabled,
            read_only,
            selection_color,
            caret_color,
        };
        let callbacks = primitive_callbacks(events);
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| TextInputEntity::new(config.clone(), callbacks.clone(), cx));
            entity.update(cx, |input, cx| {
                let focus = input.focus.clone();
                cx.on_focus(&focus, window, |input, window, cx| {
                    if let Some(focus) = input.callbacks.focus.clone() {
                        window.defer(cx, move |window, cx| focus(window, cx));
                    }
                })
                .detach();
                cx.on_blur(&focus, window, |input, window, cx| {
                    if let Some(blur) = input.callbacks.blur.clone() {
                        window.defer(cx, move |window, cx| blur(window, cx));
                    }
                })
                .detach();
            });
            self.instances.insert(id, entity.clone());
            entity
        };
        entity.update(cx, |input, cx| {
            input.update_props(&config, callbacks, cx);
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn primitive_callbacks(events: &PrimitiveEventEmitter) -> TextInputCallbacks {
    let change_events = events.clone();
    let submit_events = events.clone();
    let focus_events = events.clone();
    let blur_events = events.clone();
    TextInputCallbacks {
        change: Some(Rc::new(move |value, window, cx| {
            let _ = change_events.emit("change", UiValue::String(value), window, cx);
        })),
        submit: Some(Rc::new(move |value, window, cx| {
            let _ = submit_events.emit("submit", UiValue::String(value), window, cx);
        })),
        focus: Some(Rc::new(move |window, cx| {
            let _ = focus_events.emit("focus", UiValue::Null, window, cx);
        })),
        blur: Some(Rc::new(move |window, cx| {
            let _ = blur_events.emit("blur", UiValue::Null, window, cx);
        })),
        tab: None,
    }
}

fn string_prop(props: &PrimitiveProps, name: &str) -> Option<String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::String(value))) => Some(value.clone()),
        _ => None,
    }
}

fn bool_prop(props: &PrimitiveProps, name: &str) -> Option<bool> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Bool(value))) => Some(*value),
        _ => None,
    }
}

/// Build the compile-time `TextInput` primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
pub fn text_input_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_callback = || ObjectField::optional(ValueSchema::optional(ValueSchema::Callback));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.text_input").expect("static primitive ID"),
        export: "TextInputPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "value".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "placeholder".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            (
                "read_only".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            ("on_change".to_owned(), optional_callback()),
            ("on_submit".to_owned(), optional_callback()),
            ("on_focus".to_owned(), optional_callback()),
            ("on_blur".to_owned(), optional_callback()),
        ]),
        events: BTreeMap::from([
            (
                "change".to_owned(),
                EventSchema {
                    payload: ValueSchema::string(),
                },
            ),
            (
                "submit".to_owned(),
                EventSchema {
                    payload: ValueSchema::string(),
                },
            ),
            (
                "focus".to_owned(),
                EventSchema {
                    payload: ValueSchema::Null,
                },
            ),
            (
                "blur".to_owned(),
                EventSchema {
                    payload: ValueSchema::Null,
                },
            ),
        ]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grapheme_navigation_does_not_split_emoji_or_combining_text() {
        let buffer = TextBuffer::new("a👩‍💻e\u{301}");
        let end = buffer.content().len();
        let before_combining = buffer.previous_boundary(end);
        let before_emoji = buffer.previous_boundary(before_combining);
        assert_eq!(&buffer.content()[before_combining..], "e\u{301}");
        assert_eq!(&buffer.content()[before_emoji..before_combining], "👩‍💻");
    }

    #[test]
    fn utf16_round_trip_supports_cjk_and_surrogate_pairs() {
        let buffer = TextBuffer::new("中😀文");
        for offset in [0, 3, 7, buffer.content().len()] {
            let utf16 = buffer.offset_to_utf16(offset);
            assert_eq!(buffer.offset_from_utf16(utf16), offset);
        }
    }

    #[test]
    fn horizontal_scroll_keeps_the_active_cursor_inside_the_viewport() {
        assert_eq!(
            horizontal_scroll_for_cursor(px(200.0), px(150.0), px(100.0), px(0.0)),
            px(52.0)
        );
        assert_eq!(
            horizontal_scroll_for_cursor(px(200.0), px(10.0), px(100.0), px(50.0)),
            px(10.0)
        );
        assert_eq!(
            horizontal_scroll_for_cursor(px(80.0), px(80.0), px(100.0), px(30.0)),
            px(0.0)
        );
    }

    #[test]
    fn marked_text_replacement_tracks_ime_range() {
        let mut buffer = TextBuffer::new("");
        buffer.replace_and_mark(None, "に", Some(1..1));
        assert_eq!(buffer.content(), "に");
        assert_eq!(buffer.marked(), Some(&(0..3)));
        buffer.replace(None, "日本");
        assert_eq!(buffer.content(), "日本");
        assert!(buffer.marked().is_none());
    }
}
