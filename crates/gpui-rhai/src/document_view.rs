//! Retained native `CodeViewer` and `DiffViewer` surfaces.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, CursorStyle, Element, ElementId, Entity,
    FocusHandle, Focusable, FontWeight, GlobalElementId, HighlightStyle, InspectorElementId,
    InteractiveElement, IntoElement, KeyBinding, ListHorizontalSizingBehavior, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, ScrollHandle,
    ScrollStrategy, SharedString, StatefulInteractiveElement, Styled, StyledText, Task, Window,
    actions, div, px, relative, rgba, uniform_list,
};
use regex::RegexBuilder;
use similar::TextDiff;
use unicode_segmentation::UnicodeSegmentation as _;

use crate::text_input::{
    NativeTypography, TextInputCallbacks, TextInputConfig, TextInputEntity, native_typography,
};
use crate::{
    DiffDisplayRow, DiffRowKind, DiffViewMode, DiffWhitespace, DocumentDescriptor,
    DocumentRuntimeConfig, DocumentSource, DocumentWrap, EventSchema, ObjectField, PreparedDiff,
    PreparedDocument, PrimitiveDescriptor, PrimitiveEventEmitter, PrimitiveHandler, PrimitiveId,
    PrimitiveInstance, PrimitiveInstanceId, PrimitiveProps, PrimitiveTheme, PrimitiveValue, Style,
    SyntaxRegistry, UiValue, ValueSchema, prepare_diff_with_limits, prepare_document_with_limits,
};

const CODE_CONTEXT: &str = "GPUIRhaiCodeViewer";
const DIFF_CONTEXT: &str = "GPUIRhaiDiffViewer";

actions!(
    gpui_rhai_document,
    [
        Find,
        FindNext,
        FindPrevious,
        CloseFind,
        CopyDocumentSelection,
        NextChange,
        PreviousChange,
        ExpandAll,
        CollapseAll,
        CopyUnifiedDiff,
        ActivateDocumentLocation,
    ]
);

#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = gpui_rhai_document, no_json)]
pub struct RevealDocumentLine {
    pub side: Option<DiffSide>,
    /// One-based logical source line.
    pub line: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One pane of a neutral two-way comparison.
pub enum DiffSide {
    /// The left document descriptor.
    Left,
    /// The right document descriptor.
    Right,
}

pub fn init_document_view(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-f", Find, Some(CODE_CONTEXT)),
        KeyBinding::new("cmd-g", FindNext, Some(CODE_CONTEXT)),
        KeyBinding::new("shift-cmd-g", FindPrevious, Some(CODE_CONTEXT)),
        KeyBinding::new("escape", CloseFind, Some(CODE_CONTEXT)),
        KeyBinding::new("cmd-c", CopyDocumentSelection, Some(CODE_CONTEXT)),
        KeyBinding::new("enter", ActivateDocumentLocation, Some(CODE_CONTEXT)),
        KeyBinding::new("cmd-f", Find, Some(DIFF_CONTEXT)),
        KeyBinding::new("cmd-g", FindNext, Some(DIFF_CONTEXT)),
        KeyBinding::new("shift-cmd-g", FindPrevious, Some(DIFF_CONTEXT)),
        KeyBinding::new("escape", CloseFind, Some(DIFF_CONTEXT)),
        KeyBinding::new("cmd-c", CopyDocumentSelection, Some(DIFF_CONTEXT)),
        KeyBinding::new("enter", ActivateDocumentLocation, Some(DIFF_CONTEXT)),
        KeyBinding::new("alt-down", NextChange, Some(DIFF_CONTEXT)),
        KeyBinding::new("alt-up", PreviousChange, Some(DIFF_CONTEXT)),
        KeyBinding::new("shift-cmd-e", ExpandAll, Some(DIFF_CONTEXT)),
        KeyBinding::new("shift-cmd-c", CollapseAll, Some(DIFF_CONTEXT)),
        KeyBinding::new("alt-cmd-c", CopyUnifiedDiff, Some(DIFF_CONTEXT)),
    ]);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentSide {
    Single,
    Left,
    Right,
}

impl DocumentSide {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone)]
struct ActiveDocumentSelection {
    side: DocumentSide,
    source: Arc<str>,
    anchor: usize,
    head: usize,
}

#[derive(Clone, Default)]
struct DocumentSelection {
    active: Rc<RefCell<Option<ActiveDocumentSelection>>>,
}

impl DocumentSelection {
    fn begin(&self, side: DocumentSide, source: Arc<str>, offset: usize) {
        self.active.borrow_mut().replace(ActiveDocumentSelection {
            side,
            source,
            anchor: offset,
            head: offset,
        });
    }

    fn update(&self, side: DocumentSide, offset: usize) {
        if let Some(selection) = self
            .active
            .borrow_mut()
            .as_mut()
            .filter(|selection| selection.side == side)
        {
            selection.head = offset.min(selection.source.len());
        }
    }

    fn range_for(
        &self,
        side: DocumentSide,
        source: &Arc<str>,
        range: Range<usize>,
    ) -> Option<Range<usize>> {
        let selection = self.active.borrow();
        let selection = selection
            .as_ref()
            .filter(|selection| selection.side == side && Arc::ptr_eq(&selection.source, source))?;
        let start = selection.anchor.min(selection.head);
        let end = selection.anchor.max(selection.head);
        let clipped_start = start.max(range.start);
        let clipped_end = end.min(range.end);
        (clipped_start < clipped_end).then_some(
            clipped_start.saturating_sub(range.start)..clipped_end.saturating_sub(range.start),
        )
    }

    fn selected_text(&self) -> Option<String> {
        let selection = self.active.borrow();
        let selection = selection.as_ref()?;
        let start = selection.anchor.min(selection.head);
        let end = selection.anchor.max(selection.head);
        (start < end)
            .then(|| selection.source.get(start..end).map(ToOwned::to_owned))
            .flatten()
    }

    fn position(&self) -> Option<(DocumentSide, Arc<str>, usize)> {
        let selection = self.active.borrow();
        let selection = selection.as_ref()?;
        Some((
            selection.side,
            Arc::clone(&selection.source),
            selection.head,
        ))
    }

    fn clear(&self) {
        self.active.borrow_mut().take();
    }

    fn rebase(&self, side: DocumentSide, previous: &Arc<str>, next: Arc<str>) {
        let mut active = self.active.borrow_mut();
        let Some(selection) = active
            .as_mut()
            .filter(|selection| selection.side == side && Arc::ptr_eq(&selection.source, previous))
        else {
            return;
        };
        selection.anchor = clamp_char_boundary(&next, selection.anchor);
        selection.head = clamp_char_boundary(&next, selection.head);
        selection.source = next;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CodeViewerConfig {
    descriptor: DocumentDescriptor,
    show_line_numbers: bool,
    wrap: DocumentWrap,
    tab_size: usize,
    gutter_style: Style,
    line_style: Style,
    text_style: Style,
    loading_style: Style,
    error_style: Style,
    search_style: Style,
}

impl CodeViewerConfig {
    fn preparation_changed(&self, next: &Self) -> bool {
        self.descriptor.source != next.descriptor.source
            || self.descriptor.file_name != next.descriptor.file_name
            || self.descriptor.language != next.descriptor.language
    }
}

#[derive(Clone, Debug)]
struct DisplaySegment {
    logical_line: usize,
    range: Range<usize>,
    first: bool,
    column_offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingDisplayColumns {
    Unwrapped,
    Wrapped(usize),
}

impl From<Option<usize>> for PendingDisplayColumns {
    fn from(columns: Option<usize>) -> Self {
        columns.map_or(Self::Unwrapped, Self::Wrapped)
    }
}

fn build_code_display(
    document: &PreparedDocument,
    columns: Option<usize>,
    tab_size: usize,
) -> Arc<[DisplaySegment]> {
    let mut display = Vec::with_capacity(document.lines().len());
    for (line_index, line) in document.lines().iter().enumerate() {
        let text = document.line_text(line_index).unwrap_or_default();
        for (range, first, column_offset) in wrap_source_line(&line.range, text, columns, tab_size)
        {
            display.push(DisplaySegment {
                logical_line: line_index,
                range,
                first,
                column_offset,
            });
        }
    }
    display.into()
}

#[derive(Clone, Copy, Debug, Default)]
struct SearchOptions {
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
}

fn search_ranges(
    text: &str,
    query: &str,
    options: SearchOptions,
    limit: usize,
) -> Result<Vec<Range<usize>>, String> {
    let pattern = if options.regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let pattern = if options.whole_word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map(|regex| {
            regex
                .find_iter(text)
                .map(|found| found.start()..found.end())
                .take(limit)
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Default)]
struct SearchState {
    open: bool,
    query: String,
    options: SearchOptions,
    matches: Vec<Range<usize>>,
    current: Option<usize>,
    error: Option<String>,
}

struct CodeViewerEntity {
    config: CodeViewerConfig,
    events: PrimitiveEventEmitter,
    theme: PrimitiveTheme,
    syntaxes: SyntaxRegistry,
    document_runtime: DocumentRuntimeConfig,
    focus: FocusHandle,
    scroll: gpui::UniformListScrollHandle,
    prepared: Option<Arc<PreparedDocument>>,
    display: Arc<[DisplaySegment]>,
    display_columns: Option<usize>,
    display_job: u64,
    display_task: Option<Task<()>>,
    pending_display_columns: Option<PendingDisplayColumns>,
    viewport: Option<Bounds<Pixels>>,
    selection: DocumentSelection,
    search: SearchState,
    search_input: Entity<TextInputEntity>,
    search_job: u64,
    search_task: Option<Task<()>>,
    job: u64,
    loading: bool,
    error: Option<String>,
    prepare_task: Option<Task<()>>,
}

impl CodeViewerEntity {
    fn new(
        config: CodeViewerConfig,
        events: PrimitiveEventEmitter,
        theme: PrimitiveTheme,
        syntaxes: SyntaxRegistry,
        document_runtime: DocumentRuntimeConfig,
        search_typography: NativeTypography,
        cx: &mut Context<Self>,
    ) -> Self {
        let weak = cx.entity().downgrade();
        let callbacks = TextInputCallbacks {
            change: Some(Rc::new({
                let weak = weak.clone();
                move |value, _, cx| {
                    let _ = weak.update(cx, |viewer, cx| {
                        viewer.search.query = value;
                        viewer.start_search(cx);
                        cx.notify();
                    });
                }
            })),
            submit: Some(Rc::new(move |_, _, cx| {
                let _ = weak.update(cx, |viewer, cx| viewer.advance_match(false, cx));
            })),
            ..TextInputCallbacks::default()
        };
        let selection_color = theme_color(&theme, "selection", 0x3347_6fff);
        let caret_color = theme_color(&theme, "accent", 0x7aa2_f7ff);
        let search_input = cx.new(|cx| {
            TextInputEntity::new(
                TextInputConfig {
                    value: String::new(),
                    placeholder: "Find in document".to_owned(),
                    disabled: false,
                    read_only: false,
                    autofocus: false,
                    selection_color,
                    caret_color,
                    typography: search_typography,
                },
                callbacks,
                cx,
            )
        });
        Self {
            config,
            events,
            theme,
            syntaxes,
            document_runtime,
            focus: cx.focus_handle(),
            scroll: gpui::UniformListScrollHandle::new(),
            prepared: None,
            display: Arc::from([]),
            display_columns: None,
            display_job: 0,
            display_task: None,
            pending_display_columns: None,
            viewport: None,
            selection: DocumentSelection::default(),
            search: SearchState::default(),
            search_input,
            search_job: 0,
            search_task: None,
            job: 0,
            loading: false,
            error: None,
            prepare_task: None,
        }
    }

    fn update(
        &mut self,
        config: CodeViewerConfig,
        events: PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        search_typography: NativeTypography,
        cx: &mut Context<Self>,
    ) {
        let needs_prepare = self.config.preparation_changed(&config);
        let presentation_changed =
            self.config.wrap != config.wrap || self.config.tab_size != config.tab_size;
        self.config = config;
        self.events = events;
        self.theme = theme.clone();
        let callbacks = self.search_input.read(cx).callbacks();
        self.search_input.update(cx, |input, cx| {
            input.update_props(
                &TextInputConfig {
                    value: self.search.query.clone(),
                    placeholder: "Find in document".to_owned(),
                    disabled: false,
                    read_only: false,
                    autofocus: false,
                    selection_color: theme_color(theme, "selection", 0x3347_6fff),
                    caret_color: theme_color(theme, "accent", 0x7aa2_f7ff),
                    typography: search_typography,
                },
                callbacks,
                cx,
            );
        });
        if needs_prepare || (self.loading && presentation_changed) {
            self.start_prepare(cx);
        } else if presentation_changed {
            self.start_display(cx, true);
        } else {
            cx.notify();
        }
    }

    fn start_prepare(&mut self, cx: &mut Context<Self>) {
        self.job = self.job.saturating_add(1);
        self.display_job = self.display_job.saturating_add(1);
        self.display_task = None;
        self.pending_display_columns = None;
        let job = self.job;
        let descriptor = self.config.descriptor.clone();
        let syntaxes = self.syntaxes.clone();
        let limits = self.document_runtime.limits();
        let columns = self.wrap_columns();
        let tab_size = self.config.tab_size;
        self.loading = true;
        self.error = None;
        self.prepare_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    prepare_document_with_limits(&descriptor, &syntaxes, limits).map(|document| {
                        let display = build_code_display(&document, columns, tab_size);
                        (document, display)
                    })
                })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.job != job {
                    return;
                }
                viewer.loading = false;
                match result {
                    Ok((document, display)) => {
                        let document = Arc::new(document);
                        viewer.display_job = viewer.display_job.saturating_add(1);
                        viewer.display_task = None;
                        viewer.pending_display_columns = None;
                        if let Some(previous) = viewer.prepared.as_ref() {
                            if previous.identity() == document.identity() {
                                viewer.selection.rebase(
                                    DocumentSide::Single,
                                    &previous.text_arc(),
                                    document.text_arc(),
                                );
                            } else {
                                viewer.selection.clear();
                                viewer.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
                            }
                        }
                        viewer.error = None;
                        viewer.prepared = Some(document);
                        viewer.display = display;
                        viewer.display_columns = columns;
                        viewer.start_search(cx);
                    }
                    Err(error) => viewer.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
    }

    fn wrap_columns(&self) -> Option<usize> {
        match self.config.wrap {
            DocumentWrap::None => None,
            DocumentWrap::Column(columns) => Some(columns.max(1)),
            DocumentWrap::Viewport => self.viewport.map(|bounds| {
                let gutter = if self.config.show_line_numbers {
                    72.0
                } else {
                    16.0
                };
                let available = (f64::from(bounds.size.width) - gutter).max(20.0);
                let (text_size, _) = document_text_metrics(&self.config.text_style, &self.theme);
                let character_width = (f64::from(text_size) * 0.604).max(1.0);
                (available / character_width)
                    .floor()
                    .to_string()
                    .parse()
                    .unwrap_or(80)
                    .max(1)
            }),
        }
    }

    fn start_display(&mut self, cx: &mut Context<Self>, force: bool) {
        let Some(document) = self.prepared.clone() else {
            return;
        };
        let columns = self.wrap_columns();
        if !force && self.display_columns == columns && !self.display.is_empty() {
            return;
        }
        if !force && self.pending_display_columns == Some(columns.into()) {
            return;
        }
        self.display_job = self.display_job.saturating_add(1);
        let job = self.display_job;
        let tab_size = self.config.tab_size;
        self.pending_display_columns = Some(columns.into());
        self.display_task = Some(cx.spawn(async move |this, cx| {
            let display = cx
                .background_spawn(async move { build_code_display(&document, columns, tab_size) })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.display_job == job {
                    viewer.display = display;
                    viewer.display_columns = columns;
                    viewer.pending_display_columns = None;
                    cx.notify();
                }
            });
        }));
    }

    fn start_search(&mut self, cx: &mut Context<Self>) {
        self.search_job = self.search_job.saturating_add(1);
        let job = self.search_job;
        self.search.matches.clear();
        self.search.error = None;
        self.search.current = None;
        let Some(document) = self.prepared.as_ref() else {
            return;
        };
        if self.search.query.is_empty() {
            self.search_task = None;
            return;
        }
        let text = document.text_arc();
        let query = self.search.query.clone();
        let options = self.search.options;
        self.search_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { search_ranges(&text, &query, options, 100_000) })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.search_job != job {
                    return;
                }
                match result {
                    Ok(matches) => {
                        viewer.search.matches = matches;
                        viewer.search.current = (!viewer.search.matches.is_empty()).then_some(0);
                    }
                    Err(error) => viewer.search.error = Some(error),
                }
                cx.notify();
            });
        }));
    }

    fn advance_match(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.loading || self.search.matches.is_empty() {
            return;
        }
        let current = self.search.current.unwrap_or(0);
        let next = if backwards {
            (current + self.search.matches.len() - 1) % self.search.matches.len()
        } else {
            (current + 1) % self.search.matches.len()
        };
        self.search.current = Some(next);
        self.reveal_match(next);
        cx.notify();
    }

    fn reveal_match(&self, index: usize) {
        let Some(target) = self.search.matches.get(index) else {
            return;
        };
        if let Some(display_index) = self.display.iter().position(|segment| {
            target.start >= segment.range.start && target.start <= segment.range.end
        }) {
            self.scroll
                .scroll_to_item(display_index, ScrollStrategy::Center);
        }
    }

    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.open = true;
        self.search_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    fn find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_match(false, cx);
    }

    fn find_previous(&mut self, _: &FindPrevious, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_match(true, cx);
    }

    fn close_find(&mut self, _: &CloseFind, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.open {
            self.search.open = false;
            self.focus.focus(window);
            cx.notify();
        }
    }

    fn copy_selection(
        &mut self,
        _: &CopyDocumentSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        if let Some(text) = self.selection.selected_text() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            cx.stop_propagation();
        }
    }

    fn reveal_line(&mut self, action: &RevealDocumentLine, _: &mut Window, cx: &mut Context<Self>) {
        if action.side.is_some() || action.line == 0 {
            return;
        }
        let line = action.line - 1;
        if let Some(index) = self
            .display
            .iter()
            .position(|segment| segment.logical_line == line)
        {
            self.scroll.scroll_to_item(index, ScrollStrategy::Center);
            cx.notify();
        }
    }

    fn activate_location(
        &mut self,
        _: &ActivateDocumentLocation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        let Some(document) = self.prepared.as_ref() else {
            return;
        };
        let Some((DocumentSide::Single, source, offset)) = self.selection.position() else {
            return;
        };
        if !Arc::ptr_eq(&source, &document.text_arc()) {
            return;
        }
        if let Some(payload) = source_location_payload(document, offset, None) {
            let events = self.events.clone();
            window.defer(cx, move |window, cx| {
                let _ = events.emit("location_activate", payload, window, cx);
            });
        }
    }

    fn toggle_search_option(&mut self, option: &'static str, cx: &mut Context<Self>) {
        match option {
            "case" => self.search.options.case_sensitive = !self.search.options.case_sensitive,
            "word" => self.search.options.whole_word = !self.search.options.whole_word,
            "regex" => self.search.options.regex = !self.search.options.regex,
            _ => return,
        }
        self.start_search(cx);
        cx.notify();
    }

    fn search_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        let button = |label: &'static str, option: &'static str, active: bool| {
            let weak = weak.clone();
            div()
                .id(SharedString::from(format!(
                    "document-search-option-{option}"
                )))
                .w(px(28.0))
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .bg(if active {
                    rgba(theme_color(&self.theme, "selection", 0x3347_6fff).as_rgba_hex())
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|style| {
                    style.bg(rgba(
                        theme_color(&self.theme, "surface_hover", 0x252a_34ff).as_rgba_hex(),
                    ))
                })
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    let _ = weak.update(cx, |viewer, cx| viewer.toggle_search_option(option, cx));
                })
                .child(label)
        };
        let status = self.search.error.as_ref().map_or_else(
            || {
                if self.search.matches.is_empty() {
                    "No matches".to_owned()
                } else {
                    format!(
                        "{} / {}",
                        self.search.current.unwrap_or(0) + 1,
                        self.search.matches.len()
                    )
                }
            },
            |error| format!("Invalid regex: {error}"),
        );
        let bar = div()
            .absolute()
            .top(px(8.0))
            .right(px(8.0))
            .w(px(430.0))
            .h(px(38.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(6.0))
            .bg(rgba(
                theme_color(&self.theme, "surface_raised", 0x1a1d_24ff).as_rgba_hex(),
            ))
            .border_1()
            .border_color(rgba(
                theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
            ))
            .child(
                div()
                    .h(px(26.0))
                    .flex_1()
                    .min_w(px(0.0))
                    .px(px(6.0))
                    .border_1()
                    .border_color(rgba(
                        theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
                    ))
                    .child(self.search_input.clone()),
            )
            .child(button("Aa", "case", self.search.options.case_sensitive))
            .child(button("W", "word", self.search.options.whole_word))
            .child(button(".*", "regex", self.search.options.regex))
            .child(div().w(px(62.0)).text_size(px(10.0)).child(status));
        crate::renderer::apply_style_override(
            bar,
            &self.config.search_style,
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }

    fn content(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(error) = self.error.as_ref() {
            return crate::renderer::apply_style_override(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(
                        theme_color(&self.theme, "danger", 0xf776_8eff).as_rgba_hex(),
                    ))
                    .child(format!("CodeViewer error: {error}")),
                &self.config.error_style,
                &self.theme,
                self.theme.direction(),
            )
            .into_any_element();
        }
        let Some(document) = self.prepared.clone() else {
            return crate::renderer::apply_style_override(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(
                        theme_color(&self.theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
                    ))
                    .child("Loading document…"),
                &self.config.loading_style,
                &self.theme,
                self.theme.direction(),
            )
            .into_any_element();
        };
        let display = Arc::clone(&self.display);
        let selection = self.selection.clone();
        let search_matches = Arc::<[Range<usize>]>::from(self.search.matches.clone());
        let current_match = self.search.current;
        let theme = self.theme.clone();
        let config = self.config.clone();
        let events = self.events.clone();
        let focus = self.focus.clone();
        let disabled = self.loading;
        let line_digits = document.lines().len().max(1).to_string().len();
        let width_index = display
            .iter()
            .enumerate()
            .max_by_key(|(_, segment)| segment.range.end.saturating_sub(segment.range.start))
            .map(|(index, _)| index);
        let list = uniform_list(
            "gpui-rhai-code-viewer-lines",
            display.len(),
            move |range, _, _| {
                range
                    .filter_map(|index| {
                        let segment = display.get(index)?;
                        Some(code_line_element(
                            &document,
                            segment,
                            line_digits,
                            &config,
                            &theme,
                            &selection,
                            &search_matches,
                            current_match,
                            &events,
                            &focus,
                            disabled,
                        ))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(self.scroll.clone())
        .with_width_from_item(width_index)
        .when(matches!(self.config.wrap, DocumentWrap::None), |list| {
            list.with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
        })
        .size_full()
        .opacity(if self.loading { 0.62 } else { 1.0 });
        let _ = cx;
        list.into_any_element()
    }
}

impl Render for CodeViewerEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .key_context(CODE_CONTEXT)
            .track_focus(&self.focus)
            .tab_stop(true)
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_action(cx.listener(Self::close_find))
            .on_action(cx.listener(Self::copy_selection))
            .on_action(cx.listener(Self::reveal_line))
            .on_action(cx.listener(Self::activate_location))
            .child(self.content(cx))
            .child(DocumentBoundsRecorder {
                viewer: cx.entity(),
            });
        if self.search.open {
            root = root.child(self.search_bar(cx));
        }
        if self.loading && self.prepared.is_some() {
            root = root.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .h(px(22.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .bg(rgba(
                        theme_color(&self.theme, "surface_raised", 0x1a1d_24ee).as_rgba_hex(),
                    ))
                    .text_size(px(10.0))
                    .child("Updating document…"),
            );
        } else if let Some(error) = self
            .prepared
            .as_ref()
            .and_then(|document| document.highlight_error())
        {
            root = root.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .h(px(22.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .bg(rgba(
                        theme_color(&self.theme, "surface_raised", 0x1a1d_24ee).as_rgba_hex(),
                    ))
                    .text_size(px(10.0))
                    .text_color(rgba(
                        theme_color(&self.theme, "warning", 0xe0af_68ff).as_rgba_hex(),
                    ))
                    .child(format!("Syntax highlighting unavailable: {error}")),
            );
        }
        root
    }
}

struct DocumentBoundsRecorder {
    viewer: Entity<CodeViewerEntity>,
}

impl Element for DocumentBoundsRecorder {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut child = div().absolute().inset_0().into_any_element();
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
        self.viewer.update(cx, |viewer, cx| {
            if viewer.viewport != Some(bounds) {
                viewer.viewport = Some(bounds);
            }
            if matches!(viewer.config.wrap, DocumentWrap::Viewport)
                && viewer.wrap_columns() != viewer.display_columns
            {
                viewer.start_display(cx, false);
            }
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for DocumentBoundsRecorder {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn code_line_element(
    document: &Arc<PreparedDocument>,
    segment: &DisplaySegment,
    line_digits: usize,
    config: &CodeViewerConfig,
    theme: &PrimitiveTheme,
    selection: &DocumentSelection,
    matches: &[Range<usize>],
    current_match: Option<usize>,
    events: &PrimitiveEventEmitter,
    focus: &FocusHandle,
    disabled: bool,
) -> AnyElement {
    let line = &document.lines()[segment.logical_line];
    let text = document
        .text()
        .get(segment.range.clone())
        .unwrap_or_default();
    let mut highlights = Vec::new();
    for syntax in &line.syntax {
        let global = line.range.start + syntax.range.start..line.range.start + syntax.range.end;
        if let Some(local) = intersect_local(&global, &segment.range) {
            highlights.push((
                local,
                HighlightStyle {
                    color: Some(
                        rgba(
                            theme_color(theme, syntax.kind.theme_token(), 0xd9de_e8ff)
                                .as_rgba_hex(),
                        )
                        .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    for (index, found) in matches.iter().enumerate() {
        if let Some(local) = intersect_local(found, &segment.range) {
            highlights.push((
                local,
                HighlightStyle {
                    background_color: Some(
                        rgba(
                            theme_color(
                                theme,
                                if current_match == Some(index) {
                                    "document.search_current"
                                } else {
                                    "document.search_match"
                                },
                                if current_match == Some(index) {
                                    0xe0af_68aa
                                } else {
                                    0x7aa2_f766
                                },
                            )
                            .as_rgba_hex(),
                        )
                        .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    let source = document.text_arc();
    if let Some(range) = selection.range_for(DocumentSide::Single, &source, segment.range.clone()) {
        highlights.push((
            range,
            HighlightStyle {
                background_color: Some(
                    rgba(theme_color(theme, "selection", 0x3347_6fff).as_rgba_hex()).into(),
                ),
                ..HighlightStyle::default()
            },
        ));
    }
    let expanded = expand_tabs(text, config.tab_size, segment.column_offset);
    let highlights = compose_highlights(
        expanded.text.len(),
        &remap_highlights(highlights, &expanded.offsets),
    );
    let styled = StyledText::new(expanded.text).with_highlights(highlights);
    let interactive = InteractiveDocumentText {
        id: SharedString::from(format!(
            "code-line-{}-{}",
            segment.logical_line, segment.range.start
        ))
        .into(),
        text: styled,
        source,
        global_range: segment.range.clone(),
        offset_map: expanded.offsets.into(),
        line: segment.logical_line,
        line_start: line.range.start,
        side: DocumentSide::Single,
        selection: selection.clone(),
        focus: focus.clone(),
        events: events.clone(),
        disabled,
    };
    let (text_size, line_height) = document_text_metrics(&config.text_style, theme);
    let gutter_width = (usize_f32(line_digits) * 8.0 + 20.0).max(42.0);
    let gutter = crate::renderer::apply_style_override(
        div()
            .w(px(gutter_width))
            .h_full()
            .flex_none()
            .pr(px(10.0))
            .flex()
            .items_center()
            .justify_end()
            .text_color(rgba(
                theme_color(theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
            ))
            .child(if config.show_line_numbers && segment.first {
                (segment.logical_line + 1).to_string()
            } else {
                String::new()
            }),
        &config.gutter_style,
        theme,
        theme.direction(),
    );
    let content = crate::renderer::apply_style_override(
        div()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .font_family(".ZedMono")
            .text_size(px(text_size))
            .line_height(px(line_height))
            .text_color(rgba(
                theme_color(theme, "text_primary", 0xd9de_e8ff).as_rgba_hex(),
            ))
            .child(interactive),
        &config.text_style,
        theme,
        theme.direction(),
    );
    crate::renderer::apply_style_override(
        div()
            .w_full()
            .h(px(line_height.max(16.0)))
            .flex()
            .flex_row()
            .items_center()
            .child(gutter)
            .child(content),
        &config.line_style,
        theme,
        theme.direction(),
    )
    .into_any_element()
}

struct InteractiveDocumentText {
    id: ElementId,
    text: StyledText,
    source: Arc<str>,
    global_range: Range<usize>,
    offset_map: Arc<[(usize, usize)]>,
    line: usize,
    line_start: usize,
    side: DocumentSide,
    selection: DocumentSelection,
    focus: FocusHandle,
    events: PrimitiveEventEmitter,
    disabled: bool,
}

impl Element for InteractiveDocumentText {
    type RequestLayoutState = ();
    type PrepaintState = gpui::Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        self.text.request_layout(None, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text
            .prepaint(None, inspector, bounds, state, window, cx);
        window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text
            .paint(None, inspector, bounds, state, &mut (), window, cx);
        if self.disabled {
            return;
        }
        window.set_cursor_style(CursorStyle::IBeam, hitbox);
        let layout = self.text.layout().clone();
        let clamp = |index: Result<usize, usize>| match index {
            Ok(index) | Err(index) => index,
        };
        {
            let selection = self.selection.clone();
            let source = Arc::clone(&self.source);
            let side = self.side;
            let range = self.global_range.clone();
            let focus = self.focus.clone();
            let events = self.events.clone();
            let line = self.line;
            let line_start = self.line_start;
            let layout = layout.clone();
            let hitbox = hitbox.clone();
            let offset_map = Arc::clone(&self.offset_map);
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if !phase.bubble()
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                {
                    return;
                }
                focus.focus(window);
                let display = clamp(layout.index_for_position(event.position));
                let local = display_source_offset(&offset_map, display).min(range.len());
                let global = range.start + local;
                selection.begin(side, Arc::clone(&source), global);
                if event.click_count >= 2 {
                    let mut payload = BTreeMap::from([
                        (
                            "line".to_owned(),
                            UiValue::Integer(i64::try_from(line + 1).unwrap_or(i64::MAX)),
                        ),
                        (
                            "column".to_owned(),
                            UiValue::Integer(
                                i64::try_from(source[line_start..global].chars().count() + 1)
                                    .unwrap_or(i64::MAX),
                            ),
                        ),
                    ]);
                    if side != DocumentSide::Single {
                        payload
                            .insert("side".to_owned(), UiValue::String(side.as_str().to_owned()));
                    }
                    let payload = UiValue::Map(payload);
                    let events = events.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = events.emit("location_activate", payload, window, cx);
                    });
                }
                window.refresh();
            });
        }
        {
            let selection = self.selection.clone();
            let side = self.side;
            let range = self.global_range.clone();
            let layout = layout.clone();
            let hitbox = hitbox.clone();
            let offset_map = Arc::clone(&self.offset_map);
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble()
                    && event.pressed_button == Some(MouseButton::Left)
                    && hitbox.is_hovered(window)
                {
                    let display = clamp(layout.index_for_position(event.position));
                    let local = display_source_offset(&offset_map, display).min(range.len());
                    selection.update(side, range.start + local);
                    cx.stop_propagation();
                    window.refresh();
                }
            });
        }
        {
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left && hitbox.is_hovered(window)
                {
                    cx.stop_propagation();
                }
            });
        }
    }
}

impl IntoElement for InteractiveDocumentText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Default)]
pub struct CodeViewerPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<CodeViewerEntity>>,
    syntaxes: Option<SyntaxRegistry>,
    document_runtime: Option<DocumentRuntimeConfig>,
}

impl CodeViewerPrimitiveHandler {
    #[must_use]
    pub fn new(syntaxes: SyntaxRegistry, document_runtime: DocumentRuntimeConfig) -> Self {
        Self {
            instances: BTreeMap::new(),
            syntaxes: Some(syntaxes),
            document_runtime: Some(document_runtime),
        }
    }
}

impl PrimitiveHandler for CodeViewerPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "CodeViewPrimitive requires a stable key".to_owned())?;
        let config = parse_code_config(&instance.node.props)?;
        let search_typography = native_typography(theme, "body_small", window)?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let syntaxes = self
                .syntaxes
                .clone()
                .ok_or_else(|| "CodeViewer syntax registry is unavailable".to_owned())?;
            let config_for_entity = config.clone();
            let events_for_entity = events.clone();
            let theme_for_entity = theme.clone();
            let document_runtime = self
                .document_runtime
                .clone()
                .ok_or_else(|| "CodeViewer document runtime is unavailable".to_owned())?;
            let entity = cx.new(|cx| {
                CodeViewerEntity::new(
                    config_for_entity,
                    events_for_entity,
                    theme_for_entity,
                    syntaxes,
                    document_runtime,
                    search_typography.clone(),
                    cx,
                )
            });
            entity.update(cx, CodeViewerEntity::start_prepare);
            self.instances.insert(id, entity.clone());
            entity
        };
        entity.update(cx, |viewer, cx| {
            viewer.update(config, events.clone(), theme, search_typography, cx);
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_code_config(props: &PrimitiveProps) -> Result<CodeViewerConfig, String> {
    let descriptor = DocumentDescriptor {
        source: document_source_prop(props, "source")?,
        label: string_prop(props, "label").unwrap_or_else(|| "Code".to_owned()),
        file_name: optional_string_prop(props, "file_name"),
        language: optional_string_prop(props, "language"),
    };
    let wrap = match string_prop(props, "wrap").as_deref() {
        None | Some("none") => DocumentWrap::None,
        Some("viewport") => DocumentWrap::Viewport,
        Some("column") => DocumentWrap::Column(integer_prop(props, "wrap_column").unwrap_or(100)),
        Some(other) => return Err(format!("unknown CodeViewer wrap mode `{other}`")),
    };
    Ok(CodeViewerConfig {
        descriptor,
        show_line_numbers: bool_prop(props, "show_line_numbers").unwrap_or(true),
        wrap,
        tab_size: integer_prop(props, "tab_size").unwrap_or(4),
        gutter_style: style_prop(props, "gutter_style"),
        line_style: style_prop(props, "line_style"),
        text_style: style_prop(props, "text_style"),
        loading_style: style_prop(props, "loading_style"),
        error_style: style_prop(props, "error_style"),
        search_style: style_prop(props, "search_style"),
    })
}

fn document_source_prop(props: &PrimitiveProps, name: &str) -> Result<DocumentSource, String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::String(value))) => Ok(value.clone().into()),
        Some(PrimitiveValue::Document(document)) => Ok(document.clone().into()),
        _ => Err(format!(
            "document `{name}` must be a string or NativeTextDocument"
        )),
    }
}

fn string_prop(props: &PrimitiveProps, name: &str) -> Option<String> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::String(value))) => Some(value.clone()),
        _ => None,
    }
}

fn optional_string_prop(props: &PrimitiveProps, name: &str) -> Option<String> {
    string_prop(props, name).filter(|value| !value.is_empty())
}

fn bool_prop(props: &PrimitiveProps, name: &str) -> Option<bool> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Bool(value))) => Some(*value),
        _ => None,
    }
}

fn integer_prop(props: &PrimitiveProps, name: &str) -> Option<usize> {
    match props.get(name) {
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => usize::try_from(*value).ok(),
        _ => None,
    }
}

fn style_prop(props: &PrimitiveProps, name: &str) -> Style {
    match props.get(name) {
        Some(PrimitiveValue::Style(style)) => (**style).clone(),
        _ => Style::new(),
    }
}

fn theme_color(theme: &PrimitiveTheme, token: &str, fallback: u32) -> crate::Rgba8 {
    theme
        .color(token)
        .unwrap_or_else(|| crate::Rgba8::from_rgba_hex(fallback))
}

fn resolved_pixels(length: crate::Length) -> Option<f32> {
    match length {
        crate::Length::Pixels(value) => value.to_string().parse().ok(),
        crate::Length::Rems(value) => (value * 16.0).to_string().parse().ok(),
        _ => None,
    }
}

fn document_text_metrics(style: &Style, theme: &PrimitiveTheme) -> (f32, f32) {
    let role = style.base.typography.as_deref().unwrap_or("body");
    let typography = theme.typography(role).or_else(|| theme.typography("body"));
    let default_size = typography
        .as_ref()
        .and_then(|value| resolved_pixels(value.size))
        .unwrap_or(12.0);
    let default_line_height = typography
        .as_ref()
        .and_then(|value| resolved_pixels(value.line_height))
        .unwrap_or(18.0);
    let text_size = style
        .base
        .font_size
        .and_then(|value| theme.resolve_length(value))
        .and_then(resolved_pixels)
        .unwrap_or(default_size);
    let line_height = style
        .base
        .line_height
        .and_then(|value| theme.resolve_length(value))
        .and_then(resolved_pixels)
        .unwrap_or(default_line_height)
        .max(text_size);
    (text_size, line_height)
}

fn usize_f32(value: usize) -> f32 {
    value.to_string().parse().unwrap_or(f32::MAX)
}

struct ExpandedSegmentText {
    text: String,
    /// `(source byte offset, displayed byte offset)` at every character boundary.
    offsets: Vec<(usize, usize)>,
}

fn wrap_source_line(
    source_range: &Range<usize>,
    text: &str,
    columns: Option<usize>,
    tab_size: usize,
) -> Vec<(Range<usize>, bool, usize)> {
    let Some(columns) = columns.map(|columns| columns.max(1)) else {
        return vec![(source_range.clone(), true, 0)];
    };
    if text.is_empty() {
        return vec![(source_range.clone(), true, 0)];
    }
    let tab_size = tab_size.max(1);
    let mut segments = Vec::new();
    let mut segment_start = 0usize;
    let mut segment_column = 0usize;
    let mut logical_column = 0usize;
    for (offset, grapheme) in text.grapheme_indices(true) {
        let width = if grapheme == "\t" {
            tab_size - (logical_column % tab_size)
        } else {
            1
        };
        if logical_column > segment_column && logical_column - segment_column + width > columns {
            segments.push((
                source_range.start + segment_start..source_range.start + offset,
                segments.is_empty(),
                segment_column,
            ));
            segment_start = offset;
            segment_column = logical_column;
        }
        logical_column = logical_column.saturating_add(width);
    }
    segments.push((
        source_range.start + segment_start..source_range.end,
        segments.is_empty(),
        segment_column,
    ));
    segments
}

fn expand_tabs(text: &str, tab_size: usize, initial_column: usize) -> ExpandedSegmentText {
    let tab_size = tab_size.max(1);
    let mut output = String::with_capacity(text.len());
    let mut offsets = vec![(0, 0)];
    let mut column = initial_column;
    for (source, character) in text.char_indices() {
        if character == '\t' {
            let spaces = tab_size - (column % tab_size);
            output.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            output.push(character);
            column += 1;
        }
        offsets.push((source + character.len_utf8(), output.len()));
    }
    ExpandedSegmentText {
        text: output,
        offsets,
    }
}

fn remap_highlights(
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    offsets: &[(usize, usize)],
) -> Vec<(Range<usize>, HighlightStyle)> {
    highlights
        .into_iter()
        .filter_map(|(range, style)| {
            let start = source_display_offset(offsets, range.start)?;
            let end = source_display_offset(offsets, range.end)?;
            (start < end).then_some((start..end, style))
        })
        .collect()
}

fn compose_highlights(
    text_len: usize,
    layers: &[(Range<usize>, HighlightStyle)],
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut boundaries = layers
        .iter()
        .flat_map(|(range, _)| [range.start.min(text_len), range.end.min(text_len)])
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut composed: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    for window in boundaries.windows(2) {
        let range = window[0]..window[1];
        if range.is_empty() {
            continue;
        }
        let mut style = HighlightStyle::default();
        let mut styled = false;
        for (layer_range, layer_style) in layers {
            if layer_range.start < range.end && layer_range.end > range.start {
                overlay_highlight(&mut style, *layer_style);
                styled = true;
            }
        }
        if !styled {
            continue;
        }
        if let Some((previous_range, previous_style)) = composed.last_mut()
            && previous_range.end == range.start
            && *previous_style == style
        {
            previous_range.end = range.end;
        } else {
            composed.push((range, style));
        }
    }
    composed
}

fn overlay_highlight(base: &mut HighlightStyle, overlay: HighlightStyle) {
    if overlay.color.is_some() {
        base.color = overlay.color;
    }
    if overlay.font_weight.is_some() {
        base.font_weight = overlay.font_weight;
    }
    if overlay.font_style.is_some() {
        base.font_style = overlay.font_style;
    }
    if overlay.background_color.is_some() {
        base.background_color = overlay.background_color;
    }
    if overlay.underline.is_some() {
        base.underline = overlay.underline;
    }
    if overlay.strikethrough.is_some() {
        base.strikethrough = overlay.strikethrough;
    }
    if overlay.fade_out.is_some() {
        base.fade_out = overlay.fade_out;
    }
}

fn source_display_offset(offsets: &[(usize, usize)], source: usize) -> Option<usize> {
    offsets
        .binary_search_by_key(&source, |(source, _)| *source)
        .ok()
        .and_then(|index| offsets.get(index).map(|(_, display)| *display))
}

fn display_source_offset(offsets: &[(usize, usize)], display: usize) -> usize {
    match offsets.binary_search_by_key(&display, |(_, display)| *display) {
        Ok(index) => offsets.get(index).map_or(0, |(source, _)| *source),
        Err(index) => offsets
            .get(index.saturating_sub(1))
            .map_or(0, |(source, _)| *source),
    }
}

fn clamp_char_boundary(text: &str, requested: usize) -> usize {
    let mut offset = requested.min(text.len());
    while !text.is_char_boundary(offset) {
        offset = offset.saturating_sub(1);
    }
    offset
}

fn intersect_local(range: &Range<usize>, segment: &Range<usize>) -> Option<Range<usize>> {
    let start = range.start.max(segment.start);
    let end = range.end.min(segment.end);
    (start < end).then(|| start - segment.start..end - segment.start)
}

fn source_location_payload(
    document: &PreparedDocument,
    offset: usize,
    side: Option<DocumentSide>,
) -> Option<UiValue> {
    let offset = clamp_char_boundary(document.text(), offset);
    let (line, record) = document
        .lines()
        .iter()
        .enumerate()
        .find(|(_, line)| offset >= line.range.start && offset <= line.range.end)
        .or_else(|| document.lines().iter().enumerate().next_back())?;
    let column = document.text()[record.range.start..offset.min(record.range.end)]
        .chars()
        .count()
        + 1;
    let mut payload = BTreeMap::from([
        (
            "line".to_owned(),
            UiValue::Integer(i64::try_from(line + 1).unwrap_or(i64::MAX)),
        ),
        (
            "column".to_owned(),
            UiValue::Integer(i64::try_from(column).unwrap_or(i64::MAX)),
        ),
    ]);
    if let Some(side) = side {
        payload.insert("side".to_owned(), UiValue::String(side.as_str().to_owned()));
    }
    Some(UiValue::Map(payload))
}

fn location_schema(include_side: bool) -> ValueSchema {
    let mut fields = BTreeMap::from([
        (
            "line".to_owned(),
            ObjectField::required(ValueSchema::integer()),
        ),
        (
            "column".to_owned(),
            ObjectField::required(ValueSchema::integer()),
        ),
    ]);
    if include_side {
        fields.insert(
            "side".to_owned(),
            ObjectField::required(ValueSchema::enumeration(["left", "right"])),
        );
    }
    ValueSchema::object(fields)
}

#[must_use]
/// Return the public native code-viewer primitive contract.
///
/// # Panics
///
/// Panics only if its compile-time primitive ID becomes invalid.
pub fn code_viewer_primitive_descriptor() -> PrimitiveDescriptor {
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.code_viewer").expect("static primitive ID"),
        export: "CodeViewPrimitive".to_owned(),
        props: BTreeMap::from([
            (
                "source".to_owned(),
                ObjectField::required(ValueSchema::one_of([
                    ValueSchema::string(),
                    ValueSchema::Document,
                ])),
            ),
            (
                "label".to_owned(),
                ObjectField::required(ValueSchema::string()),
            ),
            (
                "file_name".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
            ),
            (
                "language".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
            ),
            (
                "show_line_numbers".to_owned(),
                ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(true)),
            ),
            (
                "wrap".to_owned(),
                ObjectField::optional(ValueSchema::enumeration(["none", "viewport", "column"]))
                    .with_default(UiValue::String("none".to_owned())),
            ),
            (
                "wrap_column".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(Some(20), Some(500)))
                    .with_default(UiValue::Integer(100)),
            ),
            (
                "tab_size".to_owned(),
                ObjectField::optional(ValueSchema::bounded_integer(Some(1), Some(16)))
                    .with_default(UiValue::Integer(4)),
            ),
            (
                "gutter_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "line_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "text_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "loading_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "error_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "search_style".to_owned(),
                ObjectField::required(ValueSchema::Style),
            ),
            (
                "on_location_activate".to_owned(),
                ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
            ),
        ]),
        events: BTreeMap::from([(
            "location_activate".to_owned(),
            EventSchema {
                payload: location_schema(false),
            },
        )]),
        state: crate::ComponentStateSchema::default(),
        lifecycle: true,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DiffViewerConfig {
    left: DocumentDescriptor,
    right: DocumentDescriptor,
    mode: DiffViewMode,
    whitespace: DiffWhitespace,
    context_lines: Option<usize>,
    show_line_numbers: bool,
    wrap: DocumentWrap,
    tab_size: usize,
    header_style: Style,
    gutter_style: Style,
    line_style: Style,
    text_style: Style,
    fold_style: Style,
    loading_style: Style,
    error_style: Style,
    search_style: Style,
    status_style: Style,
}

impl DiffViewerConfig {
    fn preparation_changed(&self, next: &Self) -> bool {
        self.left.source != next.left.source
            || self.left.file_name != next.left.file_name
            || self.left.language != next.left.language
            || self.right.source != next.right.source
            || self.right.file_name != next.right.file_name
            || self.right.language != next.right.language
            || self.whitespace != next.whitespace
            || self.context_lines != next.context_lines
    }
}

#[derive(Clone, Debug)]
struct DiffTextSegment {
    line: usize,
    range: Range<usize>,
    first: bool,
    column_offset: usize,
}

#[derive(Clone, Debug)]
enum DiffRenderRow {
    Content {
        aligned: usize,
        left: Option<DiffTextSegment>,
        right: Option<DiffTextSegment>,
        kind: DiffRowKind,
        hunk: Option<usize>,
    },
    Unified {
        aligned: usize,
        side: DocumentSide,
        segment: DiffTextSegment,
        kind: DiffRowKind,
        hunk: Option<usize>,
    },
    Fold {
        id: usize,
        hidden_rows: usize,
    },
}

impl DiffRenderRow {
    const fn hunk(&self) -> Option<usize> {
        match self {
            Self::Content { hunk, .. } | Self::Unified { hunk, .. } => *hunk,
            Self::Fold { .. } => None,
        }
    }

    const fn aligned(&self) -> Option<usize> {
        match self {
            Self::Content { aligned, .. } | Self::Unified { aligned, .. } => Some(*aligned),
            Self::Fold { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
struct DiffSearchMatch {
    side: DocumentSide,
    range: Range<usize>,
}

#[derive(Clone, Debug, Default)]
struct DiffSearchState {
    open: bool,
    query: String,
    options: SearchOptions,
    matches: Vec<DiffSearchMatch>,
    current: Option<usize>,
    error: Option<String>,
}

struct DiffViewerEntity {
    config: DiffViewerConfig,
    events: PrimitiveEventEmitter,
    theme: PrimitiveTheme,
    syntaxes: SyntaxRegistry,
    document_runtime: DocumentRuntimeConfig,
    focus: FocusHandle,
    scroll: gpui::UniformListScrollHandle,
    left_horizontal: ScrollHandle,
    right_horizontal: ScrollHandle,
    prepared: Option<Arc<PreparedDiff>>,
    display: Arc<[DiffRenderRow]>,
    display_columns: Option<usize>,
    display_job: u64,
    display_task: Option<Task<()>>,
    pending_display_columns: Option<PendingDisplayColumns>,
    viewport: Option<Bounds<Pixels>>,
    expanded_folds: BTreeSet<usize>,
    current_hunk: Option<usize>,
    selection: DocumentSelection,
    search: DiffSearchState,
    search_input: Entity<TextInputEntity>,
    search_job: u64,
    search_task: Option<Task<()>>,
    job: u64,
    loading: bool,
    error: Option<String>,
    prepare_task: Option<Task<()>>,
    patch_task: Option<Task<()>>,
}

impl DiffViewerEntity {
    fn new(
        config: DiffViewerConfig,
        events: PrimitiveEventEmitter,
        theme: PrimitiveTheme,
        syntaxes: SyntaxRegistry,
        document_runtime: DocumentRuntimeConfig,
        search_typography: NativeTypography,
        cx: &mut Context<Self>,
    ) -> Self {
        let weak = cx.entity().downgrade();
        let callbacks = TextInputCallbacks {
            change: Some(Rc::new({
                let weak = weak.clone();
                move |value, _, cx| {
                    let _ = weak.update(cx, |viewer, cx| {
                        viewer.search.query = value;
                        viewer.start_search(cx);
                        cx.notify();
                    });
                }
            })),
            submit: Some(Rc::new(move |_, _, cx| {
                let _ = weak.update(cx, |viewer, cx| viewer.advance_match(false, cx));
            })),
            ..TextInputCallbacks::default()
        };
        let search_input = cx.new(|cx| {
            TextInputEntity::new(
                TextInputConfig {
                    value: String::new(),
                    placeholder: "Find on both sides".to_owned(),
                    disabled: false,
                    read_only: false,
                    autofocus: false,
                    selection_color: theme_color(&theme, "selection", 0x3347_6fff),
                    caret_color: theme_color(&theme, "accent", 0x7aa2_f7ff),
                    typography: search_typography,
                },
                callbacks,
                cx,
            )
        });
        Self {
            config,
            events,
            theme,
            syntaxes,
            document_runtime,
            focus: cx.focus_handle(),
            scroll: gpui::UniformListScrollHandle::new(),
            left_horizontal: ScrollHandle::new(),
            right_horizontal: ScrollHandle::new(),
            prepared: None,
            display: Arc::from([]),
            display_columns: None,
            display_job: 0,
            display_task: None,
            pending_display_columns: None,
            viewport: None,
            expanded_folds: BTreeSet::new(),
            current_hunk: None,
            selection: DocumentSelection::default(),
            search: DiffSearchState::default(),
            search_input,
            search_job: 0,
            search_task: None,
            job: 0,
            loading: false,
            error: None,
            prepare_task: None,
            patch_task: None,
        }
    }

    fn update(
        &mut self,
        config: DiffViewerConfig,
        events: PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        search_typography: NativeTypography,
        cx: &mut Context<Self>,
    ) {
        let needs_prepare = self.config.preparation_changed(&config);
        let presentation_changed = self.config.mode != config.mode
            || self.config.wrap != config.wrap
            || self.config.tab_size != config.tab_size;
        if self.config.left.label != config.left.label
            || self.config.right.label != config.right.label
        {
            self.patch_task = None;
        }
        self.config = config;
        self.events = events;
        self.theme = theme.clone();
        let callbacks = self.search_input.read(cx).callbacks();
        self.search_input.update(cx, |input, cx| {
            input.update_props(
                &TextInputConfig {
                    value: self.search.query.clone(),
                    placeholder: "Find on both sides".to_owned(),
                    disabled: false,
                    read_only: false,
                    autofocus: false,
                    selection_color: theme_color(theme, "selection", 0x3347_6fff),
                    caret_color: theme_color(theme, "accent", 0x7aa2_f7ff),
                    typography: search_typography,
                },
                callbacks,
                cx,
            );
        });
        if needs_prepare || (self.loading && presentation_changed) {
            self.start_prepare(cx);
        } else if presentation_changed {
            self.start_display(cx, true, None);
        } else {
            cx.notify();
        }
    }

    fn start_prepare(&mut self, cx: &mut Context<Self>) {
        self.job = self.job.saturating_add(1);
        self.display_job = self.display_job.saturating_add(1);
        self.display_task = None;
        self.pending_display_columns = None;
        let job = self.job;
        let left = self.config.left.clone();
        let right = self.config.right.clone();
        let whitespace = self.config.whitespace;
        let context = self.config.context_lines;
        let syntaxes = self.syntaxes.clone();
        let limits = self.document_runtime.limits();
        let columns = self.wrap_columns();
        let mode = self.config.mode;
        let tab_size = self.config.tab_size;
        let expanded_folds = self.expanded_folds.clone();
        self.loading = true;
        self.error = None;
        self.prepare_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    prepare_diff_with_limits(&left, &right, whitespace, context, &syntaxes, limits)
                        .map(|diff| {
                            let display =
                                build_diff_display(&diff, mode, columns, tab_size, &expanded_folds);
                            (diff, display)
                        })
                })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.job != job {
                    return;
                }
                viewer.loading = false;
                match result {
                    Ok((diff, display)) => {
                        let diff = Arc::new(diff);
                        viewer.display_job = viewer.display_job.saturating_add(1);
                        viewer.display_task = None;
                        viewer.pending_display_columns = None;
                        let old_hunk = viewer.current_hunk;
                        if let Some(previous) = viewer.prepared.as_ref() {
                            let same_pair = previous.left.identity() == diff.left.identity()
                                && previous.right.identity() == diff.right.identity();
                            if same_pair {
                                viewer.selection.rebase(
                                    DocumentSide::Left,
                                    &previous.left.text_arc(),
                                    diff.left.text_arc(),
                                );
                                viewer.selection.rebase(
                                    DocumentSide::Right,
                                    &previous.right.text_arc(),
                                    diff.right.text_arc(),
                                );
                            } else {
                                viewer.selection.clear();
                                viewer.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
                            }
                        }
                        viewer.error = None;
                        viewer.prepared = Some(diff);
                        let valid_folds = viewer
                            .prepared
                            .as_ref()
                            .into_iter()
                            .flat_map(|diff| diff.collapsed.iter())
                            .filter_map(|row| match row {
                                DiffDisplayRow::Fold { id, .. } => Some(*id),
                                DiffDisplayRow::Content(_) => None,
                            })
                            .collect::<BTreeSet<_>>();
                        viewer.expanded_folds.retain(|id| valid_folds.contains(id));
                        viewer.display = display;
                        viewer.display_columns = columns;
                        viewer.start_search(cx);
                        viewer.current_hunk = old_hunk.filter(|hunk| {
                            viewer
                                .prepared
                                .as_ref()
                                .is_some_and(|diff| *hunk < diff.hunk_count)
                        });
                    }
                    Err(error) => viewer.error = Some(error.to_string()),
                }
                cx.notify();
            });
        }));
    }

    fn wrap_columns(&self) -> Option<usize> {
        match self.config.wrap {
            DocumentWrap::None => None,
            DocumentWrap::Column(columns) => Some(columns.max(1)),
            DocumentWrap::Viewport => self.viewport.map(|bounds| {
                let divisor = if self.config.mode == DiffViewMode::Split {
                    2.0
                } else {
                    1.0
                };
                let pane = f64::from(bounds.size.width) / divisor;
                let (text_size, _) = document_text_metrics(&self.config.text_style, &self.theme);
                let character_width = (f64::from(text_size) * 0.604).max(1.0);
                ((pane - 74.0).max(20.0) / character_width)
                    .floor()
                    .to_string()
                    .parse()
                    .unwrap_or(80)
                    .max(1)
            }),
        }
    }

    fn start_display(
        &mut self,
        cx: &mut Context<Self>,
        force: bool,
        reveal_aligned: Option<usize>,
    ) {
        let Some(diff) = self.prepared.clone() else {
            return;
        };
        let columns = self.wrap_columns();
        if !force && self.display_columns == columns && !self.display.is_empty() {
            return;
        }
        if !force && self.pending_display_columns == Some(columns.into()) {
            return;
        }
        self.display_job = self.display_job.saturating_add(1);
        let job = self.display_job;
        let mode = self.config.mode;
        let tab_size = self.config.tab_size;
        let expanded_folds = self.expanded_folds.clone();
        self.pending_display_columns = Some(columns.into());
        self.display_task = Some(cx.spawn(async move |this, cx| {
            let display = cx
                .background_spawn(async move {
                    build_diff_display(&diff, mode, columns, tab_size, &expanded_folds)
                })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.display_job == job {
                    viewer.display = display;
                    viewer.display_columns = columns;
                    viewer.pending_display_columns = None;
                    if let Some(index) = reveal_aligned.and_then(|aligned| {
                        viewer
                            .display
                            .iter()
                            .position(|row| row.aligned() == Some(aligned))
                    }) {
                        viewer.scroll.scroll_to_item(index, ScrollStrategy::Center);
                    }
                    cx.notify();
                }
            });
        }));
    }

    fn start_search(&mut self, cx: &mut Context<Self>) {
        self.search_job = self.search_job.saturating_add(1);
        let job = self.search_job;
        self.search.matches.clear();
        self.search.error = None;
        self.search.current = None;
        let Some(diff) = self.prepared.as_ref() else {
            return;
        };
        if self.search.query.is_empty() {
            self.search_task = None;
            return;
        }
        let left = diff.left.text_arc();
        let right = diff.right.text_arc();
        let query = self.search.query.clone();
        let options = self.search.options;
        self.search_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let left = search_ranges(&left, &query, options, 50_000)?;
                    let remaining = 100_000usize.saturating_sub(left.len());
                    let right = search_ranges(&right, &query, options, remaining)?;
                    Ok::<_, String>(
                        left.into_iter()
                            .map(|range| DiffSearchMatch {
                                side: DocumentSide::Left,
                                range,
                            })
                            .chain(right.into_iter().map(|range| DiffSearchMatch {
                                side: DocumentSide::Right,
                                range,
                            }))
                            .collect::<Vec<_>>(),
                    )
                })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.search_job != job {
                    return;
                }
                match result {
                    Ok(matches) => {
                        viewer.search.matches = matches;
                        viewer.search.current = (!viewer.search.matches.is_empty()).then_some(0);
                    }
                    Err(error) => viewer.search.error = Some(error),
                }
                cx.notify();
            });
        }));
    }

    fn reveal_match(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(target) = self.search.matches.get(index).cloned() else {
            return;
        };
        let mut expanded = false;
        let aligned = self
            .prepared
            .as_ref()
            .and_then(|diff| diff_aligned_for_offset(diff, target.side, target.range.start));
        if let Some(diff) = self.prepared.as_ref() {
            for entry in diff.collapsed.iter() {
                let DiffDisplayRow::Fold { id, full_range, .. } = entry else {
                    continue;
                };
                let contains = full_range.clone().any(|row| {
                    let row = &diff.rows[row];
                    let line = match target.side {
                        DocumentSide::Left => row.left,
                        DocumentSide::Right => row.right,
                        DocumentSide::Single => None,
                    };
                    line.is_some_and(|line| {
                        let range = &match target.side {
                            DocumentSide::Left => &diff.left,
                            DocumentSide::Right => &diff.right,
                            DocumentSide::Single => return false,
                        }
                        .lines()[line]
                            .range;
                        target.range.start >= range.start && target.range.start <= range.end
                    })
                });
                if contains {
                    expanded |= self.expanded_folds.insert(*id);
                }
            }
        }
        if expanded {
            self.start_display(cx, true, aligned);
            return;
        }
        if let Some(display_index) = self
            .display
            .iter()
            .position(|row| diff_render_row_contains(row, target.side, target.range.start))
            .or_else(|| {
                aligned.and_then(|aligned| {
                    self.display
                        .iter()
                        .position(|row| row.aligned() == Some(aligned))
                })
            })
        {
            self.scroll
                .scroll_to_item(display_index, ScrollStrategy::Center);
        }
    }

    fn advance_match(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.loading || self.search.matches.is_empty() {
            return;
        }
        let current = self.search.current.unwrap_or(0);
        let next = if backwards {
            (current + self.search.matches.len() - 1) % self.search.matches.len()
        } else {
            (current + 1) % self.search.matches.len()
        };
        self.search.current = Some(next);
        self.reveal_match(next, cx);
        cx.notify();
    }

    fn advance_hunk(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let Some(diff) = self.prepared.as_ref() else {
            return;
        };
        if diff.hunk_count == 0 {
            return;
        }
        let current = self
            .current_hunk
            .unwrap_or(if backwards { 0 } else { diff.hunk_count - 1 });
        let next = if backwards {
            (current + diff.hunk_count - 1) % diff.hunk_count
        } else {
            (current + 1) % diff.hunk_count
        };
        self.current_hunk = Some(next);
        if let Some(index) = self.display.iter().position(|row| row.hunk() == Some(next)) {
            self.scroll.scroll_to_item(index, ScrollStrategy::Center);
        }
        cx.notify();
    }

    fn find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
        self.search.open = true;
        self.search_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn find_next(&mut self, _: &FindNext, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_match(false, cx);
    }

    fn find_previous(&mut self, _: &FindPrevious, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_match(true, cx);
    }

    fn close_find(&mut self, _: &CloseFind, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.open {
            self.search.open = false;
            self.focus.focus(window);
            cx.notify();
        }
    }

    fn copy_selection(
        &mut self,
        _: &CopyDocumentSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        if let Some(text) = self.selection.selected_text() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            cx.stop_propagation();
        }
    }

    fn next_change(&mut self, _: &NextChange, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_hunk(false, cx);
    }

    fn previous_change(&mut self, _: &PreviousChange, _: &mut Window, cx: &mut Context<Self>) {
        self.advance_hunk(true, cx);
    }

    fn expand_all(&mut self, _: &ExpandAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let Some(diff) = self.prepared.as_ref() else {
            return;
        };
        self.expanded_folds = diff
            .collapsed
            .iter()
            .filter_map(|row| match row {
                DiffDisplayRow::Fold { id, .. } => Some(*id),
                DiffDisplayRow::Content(_) => None,
            })
            .collect();
        self.start_display(cx, true, None);
    }

    fn collapse_all(&mut self, _: &CollapseAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.expanded_folds.clear();
        self.start_display(cx, true, None);
    }

    fn copy_unified_diff(&mut self, _: &CopyUnifiedDiff, _: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let Some(diff) = self.prepared.as_ref() else {
            return;
        };
        let left = diff.left.text_arc();
        let right = diff.right.text_arc();
        let left_label = self.config.left.label.clone();
        let right_label = self.config.right.label.clone();
        let context = self.config.context_lines.unwrap_or(diff.rows.len());
        let timeout = self.document_runtime.limits().diff_timeout;
        let job = self.job;
        cx.stop_propagation();
        self.patch_task = Some(cx.spawn(async move |this, cx| {
            let patch = cx
                .background_spawn(async move {
                    let mut config = TextDiff::configure();
                    config
                        .algorithm(similar::Algorithm::Patience)
                        .timeout(timeout);
                    config
                        .diff_lines(left.as_ref(), right.as_ref())
                        .unified_diff()
                        .header(&left_label, &right_label)
                        .context_radius(context)
                        .to_string()
                })
                .await;
            let _ = this.update(cx, |viewer, cx| {
                if viewer.job == job && !viewer.loading {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(patch));
                }
            });
        }));
    }

    fn activate_location(
        &mut self,
        _: &ActivateDocumentLocation,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        let Some(diff) = self.prepared.as_ref() else {
            return;
        };
        let Some((side, source, offset)) = self.selection.position() else {
            return;
        };
        let document = match side {
            DocumentSide::Left => &diff.left,
            DocumentSide::Right => &diff.right,
            DocumentSide::Single => return,
        };
        if !Arc::ptr_eq(&source, &document.text_arc()) {
            return;
        }
        if let Some(payload) = source_location_payload(document, offset, Some(side)) {
            let events = self.events.clone();
            window.defer(cx, move |window, cx| {
                let _ = events.emit("location_activate", payload, window, cx);
            });
        }
    }

    fn reveal_line(&mut self, action: &RevealDocumentLine, _: &mut Window, cx: &mut Context<Self>) {
        if action.line == 0 {
            return;
        }
        let side = match action.side {
            Some(DiffSide::Left) => DocumentSide::Left,
            Some(DiffSide::Right) => DocumentSide::Right,
            _ => return,
        };
        let line = action.line - 1;
        let mut expanded = false;
        let mut aligned = None;
        if let Some(diff) = self.prepared.as_ref() {
            aligned = diff_aligned_for_line(diff, side, line);
            for entry in diff.collapsed.iter() {
                let DiffDisplayRow::Fold { id, full_range, .. } = entry else {
                    continue;
                };
                if full_range.clone().any(|row| {
                    let row = &diff.rows[row];
                    (match side {
                        DocumentSide::Left => row.left,
                        DocumentSide::Right => row.right,
                        DocumentSide::Single => None,
                    }) == Some(line)
                }) {
                    expanded |= self.expanded_folds.insert(*id);
                }
            }
        }
        if expanded {
            self.start_display(cx, true, aligned);
            return;
        }
        if let Some(index) = aligned.and_then(|aligned| {
            self.display
                .iter()
                .position(|row| row.aligned() == Some(aligned))
        }) {
            self.scroll.scroll_to_item(index, ScrollStrategy::Center);
            cx.notify();
        }
    }

    fn toggle_search_option(&mut self, option: &'static str, cx: &mut Context<Self>) {
        match option {
            "case" => self.search.options.case_sensitive = !self.search.options.case_sensitive,
            "word" => self.search.options.whole_word = !self.search.options.whole_word,
            "regex" => self.search.options.regex = !self.search.options.regex,
            _ => return,
        }
        self.start_search(cx);
        cx.notify();
    }

    fn expand_fold(&mut self, id: usize, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.expanded_folds.insert(id);
        self.start_display(cx, true, None);
    }

    fn search_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let weak = cx.entity().downgrade();
        let option = |label: &'static str, name: &'static str, active: bool| {
            let weak = weak.clone();
            div()
                .id(SharedString::from(format!("diff-search-option-{name}")))
                .w(px(28.0))
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .bg(if active {
                    rgba(theme_color(&self.theme, "selection", 0x3347_6fff).as_rgba_hex())
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|style| {
                    style.bg(rgba(
                        theme_color(&self.theme, "surface_hover", 0x252a_34ff).as_rgba_hex(),
                    ))
                })
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    let _ = weak.update(cx, |viewer, cx| viewer.toggle_search_option(name, cx));
                })
                .child(label)
        };
        let status = self.search.error.as_ref().map_or_else(
            || {
                if self.search.matches.is_empty() {
                    "No matches".to_owned()
                } else {
                    format!(
                        "{} / {}",
                        self.search.current.unwrap_or(0) + 1,
                        self.search.matches.len()
                    )
                }
            },
            |error| format!("Invalid regex: {error}"),
        );
        let bar = div()
            .absolute()
            .top(px(36.0))
            .right(px(8.0))
            .w(px(430.0))
            .h(px(38.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(6.0))
            .bg(rgba(
                theme_color(&self.theme, "surface_raised", 0x1a1d_24ff).as_rgba_hex(),
            ))
            .border_1()
            .border_color(rgba(
                theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
            ))
            .child(
                div()
                    .h(px(26.0))
                    .flex_1()
                    .min_w(px(0.0))
                    .px(px(6.0))
                    .border_1()
                    .border_color(rgba(
                        theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
                    ))
                    .child(self.search_input.clone()),
            )
            .child(option("Aa", "case", self.search.options.case_sensitive))
            .child(option("W", "word", self.search.options.whole_word))
            .child(option(".*", "regex", self.search.options.regex))
            .child(div().w(px(62.0)).text_size(px(10.0)).child(status));
        crate::renderer::apply_style_override(
            bar,
            &self.config.search_style,
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }

    fn header(&self) -> AnyElement {
        let base = div()
            .h(px(30.0))
            .flex_none()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgba(
                theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
            ))
            .bg(rgba(
                theme_color(&self.theme, "surface_raised", 0x1a1d_24ff).as_rgba_hex(),
            ))
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD);
        let header = if self.config.mode == DiffViewMode::Split {
            base.child(
                div()
                    .w(relative(0.5))
                    .px(px(10.0))
                    .child(self.config.left.label.clone()),
            )
            .child(
                div()
                    .w(relative(0.5))
                    .px(px(10.0))
                    .border_l_1()
                    .border_color(rgba(
                        theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
                    ))
                    .child(self.config.right.label.clone()),
            )
        } else {
            base.px(px(10.0)).child(format!(
                "{}  ↔  {}",
                self.config.left.label, self.config.right.label
            ))
        };
        crate::renderer::apply_style_override(
            header,
            &self.config.header_style,
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }

    fn content(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(error) = self.error.as_ref() {
            return crate::renderer::apply_style_override(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(
                        theme_color(&self.theme, "danger", 0xf776_8eff).as_rgba_hex(),
                    ))
                    .child(format!("DiffViewer error: {error}")),
                &self.config.error_style,
                &self.theme,
                self.theme.direction(),
            )
            .into_any_element();
        }
        let Some(diff) = self.prepared.clone() else {
            return crate::renderer::apply_style_override(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgba(
                        theme_color(&self.theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
                    ))
                    .child("Calculating comparison…"),
                &self.config.loading_style,
                &self.theme,
                self.theme.direction(),
            )
            .into_any_element();
        };
        if diff.hunk_count == 0 {
            return div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgba(
                    theme_color(&self.theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
                ))
                .child("No differences")
                .into_any_element();
        }
        let display = Arc::clone(&self.display);
        let selection = self.selection.clone();
        let matches = Arc::<[DiffSearchMatch]>::from(self.search.matches.clone());
        let current_match = self.search.current;
        let theme = self.theme.clone();
        let config = self.config.clone();
        let events = self.events.clone();
        let focus = self.focus.clone();
        let weak = cx.entity().downgrade();
        let left_horizontal = self.left_horizontal.clone();
        let right_horizontal = self.right_horizontal.clone();
        let disabled = self.loading;
        let list = uniform_list(
            "gpui-rhai-diff-viewer-lines",
            display.len(),
            move |range, _, _| {
                range
                    .filter_map(|index| {
                        let row = display.get(index)?;
                        Some(diff_line_element(
                            &diff,
                            row,
                            &config,
                            &theme,
                            &selection,
                            &matches,
                            current_match,
                            &events,
                            &focus,
                            &weak,
                            &left_horizontal,
                            &right_horizontal,
                            disabled,
                        ))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(self.scroll.clone())
        .when(
            matches!(self.config.wrap, DocumentWrap::None)
                && self.config.mode == DiffViewMode::Unified,
            |list| {
                list.with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
            },
        )
        .flex_1()
        .min_h(px(0.0))
        .opacity(if self.loading { 0.62 } else { 1.0 });
        list.into_any_element()
    }
}

impl Render for DiffViewerEntity {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.prepared.as_ref().map_or_else(
            || "Preparing".to_owned(),
            |diff| {
                if diff.hunk_count == 0 {
                    "No differences".to_owned()
                } else {
                    format!(
                        "Change {} of {}",
                        self.current_hunk.unwrap_or(0) + 1,
                        diff.hunk_count
                    )
                }
            },
        );
        let syntax_warning = self.prepared.as_ref().and_then(|diff| {
            diff.left
                .highlight_error()
                .or_else(|| diff.right.highlight_error())
        });
        let status_bar = crate::renderer::apply_style_override(
            div()
                .h(px(22.0))
                .flex_none()
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(rgba(
                    theme_color(&self.theme, "border", 0x353b_48ff).as_rgba_hex(),
                ))
                .bg(rgba(
                    theme_color(&self.theme, "surface_raised", 0x1a1d_24ff).as_rgba_hex(),
                ))
                .text_size(px(10.0))
                .text_color(rgba(
                    theme_color(&self.theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
                ))
                .child(if let Some(error) = syntax_warning {
                    format!("{status} · syntax fallback: {error}")
                } else {
                    status
                })
                .child(if self.loading {
                    "Updating comparison…"
                } else {
                    "⌥↑ / ⌥↓"
                }),
            &self.config.status_style,
            &self.theme,
            self.theme.direction(),
        );
        let mut root = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .key_context(DIFF_CONTEXT)
            .track_focus(&self.focus)
            .tab_stop(true)
            .on_action(cx.listener(Self::find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_previous))
            .on_action(cx.listener(Self::close_find))
            .on_action(cx.listener(Self::copy_selection))
            .on_action(cx.listener(Self::next_change))
            .on_action(cx.listener(Self::previous_change))
            .on_action(cx.listener(Self::expand_all))
            .on_action(cx.listener(Self::collapse_all))
            .on_action(cx.listener(Self::copy_unified_diff))
            .on_action(cx.listener(Self::reveal_line))
            .on_action(cx.listener(Self::activate_location))
            .child(self.header())
            .child(self.content(cx))
            .child(status_bar)
            .child(DiffBoundsRecorder {
                viewer: cx.entity(),
            });
        if self.search.open {
            root = root.child(self.search_bar(cx));
        }
        root
    }
}

struct DiffBoundsRecorder {
    viewer: Entity<DiffViewerEntity>,
}

impl Element for DiffBoundsRecorder {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut child = div().absolute().inset_0().into_any_element();
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
        self.viewer.update(cx, |viewer, cx| {
            if viewer.viewport != Some(bounds) {
                viewer.viewport = Some(bounds);
            }
            if matches!(viewer.config.wrap, DocumentWrap::Viewport)
                && viewer.wrap_columns() != viewer.display_columns
            {
                viewer.start_display(cx, false, None);
            }
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for DiffBoundsRecorder {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn append_diff_content(
    output: &mut Vec<DiffRenderRow>,
    diff: &PreparedDiff,
    aligned: usize,
    mode: DiffViewMode,
    columns: Option<usize>,
    tab_size: usize,
) {
    let row = &diff.rows[aligned];
    let left = row
        .left
        .map(|line| diff_segments(&diff.left, line, columns, tab_size));
    let right = row
        .right
        .map(|line| diff_segments(&diff.right, line, columns, tab_size));
    if mode == DiffViewMode::Split {
        let count = left
            .as_ref()
            .map_or(0, Vec::len)
            .max(right.as_ref().map_or(0, Vec::len))
            .max(1);
        for index in 0..count {
            output.push(DiffRenderRow::Content {
                aligned,
                left: left
                    .as_ref()
                    .and_then(|segments| segments.get(index))
                    .cloned(),
                right: right
                    .as_ref()
                    .and_then(|segments| segments.get(index))
                    .cloned(),
                kind: row.kind,
                hunk: row.hunk,
            });
        }
    } else {
        match row.kind {
            DiffRowKind::Equal => {
                if let Some(segments) = right.or(left) {
                    output.extend(segments.into_iter().map(|segment| DiffRenderRow::Unified {
                        aligned,
                        side: DocumentSide::Right,
                        segment,
                        kind: row.kind,
                        hunk: row.hunk,
                    }));
                }
            }
            DiffRowKind::LeftOnly => {
                if let Some(segments) = left {
                    output.extend(segments.into_iter().map(|segment| DiffRenderRow::Unified {
                        aligned,
                        side: DocumentSide::Left,
                        segment,
                        kind: row.kind,
                        hunk: row.hunk,
                    }));
                }
            }
            DiffRowKind::RightOnly => {
                if let Some(segments) = right {
                    output.extend(segments.into_iter().map(|segment| DiffRenderRow::Unified {
                        aligned,
                        side: DocumentSide::Right,
                        segment,
                        kind: row.kind,
                        hunk: row.hunk,
                    }));
                }
            }
            DiffRowKind::Modified => {
                if let Some(segments) = left {
                    output.extend(segments.into_iter().map(|segment| DiffRenderRow::Unified {
                        aligned,
                        side: DocumentSide::Left,
                        segment,
                        kind: row.kind,
                        hunk: row.hunk,
                    }));
                }
                if let Some(segments) = right {
                    output.extend(segments.into_iter().map(|segment| DiffRenderRow::Unified {
                        aligned,
                        side: DocumentSide::Right,
                        segment,
                        kind: row.kind,
                        hunk: row.hunk,
                    }));
                }
            }
        }
    }
}

fn build_diff_display(
    diff: &PreparedDiff,
    mode: DiffViewMode,
    columns: Option<usize>,
    tab_size: usize,
    expanded_folds: &BTreeSet<usize>,
) -> Arc<[DiffRenderRow]> {
    let mut rows = Vec::new();
    for entry in diff.collapsed.iter() {
        match entry {
            DiffDisplayRow::Content(index) => {
                append_diff_content(&mut rows, diff, *index, mode, columns, tab_size);
            }
            DiffDisplayRow::Fold { id, full_range, .. } if expanded_folds.contains(id) => {
                for index in full_range.clone() {
                    append_diff_content(&mut rows, diff, index, mode, columns, tab_size);
                }
            }
            DiffDisplayRow::Fold {
                id, hidden_rows, ..
            } => rows.push(DiffRenderRow::Fold {
                id: *id,
                hidden_rows: *hidden_rows,
            }),
        }
    }
    rows.into()
}

fn diff_segments(
    document: &PreparedDocument,
    line: usize,
    columns: Option<usize>,
    tab_size: usize,
) -> Vec<DiffTextSegment> {
    let record = &document.lines()[line];
    let text = document.line_text(line).unwrap_or_default();
    wrap_source_line(&record.range, text, columns, tab_size)
        .into_iter()
        .map(|(range, first, column_offset)| DiffTextSegment {
            line,
            range,
            first,
            column_offset,
        })
        .collect()
}

fn diff_render_row_contains(row: &DiffRenderRow, side: DocumentSide, offset: usize) -> bool {
    match row {
        DiffRenderRow::Content { left, right, .. } => match side {
            DocumentSide::Left => left.as_ref(),
            DocumentSide::Right => right.as_ref(),
            DocumentSide::Single => None,
        }
        .is_some_and(|segment| offset >= segment.range.start && offset <= segment.range.end),
        DiffRenderRow::Unified {
            side: row_side,
            segment,
            ..
        } => *row_side == side && offset >= segment.range.start && offset <= segment.range.end,
        DiffRenderRow::Fold { .. } => false,
    }
}

fn diff_aligned_for_offset(
    diff: &PreparedDiff,
    side: DocumentSide,
    offset: usize,
) -> Option<usize> {
    let document = match side {
        DocumentSide::Left => &diff.left,
        DocumentSide::Right => &diff.right,
        DocumentSide::Single => return None,
    };
    let line = document
        .lines()
        .partition_point(|line| line.range.start <= offset)
        .saturating_sub(1);
    diff_aligned_for_line(diff, side, line)
}

fn diff_aligned_for_line(diff: &PreparedDiff, side: DocumentSide, line: usize) -> Option<usize> {
    diff.rows.iter().position(|row| {
        (match side {
            DocumentSide::Left => row.left,
            DocumentSide::Right => row.right,
            DocumentSide::Single => None,
        }) == Some(line)
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn diff_line_element(
    diff: &Arc<PreparedDiff>,
    row: &DiffRenderRow,
    config: &DiffViewerConfig,
    theme: &PrimitiveTheme,
    selection: &DocumentSelection,
    matches: &[DiffSearchMatch],
    current_match: Option<usize>,
    events: &PrimitiveEventEmitter,
    focus: &FocusHandle,
    weak: &gpui::WeakEntity<DiffViewerEntity>,
    left_horizontal: &ScrollHandle,
    right_horizontal: &ScrollHandle,
    disabled: bool,
) -> AnyElement {
    let (_, line_height) = document_text_metrics(&config.text_style, theme);
    let line_height = line_height.max(16.0);
    match row {
        DiffRenderRow::Fold { id, hidden_rows } => {
            let id_value = *id;
            let weak = weak.clone();
            let fold = crate::renderer::apply_style_override(
                div()
                    .w_full()
                    .h(px(line_height))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!disabled, Styled::cursor_pointer)
                    .bg(rgba(
                        theme_color(theme, "diff.fold", 0x252a_34ff).as_rgba_hex(),
                    ))
                    .text_color(rgba(
                        theme_color(theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
                    ))
                    .child(format!("⋯ {hidden_rows} unchanged lines ⋯")),
                &config.fold_style,
                theme,
                theme.direction(),
            );
            if disabled {
                fold.into_any_element()
            } else {
                fold.id(("diff-fold", id_value))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        let _ = weak.update(cx, |viewer, cx| viewer.expand_fold(id_value, cx));
                    })
                    .into_any_element()
            }
        }
        DiffRenderRow::Content {
            aligned,
            left,
            right,
            kind,
            ..
        } => {
            let aligned_row = &diff.rows[*aligned];
            let left_cell = diff_cell(
                &diff.left,
                left.as_ref(),
                DocumentSide::Left,
                *kind,
                &aligned_row.left_inline,
                config,
                theme,
                selection,
                matches,
                current_match,
                events,
                focus,
                Some(left_horizontal),
                disabled,
            );
            let right_cell = diff_cell(
                &diff.right,
                right.as_ref(),
                DocumentSide::Right,
                *kind,
                &aligned_row.right_inline,
                config,
                theme,
                selection,
                matches,
                current_match,
                events,
                focus,
                Some(right_horizontal),
                disabled,
            );
            crate::renderer::apply_style_override(
                div()
                    .w_full()
                    .h(px(line_height))
                    .flex()
                    .flex_row()
                    .child(div().w(relative(0.5)).h_full().child(left_cell))
                    .child(
                        div()
                            .w(relative(0.5))
                            .h_full()
                            .border_l_1()
                            .border_color(rgba(
                                theme_color(theme, "border", 0x353b_48ff).as_rgba_hex(),
                            ))
                            .child(right_cell),
                    ),
                &config.line_style,
                theme,
                theme.direction(),
            )
            .into_any_element()
        }
        DiffRenderRow::Unified {
            aligned,
            side,
            segment,
            kind,
            ..
        } => {
            let aligned_row = &diff.rows[*aligned];
            let inline = if *side == DocumentSide::Left {
                &aligned_row.left_inline
            } else {
                &aligned_row.right_inline
            };
            crate::renderer::apply_style_override(
                div().w_full().h(px(line_height)).child(diff_cell(
                    if *side == DocumentSide::Left {
                        &diff.left
                    } else {
                        &diff.right
                    },
                    Some(segment),
                    *side,
                    *kind,
                    inline,
                    config,
                    theme,
                    selection,
                    matches,
                    current_match,
                    events,
                    focus,
                    None,
                    disabled,
                )),
                &config.line_style,
                theme,
                theme.direction(),
            )
            .into_any_element()
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn diff_cell(
    document: &PreparedDocument,
    segment: Option<&DiffTextSegment>,
    side: DocumentSide,
    kind: DiffRowKind,
    inline_ranges: &[Range<usize>],
    config: &DiffViewerConfig,
    theme: &PrimitiveTheme,
    selection: &DocumentSelection,
    matches: &[DiffSearchMatch],
    current_match: Option<usize>,
    events: &PrimitiveEventEmitter,
    focus: &FocusHandle,
    horizontal: Option<&ScrollHandle>,
    disabled: bool,
) -> AnyElement {
    let background = match (kind, side) {
        (DiffRowKind::LeftOnly | DiffRowKind::Modified, DocumentSide::Left) => {
            theme_color(theme, "diff.left_only", 0x5b2b_36aa)
        }
        (DiffRowKind::RightOnly | DiffRowKind::Modified, DocumentSide::Right) => {
            theme_color(theme, "diff.right_only", 0x244d_3aaa)
        }
        _ => crate::Rgba8::from_rgba_hex(0x0000_0000),
    };
    let Some(segment) = segment else {
        return div()
            .size_full()
            .bg(rgba(background.as_rgba_hex()))
            .into_any_element();
    };
    let line = &document.lines()[segment.line];
    let text = document
        .text()
        .get(segment.range.clone())
        .unwrap_or_default();
    let mut highlights = Vec::new();
    for syntax in &line.syntax {
        let global = line.range.start + syntax.range.start..line.range.start + syntax.range.end;
        if let Some(local) = intersect_local(&global, &segment.range) {
            highlights.push((
                local,
                HighlightStyle {
                    color: Some(
                        rgba(
                            theme_color(theme, syntax.kind.theme_token(), 0xd9de_e8ff)
                                .as_rgba_hex(),
                        )
                        .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    for inline in inline_ranges {
        let global = line.range.start + inline.start..line.range.start + inline.end;
        if let Some(local) = intersect_local(&global, &segment.range) {
            highlights.push((
                local,
                HighlightStyle {
                    background_color: Some(
                        rgba(
                            theme_color(
                                theme,
                                if side == DocumentSide::Left {
                                    "diff.inline_left"
                                } else {
                                    "diff.inline_right"
                                },
                                if side == DocumentSide::Left {
                                    0x8b3f_4faa
                                } else {
                                    0x3570_50aa
                                },
                            )
                            .as_rgba_hex(),
                        )
                        .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    for (index, found) in matches.iter().enumerate() {
        if found.side == side
            && let Some(local) = intersect_local(&found.range, &segment.range)
        {
            highlights.push((
                local,
                HighlightStyle {
                    background_color: Some(
                        rgba(
                            theme_color(
                                theme,
                                if current_match == Some(index) {
                                    "document.search_current"
                                } else {
                                    "document.search_match"
                                },
                                if current_match == Some(index) {
                                    0xe0af_68aa
                                } else {
                                    0x7aa2_f766
                                },
                            )
                            .as_rgba_hex(),
                        )
                        .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    let source = document.text_arc();
    if let Some(range) = selection.range_for(side, &source, segment.range.clone()) {
        highlights.push((
            range,
            HighlightStyle {
                background_color: Some(
                    rgba(theme_color(theme, "selection", 0x3347_6fff).as_rgba_hex()).into(),
                ),
                ..HighlightStyle::default()
            },
        ));
    }
    let expanded = expand_tabs(text, config.tab_size, segment.column_offset);
    let highlights = compose_highlights(
        expanded.text.len(),
        &remap_highlights(highlights, &expanded.offsets),
    );
    let styled = StyledText::new(expanded.text).with_highlights(highlights);
    let text = InteractiveDocumentText {
        id: SharedString::from(format!(
            "diff-{}-{}-{}",
            side.as_str(),
            segment.line,
            segment.range.start
        ))
        .into(),
        text: styled,
        source,
        global_range: segment.range.clone(),
        offset_map: expanded.offsets.into(),
        line: segment.line,
        line_start: line.range.start,
        side,
        selection: selection.clone(),
        focus: focus.clone(),
        events: events.clone(),
        disabled,
    };
    let (text_size, line_height) = document_text_metrics(&config.text_style, theme);
    let digits = document.lines().len().max(1).to_string().len();
    let gutter_width = (usize_f32(digits) * 8.0 + 24.0).max(46.0);
    let marker = match (kind, side) {
        (DiffRowKind::LeftOnly | DiffRowKind::Modified, DocumentSide::Left) => "‹",
        (DiffRowKind::RightOnly | DiffRowKind::Modified, DocumentSide::Right) => "›",
        _ => " ",
    };
    let gutter = crate::renderer::apply_style_override(
        div()
            .w(px(gutter_width))
            .h_full()
            .flex_none()
            .px(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(rgba(
                theme_color(theme, "diff.gutter", 0x1a1d_24ff).as_rgba_hex(),
            ))
            .text_color(rgba(
                theme_color(theme, "text_muted", 0x929a_a8ff).as_rgba_hex(),
            ))
            .child(marker)
            .child(if config.show_line_numbers && segment.first {
                (segment.line + 1).to_string()
            } else {
                String::new()
            }),
        &config.gutter_style,
        theme,
        theme.direction(),
    );
    let content = crate::renderer::apply_style_override(
        div()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .font_family(".ZedMono")
            .text_size(px(text_size))
            .line_height(px(line_height))
            .text_color(rgba(
                theme_color(theme, "text_primary", 0xd9de_e8ff).as_rgba_hex(),
            ))
            .child(text),
        &config.text_style,
        theme,
        theme.direction(),
    );
    let row = div()
        .size_full()
        .flex()
        .bg(rgba(background.as_rgba_hex()))
        .child(gutter)
        .child(content);
    if matches!(config.wrap, DocumentWrap::None)
        && let Some(horizontal) = horizontal
    {
        let (text_size, _) = document_text_metrics(&config.text_style, theme);
        let character_width = text_size * 0.604;
        let width = (usize_f32(document.max_line_chars()) * character_width + gutter_width + 16.0)
            .max(80.0);
        div()
            .size_full()
            .id(SharedString::from(format!(
                "diff-horizontal-{}-{}-{}",
                side.as_str(),
                segment.line,
                segment.range.start
            )))
            .overflow_x_scroll()
            .track_scroll(horizontal)
            .bg(rgba(background.as_rgba_hex()))
            .child(row.w(px(width)))
            .into_any_element()
    } else {
        row.into_any_element()
    }
}

#[derive(Default)]
pub struct DiffViewerPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, Entity<DiffViewerEntity>>,
    syntaxes: Option<SyntaxRegistry>,
    document_runtime: Option<DocumentRuntimeConfig>,
}

impl DiffViewerPrimitiveHandler {
    #[must_use]
    pub fn new(syntaxes: SyntaxRegistry, document_runtime: DocumentRuntimeConfig) -> Self {
        Self {
            instances: BTreeMap::new(),
            syntaxes: Some(syntaxes),
            document_runtime: Some(document_runtime),
        }
    }
}

impl PrimitiveHandler for DiffViewerPrimitiveHandler {
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        events: &PrimitiveEventEmitter,
        theme: &PrimitiveTheme,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or_else(|| "DiffViewPrimitive requires a stable key".to_owned())?;
        let config = parse_diff_config(&instance.node.props)?;
        let search_typography = native_typography(theme, "body_small", window)?;
        let entity = if let Some(entity) = self.instances.get(&id) {
            entity.clone()
        } else {
            let syntaxes = self
                .syntaxes
                .clone()
                .ok_or_else(|| "DiffViewer syntax registry is unavailable".to_owned())?;
            let document_runtime = self
                .document_runtime
                .clone()
                .ok_or_else(|| "DiffViewer document runtime is unavailable".to_owned())?;
            let entity = cx.new(|cx| {
                DiffViewerEntity::new(
                    config.clone(),
                    events.clone(),
                    theme.clone(),
                    syntaxes,
                    document_runtime,
                    search_typography.clone(),
                    cx,
                )
            });
            entity.update(cx, DiffViewerEntity::start_prepare);
            self.instances.insert(id, entity.clone());
            entity
        };
        entity.update(cx, |viewer, cx| {
            viewer.update(config, events.clone(), theme, search_typography, cx);
        });
        Ok(entity.into_any_element())
    }

    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

fn parse_diff_config(props: &PrimitiveProps) -> Result<DiffViewerConfig, String> {
    let descriptor = |prefix: &str| -> Result<DocumentDescriptor, String> {
        Ok(DocumentDescriptor {
            source: document_source_prop(props, &format!("{prefix}_source"))?,
            label: string_prop(props, &format!("{prefix}_label"))
                .unwrap_or_else(|| prefix.to_owned()),
            file_name: optional_string_prop(props, &format!("{prefix}_file_name")),
            language: optional_string_prop(props, &format!("{prefix}_language")),
        })
    };
    let mode = match string_prop(props, "mode").as_deref() {
        None | Some("unified") => DiffViewMode::Unified,
        Some("split") => DiffViewMode::Split,
        Some(other) => return Err(format!("unknown DiffViewer mode `{other}`")),
    };
    let whitespace = match string_prop(props, "whitespace").as_deref() {
        None | Some("exact") => DiffWhitespace::Exact,
        Some("ignore_changes") => DiffWhitespace::IgnoreChanges,
        Some("ignore_all") => DiffWhitespace::IgnoreAll,
        Some(other) => return Err(format!("unknown DiffViewer whitespace mode `{other}`")),
    };
    let context_lines = match props.get("context_lines") {
        Some(PrimitiveValue::Data(UiValue::String(value))) if value == "all" => None,
        Some(PrimitiveValue::Data(UiValue::Integer(value))) => Some(
            usize::try_from(*value).map_err(|_| "context_lines must be non-negative".to_owned())?,
        ),
        _ => Some(3),
    };
    let wrap = match string_prop(props, "wrap").as_deref() {
        None | Some("none") => DocumentWrap::None,
        Some("viewport") => DocumentWrap::Viewport,
        Some("column") => DocumentWrap::Column(integer_prop(props, "wrap_column").unwrap_or(100)),
        Some(other) => return Err(format!("unknown DiffViewer wrap mode `{other}`")),
    };
    Ok(DiffViewerConfig {
        left: descriptor("left")?,
        right: descriptor("right")?,
        mode,
        whitespace,
        context_lines,
        show_line_numbers: bool_prop(props, "show_line_numbers").unwrap_or(true),
        wrap,
        tab_size: integer_prop(props, "tab_size").unwrap_or(4),
        header_style: style_prop(props, "header_style"),
        gutter_style: style_prop(props, "gutter_style"),
        line_style: style_prop(props, "line_style"),
        text_style: style_prop(props, "text_style"),
        fold_style: style_prop(props, "fold_style"),
        loading_style: style_prop(props, "loading_style"),
        error_style: style_prop(props, "error_style"),
        search_style: style_prop(props, "search_style"),
        status_style: style_prop(props, "status_style"),
    })
}

fn optional_document_fields(prefix: &str) -> Vec<(String, ObjectField)> {
    vec![
        (
            format!("{prefix}_source"),
            ObjectField::required(ValueSchema::one_of([
                ValueSchema::string(),
                ValueSchema::Document,
            ])),
        ),
        (
            format!("{prefix}_label"),
            ObjectField::required(ValueSchema::string()),
        ),
        (
            format!("{prefix}_file_name"),
            ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
        ),
        (
            format!("{prefix}_language"),
            ObjectField::optional(ValueSchema::optional(ValueSchema::string())),
        ),
    ]
}

#[must_use]
/// Return the public native diff-viewer primitive contract.
///
/// # Panics
///
/// Panics only if its compile-time primitive ID becomes invalid.
#[allow(clippy::too_many_lines)]
pub fn diff_viewer_primitive_descriptor() -> PrimitiveDescriptor {
    let mut props = BTreeMap::new();
    props.extend(optional_document_fields("left"));
    props.extend(optional_document_fields("right"));
    props.extend([
        (
            "mode".to_owned(),
            ObjectField::optional(ValueSchema::enumeration(["unified", "split"]))
                .with_default(UiValue::String("unified".to_owned())),
        ),
        (
            "whitespace".to_owned(),
            ObjectField::optional(ValueSchema::enumeration([
                "exact",
                "ignore_changes",
                "ignore_all",
            ]))
            .with_default(UiValue::String("exact".to_owned())),
        ),
        (
            "context_lines".to_owned(),
            ObjectField::optional(ValueSchema::one_of([
                ValueSchema::bounded_integer(Some(0), Some(100)),
                ValueSchema::enumeration(["all"]),
            ]))
            .with_default(UiValue::Integer(3)),
        ),
        (
            "show_line_numbers".to_owned(),
            ObjectField::optional(ValueSchema::Bool).with_default(UiValue::Bool(true)),
        ),
        (
            "wrap".to_owned(),
            ObjectField::optional(ValueSchema::enumeration(["none", "viewport", "column"]))
                .with_default(UiValue::String("none".to_owned())),
        ),
        (
            "wrap_column".to_owned(),
            ObjectField::optional(ValueSchema::bounded_integer(Some(20), Some(500)))
                .with_default(UiValue::Integer(100)),
        ),
        (
            "tab_size".to_owned(),
            ObjectField::optional(ValueSchema::bounded_integer(Some(1), Some(16)))
                .with_default(UiValue::Integer(4)),
        ),
        (
            "header_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "gutter_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "line_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "text_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "fold_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "loading_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "error_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "search_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "status_style".to_owned(),
            ObjectField::required(ValueSchema::Style),
        ),
        (
            "on_location_activate".to_owned(),
            ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
        ),
    ]);
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.diff_viewer").expect("static primitive ID"),
        export: "DiffViewPrimitive".to_owned(),
        props,
        events: BTreeMap::from([(
            "location_activate".to_owned(),
            EventSchema {
                payload: location_schema(true),
            },
        )]),
        state: crate::ComponentStateSchema::default(),
        lifecycle: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_selection_copies_original_cross_line_text() {
        let source: Arc<str> = Arc::from("one\ntwo\nthree");
        let selection = DocumentSelection::default();
        selection.begin(DocumentSide::Single, Arc::clone(&source), 2);
        selection.update(DocumentSide::Single, 9);
        assert_eq!(selection.selected_text().as_deref(), Some("e\ntwo\nt"));
        assert_eq!(
            selection.range_for(DocumentSide::Single, &source, 4..7),
            Some(0..3)
        );
    }

    #[test]
    fn code_descriptor_exposes_string_or_native_document_without_rhai_rows() {
        let descriptor = code_viewer_primitive_descriptor();
        let source = &descriptor.props["source"].schema;
        assert!(
            matches!(source, ValueSchema::OneOf { variants } if variants.contains(&ValueSchema::Document))
        );
        assert!(descriptor.lifecycle);
    }

    #[test]
    fn tab_expansion_preserves_source_and_display_offsets() {
        let expanded = expand_tabs("\tvalue", 4, 0);
        assert_eq!(expanded.text, "    value");
        assert_eq!(source_display_offset(&expanded.offsets, 1), Some(4));
        assert_eq!(display_source_offset(&expanded.offsets, 2), 0);
        assert_eq!(display_source_offset(&expanded.offsets, 4), 1);
    }

    #[test]
    fn wrapping_respects_tabs_and_unicode_grapheme_boundaries() {
        let tabbed = wrap_source_line(&(10..13), "\t12", Some(4), 4);
        assert_eq!(tabbed, vec![(10..11, true, 0), (11..13, false, 4)]);

        let family = "👨‍👩‍👧x";
        let unicode = wrap_source_line(&(0..family.len()), family, Some(1), 4);
        assert_eq!(unicode.len(), 2);
        assert_eq!(&family[unicode[0].0.clone()], "👨‍👩‍👧");
        assert_eq!(&family[unicode[1].0.clone()], "x");
    }

    #[test]
    fn overlapping_highlight_layers_become_ordered_composed_runs() {
        let syntax_color = rgba(0x1122_33ff).into();
        let selection_color = rgba(0x4455_66ff).into();
        let runs = compose_highlights(
            4,
            &[
                (
                    0..4,
                    HighlightStyle {
                        color: Some(syntax_color),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    1..3,
                    HighlightStyle {
                        background_color: Some(selection_color),
                        ..HighlightStyle::default()
                    },
                ),
            ],
        );
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].0, 0..1);
        assert_eq!(runs[1].0, 1..3);
        assert_eq!(runs[2].0, 3..4);
        assert_eq!(runs[1].1.color, Some(syntax_color));
        assert_eq!(runs[1].1.background_color, Some(selection_color));
    }

    #[test]
    fn native_search_supports_case_word_and_regex_policies() {
        let exact = search_ranges(
            "Alpha alphabet alpha",
            "alpha",
            SearchOptions {
                case_sensitive: true,
                whole_word: true,
                regex: false,
            },
            10,
        )
        .unwrap();
        assert_eq!(exact, vec![15..20]);
        let regex = search_ranges(
            "port=80 port=443",
            r"port=\d+",
            SearchOptions {
                regex: true,
                ..SearchOptions::default()
            },
            10,
        )
        .unwrap();
        assert_eq!(regex, vec![0..7, 8..16]);
    }
}
