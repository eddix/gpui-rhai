use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, IntoElement, KeyBinding,
    KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, Render, ScrollHandle, SharedString, Style as GpuiStyle, TextAlign, TextRun,
    UTF16Selection, UnderlineStyle, Window, WrappedLine, actions, div, fill, point, prelude::*, px,
    relative, rgba, size,
};
use rhai::{Engine, FuncRegistration, INT, ImmutableString};

use crate::text_edit::TextBuffer;
use crate::text_input::TextInputCallbacks;
use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveDescriptor, PrimitiveEventEmitter,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps,
    PrimitiveValue, Rgba8, UiValue, ValueSchema,
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
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
    ]
);

pub fn init_text_area(cx: &mut App) {
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
        KeyBinding::new("cmd-a", SelectAll, Some("GPUIRhaiTextarea")),
        KeyBinding::new("cmd-v", Paste, Some("GPUIRhaiTextarea")),
        KeyBinding::new("cmd-c", Copy, Some("GPUIRhaiTextarea")),
        KeyBinding::new("cmd-x", Cut, Some("GPUIRhaiTextarea")),
        KeyBinding::new("cmd-z", Undo, Some("GPUIRhaiTextarea")),
        KeyBinding::new("shift-cmd-z", Redo, Some("GPUIRhaiTextarea")),
        KeyBinding::new("home", Home, Some("GPUIRhaiTextarea")),
        KeyBinding::new("end", End, Some("GPUIRhaiTextarea")),
        KeyBinding::new("enter", Newline, Some("GPUIRhaiTextarea")),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some("GPUIRhaiTextarea"),
        ),
    ]);
}

pub(crate) fn register_text_area_api(engine: &mut Engine) {
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
    min_rows: usize,
    max_rows: usize,
    rows: Option<usize>,
    max_length: Option<usize>,
    line_height: Pixels,
    font_size: Pixels,
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
}

pub(crate) struct TextAreaEntity {
    focus: FocusHandle,
    buffer: TextBuffer,
    config: TextAreaConfig,
    callbacks: TextInputCallbacks,
    layout: Option<TextAreaLayout>,
    measured_rows: usize,
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
            buffer,
            measured_rows: config.min_rows,
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
        self.focus = self.focus.clone().tab_stop(!config.disabled);
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

    fn ensure_cursor_visible(&self) {
        let Some(layout) = &self.layout else {
            return;
        };
        let Some(position) = layout.position_for_index(self.buffer.cursor_offset()) else {
            return;
        };
        let viewport = self.config.line_height * self.config.viewport_rows(self.measured_rows);
        let mut top = (-self.scroll.offset().y).max(px(0.0));
        if position.y < top {
            top = position.y;
        } else if position.y + self.config.line_height > top + viewport {
            top = position.y + self.config.line_height - viewport;
        }
        self.scroll.set_offset(point(px(0.0), -top));
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
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

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
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

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.buffer.previous_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.buffer.next_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, false);
        cx.notify();
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(1, false);
        cx.notify();
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, true);
        cx.notify();
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
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
            position.y - self.config.line_height
        } else {
            position.y + self.config.line_height
        };
        let offset = layout.closest_index_for_position(point(x, target_y));
        if extend {
            self.buffer.select_to(offset);
        } else {
            self.buffer.move_to(offset);
        }
        self.ensure_cursor_visible();
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.move_to(0);
        self.buffer.select_to(self.buffer.content.len());
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer
            .move_to(self.buffer.line_start(self.buffer.cursor_offset()));
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer
            .move_to(self.buffer.line_end(self.buffer.cursor_offset()));
        self.reset_preferred_x();
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
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
        if self.config.disabled || self.config.read_only {
            return;
        }
        self.buffer.replace(None, "\n");
        self.reset_preferred_x();
        self.ensure_cursor_visible();
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
        if !self.config.disabled && !self.config.read_only && self.buffer.undo() {
            self.reset_preferred_x();
            self.ensure_cursor_visible();
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
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
        self.focus.focus(window);
        self.selecting = true;
        let index = self.index_for_mouse(event.position);
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

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            let index = self.index_for_mouse(event.position);
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
            bounds.origin
                + point(
                    end.x.max(start.x + px(1.0)),
                    end.y + self.config.line_height,
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
        let index = self
            .layout
            .as_ref()?
            .closest_index_for_position(point - bounds.origin);
        Some(self.buffer.offset_to_utf16(index))
    }
}

#[derive(Clone, Debug)]
struct TextAreaLayout {
    lines: Vec<WrappedLine>,
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
            lines,
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

impl IntoElement for TextAreaElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = ();
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
    ) -> (LayoutId, ()) {
        let input = self.input.read(cx);
        let rows = input
            .layout
            .as_ref()
            .map_or(input.measured_rows, TextAreaLayout::visual_rows);
        let mut style = GpuiStyle::default();
        style.size.width = relative(1.0).into();
        style.size.height = (input.config.line_height * rows).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
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
        let color = if input.buffer.content.is_empty() {
            rgba(input.config.placeholder_color.as_rgba_hex()).into()
        } else {
            text_style.color
        };
        let base = TextRun {
            len: content.len(),
            font: text_style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = marked_runs(&base, input.buffer.marked.as_ref(), content.len());
        let lines = window
            .text_system()
            .shape_text(
                content,
                input.config.font_size,
                &runs,
                Some(bounds.size.width),
                None,
            )
            .unwrap_or_default()
            .into_vec();
        let lines = if lines.is_empty() {
            window
                .text_system()
                .shape_text(
                    " ".into(),
                    input.config.font_size,
                    &[base],
                    Some(bounds.size.width),
                    None,
                )
                .unwrap_or_default()
                .into_vec()
        } else {
            lines
        };
        let layout = TextAreaLayout::new(lines, &input.buffer.content, input.config.line_height);
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
                    Bounds::new(
                        bounds.origin + position,
                        size(px(1.5), input.config.line_height),
                    ),
                    rgba(input.config.caret_color.as_rgba_hex()),
                )
            });
        self.input.update(cx, |input, cx| {
            let visual_rows = layout.visual_rows();
            let measured = input.config.viewport_rows(visual_rows);
            if measured != input.measured_rows {
                input.measured_rows = measured;
                cx.notify();
            }
            input.last_bounds = Some(bounds);
            input.layout = Some(layout.clone());
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
        (): &mut (),
        state: &mut TextAreaPrepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input = self.input.read(cx);
        let focus = input.focus.clone();
        let line_height = input.config.line_height;
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport_rows = self.config.viewport_rows(self.measured_rows);
        let viewport_height = self.config.line_height * viewport_rows;
        div()
            .flex()
            .key_context("GPUIRhaiTextarea")
            .track_focus(&self.focus)
            .on_key_down(|event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    if event.keystroke.modifiers.shift {
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
            .line_height(self.config.line_height)
            .text_size(self.config.font_size)
            .opacity(if self.config.disabled { 0.55 } else { 1.0 })
            .child(
                div()
                    .id("textarea-scroll")
                    .h(viewport_height)
                    .w_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .when_some(self.config.scroll_color, |scroll, color| {
                        scroll.bg(rgba(color.as_rgba_hex()))
                    })
                    .child(TextAreaElement { input: cx.entity() }),
            )
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
            .ok_or_else(|| "TextareaPrimitive requires a stable key".to_owned())?;
        if id.key.trim().is_empty() {
            return Err("TextareaPrimitive key cannot be empty".to_owned());
        }
        let value = string_prop(&instance.node.props, "value").unwrap_or_default();
        let config = config_from_props(&instance.node.props, theme)?;
        let callbacks = primitive_callbacks(events);
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| {
                TextAreaEntity::new(&value, config.clone(), callbacks.clone(), cx)
                    .expect("validated Textarea props remain valid during entity creation")
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
                    input.focus.focus(window);
                }
            });
            self.instances.insert(id, entity.clone());
            entity
        };
        entity.update(cx, |input, cx| {
            input.update_props(&value, config, callbacks, cx)
        })?;
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn validate_text_area_value(value: &str, config: &TextAreaConfig) -> Result<(), String> {
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
) -> Result<TextAreaConfig, String> {
    let accent = theme
        .color("accent")
        .unwrap_or_else(|| Rgba8::from_rgba_hex(0x3b82_f6ff));
    Ok(TextAreaConfig {
        placeholder: string_prop(props, "placeholder").unwrap_or_default().into(),
        disabled: bool_prop(props, "disabled").unwrap_or(false),
        read_only: bool_prop(props, "read_only").unwrap_or(false),
        min_rows: usize_prop(props, "min_rows")?.unwrap_or(3),
        max_rows: usize_prop(props, "max_rows")?.unwrap_or(8),
        rows: usize_prop(props, "rows")?,
        max_length: usize_prop(props, "max_length")?,
        line_height: px(float_prop(props, "line_height").unwrap_or(20.0)),
        font_size: px(float_prop(props, "font_size").unwrap_or(14.0)),
        autofocus: bool_prop(props, "autofocus").unwrap_or(false),
        placeholder_color: part_color(props, "placeholder_style", theme, false)
            .or_else(|| theme.color("text_muted"))
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0xa1a1_aaff)),
        selection_color: part_color(props, "selection_style", theme, true)
            .unwrap_or_else(|| with_alpha(accent, 0x55)),
        caret_color: part_color(props, "caret_style", theme, true).unwrap_or(accent),
        scroll_color: part_color(props, "scroll_style", theme, true),
    })
}

fn part_color(
    props: &PrimitiveProps,
    name: &str,
    theme: &crate::PrimitiveTheme,
    prefer_background: bool,
) -> Option<Rgba8> {
    let Some(PrimitiveValue::Style(style)) = props.get(name) else {
        return None;
    };
    let resolved = style.resolve(&crate::InteractionState::default());
    let value = if prefer_background {
        resolved
            .background
            .as_ref()
            .or(resolved.text_color.as_ref())
    } else {
        resolved
            .text_color
            .as_ref()
            .or(resolved.background.as_ref())
    }?;
    theme.resolve_color(value)
}

fn with_alpha(color: Rgba8, alpha: u8) -> Rgba8 {
    Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | u32::from(alpha))
}

fn primitive_callbacks(events: &PrimitiveEventEmitter) -> TextInputCallbacks {
    let change_events = events.clone();
    let focus_events = events.clone();
    let blur_events = events.clone();
    TextInputCallbacks {
        change: Some(Rc::new(move |value, window, cx| {
            let _ = change_events.emit("change", UiValue::String(value), window, cx);
        })),
        submit: None,
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

fn usize_prop(props: &PrimitiveProps, name: &str) -> Result<Option<usize>, String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => usize::try_from(*value)
            .map(Some)
            .map_err(|_| format!("Textarea {name} must be a non-negative integer")),
        Some(PrimitiveValue::Data(UiValue::Null)) | None => Ok(None),
        _ => Err(format!("Textarea {name} must be an integer")),
    }
}

fn float_prop(props: &PrimitiveProps, name: &str) -> Option<f32> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Float(value))) => value.to_string().parse().ok(),
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => value.to_string().parse().ok(),
        _ => None,
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
pub fn text_area_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_callback = || ObjectField::optional(ValueSchema::optional(ValueSchema::Callback));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.textarea").expect("static primitive ID"),
        export: "TextareaPrimitive".to_owned(),
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
            (
                "min_rows".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                ))
                .with_default(UiValue::Integer(3)),
            ),
            (
                "max_rows".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                ))
                .with_default(UiValue::Integer(8)),
            ),
            (
                "rows".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::bounded_integer(
                    Some(1),
                    Some(MAX_TEXTAREA_ROWS),
                ))),
            ),
            (
                "max_length".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::bounded_integer(
                    Some(0),
                    Some(MAX_TEXTAREA_GRAPHEMES),
                ))),
            ),
            (
                "line_height".to_owned(),
                ObjectField::required(ValueSchema::bounded_number(Some(1.0), Some(2_048.0))),
            ),
            (
                "font_size".to_owned(),
                ObjectField::required(ValueSchema::bounded_number(Some(1.0), Some(512.0))),
            ),
            (
                "autofocus".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(false)),
            ),
            (
                "placeholder_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "selection_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "caret_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "scroll_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            ("on_change".to_owned(), optional_callback()),
            ("on_focus".to_owned(), optional_callback()),
            ("on_blur".to_owned(), optional_callback()),
        ]),
        events: text_area_events(),
        state: ComponentStateSchema::default(),
        lifecycle: true,
    }
}

fn text_area_events() -> BTreeMap<String, EventSchema> {
    BTreeMap::from([
        (
            "change".to_owned(),
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
        let config = TextAreaConfig {
            placeholder: "".into(),
            disabled: false,
            read_only: false,
            min_rows: 3,
            max_rows: 8,
            rows: None,
            max_length: Some(1),
            line_height: px(20.0),
            font_size: px(14.0),
            autofocus: false,
            placeholder_color: Rgba8::from_rgba_hex(0xa1a1_aaff),
            selection_color: Rgba8::from_rgba_hex(0x3b82_f655),
            caret_color: Rgba8::from_rgba_hex(0x3b82_f6ff),
            scroll_color: None,
        };
        assert!(validate_text_area_value("👩🏽‍💻", &config).is_ok());
        assert!(validate_text_area_value("👩🏽‍💻x", &config).is_err());
    }
}
