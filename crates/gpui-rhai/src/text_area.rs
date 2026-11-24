#[cfg(not(feature = "vendor-gpui"))]
use crate::presentation_backend::PublicWindowExt as _;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, IntoElement, KeyBinding,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, Render, ScrollHandle, SharedString, TextAlign, TextRun, UTF16Selection,
    UnderlineStyle, Window, WrappedLine, actions, div, fill, point, prelude::*, px, rgba, size,
};
use rhai::{Engine, FuncRegistration, INT, ImmutableString};

use crate::text_edit::TextBuffer;
use crate::text_input::{TextInputCallbacks, request_editor_layout, text_style_defaults};
use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveValue, Rgba8, Style, UiValue, ValueSchema,
};

const MAX_TEXTAREA_ROWS: i64 = 1_000;
const MAX_TEXTAREA_GRAPHEMES: i64 = 1_000_000;

actions!(
    gpui_rhai_textarea,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        Newline,
        Submit,
        AltSubmit,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
    ]
);

pub fn init_text_area(cx: &mut App) {
    use crate::native_shortcut::platform_binding;
    const EDITOR: &str = "GPUIRhaiTextarea";
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("GPUIRhaiTextarea")),
        KeyBinding::new("delete", Delete, Some("GPUIRhaiTextarea")),
        KeyBinding::new("left", Left, Some("GPUIRhaiTextarea")),
        KeyBinding::new("right", Right, Some("GPUIRhaiTextarea")),
        KeyBinding::new("up", Up, Some("GPUIRhaiTextarea")),
        KeyBinding::new("down", Down, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-left", SelectLeft, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-right", SelectRight, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-up", SelectUp, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-down", SelectDown, Some("GPUIRhaiTextarea")),
        platform_binding(cx, "secondary-a", SelectAll, EDITOR),
        platform_binding(cx, "secondary-v", Paste, EDITOR),
        platform_binding(cx, "secondary-c", Copy, EDITOR),
        platform_binding(cx, "secondary-x", Cut, EDITOR),
        platform_binding(cx, "secondary-z", Undo, EDITOR),
        platform_binding(cx, "shift-secondary-z", Redo, EDITOR),
        KeyBinding::new("home", Home, Some("GPUIRhaiTextarea")),
        KeyBinding::new("end", End, Some("GPUIRhaiTextarea")),
        KeyBinding::new("enter", Newline, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-enter", Newline, Some("GPUIRhaiTextarea")),
        KeyBinding::new("enter", Submit, Some("GPUIRhaiTextarea && submit_enter")),
        KeyBinding::new(
            "alt-enter",
            AltSubmit,
            Some("GPUIRhaiTextarea && alt_submit"),
        ),
        platform_binding(
            cx,
            "secondary-enter",
            AltSubmit,
            "GPUIRhaiTextarea && alt_mod_submit",
        ),
        platform_binding(
            cx,
            "secondary-enter",
            Submit,
            "GPUIRhaiTextarea && submit_mod_enter",
        ),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some("GPUIRhaiTextarea"),
        ),
    ]);
}

pub(crate) fn register_text_area_api(engine: &mut Engine) {
    FuncRegistration::new("text_initial")
        .in_global_namespace()
        .register_into_engine(engine, |value: ImmutableString| -> String {
            static LETTER_OR_NUMBER: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            LETTER_OR_NUMBER
                .get_or_init(|| regex::Regex::new(r"[\p{L}\p{N}]").expect("fixed Unicode class"))
                .find(value.as_str())
                .map_or_else(String::new, |letter| letter.as_str().to_uppercase())
        });
    FuncRegistration::new("grapheme_count")
        .in_global_namespace()
        .register_into_engine(engine, |value: ImmutableString| -> INT {
            INT::try_from(
                unicode_segmentation::UnicodeSegmentation::graphemes(value.as_str(), true).count(),
            )
            .unwrap_or(INT::MAX)
        });
}

#[derive(Clone, Debug)]
struct TextAreaConfig {
    placeholder: SharedString,
    disabled: bool,
    read_only: bool,
    submit_key: Option<String>,
    alt_submit_key: Option<String>,
    min_rows: usize,
    max_rows: usize,
    rows: Option<usize>,
    min_viewport_height: Option<Pixels>,
    max_viewport_height: Option<Pixels>,
    max_length: Option<usize>,
    autofocus: bool,
    placeholder_color: Rgba8,
    selection_color: Rgba8,
    caret_color: Rgba8,
    scroll_color: Option<Rgba8>,
}

impl TextAreaConfig {
    fn viewport_rows(&self, measured: usize) -> usize {
        self.rows
            .unwrap_or_else(|| measured.clamp(self.min_rows, self.max_rows))
    }

    fn viewport_height(&self, line_height: Pixels, measured: usize) -> Pixels {
        let natural = line_height * self.viewport_rows(measured);
        let floored = natural.max(self.min_viewport_height.unwrap_or(px(0.0)));
        floored.min(self.max_viewport_height.unwrap_or(floored))
    }
}

pub(crate) struct TextAreaEntity {
    focus: FocusHandle,
    surface: Option<crate::renderer::NativeEditorSurface>,
    buffer: TextBuffer,
    config: TextAreaConfig,
    callbacks: TextInputCallbacks,
    layout: Option<TextAreaLayout>,
    measured_rows: usize,
    caret_reveal_pending: bool,
    last_bounds: Option<Bounds<Pixels>>,
    preferred_x: Option<Pixels>,
    selecting: bool,
    scroll: ScrollHandle,
}

impl TextAreaEntity {
    fn new(
        value: &str,
        config: TextAreaConfig,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) -> Result<Self, String> {
        validate_text_area_value(value, &config)?;
        let mut buffer = TextBuffer::new(value);
        buffer.set_max_length(config.max_length);
        Ok(Self {
            focus: cx.focus_handle().tab_stop(!config.disabled),
            surface: None,
            buffer,
            measured_rows: config.min_rows,
            caret_reveal_pending: false,
            config,
            callbacks,
            layout: None,
            last_bounds: None,
            preferred_x: None,
            selecting: false,
            scroll: ScrollHandle::new(),
        })
    }

    fn update_props(
        &mut self,
        value: &str,
        config: TextAreaConfig,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        validate_text_area_value(value, &config)?;
        if self.buffer.marked().is_none() {
            self.buffer.set_controlled(value);
        }
        self.buffer.set_max_length(config.max_length);
        self.focus = crate::renderer::configure_editor_focus(
            &self.focus,
            self.surface.as_ref(),
            config.disabled,
        );
        self.config = config;
        self.callbacks = callbacks;
        cx.notify();
        Ok(())
    }

    fn emit_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(change) = self.callbacks.change.clone() {
            let value = self.buffer.content.clone();
            window.defer(cx, move |window, cx| change(value, window, cx));
        }
    }

    fn reset_preferred_x(&mut self) {
        self.preferred_x = None;
    }

    fn ensure_cursor_visible(&mut self) {
        self.caret_reveal_pending = true;
    }

    fn reveal_caret_in_viewport(&mut self, window: &Window) -> bool {
        if !std::mem::take(&mut self.caret_reveal_pending) || !self.focus.is_focused(window) {
            return false;
        }
        let Some(layout) = &self.layout else {
            return false;
        };
        let Some(position) = layout.position_for_index(self.buffer.cursor_offset()) else {
            return false;
        };
        let viewport = self.scroll.bounds().size.height;
        if viewport <= px(0.0) {
            return false;
        }
        let current = -self.scroll.offset().y;
        let mut top = current;
        if position.y < top {
            top = position.y;
        } else if position.y + layout.line_height > top + viewport {
            top = position.y + layout.line_height - viewport;
        }
        top = top.clamp(px(0.0), self.scroll.max_offset().y);
        if top == current {
            return false;
        }
        self.scroll.set_offset(point(px(0.0), -top));
        true
    }

    /// Whether a script capture handler above the editor takes the key that
    /// triggered the binding being run; then the binding does nothing. Not
    /// while an input method composes text: its keys are the composition's.
    fn script_takes_key(&self, window: &mut Window, cx: &mut App) -> bool {
        self.buffer.marked.is_none()
            && self
                .callbacks
                .key_capture
                .as_ref()
                .is_some_and(|take| take(window, cx))
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.previous_boundary(self.buffer.cursor_offset())
        } else {
            self.buffer.selected.start
        };
        self.buffer.move_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.next_boundary(self.buffer.selected.end)
        } else {
            self.buffer.selected.end
        };
        self.buffer.move_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = self.buffer.previous_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = self.buffer.next_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.move_vertical(-1, false);
        cx.notify();
    }

    fn down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.move_vertical(1, false);
        cx.notify();
    }

    fn select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.move_vertical(-1, true);
        cx.notify();
    }

    fn select_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.move_vertical(1, true);
        cx.notify();
    }

    fn move_vertical(&mut self, step: i32, extend: bool) {
        let Some(layout) = &self.layout else {
            return;
        };
        let cursor = self.buffer.cursor_offset();
        let Some(position) = layout.position_for_index(cursor) else {
            return;
        };
        let x = self.preferred_x.unwrap_or(position.x);
        self.preferred_x = Some(x);
        let target_y = if step.is_negative() {
            position.y - layout.line_height
        } else {
            position.y + layout.line_height
        };
        let offset = layout.closest_index_for_position(point(x, target_y));
        if extend {
            self.buffer.select_to(offset);
        } else {
            self.buffer.move_to(offset);
        }
        self.ensure_cursor_visible();
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.buffer.move_to(0);
        self.buffer.select_to(self.buffer.content.len());
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.buffer
            .move_to(self.buffer.line_start(self.buffer.cursor_offset()));
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.buffer
            .move_to(self.buffer.line_end(self.buffer.cursor_offset()));
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.config.disabled || self.config.read_only {
            return;
        }
        if self.buffer.selected.is_empty() {
            let previous = self.buffer.previous_boundary(self.buffer.cursor_offset());
            self.buffer.select_to(previous);
        }
        self.buffer.replace(None, "");
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.config.disabled || self.config.read_only {
            return;
        }
        if self.buffer.selected.is_empty() {
            let next = self.buffer.next_boundary(self.buffer.cursor_offset());
            self.buffer.select_to(next);
        }
        self.buffer.replace(None, "");
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.config.disabled || self.config.read_only {
            return;
        }
        self.buffer.replace(None, "\n");
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        self.submit_with(self.callbacks.submit.clone(), window, cx);
    }

    fn alt_submit(&mut self, _: &AltSubmit, window: &mut Window, cx: &mut Context<Self>) {
        self.submit_with(self.callbacks.alt_submit.clone(), window, cx);
    }

    fn submit_with(
        &mut self,
        callback: Option<crate::text_input::TextValueHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.script_takes_key(window, cx) {
            return;
        }
        // Platform IME normally consumes confirmation before key dispatch. Keep
        // this guard as well for hosts which deliver a key while text is marked.
        if !self
            .buffer
            .permits_submit(self.config.disabled, self.config.read_only)
        {
            return;
        }
        if let Some(submit) = callback {
            let value = self.buffer.content.clone();
            window.defer(cx, move |window, cx| submit(value, window, cx));
        }
    }

    fn copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if !self.buffer.selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.buffer.content[self.buffer.selected.clone()].to_owned(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.config.disabled || self.config.read_only || self.buffer.selected.is_empty() {
            return;
        }
        self.copy(&Copy, window, cx);
        self.buffer.replace(None, "");
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.config.disabled || self.config.read_only {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.buffer.replace(None, &text);
            self.ensure_cursor_visible();
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if !self.config.disabled && !self.config.read_only && self.buffer.undo() {
            self.reset_preferred_x();
            self.ensure_cursor_visible();
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if !self.config.disabled && !self.config.read_only && self.buffer.redo() {
            self.reset_preferred_x();
            self.ensure_cursor_visible();
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        if !self.config.disabled {
            window.show_character_palette();
        }
    }

    fn index_for_mouse(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(layout)) = (&self.last_bounds, &self.layout) else {
            return 0;
        };
        layout.closest_index_for_position(position - bounds.origin)
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.disabled {
            return;
        }
        self.focus.focus(window, cx);
        let Some(position) = window.event_position_in_layout(event.position) else {
            return;
        };
        self.selecting = true;
        let index = self.index_for_mouse(position);
        if event.modifiers.shift {
            self.buffer.select_to(index);
        } else {
            self.buffer.move_to(index);
        }
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selecting
            && let Some(position) = window.event_position_in_layout(event.position)
        {
            let index = self.index_for_mouse(position);
            self.buffer.select_to(index);
            self.reset_preferred_x();
            self.ensure_cursor_visible();
            cx.notify();
        }
    }
}

impl EntityInputHandler for TextAreaEntity {
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

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_marked = self.buffer.marked().is_some();
        self.buffer.unmark();
        if was_marked {
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.config.disabled || self.config.read_only {
            return;
        }
        self.buffer.replace(range_utf16.as_ref(), text);
        self.ensure_cursor_visible();
        self.emit_change(window, cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.config.disabled || self.config.read_only {
            return;
        }
        self.buffer
            .replace_and_mark(range_utf16.as_ref(), text, selected_utf16);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.layout.as_ref()?;
        let range = self.buffer.range_from_utf16(&range_utf16);
        let start = layout.position_for_index(range.start)?;
        let end = layout.position_for_index(range.end)?;
        Some(Bounds::from_corners(
            bounds.origin + start,
            bounds.origin + point(end.x.max(start.x + px(1.0)), end.y + layout.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let index = self
            .layout
            .as_ref()?
            .closest_index_for_position(point - bounds.origin);
        Some(self.buffer.offset_to_utf16(index))
    }
}

#[derive(Clone, Debug)]
struct TextAreaLayout {
    lines: Vec<Arc<WrappedLine>>,
    starts: Vec<usize>,
    row_starts: Vec<usize>,
    line_height: Pixels,
}

impl TextAreaLayout {
    fn new(lines: Vec<WrappedLine>, content: &str, line_height: Pixels) -> Self {
        let mut starts = Vec::with_capacity(lines.len());
        let mut offset = 0;
        for line in content.split('\n') {
            starts.push(offset);
            offset += line.len() + 1;
        }
        while starts.len() < lines.len() {
            starts.push(content.len());
        }
        starts.truncate(lines.len());
        let mut row_starts = Vec::with_capacity(lines.len());
        let mut row = 0;
        for line in &lines {
            row_starts.push(row);
            row += line.wrap_boundaries().len() + 1;
        }
        Self {
            lines: lines.into_iter().map(Arc::new).collect(),
            starts,
            row_starts,
            line_height,
        }
    }

    fn visual_rows(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.wrap_boundaries().len() + 1)
            .sum::<usize>()
            .max(1)
    }

    fn position_for_index(&self, index: usize) -> Option<Point<Pixels>> {
        let line_index = self
            .starts
            .iter()
            .enumerate()
            .rev()
            .find_map(|(line, start)| (*start <= index).then_some(line))?;
        let line = &self.lines[line_index];
        let local = index
            .saturating_sub(self.starts[line_index])
            .min(line.len());
        let mut position = line.position_for_index(local, self.line_height)?;
        position.y += self.line_height * self.row_starts[line_index];
        Some(position)
    }

    fn closest_index_for_position(&self, position: Point<Pixels>) -> usize {
        let row = pixels_to_usize((position.y / self.line_height).max(0.0));
        let line_index = self
            .row_starts
            .iter()
            .enumerate()
            .rev()
            .find_map(|(line, start)| (*start <= row).then_some(line))
            .unwrap_or(0);
        let line = &self.lines[line_index];
        let local_y = position.y - self.line_height * self.row_starts[line_index];
        let local = line
            .closest_index_for_position(point(position.x, local_y), self.line_height)
            .unwrap_or_else(|boundary| boundary);
        self.starts[line_index] + local.min(line.len())
    }

    fn selection_quads(
        &self,
        selection: &Range<usize>,
        origin: Point<Pixels>,
        color: Rgba8,
    ) -> Vec<PaintQuad> {
        if selection.is_empty() {
            return Vec::new();
        }
        let mut quads = Vec::new();
        for (line_index, line) in self.lines.iter().enumerate() {
            let global_start = self.starts[line_index];
            let global_end = global_start + line.len();
            if selection.end < global_start || selection.start > global_end {
                continue;
            }
            for (visual_row, local_range) in visual_ranges(line).into_iter().enumerate() {
                let row_global = global_start + local_range.start..global_start + local_range.end;
                let start = selection.start.max(row_global.start);
                let end = selection.end.min(row_global.end);
                if start >= end {
                    continue;
                }
                let row_start_x = line.unwrapped_layout.x_for_index(local_range.start);
                let x1 = line.unwrapped_layout.x_for_index(start - global_start) - row_start_x;
                let x2 = line.unwrapped_layout.x_for_index(end - global_start) - row_start_x;
                let y = self.line_height * (self.row_starts[line_index] + visual_row);
                quads.push(fill(
                    Bounds::new(
                        origin + point(x1, y),
                        size((x2 - x1).max(px(1.0)), self.line_height),
                    ),
                    rgba(color.as_rgba_hex()),
                ));
            }
        }
        quads
    }
}

fn visual_ranges(line: &WrappedLine) -> Vec<Range<usize>> {
    let mut starts = vec![0];
    for boundary in line.wrap_boundaries() {
        let run = &line.runs()[boundary.run_ix];
        starts.push(run.glyphs[boundary.glyph_ix].index);
    }
    let mut ranges = starts
        .windows(2)
        .map(|window| window[0]..window[1])
        .collect::<Vec<_>>();
    ranges.push(starts.last().copied().unwrap_or(0)..line.len());
    ranges
}

struct TextAreaElement {
    input: Entity<TextAreaEntity>,
}

struct TextAreaPrepaint {
    layout: TextAreaLayout,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

#[cfg(feature = "vendor-gpui")]
impl IntoElement for TextAreaElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = usize;
    type PrepaintState = TextAreaPrepaint;

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
    ) -> (LayoutId, usize) {
        let input = self.input.read(cx);
        let rows = input
            .layout
            .as_ref()
            .map_or(input.measured_rows, TextAreaLayout::visual_rows);
        (
            request_editor_layout(window, window.line_height() * rows),
            rows,
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        laid_out_rows: &mut usize,
        window: &mut Window,
        cx: &mut App,
    ) -> TextAreaPrepaint {
        let input = self.input.read(cx);
        let content: SharedString = if input.buffer.content.is_empty() {
            input.config.placeholder.clone()
        } else {
            input.buffer.content.clone().into()
        };
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let color = if input.buffer.content.is_empty() {
            rgba(input.config.placeholder_color.as_rgba_hex()).into()
        } else {
            text_style.color
        };
        let base = TextRun {
            len: content.len(),
            font: text_style.font(),
            #[cfg(feature = "vendor-gpui")]
            letter_spacing: text_style.letter_spacing,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = marked_runs(&base, input.buffer.marked.as_ref(), content.len());
        let lines = window
            .text_system()
            .shape_text(content, font_size, &runs, Some(bounds.size.width), None)
            .unwrap_or_default()
            .into_vec();
        let lines = if lines.is_empty() {
            window
                .text_system()
                .shape_text(
                    " ".into(),
                    font_size,
                    &[base],
                    Some(bounds.size.width),
                    None,
                )
                .unwrap_or_default()
                .into_vec()
        } else {
            lines
        };
        let layout = TextAreaLayout::new(lines, &input.buffer.content, line_height);
        let selection = layout.selection_quads(
            &input.buffer.selected,
            bounds.origin,
            input.config.selection_color,
        );
        let cursor = input
            .focus
            .is_focused(window)
            .then(|| layout.position_for_index(input.buffer.cursor_offset()))
            .flatten()
            .map(|position| {
                fill(
                    Bounds::new(bounds.origin + position, size(px(1.5), line_height)),
                    rgba(input.config.caret_color.as_rgba_hex()),
                )
            });
        self.input.update(cx, |input, cx| {
            let visual_rows = layout.visual_rows();
            let measured = input.config.viewport_rows(visual_rows);
            let relayout = visual_rows != *laid_out_rows || measured != input.measured_rows;
            input.measured_rows = measured;
            input.last_bounds = Some(bounds);
            input.layout = Some(layout.clone());
            if relayout || input.caret_reveal_pending {
                // A capped viewport can keep its height while the child
                // grows. Relayout that extent before reading ScrollHandle's
                // range; the offset of this prepaint has already been chosen.
                cx.defer_in(window, move |input, window, cx| {
                    if relayout || input.reveal_caret_in_viewport(window) {
                        cx.notify();
                    }
                });
            }
        });
        TextAreaPrepaint {
            layout,
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _laid_out_rows: &mut usize,
        state: &mut TextAreaPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input = self.input.read(cx);
        let focus = input.focus.clone();
        let line_height = state.layout.line_height;
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for quad in state.selection.drain(..) {
            window.paint_quad(quad);
        }
        for (index, line) in state.layout.lines.iter().enumerate() {
            let y = line_height * state.layout.row_starts[index];
            let _ = line.paint(
                bounds.origin + point(px(0.0), y),
                line_height,
                TextAlign::Left,
                Some(bounds),
                window,
                cx,
            );
        }
        if let Some(cursor) = state.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}

fn marked_runs(base: &TextRun, marked: Option<&Range<usize>>, len: usize) -> Vec<TextRun> {
    let Some(marked) = marked.filter(|range| range.end <= len) else {
        return vec![base.clone()];
    };
    vec![
        TextRun {
            len: marked.start,
            ..base.clone()
        },
        TextRun {
            len: marked.end - marked.start,
            underline: Some(UnderlineStyle {
                color: Some(base.color),
                thickness: px(1.0),
                #[cfg(feature = "vendor-gpui")]
                paint_tag: None,
                #[cfg(feature = "vendor-gpui")]
                baseline_offset: None,
                wavy: false,
            }),
            ..base.clone()
        },
        TextRun {
            len: len - marked.end,
            ..base.clone()
        },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
}

impl Render for TextAreaEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::app::read_host_frame_epoch(cx);
        let viewport_height = self
            .config
            .viewport_height(window.line_height(), self.measured_rows);
        let key_context = match (
            self.callbacks.submit.is_some(),
            self.config.submit_key.as_deref(),
            self.callbacks
                .alt_submit
                .as_ref()
                .map(|_| self.config.alt_submit_key.as_deref().unwrap_or("alt_enter")),
        ) {
            (true, Some("enter"), Some("alt_enter")) => "GPUIRhaiTextarea submit_enter alt_submit",
            (true, Some("enter"), Some("mod_enter")) => {
                "GPUIRhaiTextarea submit_enter alt_mod_submit"
            }
            (true, Some("mod_enter"), Some("alt_enter")) => {
                "GPUIRhaiTextarea submit_mod_enter alt_submit"
            }
            (true, Some("enter"), _) => "GPUIRhaiTextarea submit_enter",
            (true, Some("mod_enter"), _) => "GPUIRhaiTextarea submit_mod_enter",
            (_, _, Some("alt_enter")) => "GPUIRhaiTextarea alt_submit",
            (_, _, Some("mod_enter")) => "GPUIRhaiTextarea alt_mod_submit",
            _ => "GPUIRhaiTextarea",
        };
        let element = div()
            .flex()
            .key_context(key_context)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|editor, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                if key == "escape" && editor.buffer.marked().is_some() {
                    // Platform input may already consume this. Hosts that still
                    // forward the key must not let a parent interrupt work
                    // while the focused editor has pending composition.
                    cx.stop_propagation();
                } else if key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::alt_submit))
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
            .child(
                div()
                    .id("textarea-scroll")
                    .h(viewport_height)
                    .w_full()
                    .overflow_y_scroll()
                    .restrict_scroll_to_axis()
                    .track_scroll(&self.scroll)
                    .when_some(self.config.scroll_color, |scroll, color| {
                        scroll.bg(rgba(color.as_rgba_hex()))
                    })
                    .child(TextAreaElement { input: cx.entity() }),
            );
        if let Some(surface) = &self.surface {
            surface.decorate(element)
        } else {
            element.into_any_element()
        }
    }
}

impl Focusable for TextAreaEntity {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

#[derive(Default)]
pub struct TextAreaPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<TextAreaEntity>>,
}

impl PrimitiveHandler for TextAreaPrimitiveHandler {
    fn style_defaults(&self, props: &PrimitiveProps) -> Result<Style, String> {
        text_style_defaults(props)
    }

    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn accessibility_actions(
        &self,
        _instance: &PrimitiveInstanceId,
    ) -> Vec<gpui::AccessibleAction> {
        vec![
            gpui::AccessibleAction::Focus,
            gpui::AccessibleAction::SetValue,
        ]
    }

    fn perform_accessibility_action(
        &mut self,
        instance: &PrimitiveInstanceId,
        action: gpui::AccessibleAction,
        data: Option<&gpui::accesskit::ActionData>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), String> {
        let entity = self
            .instances
            .get(instance)
            .cloned()
            .ok_or_else(|| "textarea accessibility target is stale".to_owned())?;
        match action {
            gpui::AccessibleAction::Focus => {
                let (disabled, focus) = {
                    let input = entity.read(cx);
                    (input.config.disabled, input.focus.clone())
                };
                if disabled {
                    return Err("disabled textarea cannot receive focus".to_owned());
                }
                focus.focus(window, cx);
                Ok(())
            }
            gpui::AccessibleAction::SetValue => {
                let Some(gpui::accesskit::ActionData::Value(value)) = data else {
                    return Err("textarea SetValue requires string data".to_owned());
                };
                let callback = {
                    let input = entity.read(cx);
                    if input.config.disabled || input.config.read_only {
                        return Err("textarea is disabled or read-only".to_owned());
                    }
                    input.callbacks.change.clone()
                };
                if let Some(callback) = callback {
                    callback(value.to_string(), window, cx);
                }
                Ok(())
            }
            _ => Err("unsupported textarea accessibility action".to_owned()),
        }
    }

    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveContext,
        theme: &crate::PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<gpui::AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "TextareaPrimitive requires a stable key".to_owned())?;
        if id.key().trim().is_empty() {
            return Err("TextareaPrimitive key cannot be empty".to_owned());
        }
        let value = instance
            .node
            .props
            .string("value")
            .unwrap_or_default()
            .to_owned();
        let config = config_from_props(&instance.node.props, theme, window.rem_size())?;
        let callbacks = primitive_callbacks(events);
        let shared_focus = instance.focus_handle().cloned();
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| {
                let mut input = TextAreaEntity::new(&value, config.clone(), callbacks.clone(), cx)
                    .expect("validated Textarea props remain valid during entity creation");
                if let Some(focus) = shared_focus.clone() {
                    input.focus = focus.tab_stop(!config.disabled);
                }
                input
            });
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
                if input.config.autofocus && !input.config.disabled {
                    input.focus.focus(window, cx);
                }
            });
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |input, cx| {
            input.surface = events.editor_surface();
            input.update_props(&value, config, callbacks, cx)
        })?;
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn validate_text_area_value(value: &str, config: &TextAreaConfig) -> Result<(), String> {
    if config.submit_key.as_deref() == Some("mod_enter")
        && config.alt_submit_key.as_deref() == Some("mod_enter")
    {
        return Err("Textarea primary and alternate submit keys must differ".to_owned());
    }
    if config.min_rows == 0 || config.max_rows < config.min_rows {
        return Err("Textarea requires 1 <= min_rows <= max_rows".to_owned());
    }
    if config.rows == Some(0) {
        return Err("Textarea rows must be greater than zero".to_owned());
    }
    if let Some(max_length) = config.max_length
        && unicode_segmentation::UnicodeSegmentation::graphemes(value, true).count() > max_length
    {
        return Err(format!(
            "Textarea controlled value exceeds max_length {max_length}"
        ));
    }
    Ok(())
}

fn config_from_props(
    props: &PrimitiveProps,
    theme: &crate::PrimitiveTheme,
    rem_size: Pixels,
) -> Result<TextAreaConfig, String> {
    let limit = |name| match props.get(name) {
        Some(PrimitiveValue::Length(value)) => theme
            .resolve_length(*value)
            .and_then(|length| match length {
                crate::Length::Pixels(value) => Some(value),
                crate::Length::Rems(value) => Some(value * f64::from(rem_size)),
                _ => None,
            })
            .and_then(|value| value.to_string().parse::<f32>().ok())
            .filter(|height| height.is_finite() && *height > 0.0)
            .map(|height| Some(px(height)))
            .ok_or_else(|| format!("Textarea {name} must resolve to positive pixels")),
        _ => Ok(None),
    };
    let min_viewport_height = limit("min_viewport_height")?;
    let max_viewport_height = limit("max_viewport_height")?;
    if let (Some(min), Some(max)) = (min_viewport_height, max_viewport_height)
        && min > max
    {
        return Err("Textarea min_viewport_height must not exceed max_viewport_height".to_owned());
    }
    let accent = theme
        .color("accent")
        .unwrap_or_else(|| Rgba8::from_rgba_hex(0x3b82_f6ff));
    Ok(TextAreaConfig {
        placeholder: props.string("placeholder").unwrap_or_default().into(),
        disabled: props.boolean("disabled").unwrap_or(false),
        read_only: props.boolean("read_only").unwrap_or(false),
        submit_key: props.string("submit_key").map(str::to_owned),
        alt_submit_key: props.string("alt_submit_key").map(str::to_owned),
        min_rows: usize_prop(props, "min_rows")?.unwrap_or(3),
        max_rows: usize_prop(props, "max_rows")?.unwrap_or(8),
        rows: usize_prop(props, "rows")?,
        min_viewport_height,
        max_viewport_height,
        max_length: usize_prop(props, "max_length")?,
        autofocus: props.boolean("autofocus").unwrap_or(false),
        placeholder_color: theme
            .part_color(props, "placeholder_style", false)
            .or_else(|| theme.color("text_muted"))
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0xa1a1_aaff)),
        selection_color: theme
            .part_color(props, "selection_style", true)
            .unwrap_or_else(|| with_alpha(accent, 0x55)),
        caret_color: theme
            .part_color(props, "caret_style", true)
            .unwrap_or(accent),
        scroll_color: theme.part_color(props, "scroll_style", true),
    })
}

fn with_alpha(color: Rgba8, alpha: u8) -> Rgba8 {
    Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | u32::from(alpha))
}

fn primitive_callbacks(events: &PrimitiveContext) -> TextInputCallbacks {
    let key_events = events.clone();
    let change_events = events.clone();
    let focus_events = events.clone();
    let blur_events = events.clone();
    let submit_events = events.clone();
    let alt_submit_events = events.clone();
    TextInputCallbacks {
        change: Some(Rc::new(move |value, window, cx| {
            let _ = change_events.emit("change", UiValue::String(value), window, cx);
        })),
        submit: events.observes("submit").then(|| {
            Rc::new(move |value, window: &mut Window, cx: &mut App| {
                let _ = submit_events.emit("submit", UiValue::String(value), window, cx);
            }) as crate::text_input::TextValueHandler
        }),
        alt_submit: events.observes("alt_submit").then(|| {
            Rc::new(move |value, window: &mut Window, cx: &mut App| {
                let _ = alt_submit_events.emit("alt_submit", UiValue::String(value), window, cx);
            }) as crate::text_input::TextValueHandler
        }),
        focus: Some(Rc::new(move |window, cx| {
            let _ = focus_events.emit("focus", UiValue::Null, window, cx);
        })),
        blur: Some(Rc::new(move |window, cx| {
            let _ = blur_events.emit("blur", UiValue::Null, window, cx);
        })),
        tab: None,
        key_capture: Some(Rc::new(move |window, cx| {
            key_events.script_takes_key(window, cx)
        })),
        ..TextInputCallbacks::default()
    }
}

fn usize_prop(props: &PrimitiveProps, name: &str) -> Result<Option<usize>, String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => usize::try_from(*value)
            .map(Some)
            .map_err(|_| format!("Textarea {name} must be a non-negative integer")),
        Some(PrimitiveValue::Data(UiValue::Null)) | None => Ok(None),
        _ => Err(format!("Textarea {name} must be an integer")),
    }
}

fn pixels_to_usize(value: f32) -> usize {
    value.max(0.0).to_string().parse().unwrap_or(usize::MAX)
}

/// Build the compile-time Textarea primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
#[allow(clippy::too_many_lines)] // One declarative list of documented props and events.
pub fn text_area_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_callback = || ObjectField::optional(ValueSchema::optional(ValueSchema::Callback));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.textarea").expect("static primitive ID"),
        export: "TextareaPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "value".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "The field's text; controlled, so store each `change` payload here; more than `max_length` graphemes is an error.",
                ),
            ),
            (
                "placeholder".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string()))
                    .with_doc("Hint text shown in the `placeholder_style` color while `value` is empty."),
            ),
            (
                "disabled".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Blocks focus, clicks and edits and dims the field."),
            ),
            (
                "read_only".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Keeps focus, selection and copy but blocks every edit."),
            ),
            (
                "min_rows".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                ))
                .with_default(UiValue::Integer(3))
                .with_doc(
                    "Fewest visible rows when `rows` is unset; the field grows with its wrapped text from here up to `max_rows`.",
                ),
            ),
            (
                "max_rows".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                ))
                .with_default(UiValue::Integer(8))
                .with_doc(
                    "Most visible rows when `rows` is unset, at least `min_rows`; longer text scrolls vertically.",
                ),
            ),
            (
                "min_viewport_height".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Length))
                    .with_doc("Optional positive pixel-resolving floor for the native scroll viewport, excluding outer editor padding and borders."),
            ),
            (
                "max_viewport_height".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Length))
                    .with_doc("Optional positive pixel-resolving cap for the native scroll viewport, excluding outer editor padding and borders."),
            ),
            (
                "rows".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                )))
                .with_doc(
                    "Fixed number of visible rows that overrides `min_rows` and `max_rows`; `()` lets the height follow the text.",
                ),
            ),
            (
                "max_length".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::bounded_integer(
                    Some(0),
                    Some(MAX_TEXTAREA_GRAPHEMES),
                )))
                .with_doc(
                    "Most graphemes the text may hold; typing, pasting and IME commits are cut to fit, and `()` means no limit.",
                ),
            ),
            (
                "typography".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Theme typography role, such as `body`, that sets the font family, size, weight and row height.",
                ),
            ),
            (
                "autofocus".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Focuses the field once when it first mounts, unless it is disabled."),
            ),
            (
                "placeholder_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Its text color, or else its background, colors the placeholder; theme `text_muted` when it sets neither.",
                ),
            ),
            (
                "selection_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Its background, or else its text color, fills selected text; a translucent theme `accent` when it sets neither.",
                ),
            ),
            (
                "caret_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Its background, or else its text color, colors the caret; theme `accent` when it sets neither.",
                ),
            ),
            (
                "scroll_style".to_owned(),
                ObjectField::required(ValueSchema::Style).with_doc(
                    "Its background, or else its text color, fills the scrolling text viewport; transparent when it sets neither.",
                ),
            ),
            (
                "on_change".to_owned(),
                optional_callback().with_doc(
                    "Called with the full new text after each edit, newline, paste, cut, undo, redo or committed IME composition.",
                ),
            ),
            (
                "submit_key".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::String {
                    allowed: vec!["enter".to_owned(), "mod_enter".to_owned()],
                })).with_doc("With a submit handler: `enter` sends and Shift+Enter inserts a newline; `mod_enter` sends with Cmd+Enter on macOS or Ctrl+Enter elsewhere. Unset preserves native Enter newlines."),
            ),
            (
                "on_submit".to_owned(),
                optional_callback().with_doc("Called with the full text on the configured submit key; disabled, read-only and composing fields never submit."),
            ),
            (
                "alt_submit_key".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::String {
                    allowed: vec!["alt_enter".to_owned(), "mod_enter".to_owned()],
                })).with_doc("Shortcut for on_alt_submit. Defaults to Alt+Enter; mod_enter uses Cmd+Enter on macOS or Ctrl+Enter elsewhere. Must differ from submit_key."),
            ),
            (
                "on_alt_submit".to_owned(),
                optional_callback().with_doc("Optional alternate submission (Alt+Enter by default). Uses the same native focus and composition guards as the main submit."),
            ),
            (
                "on_focus".to_owned(),
                optional_callback().with_doc("Called when the field gains keyboard focus."),
            ),
            (
                "on_blur".to_owned(),
                optional_callback().with_doc("Called when the field loses keyboard focus."),
            ),
        ]),
        events: text_area_events(),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

fn text_area_events() -> BTreeMap<String, EventSchema> {
    BTreeMap::from([
        (
            "alt_submit".to_owned(),
            EventSchema {
                doc: Some("Emitted by Alt+Enter only when explicitly enabled; reports the full text without changing it.".to_owned()),
                payload: ValueSchema::string(),
            },
        ),
        (
            "submit".to_owned(),
            EventSchema {
                doc: Some("Emitted with the full text on the configured submit key, without editing or clearing the buffer.".to_owned()),
                payload: ValueSchema::string(),
            },
        ),
        (
            "change".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted after each edit and when IME composition commits, not during it; the payload is the full new text."
                        .to_owned(),
                ),
                payload: ValueSchema::string(),
            },
        ),
        (
            "focus".to_owned(),
            EventSchema {
                doc: Some(
                    "Emitted when the field gains keyboard focus; the payload is `()`.".to_owned(),
                ),
                payload: ValueSchema::Null,
            },
        ),
        (
            "blur".to_owned(),
            EventSchema {
                doc: Some("Emitted when the field loses keyboard focus; the payload is `()`.".to_owned()),
                payload: ValueSchema::Null,
            },
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_line_layout_maps_positions_and_selection() {
        // Native shaping is covered in GPUI tests; this guards the stable row
        // bookkeeping used around WrappedLine values.
        let layout = TextAreaLayout {
            lines: Vec::new(),
            starts: Vec::new(),
            row_starts: Vec::new(),
            line_height: px(20.0),
        };
        assert_eq!(layout.visual_rows(), 1);
    }

    #[test]
    fn config_rejects_controlled_values_over_grapheme_limit() {
        let mut config = TextAreaConfig {
            placeholder: "".into(),
            disabled: false,
            read_only: false,
            submit_key: None,
            alt_submit_key: None,
            min_rows: 3,
            max_rows: 8,
            rows: None,
            min_viewport_height: None,
            max_viewport_height: None,
            max_length: Some(1),
            autofocus: false,
            placeholder_color: Rgba8::from_rgba_hex(0xa1a1_aaff),
            selection_color: Rgba8::from_rgba_hex(0x3b82_f655),
            caret_color: Rgba8::from_rgba_hex(0x3b82_f6ff),
            scroll_color: None,
        };
        assert!(validate_text_area_value("👩🏽‍💻", &config).is_ok());
        assert!(validate_text_area_value("👩🏽‍💻x", &config).is_err());
        config.submit_key = Some("mod_enter".into());
        config.alt_submit_key = Some("mod_enter".into());
        assert!(
            validate_text_area_value("", &config)
                .unwrap_err()
                .contains("must differ")
        );
    }
}
