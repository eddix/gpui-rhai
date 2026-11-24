//! Native text input behavior adapted from GPUI's Apache-2.0 `examples/input.rs`.

#[cfg(not(feature = "vendor-gpui"))]
use crate::presentation_backend::PublicWindowExt as _;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    App, AvailableSpace, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, Div, Element,
    ElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable,
    FontFallbacks, FontWeight, GlobalElementId, IntoElement, KeyBinding, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, Render,
    ShapedLine, SharedString, Style as GpuiStyle, TextAlign, TextRun, UTF16Selection,
    UnderlineStyle, Window, actions, div, fill, point, prelude::*, px, relative, rgba, size,
};

use crate::segmented_entry::{CodeFrame, CodePresentation};
pub use crate::text_edit::TextBuffer;

use crate::{
    ComponentStateSchema, EventSchema, ObjectField, PrimitiveContext, PrimitiveDescriptor,
    PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps, Rgba8,
    Style, UiValue, ValueSchema,
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
        KeyBinding::new("up", Home, Some("GPUIRhaiCodeEntry")),
        KeyBinding::new("down", End, Some("GPUIRhaiCodeEntry")),
        KeyBinding::new("enter", Submit, Some("GPUIRhaiTextInput")),
        KeyBinding::new(
            "ctrl-cmd-space",
            ShowCharacterPalette,
            Some("GPUIRhaiTextInput"),
        ),
    ]);
}

pub(crate) type TextValueHandler = Rc<dyn Fn(String, &mut Window, &mut App)>;
type TextAccessValueHandler = Rc<dyn Fn(UiValue, &mut Window, &mut App)>;
pub(crate) type TextSignalHandler = Rc<dyn Fn(&mut Window, &mut App)>;
pub(crate) type TextTabHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
/// Whether a script capture handler above the editor takes the key being
/// dispatched (see [`crate::PrimitiveContext`]).
pub(crate) type TextKeyCaptureHandler = Rc<dyn Fn(&mut Window, &mut App) -> bool>;

#[derive(Clone, Default)]
pub(crate) struct TextInputCallbacks {
    pub change: Option<TextValueHandler>,
    pub submit: Option<TextValueHandler>,
    pub alt_submit: Option<TextValueHandler>,
    pub focus: Option<TextSignalHandler>,
    pub blur: Option<TextSignalHandler>,
    pub tab: Option<TextTabHandler>,
    pub key_capture: Option<TextKeyCaptureHandler>,
    pub native_set_value: Option<TextAccessValueHandler>,
    pub native_increment: Option<TextSignalHandler>,
    pub native_decrement: Option<TextSignalHandler>,
    pub intents: Option<Rc<TextInputIntents>>,
}

/// Optional compound-field behavior. Ordinary editors keep their native key
/// bindings and paste path, and do not allocate this set of callbacks.
#[derive(Default)]
pub(crate) struct TextInputIntents {
    paste: Option<TextAccessValueHandler>,
    move_before: Option<TextSignalHandler>,
    empty_backspace: Option<TextSignalHandler>,
    cancel: Option<TextSignalHandler>,
    submit_on_tab: bool,
    rtl: bool,
}

pub(crate) struct TextInputEntity {
    focus: FocusHandle,
    buffer: TextBuffer,
    placeholder: SharedString,
    placeholder_color: Option<Rgba8>,
    disabled: bool,
    read_only: bool,
    selection_color: Rgba8,
    caret_color: Rgba8,
    callbacks: TextInputCallbacks,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    selecting: bool,
    width_preference: EditorWidthPreference,
    code: Option<Rc<CodePresentation>>,
    accepted_code: Option<String>,
    last_code: Option<Box<CodeFrame>>,
    code_motion: RefCell<crate::segmented_motion::SegmentedMotion>,
    code_driver: Option<crate::renderer::NativeNodeMotion>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum EditorWidthPreference {
    Stable,
    Text,
}

#[derive(Clone)]
pub(crate) struct TextInputConfig {
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) placeholder_color: Option<Rgba8>,
    pub(crate) disabled: bool,
    pub(crate) read_only: bool,
    pub(crate) autofocus: bool,
    pub(crate) selection_color: Rgba8,
    pub(crate) caret_color: Rgba8,
}

fn placeholder_ink(color: Option<Rgba8>, inherited: gpui::Hsla) -> gpui::Hsla {
    color.map_or(inherited, |color| rgba(color.as_rgba_hex()).into())
}

#[derive(Clone, Debug)]
pub(crate) struct NativeTypography {
    pub(crate) family: Option<String>,
    pub(crate) fallbacks: Vec<String>,
    pub(crate) font_size: Pixels,
    pub(crate) line_height: Pixels,
    pub(crate) weight: u16,
}

impl NativeTypography {
    /// Apply a native chrome owner's typography before adding its editor.
    pub(crate) fn apply(&self, mut root: Div) -> Div {
        if let Some(family) = &self.family {
            root = root.font_family(family.clone());
        }
        if !self.fallbacks.is_empty() {
            root.text_style().font_fallbacks =
                Some(FontFallbacks::from_fonts(self.fallbacks.clone()));
        }
        root.font_weight(FontWeight(f32::from(self.weight)))
            .text_size(self.font_size)
            .line_height(self.line_height)
    }
}

pub(crate) fn text_style_defaults(props: &PrimitiveProps) -> Result<Style, String> {
    let style = Style::new()
        .typography(props.string("typography").unwrap_or("body"))
        .map_err(|e| e.to_string())?;
    if props.boolean("disabled").unwrap_or(false) {
        style.opacity(0.55).map_err(|e| e.to_string())
    } else {
        Ok(style)
    }
}

/// One native measurement contract for single- and multi-line editors. A
/// percentage-only leaf has no intrinsic width; the preference is stable as
/// content changes, while the allocator still owns known/fill/minmax widths.
pub(crate) fn request_editor_layout(window: &mut Window, height: Pixels) -> LayoutId {
    request_text_width(window, height, None)
}

fn request_text_width(window: &mut Window, height: Pixels, intrinsic: Option<&str>) -> LayoutId {
    let mut style = GpuiStyle::default();
    if intrinsic.is_none() {
        style.size.width = relative(1.0).into();
    }
    style.size.height = height.into();
    let text_style = window.text_style();
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let shape = |content: SharedString| {
        window.text_system().shape_line(
            content.clone(),
            font_size,
            &[TextRun {
                len: content.len(),
                font: text_style.font(),
                #[cfg(feature = "vendor-gpui")]
                letter_spacing: text_style.letter_spacing,
                color: text_style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        )
    };
    let zero = shape("0".into()).width;
    let preferred = match intrinsic {
        None => px((f32::from(zero) * 20.0).min(f32::MAX)),
        Some(text) => {
            let text = single_line(text).unwrap_or_default();
            (zero * 2.0).max(shape(text.into_owned().into()).width + px(2.0))
        }
    };
    window.request_measured_layout(style, move |known, available, _, _| {
        size(
            known.width.unwrap_or_else(|| match available.width {
                AvailableSpace::MinContent => px(0.0),
                AvailableSpace::MaxContent => preferred,
                AvailableSpace::Definite(width) => preferred.min(width),
            }),
            known.height.unwrap_or(height),
        )
    })
}

pub(crate) fn native_typography(
    theme: &crate::PrimitiveTheme,
    role: &str,
    window: &Window,
) -> Result<NativeTypography, String> {
    // Without a token base the role is unknown; fall back to the platform text
    // size (the window rem) so native text works without a design language.
    let Some(typography) = theme.typography(role) else {
        let font_size = window.rem_size();
        return Ok(NativeTypography {
            family: None,
            fallbacks: Vec::new(),
            font_size,
            line_height: (font_size * 1.25).round(),
            weight: 400,
        });
    };
    Ok(NativeTypography {
        family: typography.family,
        fallbacks: typography.fallbacks,
        font_size: native_length(typography.size, window.rem_size(), role, "size")?,
        line_height: native_length(
            typography.line_height,
            window.rem_size(),
            role,
            "line_height",
        )?,
        weight: typography.weight,
    })
}

fn native_length(
    value: crate::Length,
    rem_size: Pixels,
    role: &str,
    field: &str,
) -> Result<Pixels, String> {
    match value {
        crate::Length::Pixels(value) => value
            .to_string()
            .parse()
            .map(px)
            .map_err(|_| format!("typography `{role}.{field}` cannot fit native f32 pixels")),
        crate::Length::Rems(value) => value
            .to_string()
            .parse::<f32>()
            .map(|value| rem_size * value)
            .map_err(|_| format!("typography `{role}.{field}` cannot fit native f32 rems")),
        crate::Length::Relative(_) | crate::Length::Token(_) => Err(format!(
            "typography `{role}.{field}` must resolve to pixels or rems"
        )),
    }
}

impl TextInputEntity {
    pub(crate) fn callbacks(&self) -> TextInputCallbacks {
        self.callbacks.clone()
    }

    pub(crate) fn new(
        config: TextInputConfig,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus: cx.focus_handle().tab_stop(!config.disabled),
            buffer: TextBuffer::new(&config.value),
            placeholder: config.placeholder.into(),
            placeholder_color: config.placeholder_color,
            disabled: config.disabled,
            read_only: config.read_only,
            selection_color: config.selection_color,
            caret_color: config.caret_color,
            callbacks,
            last_layout: None,
            last_bounds: None,
            scroll_x: px(0.0),
            selecting: false,
            width_preference: EditorWidthPreference::Stable,
            code: None,
            accepted_code: None,
            last_code: None,
            code_motion: RefCell::default(),
            code_driver: None,
        }
    }

    pub(crate) fn update_props(
        &mut self,
        config: &TextInputConfig,
        callbacks: TextInputCallbacks,
        cx: &mut Context<Self>,
    ) {
        sync_controlled(&mut self.buffer, &config.value);
        self.focus = self.focus.clone().tab_stop(!config.disabled);
        self.placeholder = config.placeholder.clone().into();
        self.placeholder_color = config.placeholder_color;
        self.disabled = config.disabled;
        self.read_only = config.read_only;
        self.selection_color = config.selection_color;
        self.caret_color = config.caret_color;
        self.callbacks = callbacks;
        cx.notify();
    }

    fn configure_primitive(
        &mut self,
        code: Option<Rc<CodePresentation>>,
        config: &TextInputConfig,
        callbacks: TextInputCallbacks,
        props: &PrimitiveProps,
        native_motion: Option<crate::renderer::NativeNodeMotion>,
        cx: &mut Context<Self>,
    ) {
        let before = self.code.as_ref().map(|code| code.spec);
        let after = code.as_ref().map(|code| code.spec);
        let replaced = after.is_some()
            && self
                .accepted_code
                .as_ref()
                .is_some_and(|current| current != &config.value);
        if before != after || after.is_some() && (replaced || config.disabled || config.read_only) {
            self.buffer.cancel_composition();
        }
        let reset = before != after
            || self.buffer.marked().is_none() && self.buffer.content != config.value;
        self.buffer.set_max_length(after.map(|spec| spec.length));
        if let Some(after) = after {
            let previous = self
                .accepted_code
                .as_deref()
                .filter(|_| before == Some(after));
            self.code_motion.get_mut().accepted(
                previous,
                &config.value,
                !config.disabled && !config.read_only,
            );
            self.code_driver = native_motion;
        } else {
            self.code_motion = RefCell::default();
            self.code_driver = None;
        }
        self.accepted_code = after.map(|_| config.value.clone());
        self.code = code;
        self.width_preference = if props.boolean("intrinsic_width") == Some(true) {
            EditorWidthPreference::Text
        } else {
            EditorWidthPreference::Stable
        };
        self.update_props(config, callbacks, cx);
        if reset && after.is_some() {
            self.select_insertion_point(self.buffer.content.len());
        }
    }

    fn segmented_prepaint(
        &self,
        code: &CodePresentation,
        bounds: Bounds<Pixels>,
        window: &mut Window,
    ) -> TextPrepaint {
        let mut frame = code.frame(
            bounds,
            &self.buffer,
            self.focus.is_focused(window),
            !self.disabled && !self.read_only,
            window,
        );
        frame.sample_edit_motion(
            &mut self.code_motion.borrow_mut(),
            self.code_driver.as_ref(),
            self.buffer.marked().is_none(),
            window,
        );
        TextPrepaint::segmented(frame, bounds.origin)
    }

    fn emit_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.accepted_code.as_deref() == Some(self.buffer.content.as_str()) {
            return;
        }
        if let Some(change) = self.callbacks.change.clone() {
            let value = self.buffer.content.clone();
            window.defer(cx, move |window, cx| change(value, window, cx));
        }
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.move_before(false, window, cx) {
            return;
        }
        if let Some(code) = &self.code
            && self.buffer.marked().is_none()
        {
            let previous = self.buffer.previous_boundary(self.buffer.selected.start);
            code.spec.select(&mut self.buffer, previous);
            cx.notify();
            return;
        }
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.previous_boundary(self.buffer.cursor_offset())
        } else {
            self.buffer.selected.start
        };
        self.buffer.move_to(offset);
        cx.notify();
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.move_before(true, window, cx) {
            return;
        }
        if let Some(code) = &self.code
            && self.buffer.marked().is_none()
        {
            let next = self.buffer.next_boundary(self.buffer.selected.start);
            code.spec.select(&mut self.buffer, next);
            cx.notify();
            return;
        }
        let offset = if self.buffer.selected.is_empty() {
            self.buffer.next_boundary(self.buffer.selected.end)
        } else {
            self.buffer.selected.end
        };
        self.buffer.move_to(offset);
        cx.notify();
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = self.buffer.previous_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        cx.notify();
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        let offset = self.buffer.next_boundary(self.buffer.cursor_offset());
        self.buffer.select_to(offset);
        cx.notify();
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.buffer.move_to(0);
        self.buffer.select_to(self.buffer.content.len());
        cx.notify();
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.select_insertion_point(0);
        cx.notify();
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        self.select_insertion_point(self.buffer.content.len());
        cx.notify();
    }

    fn select_insertion_point(&mut self, offset: usize) {
        if let Some(code) = &self.code
            && self.buffer.marked().is_none()
        {
            code.spec.select(&mut self.buffer, offset);
        } else {
            self.buffer.move_to(offset);
        }
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

    fn emit_edit_intent(
        &self,
        handler: Option<&TextSignalHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.buffer.permits_submit(self.disabled, self.read_only) {
            return false;
        }
        let Some(handler) = handler.cloned() else {
            return false;
        };
        window.defer(cx, move |window, cx| handler(window, cx));
        true
    }

    fn move_before(&self, right: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(intents) = self.callbacks.intents.as_ref() else {
            return false;
        };
        right == intents.rtl
            && self.buffer.selected == (0..0)
            && self.emit_edit_intent(intents.move_before.as_ref(), window, cx)
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.disabled || self.read_only {
            return;
        }
        if self.buffer.content.is_empty()
            && self.emit_edit_intent(
                self.callbacks
                    .intents
                    .as_ref()
                    .and_then(|intents| intents.empty_backspace.as_ref()),
                window,
                cx,
            )
        {
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
        if self.script_takes_key(window, cx) {
            return;
        }
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
        if self.disabled || self.read_only || self.buffer.selected.is_empty() {
            return;
        }
        self.copy(&Copy, window, cx);
        self.buffer.replace(None, "");
        self.emit_change(window, cx);
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.disabled || self.read_only {
            return;
        }
        if let Some(clipboard) = cx.read_from_clipboard().and_then(|item| item.text())
            && let Some(text) = single_line(&clipboard)
        {
            if let Some(code) = &self.code {
                code.spec.replace(&mut self.buffer, None, &text, true);
            } else {
                self.buffer.replace(None, &text);
            }
            if let Some(paste) = self
                .callbacks
                .intents
                .as_ref()
                .and_then(|intents| intents.paste.clone())
            {
                // Keep native selection and undo for an ordinary pasted token.
                // A compound control can also read the unflattened list once.
                let payload = UiValue::Map(BTreeMap::from([
                    ("clipboard".into(), UiValue::String(clipboard)),
                    ("value".into(), UiValue::String(self.buffer.content.clone())),
                ]));
                window.defer(cx, move |window, cx| paste(payload, window, cx));
            } else {
                self.emit_change(window, cx);
            }
            cx.notify();
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "tab" => {
                let shift = event.keystroke.modifiers.shift;
                if let Some(tab) = self.callbacks.tab.clone() {
                    window.defer(cx, move |window, cx| tab(shift, window, cx));
                } else if !shift
                    && self
                        .callbacks
                        .intents
                        .as_ref()
                        .is_some_and(|intents| intents.submit_on_tab)
                    && self.buffer.permits_submit(self.disabled, self.read_only)
                    && !self.buffer.content.trim().is_empty()
                    && self.callbacks.submit.is_some()
                {
                    self.submit(&Submit, window, cx);
                } else if shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                cx.stop_propagation();
            }
            "escape"
                if !self.buffer.content.is_empty()
                    && self.emit_edit_intent(
                        self.callbacks
                            .intents
                            .as_ref()
                            .and_then(|intents| intents.cancel.as_ref()),
                        window,
                        cx,
                    ) =>
            {
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    fn undo(&mut self, _: &Undo, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if !self.disabled && !self.read_only && self.buffer.undo() {
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if !self.disabled && !self.read_only && self.buffer.redo() {
            self.emit_change(window, cx);
            cx.notify();
        }
    }

    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.script_takes_key(window, cx) {
            return;
        }
        if self.buffer.permits_submit(self.disabled, self.read_only)
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
        if let Some(code) = &self.last_code {
            return code.index_for_point(position, self.buffer.content.len());
        }
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
        self.focus.focus(window, cx);
        let Some(position) = window.event_position_in_layout(event.position) else {
            return;
        };
        self.selecting = true;
        let index = self.index_for_mouse(position);
        if event.modifiers.shift {
            self.buffer.select_to(index);
        } else {
            self.select_insertion_point(index);
        }
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

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_marked = self.buffer.marked().is_some();
        if let (Some(code), Some(marked)) = (&self.code, self.buffer.marked()) {
            let text = self.buffer.content[marked.clone()].to_owned();
            code.spec.replace(&mut self.buffer, None, &text, false);
        } else {
            self.buffer.unmark();
        }
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
        if self.disabled || self.read_only {
            return;
        }
        let Some(text) = single_line(text) else {
            return;
        };
        if let Some(code) = &self.code {
            code.spec
                .replace(&mut self.buffer, range_utf16, &text, false);
        } else {
            self.buffer.replace(range_utf16.as_ref(), &text);
        }
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
        if self.disabled || self.read_only {
            return;
        }
        let Some(text) = single_line(text) else {
            return;
        };
        // Composition steps only show in the field; `change` waits for the commit.
        self.buffer
            .replace_and_mark(range_utf16.as_ref(), &text, selected_utf16);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.buffer.range_from_utf16(&range_utf16);
        if let Some(code) = &self.last_code {
            return code.bounds_for_range(&range, self.buffer.content.len());
        }
        let line = self.last_layout.as_ref()?;
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
        if let Some(code) = &self.last_code {
            return Some(
                self.buffer
                    .offset_to_utf16(code.index_for_point(point, self.buffer.content.len())),
            );
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
    code: Option<Box<CodeFrame>>,
}

impl TextPrepaint {
    fn segmented(frame: CodeFrame, origin: Point<Pixels>) -> Self {
        Self {
            line: None,
            cursor: None,
            selection: None,
            origin,
            scroll_x: px(0.),
            code: Some(Box::new(frame)),
        }
    }
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

#[cfg(feature = "vendor-gpui")]
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
        let height = window.line_height();
        let input = self.input.read(cx);
        if let Some(code) = &input.code {
            return (code.request_layout(window), ());
        }
        let intrinsic = (input.width_preference == EditorWidthPreference::Text).then(|| {
            if input.buffer.content.is_empty() {
                input.placeholder.as_ref()
            } else {
                input.buffer.content.as_str()
            }
        });
        (request_text_width(window, height, intrinsic), ())
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
        if let Some(code) = &input.code {
            return input.segmented_prepaint(code, bounds, window);
        }
        let content: SharedString = input.buffer.content.clone().into();
        let selected = input.buffer.selected.clone();
        let cursor = input.buffer.cursor_offset();
        let text_style = window.text_style();
        let (display, color) = if content.is_empty() {
            (
                input.placeholder.clone(),
                placeholder_ink(input.placeholder_color, text_style.color),
            )
        } else {
            (content, text_style.color)
        };
        let run = TextRun {
            len: display.len(),
            font: text_style.font(),
            #[cfg(feature = "vendor-gpui")]
            letter_spacing: text_style.letter_spacing,
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
                        #[cfg(feature = "vendor-gpui")]
                        paint_tag: None,
                        #[cfg(feature = "vendor-gpui")]
                        baseline_offset: None,
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
            code: None,
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
        if !self.input.read(cx).disabled {
            window.handle_input(
                &focus,
                ElementInputHandler::new(bounds, self.input.clone()),
                cx,
            );
        }
        if let Some(frame) = state.code.take() {
            if let Some(code) = self.input.read(cx).code.clone() {
                code.paint(&frame, window, cx);
            }
            self.input.update(cx, |input, _| {
                input.last_code = Some(frame);
                input.last_layout = None;
                input.last_bounds = Some(bounds);
            });
            return;
        }
        let Some(line) = state.line.take() else {
            return;
        };
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(selection) = state.selection.take() {
                window.paint_quad(selection);
            }
            let _ = line.paint(
                state.origin,
                window.line_height(),
                TextAlign::Left,
                None,
                window,
                cx,
            );
            if focus.is_focused(window)
                && let Some(cursor) = state.cursor.take()
            {
                window.paint_quad(cursor);
            }
        });
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_code = None;
            input.last_bounds = Some(bounds);
            input.scroll_x = state.scroll_x;
        });
    }
}

impl Render for TextInputEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::app::read_host_frame_epoch(cx);
        let submit_enabled = self.callbacks.submit.is_some();
        div()
            .flex()
            .h_full()
            .when(
                self.code.is_none() && self.width_preference == EditorWidthPreference::Stable,
                gpui::Styled::w_full,
            )
            .items_center()
            .key_context(if self.code.is_some() {
                "GPUIRhaiTextInput GPUIRhaiCodeEntry"
            } else {
                "GPUIRhaiTextInput"
            })
            .when(
                self.code.as_ref().is_none_or(|code| !code.is_success()),
                |input| input.track_focus(&self.focus),
            )
            .on_key_down(cx.listener(Self::key_down))
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
    fn style_defaults(&self, props: &PrimitiveProps) -> Result<Style, String> {
        text_style_defaults(props)
    }

    fn uses_primary_focus(&self) -> bool {
        true
    }

    fn accessibility_actions_for_props(
        &self,
        _instance: &PrimitiveInstanceId,
        props: &PrimitiveProps,
    ) -> Vec<gpui::AccessibleAction> {
        if props.boolean("code_verified") == Some(true) {
            return Vec::new();
        }
        let mut actions = vec![
            gpui::AccessibleAction::Focus,
            gpui::AccessibleAction::SetValue,
        ];
        for (prop, action) in [
            ("on_increment", gpui::AccessibleAction::Increment),
            ("on_decrement", gpui::AccessibleAction::Decrement),
        ] {
            if matches!(props.get(prop), Some(crate::PrimitiveValue::Callback(_))) {
                actions.push(action);
            }
        }
        actions
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
            .ok_or_else(|| "text input accessibility target is stale".to_owned())?;
        match action {
            gpui::AccessibleAction::Focus => {
                let (disabled, focus) = {
                    let input = entity.read(cx);
                    (input.disabled, input.focus.clone())
                };
                if disabled {
                    return Err("disabled text input cannot receive focus".to_owned());
                }
                focus.focus(window, cx);
                Ok(())
            }
            gpui::AccessibleAction::SetValue => {
                use gpui::accesskit::ActionData;
                let input = entity.read(cx);
                if input.disabled || input.read_only {
                    return Err("text input is disabled or read-only".to_owned());
                }
                if let Some(callback) = input.callbacks.native_set_value.clone() {
                    let value = match data {
                        Some(ActionData::Value(text)) => UiValue::String(text.to_string()),
                        Some(ActionData::NumericValue(value)) if value.is_finite() => {
                            UiValue::Float(*value)
                        }
                        _ => return Err("SetValue requires a string or finite number".to_owned()),
                    };
                    callback(value, window, cx);
                } else {
                    let Some(ActionData::Value(value)) = data else {
                        return Err("text input SetValue requires string data".to_owned());
                    };
                    if let Some(callback) = input.callbacks.change.clone() {
                        let text = input.code.as_ref().map_or_else(
                            || value.to_string(),
                            |code| code.spec.normalize(value.as_ref()),
                        );
                        if !value.is_empty() && text.is_empty() {
                            return Ok(());
                        }
                        if input.accepted_code.as_deref() != Some(text.as_str()) {
                            callback(text, window, cx);
                        }
                    }
                }
                Ok(())
            }
            gpui::AccessibleAction::Increment | gpui::AccessibleAction::Decrement => {
                let callback = {
                    let input = entity.read(cx);
                    if input.disabled || input.read_only {
                        return Err("text input is disabled or read-only".to_owned());
                    }
                    if action == gpui::AccessibleAction::Increment {
                        input.callbacks.native_increment.clone()
                    } else {
                        input.callbacks.native_decrement.clone()
                    }
                }
                .ok_or_else(|| "text input has no stepping action".to_owned())?;
                callback(window, cx);
                Ok(())
            }
            _ => Err("unsupported text input accessibility action".to_owned()),
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
            .ok_or_else(|| "TextInputPrimitive requires a stable key".to_owned())?;
        let code = CodePresentation::resolve(&instance.node.props, theme, window).map(Rc::new);
        let raw_value = instance.node.props.string("value").unwrap_or_default();
        let value = code.as_ref().map_or_else(
            || raw_value.to_owned(),
            |code| code.spec.normalize(raw_value),
        );
        let placeholder = instance
            .node
            .props
            .string("placeholder")
            .unwrap_or_default()
            .to_owned();
        let disabled = instance.node.props.boolean("disabled").unwrap_or(false);
        let read_only = instance.node.props.boolean("read_only").unwrap_or(false);
        let autofocus = instance.node.props.boolean("autofocus").unwrap_or(false);
        let selection_color = theme
            .color("selection")
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0x292e_42ff));
        let caret_color = theme
            .color("accent")
            .unwrap_or_else(|| Rgba8::from_rgba_hex(0x7aa2_f7ff));
        let config = TextInputConfig {
            value,
            placeholder,
            placeholder_color: theme
                .part_color(&instance.node.props, "placeholder_style", false)
                .or_else(|| theme.color("text_muted")),
            disabled,
            read_only,
            autofocus,
            selection_color,
            caret_color,
        };
        let callbacks = primitive_callbacks(events, &instance.node.props, theme.direction());
        let shared_focus = instance.focus_handle().cloned();
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| {
                let mut input = TextInputEntity::new(config.clone(), callbacks.clone(), cx);
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
                // Matches TextArea: focus once on first mount when requested.
                if config.autofocus && !config.disabled {
                    input.focus.focus(window, cx);
                }
            });
            self.instances.insert(id.clone(), entity.clone());
            entity
        };
        entity.update(cx, |input, cx| {
            if code.is_none() && input.code.is_some() {
                input
                    .code_motion
                    .get_mut()
                    .stop(input.code_driver.as_ref(), window);
            }
            input.configure_primitive(
                code,
                &config,
                callbacks,
                &instance.node.props,
                events.native_motion(),
                cx,
            );
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

/// Applies the controlled value unless IME composition is in progress, so a
/// re-render with the last committed value keeps the marked text.
fn sync_controlled(buffer: &mut TextBuffer, value: &str) {
    if buffer.marked().is_none() {
        buffer.set_controlled(value);
    }
}

/// Text for a single-line field. A text of line breaks alone, the Enter key
/// reaching the input handler unhandled, inserts nothing; line breaks inside
/// other text become spaces, as in a paste. GPUI refuses to shape a line that
/// holds a line break.
fn single_line(text: &str) -> Option<std::borrow::Cow<'_, str>> {
    if !text.is_empty()
        && text
            .chars()
            .all(|character| matches!(character, '\n' | '\r'))
    {
        return None;
    }
    if text.contains(['\n', '\r']) {
        return Some(std::borrow::Cow::Owned(
            text.replace("\r\n", " ").replace(['\n', '\r'], " "),
        ));
    }
    Some(std::borrow::Cow::Borrowed(text))
}

fn primitive_callbacks(
    events: &PrimitiveContext,
    props: &PrimitiveProps,
    direction: crate::TextDirection,
) -> TextInputCallbacks {
    let key_events = events.clone();
    let change_events = events.clone();
    let submit_events = events.clone();
    let focus_events = events.clone();
    let blur_events = events.clone();
    TextInputCallbacks {
        change: Some(Rc::new(move |value, window, cx| {
            let _ = change_events.emit("change", UiValue::String(value), window, cx);
        })),
        // Without a submit handler the field leaves Enter to its ancestors.
        submit: events.observes("submit").then(|| {
            Rc::new(move |value, window: &mut Window, cx: &mut App| {
                let _ = submit_events.emit("submit", UiValue::String(value), window, cx);
            }) as TextValueHandler
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
        native_set_value: events.observes("set_value").then(|| {
            let events = events.clone();
            Rc::new(move |value, window: &mut Window, cx: &mut App| {
                let _ = events.emit("set_value", value, window, cx);
            }) as TextAccessValueHandler
        }),
        alt_submit: None,
        native_increment: text_signal_handler(events, "increment"),
        native_decrement: text_signal_handler(events, "decrement"),
        intents: text_input_intents(events, props, direction),
    }
}

fn text_input_intents(
    events: &PrimitiveContext,
    props: &PrimitiveProps,
    direction: crate::TextDirection,
) -> Option<Rc<TextInputIntents>> {
    let paste = events.observes("paste").then(|| {
        let events = events.clone();
        Rc::new(move |payload, window: &mut Window, cx: &mut App| {
            let _ = events.emit("paste", payload, window, cx);
        }) as TextAccessValueHandler
    });
    let move_before = text_signal_handler(events, "move_before");
    let empty_backspace = text_signal_handler(events, "empty_backspace");
    let cancel = text_signal_handler(events, "cancel");
    let submit_on_tab = props.boolean("submit_on_tab") == Some(true);
    (paste.is_some()
        || move_before.is_some()
        || empty_backspace.is_some()
        || cancel.is_some()
        || submit_on_tab)
        .then(|| {
            Rc::new(TextInputIntents {
                paste,
                move_before,
                empty_backspace,
                cancel,
                submit_on_tab,
                rtl: direction == crate::TextDirection::RightToLeft,
            })
        })
}

fn text_signal_handler(events: &PrimitiveContext, name: &'static str) -> Option<TextSignalHandler> {
    events.observes(name).then(|| {
        let events = events.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| {
            let _ = events.emit(name, UiValue::Null, window, cx);
        }) as TextSignalHandler
    })
}

fn code_style_prop(name: &str, doc: &str) -> (String, ObjectField) {
    (
        name.to_owned(),
        ObjectField::optional(ValueSchema::Style).with_doc(doc),
    )
}

/// Build the compile-time `TextInput` primitive schema.
///
/// # Panics
///
/// Panics only if the static built-in primitive ID becomes invalid.
#[must_use]
#[allow(clippy::too_many_lines)] // One declarative list of documented props and events.
pub fn text_input_primitive_descriptor() -> PrimitiveDescriptor {
    let optional_callback = || ObjectField::optional(ValueSchema::optional(ValueSchema::Callback));
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.text_input").expect("static primitive ID"),
        export: "TextInputPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "value".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "The field's text; controlled, so store each `change` payload here or the next render restores the old text; text still being composed with an IME stays until it commits.",
                ),
            ),
            (
                "placeholder".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string()))
                    .with_doc("Hint text shown in the placeholder part color while `value` is empty."),
            ),
            (
                "placeholder_style".to_owned(),
                ObjectField::optional(ValueSchema::Style).with_doc("Text color (or background) of the placeholder; falls back to theme text_muted, then inherited text color, never a fixed alpha."),
            ),
            (
                "intrinsic_width".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_doc(
                    "Prefer the actual shaped text width including the caret, rather than a stable 20-digit preference. Still shrinks to the allocated width.",
                ),
            ),
            (
                "code_length".to_owned(),
                ObjectField::optional(ValueSchema::Integer {min:Some(1),max:Some(24)})
                    .with_doc("Draw this many fixed-position code cells over the same editor. Constrains committed input to the configured alphabet."),
            ),
            (
                "code_separator".to_owned(),
                ObjectField::optional(ValueSchema::Integer {min:Some(0),max:Some(23)})
                    .with_doc("Before which logical cell to draw a divider; zero or outside the current code length draws none."),
            ),
            (
                "code_alphanumeric".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_doc("Accept uppercase-normalized Latin letters as well as digits in the segmented editor."),
            ),
            code_style_prop("code_cell_style", "Cell metrics, radius, edge, fill, shadows, and digit size."),
            code_style_prop("code_filled_style", "Paint refinement for occupied cells."),
            code_style_prop("code_active_style", "Outline, radius and optional halo of the active cell."),
            code_style_prop("code_separator_style", "Separator width, height and color."),
            code_style_prop("code_caret_style", "Text color of the caret in an unfilled active cell."),
            (
                "code_success_label".to_owned(),ObjectField::optional(ValueSchema::optional(ValueSchema::string()))
                    .with_doc("The native confirmation label used for constrained sizing; code_verified switches to the non-editable pill."),
            ),
            code_style_prop("code_success_style","Success capsule fill, edge, shape, height, text, and spacing."),
            code_style_prop("code_success_icon_style","Width and ink of the confirmation check."),
            (
                "code_verified".to_owned(),ObjectField::optional(ValueSchema::Bool)
                    .with_doc("Presents the non-editable confirmation while keeping the measured footprint."),
            ),
            (
                "code_discarding".to_owned(),ObjectField::optional(ValueSchema::Bool)
                    .with_doc("Uses the committed primitive progress to fade a failed attempt in reverse cell order before clearing."),
            ),
            (
                "code_centered".to_owned(),ObjectField::optional(ValueSchema::Bool)
                    .with_doc("Centers cells and confirmation content inside the editor's allocated bounds."),
            ),
            (
                "on_set_value".to_owned(),
                optional_callback().with_doc("Optional override for a complete accessibility SetValue edit."),
            ),
            (
                "on_increment".to_owned(),
                optional_callback().with_doc("Optional handler for native accessibility increment."),
            ),
            (
                "on_decrement".to_owned(),
                optional_callback().with_doc("Optional handler for native accessibility decrement."),
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
                "autofocus".to_owned(),
                ObjectField::optional(ValueSchema::Bool)
                    .with_default(UiValue::Bool(false))
                    .with_doc("Focuses the field once when it first mounts, unless it is disabled."),
            ),
            (
                "typography".to_owned(),
                ObjectField::required(ValueSchema::string()).with_doc(
                    "Theme typography role, such as `control`, that sets the font family, size, weight and line height.",
                ),
            ),
            (
                "on_change".to_owned(),
                optional_callback().with_doc(
                    "Called with the full new text after each edit, paste, cut, undo, redo or committed IME composition.",
                ),
            ),
            (
                "on_submit".to_owned(),
                optional_callback().with_doc(
                    "Called with the current text on Enter unless disabled, read-only or composing; without it, Enter reaches the field's ancestors.",
                ),
            ),
            (
                "submit_on_tab".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_doc(
                    "With on_submit, forward Tab submits nonblank text instead of traversing focus. Empty input and Shift-Tab still traverse normally.",
                ),
            ),
            (
                "on_paste".to_owned(),
                optional_callback().with_doc(
                    "If set, replaces the change notification for paste with a single notification containing the raw clipboard and the resulting single-line value.",
                ),
            ),
            (
                "on_move_before".to_owned(),
                optional_callback().with_doc(
                    "Called on the backward logical arrow with an empty selection at offset zero. Use to move into a preceding chip or field.",
                ),
            ),
            (
                "on_empty_backspace".to_owned(),
                optional_callback().with_doc("Called for Backspace in an empty editor."),
            ),
            (
                "on_cancel".to_owned(),
                optional_callback().with_doc("Handles Escape only with an editable, nonempty, unmarked draft."),
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
        events: BTreeMap::from([
            ("paste".to_owned(), EventSchema {
                doc: Some("Raw clipboard text and the resulting single-line value after replacing the native selection.".to_owned()),
                payload: ValueSchema::Object {
                    fields: BTreeMap::from([
                        ("clipboard".to_owned(), ObjectField::required(ValueSchema::string())),
                        ("value".to_owned(), ObjectField::required(ValueSchema::string())),
                    ]),
                    allow_unknown: false,
                },
            }),
            ("move_before".to_owned(), EventSchema {
                doc: Some("A logical backward move at the first insertion point, outside IME composition.".to_owned()),
                payload: ValueSchema::Null,
            }),
            ("empty_backspace".to_owned(), EventSchema {
                doc: Some("Backspace when the editor has no text.".to_owned()),
                payload: ValueSchema::Null,
            }),
            ("cancel".to_owned(), EventSchema {
                doc: Some("Escape when the editor has an editable draft and no IME composition.".to_owned()),
                payload: ValueSchema::Null,
            }),
            ("set_value".to_owned(), EventSchema { doc: Some("A complete native accessibility edit, with string or numeric data.".to_owned()), payload: ValueSchema::UiValue }),
            ("increment".to_owned(), EventSchema { doc: Some("Native accessibility increment.".to_owned()), payload: ValueSchema::Null }),
            ("decrement".to_owned(), EventSchema { doc: Some("Native accessibility decrement.".to_owned()), payload: ValueSchema::Null }),
            (
                "change".to_owned(),
                EventSchema {
                    doc: Some(
                        "Emitted after each edit, undo and redo and when IME composition commits, not during it; the payload is the full new text."
                            .to_owned(),
                    ),
                    payload: ValueSchema::string(),
                },
            ),
            (
                "submit".to_owned(),
                EventSchema {
                    doc: Some(
                        "Emitted when Enter is pressed in an enabled field; the payload is the current text."
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
                    doc: Some(
                        "Emitted when the field loses keyboard focus; the payload is `()`.".to_owned(),
                    ),
                    payload: ValueSchema::Null,
                },
            ),
        ]),
        state: ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_preserves_the_supplied_ink_instead_of_applying_fixed_opacity() {
        let inherited: gpui::Hsla = rgba(0xffff_ffff).into();
        let supplied = Rgba8::from_rgba_hex(0x99aa_bbff);
        assert_eq!(
            placeholder_ink(Some(supplied), inherited),
            rgba(0x99aa_bbff).into()
        );
        assert_eq!(placeholder_ink(None, inherited), inherited);
        let translucent = Rgba8::from_rgba_hex(0x99aa_bb80);
        assert_eq!(
            placeholder_ink(Some(translucent), inherited),
            rgba(0x99aa_bb80).into()
        );
    }

    #[test]
    fn a_single_line_field_keeps_line_breaks_out() {
        assert_eq!(single_line("\n"), None);
        assert_eq!(single_line("\r\n"), None);
        assert_eq!(single_line("").as_deref(), Some(""));
        assert_eq!(single_line("svc").as_deref(), Some("svc"));
        assert_eq!(single_line("a\nb\r\nc").as_deref(), Some("a b c"));
    }

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

    #[test]
    fn controlled_render_during_ime_composition_keeps_the_marked_text() {
        let mut buffer = TextBuffer::new("a");
        buffer.move_to(1);
        buffer.replace_and_mark(None, "に", Some(1..1));
        sync_controlled(&mut buffer, "a");
        assert_eq!(buffer.content(), "aに");
        assert_eq!(buffer.marked(), Some(&(1..4)));
        buffer.replace(None, "日本");
        sync_controlled(&mut buffer, "a日本");
        assert_eq!(buffer.content(), "a日本");
        sync_controlled(&mut buffer, "b");
        assert_eq!(buffer.content(), "b");
    }
}
