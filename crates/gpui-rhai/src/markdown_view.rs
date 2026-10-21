//! Retained native Markdown surface over the shared document text machinery.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, Focusable, FontStyle,
    FontWeight, HighlightStyle, InteractiveElement, IntoElement, KeyDownEvent, ListAlignment,
    ListSizingBehavior, ListState, ParentElement, Render, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, StyledText, Task, Window, div, img, list, px, rgba,
};

use crate::document_view::{
    DocumentSelection, DocumentSide, InteractiveDocumentText, compose_highlights,
    document_source_prop, document_text_metrics, resolve_document_monospace_family,
    resolved_pixels, theme_color,
};
use crate::markdown::{BlockKind, Mark, MarkdownText, PreparedMarkdown, TableCell};
use crate::{
    DocumentRuntimeConfig, DocumentSource, EventSchema, ObjectField, PrimitiveContext,
    PrimitiveDescriptor, PrimitiveHandler, PrimitiveId, PrimitiveInstance, PrimitiveInstanceId,
    PrimitiveTheme, PrimitiveValue, Style, SyntaxRegistry, UiValue, ValueSchema,
};

gpui::actions!(
    gpui_rhai_markdown,
    [CopyMarkdownSelection, CopyMarkdownSource, SelectAllMarkdown]
);

/// Install focus-scoped native Markdown commands.
pub fn init_markdown_view(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new(
            "cmd-c",
            CopyMarkdownSelection,
            Some("GPUIRhaiMarkdownViewer"),
        ),
        gpui::KeyBinding::new(
            "ctrl-c",
            CopyMarkdownSelection,
            Some("GPUIRhaiMarkdownViewer"),
        ),
        gpui::KeyBinding::new(
            "alt-cmd-c",
            CopyMarkdownSource,
            Some("GPUIRhaiMarkdownViewer"),
        ),
        gpui::KeyBinding::new(
            "ctrl-alt-c",
            CopyMarkdownSource,
            Some("GPUIRhaiMarkdownViewer"),
        ),
        gpui::KeyBinding::new("cmd-a", SelectAllMarkdown, Some("GPUIRhaiMarkdownViewer")),
        gpui::KeyBinding::new("ctrl-a", SelectAllMarkdown, Some("GPUIRhaiMarkdownViewer")),
    ]);
}

#[derive(Clone, PartialEq)]
struct Config {
    source: DocumentSource,
    label: String,
    content: bool,
    max_height: Option<f32>,
    copy_code: bool,
    parts: BTreeMap<String, Style>,
    images: BTreeMap<String, String>,
    heading_roles: Vec<String>,
}

impl Config {
    fn part(&self, name: &str) -> Style {
        self.parts.get(name).cloned().unwrap_or_default()
    }
}

struct MarkdownViewerEntity {
    config: Config,
    context: PrimitiveContext,
    theme: PrimitiveTheme,
    syntaxes: SyntaxRegistry,
    limits: DocumentRuntimeConfig,
    focus: FocusHandle,
    mono: SharedString,
    prepared: Option<Arc<PreparedMarkdown>>,
    selection: DocumentSelection,
    scroll: ListState,
    scroll_source: crate::ScrollbarSourceHandle,
    tables: Rc<RefCell<BTreeMap<usize, TableState>>>,
    table_viewport_policy: Rc<Cell<(bool, crate::TextDirection)>>,
    width: Option<gpui::Pixels>,
    task: Option<Task<()>>,
    canceled: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    job: u64,
    loading: bool,
    error: Option<String>,
}

impl Drop for MarkdownViewerEntity {
    fn drop(&mut self) {
        self.canceled.store(true, Ordering::Release);
        self.alive.store(false, Ordering::Release);
    }
}

impl MarkdownViewerEntity {
    fn copy(&mut self, source: bool, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let text = if source {
            self.prepared
                .as_ref()
                .map(|prepared| prepared.source.to_string())
        } else {
            self.selection.selected_text()
        };
        if let Some(text) = text {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        }
        cx.stop_propagation();
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(prepared) = &self.prepared {
            self.selection
                .begin(DocumentSide::Single, prepared.plain.clone(), 0);
            self.selection
                .update(DocumentSide::Single, prepared.plain.len());
            cx.notify();
        }
        cx.stop_propagation();
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        self.canceled.store(true, Ordering::Release);
        self.canceled = Arc::new(AtomicBool::new(false));
        self.task = None;
        self.job = self.job.wrapping_add(1);
        let job = self.job;
        let canceled = self.canceled.clone();
        let alive = self.alive.clone();
        let request = match crate::MarkdownPreparation::new(
            self.config.source.clone(),
            &self.syntaxes,
            self.limits.limits(),
        ) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error.to_string());
                self.loading = false;
                cx.notify();
                return;
            }
        };
        let cache = self.limits.markdown_cache();
        if let Some(prepared) = cache.as_ref().and_then(|cache| cache.get(request.key()))
            && prepared.key() == request.key()
        {
            self.limits.markdown_cache_hit();
            self.install_prepared(prepared.model());
            self.loading = false;
            self.error = None;
            if self.config.content {
                self.context.invalidate_intrinsic_size(cx);
            }
            cx.notify();
            return;
        }
        self.loading = true;
        self.error = None;
        let runtime = self.limits.clone();
        self.task = Some(cx.spawn(async move |owner, cx| {
            // A foreground boundary coalesces same-cycle streaming replacements;
            // dropping the owned task and cancellation flag stops obsolete work.
            let result = cx
                .background_spawn(
                    async move { runtime.prepare_markdown(&request, &canceled, &alive) },
                )
                .await;
            let _ = owner.update(cx, |viewer, cx| {
                if viewer.job != job || !viewer.alive.load(Ordering::Acquire) {
                    return;
                }
                viewer.loading = false;
                match result {
                    Ok(prepared) => {
                        viewer.install_prepared(prepared.model());
                        if let Some(cache) = &cache {
                            cache.insert(prepared);
                        }
                    }
                    Err(error) => viewer.error = Some(error),
                }
                if viewer.config.content {
                    // Async native display changes do not edit the enclosing
                    // script row. Invalidate its measured height through the
                    // current placement/lease, not the job's original context.
                    viewer.context.invalidate_intrinsic_size(cx);
                }
                cx.notify();
            });
        }));
    }

    fn install_prepared(&mut self, prepared: Arc<PreparedMarkdown>) {
        if let Some(previous) = &self.prepared {
            self.selection.rebase_unchanged_prefix(
                DocumentSide::Single,
                &previous.plain,
                prepared.plain.clone(),
            );
            // Appended reference definitions can change earlier blocks. Cached
            // and freshly prepared models use exactly the same rebasing path.
            let prefix = previous
                .blocks
                .iter()
                .zip(&prepared.blocks)
                .take_while(|(old, new)| old == new)
                .count();
            resize_measured_list(
                &self.scroll,
                previous.blocks.len(),
                prepared.blocks.len(),
                prefix,
            );
            self.tables.borrow_mut().retain(|index, state| {
                match (previous.blocks.get(*index), prepared.blocks.get(*index)) {
                    (
                        Some(crate::markdown::MarkdownBlock {
                            kind: BlockKind::Table { rows: old, .. },
                            ..
                        }),
                        Some(crate::markdown::MarkdownBlock {
                            kind: BlockKind::Table { rows: next, .. },
                            ..
                        }),
                    ) => {
                        let prefix = old
                            .iter()
                            .zip(next.iter())
                            .take_while(|(old, next)| old == next)
                            .count();
                        resize_measured_list(&state.rows, old.len(), next.len(), prefix);
                        state.cells.borrow_mut().retain(|(row, column), state| {
                            match (
                                old.get(*row).and_then(|row| row.get(*column)),
                                next.get(*row).and_then(|row| row.get(*column)),
                            ) {
                                (Some(old), Some(next)) => {
                                    let prefix = old
                                        .fragments
                                        .iter()
                                        .zip(next.fragments.iter())
                                        .take_while(|(old, next)| old == next)
                                        .count();
                                    resize_measured_list(
                                        &state.fragments,
                                        old.fragments.len(),
                                        next.fragments.len(),
                                        prefix,
                                    );
                                    true
                                }
                                _ => false,
                            }
                        });
                        true
                    }
                    _ => false,
                }
            });
        } else {
            self.scroll
                .reset_with_uniform_height(prepared.blocks.len(), px(36.0));
        }
        self.prepared = Some(prepared);
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        let command = key.modifiers.platform || key.modifiers.control;
        if command && key.key == "c" {
            if self.loading {
                return;
            }
            let text = if key.modifiers.alt {
                self.prepared
                    .as_ref()
                    .map(|prepared| prepared.source.to_string())
            } else {
                self.selection.selected_text()
            };
            if let Some(text) = text {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            }
        } else if command && key.key == "a" {
            if let Some(prepared) = &self.prepared {
                self.selection
                    .begin(DocumentSide::Single, prepared.plain.clone(), 0);
                self.selection
                    .update(DocumentSide::Single, prepared.plain.len());
                cx.notify();
            }
        } else {
            if !scroll_markdown_key(&self.scroll, key.key.as_str()) {
                // A content-height message, or a reader already at this edge,
                // leaves keyboard scrolling available to its conversation.
                return;
            }
            cx.notify();
        }
        cx.stop_propagation();
    }
}

impl Focusable for MarkdownViewerEntity {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for MarkdownViewerEntity {
    #[allow(clippy::too_many_lines)] // One document viewport, focus command route and measured-width recorder.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Reserve a stable logical gutter whenever this document can scroll.
        // Descendant prepaint runs while GPUI mutably borrows the parent list;
        // it must not query that ancestor's range or introduce a layout feedback
        // loop. Sharing policy also updates retained tables after direction or
        // height-mode changes without replacing their native scroll identities.
        self.table_viewport_policy.set((
            !self.config.content || self.config.max_height.is_some(),
            self.theme.direction(),
        ));
        let mut root = div()
            .id("markdown-document")
            .relative()
            .w_full()
            .min_w(px(0.0))
            .overflow_hidden()
            .track_focus(&self.focus)
            .key_context("GPUIRhaiMarkdownViewer")
            .role(Role::Document)
            .aria_label(self.config.label.clone())
            .on_key_down(cx.listener(Self::key_down));
        root = root
            .on_action(
                cx.listener(|viewer, _: &CopyMarkdownSelection, _, cx| viewer.copy(false, cx)),
            )
            .on_action(cx.listener(|viewer, _: &CopyMarkdownSource, _, cx| viewer.copy(true, cx)))
            .on_action(cx.listener(|viewer, _: &SelectAllMarkdown, _, cx| viewer.select_all(cx)));
        if !self.config.content {
            root = root.h_full().min_h(px(0.0));
        }
        if let Some(error) = &self.error {
            return root.child(crate::renderer::apply_style_override(
                div().child(error.clone()),
                &self.config.part("error"),
                &self.theme,
                self.theme.direction(),
            ));
        }
        let Some(prepared) = self.prepared.clone() else {
            return root.child(crate::renderer::apply_style_override(
                div().child("Preparing Markdown…"),
                &self.config.part("loading"),
                &self.theme,
                self.theme.direction(),
            ));
        };
        let paint = Paint {
            prepared,
            config: self.config.clone(),
            theme: self.theme.clone(),
            context: self.context.clone(),
            focus: self.focus.clone(),
            selection: self.selection.clone(),
            mono: self.mono.clone(),
            disabled: self.loading,
            tables: self.tables.clone(),
            table_viewport_policy: self.table_viewport_policy.clone(),
            document_source: self.scroll_source.clone(),
        };
        // Infer measures with MaxContent width before prepaint. Constrain its
        // rows to the actual native list viewport once known, or a wrapped tall
        // item can be remeasured as one line and lose its internal scroll anchor.
        // This is the list's content box, not the outer scripted node including
        // its border/padding. The width recorder below invalidates on resize.
        let content_width = self
            .config
            .content
            .then(|| self.scroll.viewport_bounds().size.width)
            .filter(|width| *width > px(0.0));
        let content = list(self.scroll.clone(), move |index, window, cx| {
            let item = paint.block(index, window, cx);
            match content_width {
                Some(width) => div().w(width).child(item).into_any_element(),
                None => item,
            }
        })
        .w_full()
        .with_sizing_behavior(if self.config.content {
            ListSizingBehavior::Infer
        } else {
            ListSizingBehavior::Auto
        });
        if self.config.content {
            if let Some(max) = self.config.max_height {
                root = root.child(content.max_h(px(max)));
            } else {
                root = root.child(content);
            }
        } else {
            root = root.child(content.h_full());
        }
        root = root.child(
            self.context
                .scrollbars(&self.theme)
                .with_scroll_axis(Some(gpui::Axis::Vertical))
                .element_with_wheel_routing(
                    "markdown-scrollbar",
                    self.scroll_source.clone(),
                    crate::ScrollbarSpec::new("hidden", "auto").expect("static scrollbar mode"),
                ),
        );
        if self.loading {
            root = root.child(crate::renderer::apply_style_override(
                div()
                    .absolute()
                    .right(px(0.0))
                    .top(px(0.0))
                    .child("Updating…"),
                &self.config.part("loading"),
                &self.theme,
                self.theme.direction(),
            ));
        }
        let weak = cx.entity().downgrade();
        root = root.child(
            gpui::canvas(
                move |_, _, cx| {
                    let _ = weak.update(cx, |viewer, cx| {
                        // List prepaint has installed the actual content-box
                        // viewport by now. The surrounding scripted node can
                        // add border/padding, so its root bounds are not an
                        // interchangeable measurement width.
                        let width = viewer.scroll.viewport_bounds().size.width;
                        if viewer.width != Some(width) {
                            viewer.width = Some(width);
                            viewer.scroll.remeasure();
                            for table in viewer.tables.borrow().values() {
                                table.rows.remeasure();
                                for cell in table.cells.borrow().values() {
                                    cell.fragments.remeasure();
                                }
                            }
                            if viewer.config.content {
                                viewer.context.invalidate_intrinsic_size(cx);
                            }
                            cx.notify();
                        }
                    });
                },
                |_, (), _, _| {},
            )
            .absolute()
            .inset_0(),
        );
        root
    }
}

#[derive(Clone)]
struct Paint {
    prepared: Arc<PreparedMarkdown>,
    config: Config,
    theme: PrimitiveTheme,
    context: PrimitiveContext,
    focus: FocusHandle,
    selection: DocumentSelection,
    mono: SharedString,
    disabled: bool,
    tables: Rc<RefCell<BTreeMap<usize, TableState>>>,
    table_viewport_policy: Rc<Cell<(bool, crate::TextDirection)>>,
    document_source: crate::ScrollbarSourceHandle,
}

#[derive(Clone)]
struct TableState {
    rows: ListState,
    // Retained with each actual source: a table replacement or cell-cache
    // eviction must retire captured drags under the same visual element id.
    rows_source: crate::ScrollbarSourceHandle,
    horizontal: ScrollHandle,
    horizontal_source: crate::ScrollbarSourceHandle,
    cells: Rc<RefCell<BTreeMap<(usize, usize), CellState>>>,
}

#[derive(Clone)]
struct CellState {
    fragments: ListState,
    source: crate::ScrollbarSourceHandle,
}

/// A wide table's list is laid out at its logical column width. Its Y track
/// belongs to the horizontal wrapper's visible viewport, not that offscreen
/// logical edge. All scrolling and drag ownership still belong to `rows`.
struct TableRowsSource {
    rows: ListState,
    horizontal: ScrollHandle,
    policy: Rc<Cell<(bool, crate::TextDirection)>>,
}

impl crate::ScrollbarSource for TableRowsSource {
    fn viewport(&self) -> gpui::Bounds<gpui::Pixels> {
        let mut viewport = self
            .rows
            .viewport_bounds()
            .intersect(&self.horizontal.bounds());
        let (reserve_document_track, direction) = self.policy.get();
        if reserve_document_track {
            // A document-level Y bar paints later. Keep the nested bar one
            // logical track inward so the parent cannot cover its hitbox.
            let inset = crate::SCROLLBAR_TRACK_WIDTH.min(viewport.size.width);
            viewport.size.width -= inset;
            if direction == crate::TextDirection::RightToLeft {
                viewport.origin.x += inset;
            }
        }
        viewport
    }
    fn offset(&self) -> gpui::Point<gpui::Pixels> {
        self.rows.scroll_px_offset_for_scrollbar()
    }
    fn max_offset(&self) -> gpui::Point<gpui::Pixels> {
        self.rows.max_offset_for_scrollbar()
    }
    fn set_offset(&self, offset: gpui::Point<gpui::Pixels>) {
        self.rows.set_offset_from_scrollbar(offset);
    }
    fn drag_started(&self) {
        self.rows.scrollbar_drag_started();
    }
    fn drag_ended(&self) {
        self.rows.scrollbar_drag_ended();
    }
}

impl Paint {
    #[allow(clippy::too_many_lines)] // One bounded native block's presentation by semantic kind.
    fn block(&self, index: usize, _: &mut Window, cx: &mut App) -> AnyElement {
        let Some(block) = self.prepared.blocks.get(index) else {
            return div().into_any_element();
        };
        if let BlockKind::Table { alignments, rows } = &block.kind {
            return self.table(index, alignments, rows, cx);
        }
        let part = match block.kind {
            BlockKind::Heading(_) => "heading",
            BlockKind::Code { .. } => "code_block",
            BlockKind::Rule => "rule",
            _ if block.quote > 0 => "quote",
            _ => "paragraph",
        };
        let mut row = div()
            .id(SharedString::from(format!("markdown-block-{index}")))
            .w_full()
            .min_w(px(0.0));
        if block.indent > 0 {
            row = row.pl(px(number_f32(block.indent.min(32)) * 18.0));
        }
        if block.quote > 0 {
            row = row.pl(px(number_f32(block.quote.min(16)) * 12.0));
        }
        match &block.kind {
            BlockKind::Heading(level) => {
                row = row
                    .role(Role::Heading)
                    .aria_level(usize::from(*level))
                    .aria_label(block.text.text.clone());
            }
            BlockKind::Code {
                language,
                source,
                first,
            } => {
                row = row.font_family(self.mono.clone());
                if *first {
                    let mut header = div()
                        .flex()
                        .justify_between()
                        .text_size(px(11.0))
                        .text_color(rgba(
                            theme_color(&self.theme, "text_muted", 0x9999_99ff).as_rgba_hex(),
                        ))
                        .child(if language.is_empty() {
                            "Code".to_owned()
                        } else {
                            language.clone()
                        });
                    if self.config.copy_code && !self.disabled {
                        let code = source.clone();
                        let focus_color =
                            rgba(theme_color(&self.theme, "focus_ring", 0x7aa2_f7ff).as_rgba_hex());
                        header = header.child(
                            div()
                                .id(SharedString::from(format!("markdown-copy-{index}")))
                                .role(Role::Button)
                                .aria_label("Copy code")
                                .tab_index(0)
                                .focus_visible(move |style| {
                                    style.border_1().border_color(focus_color)
                                })
                                .cursor_pointer()
                                .child("Copy")
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        code.to_string(),
                                    ));
                                })
                                .on_a11y_action(gpui::AccessibleAction::Click, {
                                    let code = source.clone();
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            code.to_string(),
                                        ));
                                    }
                                })
                                .on_key_down({
                                    let code = source.clone();
                                    move |event, _, cx| {
                                        if event.keystroke.key == "enter"
                                            || event.keystroke.key == "space"
                                        {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                                code.to_string(),
                                            ));
                                            cx.stop_propagation();
                                        }
                                    }
                                }),
                        );
                    }
                    row = row.child(header);
                }
                row = row.role(Role::Group).aria_label(format!(
                    "{} code block",
                    if language.is_empty() {
                        "Plain text"
                    } else {
                        language
                    }
                ));
            }
            BlockKind::Rule => {
                return crate::renderer::apply_style_override(
                    div().w_full().child(row.h(px(1.0)).my(px(8.0)).bg(rgba(
                        theme_color(&self.theme, "border", 0x5555_55ff).as_rgba_hex(),
                    ))),
                    &self.config.part(part),
                    &self.theme,
                    self.theme.direction(),
                )
                .into_any_element();
            }
            _ if block.list_item => {
                row = row.role(Role::ListItem).aria_label(block.text.text.clone());
            }
            _ => {
                row = row.role(Role::Paragraph);
            }
        }
        row = row.child(self.text(&block.text, index, part));
        crate::renderer::apply_style_override(
            div().w_full().child(row),
            &self.config.part(part),
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }

    #[allow(clippy::too_many_lines)] // Shared shaping plus inline mark and accessible-link overlays.
    fn text(&self, text: &MarkdownText, id: usize, part: &str) -> AnyElement {
        let mut highlights = Vec::new();
        let mut links = Vec::new();
        let mut focus_links = Vec::new();
        let mut images = Vec::new();
        for mark in &text.marks {
            let mut style = HighlightStyle::default();
            match &mark.mark {
                Mark::Strong => style.font_weight = Some(FontWeight::BOLD),
                Mark::Emphasis => style.font_style = Some(FontStyle::Italic),
                Mark::Strike => {
                    style.strikethrough = Some(gpui::StrikethroughStyle {
                        thickness: px(1.0),
                        color: None,
                    });
                }
                Mark::Code => {
                    let part = self.config.part("inline_code");
                    style.background_color = Some(
                        rgba(
                            part.base
                                .background
                                .as_ref()
                                .and_then(|color| self.theme.resolve_color(color))
                                .unwrap_or_else(|| {
                                    theme_color(&self.theme, "surface_raised", 0x3333_33ff)
                                })
                                .as_rgba_hex(),
                        )
                        .into(),
                    );
                    if let Some(color) = part
                        .base
                        .text_color
                        .as_ref()
                        .and_then(|color| self.theme.resolve_color(color))
                    {
                        style.color = Some(rgba(color.as_rgba_hex()).into());
                    }
                }
                Mark::Link { href, label, first } => {
                    style.color = Some(
                        rgba(
                            self.config
                                .part("link")
                                .base
                                .text_color
                                .as_ref()
                                .and_then(|color| self.theme.resolve_color(color))
                                .unwrap_or_else(|| {
                                    theme_color(&self.theme, "text.accent", 0x7aa2_f7ff)
                                })
                                .as_rgba_hex(),
                        )
                        .into(),
                    );
                    style.underline = Some(gpui::UnderlineStyle {
                        thickness: px(1.0),
                        color: None,
                        wavy: false,
                    });
                    links.push((mark.range.clone(), href.clone(), label.clone()));
                    if *first {
                        focus_links.push((mark.range.clone(), href.clone(), label.clone()));
                    }
                }
                Mark::Image(destination) => images.push((
                    mark.range.clone(),
                    destination.clone(),
                    text.text
                        .get(mark.range.clone())
                        .unwrap_or_default()
                        .to_owned(),
                )),
                Mark::Syntax(kind) => {
                    style.color = Some(
                        rgba(
                            theme_color(&self.theme, kind.theme_token(), 0xeeee_eeff).as_rgba_hex(),
                        )
                        .into(),
                    );
                }
            }
            highlights.push((mark.range.clone(), style));
        }
        if let Some(range) = self.selection.range_for(
            DocumentSide::Single,
            &self.prepared.plain,
            text.range.clone(),
        ) {
            highlights.push((
                range,
                HighlightStyle {
                    background_color: Some(
                        rgba(theme_color(&self.theme, "selection", 0x3347_6fff).as_rgba_hex())
                            .into(),
                    ),
                    ..HighlightStyle::default()
                },
            ));
        }
        let inline_mono = self
            .config
            .part("inline_code")
            .base
            .font_family
            .map_or_else(|| self.mono.clone(), SharedString::from);
        let code_fonts = text
            .marks
            .iter()
            .filter(|mark| matches!(mark.mark, Mark::Code))
            .map(|mark| (mark.range.clone(), inline_mono.clone()))
            .collect::<Vec<_>>();
        let styled = StyledText::new(text.text.clone())
            .with_highlights(compose_highlights(text.text.len(), &highlights))
            .with_font_family_overrides(code_fonts);
        let offset_map = text
            .text
            .char_indices()
            .map(|(offset, _)| (offset, offset))
            .chain(std::iter::once((text.text.len(), text.text.len())))
            .collect::<Vec<_>>();
        let interactive = InteractiveDocumentText {
            id: SharedString::from(format!("markdown-text-{id}-{}", text.range.start)).into(),
            text: styled,
            source: self.prepared.plain.clone(),
            global_range: text.range.clone(),
            offset_map: offset_map.into(),
            line: 0,
            line_start: text.range.start,
            side: DocumentSide::Single,
            selection: self.selection.clone(),
            focus: self.focus.clone(),
            events: self.context.clone(),
            links: links.clone().into(),
            disabled: self.disabled,
        };
        let mut part_style = self.config.part(part);
        if part == "heading"
            && part_style.base.typography.is_none()
            && let Some(crate::markdown::MarkdownBlock {
                kind: BlockKind::Heading(level),
                ..
            }) = self.prepared.blocks.get(id)
        {
            part_style.base.typography = self
                .config
                .heading_roles
                .get(usize::from(*level).saturating_sub(1))
                .cloned();
            part_style.base.font_weight.get_or_insert(600);
        }
        let (text_size, line_height) = document_text_metrics(&part_style, &self.theme);
        let mut content = div()
            .w_full()
            .min_w(px(0.0))
            .relative()
            .text_size(px(text_size))
            .line_height(px(line_height))
            .text_color(rgba(
                theme_color(&self.theme, "text_primary", 0xeeee_eeff).as_rgba_hex(),
            ))
            .child(interactive);
        // Focus-only overlays expose each inline link without drawing a second
        // label or disturbing native text shaping/selection. Normal hitboxes do
        // not occlude the underlying document text's pointer listeners.
        if !self.disabled {
            for (index, (range, href, label)) in focus_links.into_iter().enumerate() {
                let events = self.context.clone();
                let payload = link_payload(href.clone(), label.clone());
                let focus_color =
                    rgba(theme_color(&self.theme, "focus_ring", 0x7aa2_f7ff).as_rgba_hex());
                content = content.child(
                    div()
                        .id(SharedString::from(format!(
                            "markdown-link-{}-{index}-{}",
                            text.range.start, range.start
                        )))
                        .absolute()
                        .inset_0()
                        .role(Role::Link)
                        .aria_label(label)
                        .aria_description(href)
                        .tab_index(0)
                        .focus_visible(move |style| style.border_1().border_color(focus_color))
                        .on_a11y_action(gpui::AccessibleAction::Click, {
                            let events = events.clone();
                            let payload = payload.clone();
                            move |_, window, cx| {
                                let _ = events.emit("link", payload.clone(), window, cx);
                            }
                        })
                        .on_key_down(move |event, window, cx| {
                            if event.keystroke.key == "enter" {
                                let _ = events.emit("link", payload.clone(), window, cx);
                                cx.stop_propagation();
                            }
                        }),
                );
            }
        }
        for (index, (range, destination, alt)) in images.into_iter().enumerate() {
            if let Some(id) = self
                .config
                .images
                .get(&destination)
                .and_then(|id| crate::AssetId::parse(id).ok())
                && let Ok(source) = self.context.image_asset(&id)
            {
                let mut image = div()
                    .id(SharedString::from(format!(
                        "markdown-image-{}-{index}",
                        text.range.start
                    )))
                    .role(Role::Image)
                    .aria_label(alt)
                    .child(img(source).max_w_full().max_h(px(128.0)));
                if let Some((href, label)) = text.marks.iter().find_map(|mark| {
                    if mark.range.start <= range.start
                        && mark.range.end >= range.end
                        && let Mark::Link { href, label, .. } = &mark.mark
                    {
                        Some((href.clone(), label.clone()))
                    } else {
                        None
                    }
                }) {
                    let events = self.context.clone();
                    let payload = link_payload(href, label);
                    image = image.on_click(move |_, window, cx| {
                        let _ = events.emit("link", payload.clone(), window, cx);
                    });
                }
                content = content.child(image);
            }
        }
        crate::renderer::apply_style_override(
            content,
            &typography_only(&part_style),
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }

    #[allow(clippy::too_many_lines)] // Bounded native logical rows, cell fragment viewports and shared scrollbars.
    fn table(
        &self,
        index: usize,
        alignments: &[pulldown_cmark::Alignment],
        rows: &Arc<Vec<Vec<TableCell>>>,
        _: &mut App,
    ) -> AnyElement {
        // Each table is a bounded nested measured list. Neither thousands of rows
        // nor a single huge cell is materialized in one GPUI layout operation.
        let rows = rows.clone();
        let columns = alignments.len();
        let width = number_f32(columns) * 200.0;
        let paint = self.clone();
        let alignments = alignments.to_vec();
        let count = rows.len();
        let state = self
            .tables
            .borrow_mut()
            .entry(index)
            .or_insert_with(|| {
                let rows = ListState::new(count, ListAlignment::Top, px(160.0))
                    .with_uniform_item_height(px(32.0));
                let horizontal = ScrollHandle::new();
                TableState {
                    rows_source: crate::ScrollbarSourceHandle::new(TableRowsSource {
                        rows: rows.clone(),
                        horizontal: horizontal.clone(),
                        policy: self.table_viewport_policy.clone(),
                    }),
                    rows,
                    horizontal_source: crate::ScrollbarSourceHandle::new(horizontal.clone()),
                    horizontal,
                    cells: Rc::new(RefCell::new(BTreeMap::new())),
                }
            })
            .clone();
        let cell_states = state.cells.clone();
        // with_scroll_parent installs the newest/nearest parent first. Add
        // from outside in so the native chain precedes inherited Rhai owners.
        let cell_scrollbars = self
            .context
            .scrollbars(&self.theme)
            .with_scroll_axis(Some(gpui::Axis::Vertical))
            .with_scroll_parent(self.document_source.clone(), Some(gpui::Axis::Vertical))
            .with_scroll_parent(state.rows_source.clone(), Some(gpui::Axis::Vertical))
            .with_scroll_parent(
                state.horizontal_source.clone(),
                Some(gpui::Axis::Horizontal),
            );
        let table = list(state.rows.clone(), move |row_index, _, _| {
            let mut row = div()
                .id(SharedString::from(format!(
                    "markdown-table-{index}-row-{row_index}"
                )))
                .flex()
                .w(px(width))
                .min_h(px(28.0))
                .role(Role::Row)
                .aria_row_index(row_index + 1);
            if let Some(cells) = rows.get(row_index) {
                for (column, cell) in cells.iter().enumerate() {
                    let mut content = div()
                        .id(SharedString::from(format!(
                            "markdown-cell-{index}-{row_index}-{column}"
                        )))
                        .debug_selector(move || {
                            format!("markdown-cell:{index}:{row_index}:{column}")
                        })
                        .w(px(200.0))
                        .flex_none()
                        .px(px(8.0))
                        .py(px(5.0))
                        .role(if row_index == 0 {
                            Role::ColumnHeader
                        } else {
                            Role::Cell
                        })
                        .aria_column_index(column + 1);
                    if row_index == 0 {
                        content = content.font_weight(FontWeight::SEMIBOLD);
                    }
                    content = match alignments.get(column) {
                        Some(pulldown_cmark::Alignment::Center) => {
                            content.text_align(gpui::TextAlign::Center)
                        }
                        Some(pulldown_cmark::Alignment::Right) => {
                            content.text_align(gpui::TextAlign::Right)
                        }
                        _ => content,
                    };
                    if cell.fragments.len() <= 1 {
                        row = row.child(content.child(paint.text(&cell.text, index, "table")));
                    } else {
                        // Visual fragments never become logical table rows or
                        // alter row-major plain-copy order. One huge cell owns a
                        // bounded native viewport of its own fragments.
                        let key = (row_index, column);
                        let state = {
                            let mut cache = cell_states.borrow_mut();
                            if cache.len() >= 512
                                && !cache.contains_key(&key)
                                && let Some(oldest) = cache.keys().next().copied()
                            {
                                cache.remove(&oldest);
                            }
                            cache
                                .entry(key)
                                .or_insert_with(|| {
                                    let fragments = ListState::new(
                                        cell.fragments.len(),
                                        ListAlignment::Top,
                                        px(44.0),
                                    )
                                    .with_uniform_item_height(px(22.0));
                                    CellState {
                                        source: crate::ScrollbarSourceHandle::new(
                                            fragments.clone(),
                                        ),
                                        fragments,
                                    }
                                })
                                .clone()
                        };
                        let scrollbar = cell_scrollbars.element_with_wheel_routing(
                            SharedString::from(format!(
                                "markdown-cell-scroll-{index}-{row_index}-{column}"
                            )),
                            state.source,
                            crate::ScrollbarSpec::new("hidden", "auto")
                                .expect("static scrollbar mode"),
                        );
                        let fragments = cell.fragments.clone();
                        let paint = paint.clone();
                        row = row.child(
                            content.child(
                                div()
                                    .debug_selector(move || {
                                        format!(
                                            "markdown-cell-viewport:{index}:{row_index}:{column}"
                                        )
                                    })
                                    .relative()
                                    .w_full()
                                    .h(px(128.0))
                                    .overflow_hidden()
                                    .child(
                                        list(state.fragments, move |fragment, _, _| {
                                            fragments.get(fragment).map_or_else(
                                                || div().into_any_element(),
                                                |text| paint.text(text, index, "table"),
                                            )
                                        })
                                        .w_full()
                                        .h_full(),
                                    )
                                    .child(scrollbar),
                            ),
                        );
                    }
                }
            }
            row.border_b_1()
                .border_color(rgba(
                    theme_color(&paint.theme, "border", 0x5555_55ff).as_rgba_hex(),
                ))
                .into_any_element()
        })
        .w(px(width))
        .h(px((number_f32(count) * 32.0).min(320.0)));
        let wrapper = div()
            .id(SharedString::from(format!("markdown-table-{index}")))
            .debug_selector(move || format!("markdown-table:{index}"))
            .w_full()
            .min_w(px(0.0))
            .overflow_x_scroll()
            .restrict_scroll_to_axis()
            .track_scroll(&state.horizontal)
            .role(Role::Table)
            .aria_row_count(count)
            .aria_column_count(columns)
            .child(table);
        let scrollbars = self
            .context
            .scrollbars(&self.theme)
            .with_scroll_parent(self.document_source.clone(), Some(gpui::Axis::Vertical));
        crate::renderer::apply_style_override(
            div()
                .w_full()
                .relative()
                .overflow_hidden()
                .child(wrapper)
                .child(
                    scrollbars
                        .clone()
                        .with_scroll_axis(Some(gpui::Axis::Horizontal))
                        .with_scroll_parent(state.rows_source.clone(), Some(gpui::Axis::Vertical))
                        .element_with_wheel_routing(
                            SharedString::from(format!("markdown-table-x-{index}")),
                            state.horizontal_source,
                            crate::ScrollbarSpec::new("auto", "hidden")
                                .expect("static scrollbar mode"),
                        ),
                )
                .child(
                    scrollbars
                        .with_scroll_axis(Some(gpui::Axis::Vertical))
                        .element_with_wheel_routing(
                            SharedString::from(format!("markdown-table-y-{index}")),
                            state.rows_source,
                            crate::ScrollbarSpec::new("hidden", "auto")
                                .expect("static scrollbar mode"),
                        ),
                ),
            &self.config.part("table"),
            &self.theme,
            self.theme.direction(),
        )
        .into_any_element()
    }
}

fn link_payload(href: String, text: String) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("href".to_owned(), UiValue::String(href)),
        ("text".to_owned(), UiValue::String(text)),
    ]))
}
fn number_f32(value: usize) -> f32 {
    value.to_string().parse().unwrap_or(f32::MAX)
}

fn scroll_markdown_key(scroll: &ListState, key: &str) -> bool {
    let maximum = scroll.max_offset_for_scrollbar().y;
    if maximum <= px(0.0) || scroll.viewport_bounds().size.height <= px(0.0) {
        return false;
    }
    // Relative movement uses the native viewport offset. Clamping before
    // mutation also keeps a no-op edge key from changing its logical anchor.
    let offset = scroll.scroll_px_offset_for_scrollbar();
    let before = offset.y.clamp(-maximum, px(0.0));
    let target = match key {
        "up" => before + px(24.0),
        "down" => before - px(24.0),
        "pageup" => before + px(320.0),
        "pagedown" => before - px(320.0),
        "home" => px(0.0),
        "end" => -maximum,
        _ => return false,
    }
    .clamp(-maximum, px(0.0));
    if target == before {
        return false;
    }
    if key == "end" {
        // The pixel maximum contains estimates for unmeasured blocks. A pixel
        // jump can stop short once the destination is measured. GPUI's native
        // item-count tail anchor measures backwards from the actual last item.
        scroll.scroll_to_end();
        return true;
    }
    scroll.set_offset_from_scrollbar(gpui::point(offset.x, target));
    scroll
        .scroll_px_offset_for_scrollbar()
        .y
        .clamp(-maximum, px(0.0))
        != before
}

fn typography_only(style: &Style) -> Style {
    let mut result = Style::default();
    result.base.typography.clone_from(&style.base.typography);
    result.base.font_size = style.base.font_size;
    result.base.font_family.clone_from(&style.base.font_family);
    result
        .base
        .font_fallbacks
        .clone_from(&style.base.font_fallbacks);
    result
        .base
        .font_features
        .clone_from(&style.base.font_features);
    result.base.font_weight = style.base.font_weight;
    result.base.font_slant = style.base.font_slant;
    result.base.line_height = style.base.line_height;
    result.base.text_color.clone_from(&style.base.text_color);
    result.base.text_align = style.base.text_align;
    result.base.white_space = style.base.white_space;
    result.base.text_ellipsis = style.base.text_ellipsis;
    result.base.line_clamp = style.base.line_clamp;
    result
}

fn resize_measured_list(state: &ListState, old: usize, next: usize, changed_from: usize) {
    // Remeasure surviving rows in place: replacing every row after a semantic
    // change would discard the reader's measured scroll anchor too.
    let surviving = old.min(next);
    if changed_from < surviving {
        state.remeasure_items(changed_from..surviving);
    }
    if next > old {
        state.splice(old..old, next - old);
    }
    if next < old {
        state.splice(next..old, 0);
    }
}

/// Native Markdown primitive lifecycle. Source components supply typography and
/// parts; this handler owns only native parsing, interaction and virtualization.
pub struct MarkdownViewerPrimitiveHandler {
    instances: BTreeMap<PrimitiveInstanceId, OwnedMarkdownViewer>,
    syntaxes: SyntaxRegistry,
    limits: DocumentRuntimeConfig,
}

struct OwnedMarkdownViewer {
    entity: Entity<MarkdownViewerEntity>,
    alive: Arc<AtomicBool>,
}
impl Drop for OwnedMarkdownViewer {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}

impl MarkdownViewerPrimitiveHandler {
    #[must_use]
    pub fn new(syntaxes: SyntaxRegistry, limits: DocumentRuntimeConfig) -> Self {
        Self {
            instances: BTreeMap::new(),
            syntaxes,
            limits,
        }
    }
}

impl PrimitiveHandler for MarkdownViewerPrimitiveHandler {
    #[allow(clippy::too_many_lines)] // Typed prop normalization and retained entity lifecycle.
    fn render(
        &mut self,
        instance: &PrimitiveInstance,
        context: &PrimitiveContext,
        theme: &PrimitiveTheme,
        _: &mut Window,
        cx: &mut App,
    ) -> Result<AnyElement, String> {
        let id = instance
            .id
            .clone()
            .ok_or("MarkdownViewer requires a stable key")?;
        let props = &instance.node.props;
        let config = Config {
            source: document_source_prop(props, "source")?,
            label: props.string("label").unwrap_or("Markdown").to_owned(),
            content: props.string("height") == Some("content"),
            max_height: match props.get("max_height") {
                Some(PrimitiveValue::Length(value)) => Some(
                    theme
                        .resolve_length(*value)
                        .and_then(resolved_pixels)
                        .ok_or("max_height must resolve to pixels")?,
                ),
                _ => None,
            },
            copy_code: matches!(props.data("code_block_actions"), Some(UiValue::Array(actions)) if actions.contains(&UiValue::String("copy".to_owned()))),
            parts: PARTS
                .iter()
                .map(|part| {
                    (
                        (*part).to_owned(),
                        props
                            .style(&format!("{part}_style"))
                            .cloned()
                            .unwrap_or_default(),
                    )
                })
                .collect(),
            images: if let Some(UiValue::Map(images)) = props.data("image_assets") {
                images
                    .iter()
                    .filter_map(|(href, value)| {
                        if let UiValue::String(id) = value {
                            Some((href.clone(), id.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                BTreeMap::new()
            },
            heading_roles: if let Some(UiValue::Array(roles)) = props.data("heading_roles") {
                roles
                    .iter()
                    .filter_map(|role| {
                        if let UiValue::String(role) = role {
                            Some(role.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            },
        };
        let entity = self
            .instances
            .entry(id)
            .or_insert_with(|| {
                let alive = Arc::new(AtomicBool::new(true));
                let scroll = ListState::new(0, ListAlignment::Top, px(200.0))
                    .with_uniform_item_height(px(36.0));
                let entity = cx.new(|cx| MarkdownViewerEntity {
                    config: config.clone(),
                    context: context.clone(),
                    theme: theme.clone(),
                    syntaxes: self.syntaxes.clone(),
                    limits: self.limits.clone(),
                    focus: cx.focus_handle(),
                    mono: resolve_document_monospace_family(cx),
                    prepared: None,
                    selection: DocumentSelection::default(),
                    scroll_source: crate::ScrollbarSourceHandle::new(scroll.clone()),
                    scroll,
                    tables: Rc::new(RefCell::new(BTreeMap::new())),
                    table_viewport_policy: Rc::new(Cell::new((false, theme.direction()))),
                    width: None,
                    task: None,
                    canceled: Arc::new(AtomicBool::new(false)),
                    alive: alive.clone(),
                    job: 0,
                    loading: false,
                    error: None,
                });
                entity.update(cx, MarkdownViewerEntity::start);
                OwnedMarkdownViewer { entity, alive }
            })
            .entity
            .clone();
        entity.update(cx, |viewer, cx| {
            let changed = viewer.config.source != config.source;
            let style_changed = viewer.config.parts != config.parts
                || config
                    .parts
                    .values()
                    .filter_map(|style| style.base.typography.as_deref())
                    .chain(config.heading_roles.iter().map(String::as_str))
                    .chain(std::iter::once("body"))
                    .any(|role| viewer.theme.typography(role) != theme.typography(role));
            if viewer.config.source.identity() != config.source.identity() {
                viewer.selection.clear();
                viewer.scroll.reset(0);
                viewer.prepared = None;
                viewer.tables.borrow_mut().clear();
            }
            viewer.config = config;
            viewer.context = context.clone();
            viewer.theme = theme.clone();
            if changed {
                viewer.start(cx);
            } else if style_changed {
                viewer.scroll.remeasure();
                for table in viewer.tables.borrow().values() {
                    table.rows.remeasure();
                    for cell in table.cells.borrow().values() {
                        cell.fragments.remeasure();
                    }
                }
            }
            cx.notify();
        });
        Ok(entity.into_any_element())
    }
    fn unmount(&mut self, instance: &PrimitiveInstanceId) {
        self.instances.remove(instance);
    }
}

const PARTS: &[&str] = &[
    "heading",
    "paragraph",
    "code_block",
    "inline_code",
    "quote",
    "table",
    "link",
    "rule",
    "loading",
    "error",
];

/// The public native primitive contract used by the source-owned `MarkdownViewer`.
///
/// # Panics
/// Panics only if the compile-time primitive identifier becomes invalid.
#[must_use]
pub fn markdown_viewer_primitive_descriptor() -> PrimitiveDescriptor {
    let mut props = BTreeMap::from([
        ("source".to_owned(), ObjectField::required(ValueSchema::one_of([ValueSchema::string(), ValueSchema::Document])).with_doc("Inline Markdown or an opaque Host-owned native text document.")),
        ("label".to_owned(), ObjectField::required(ValueSchema::string()).with_doc("Accessible document name.")),
        ("height".to_owned(), ObjectField::optional(ValueSchema::enumeration(["fill", "content"])).with_doc("Fill the box or grow to the measured content height.")),
        ("max_height".to_owned(), ObjectField::optional(ValueSchema::optional(ValueSchema::Length)).with_doc("Maximum content height as an absolute-resolvable Length (px/rem); relative lengths are not supported. Longer content scrolls natively.")),
        ("code_block_actions".to_owned(), ObjectField::optional(ValueSchema::Array { items: Box::new(ValueSchema::enumeration(["copy"])), max_items: Some(1) }).with_doc("Code block actions; copy writes code without Markdown fences.")),
        ("image_assets".to_owned(), ObjectField::optional(ValueSchema::Map { values: Box::new(ValueSchema::string()) }).with_doc("Explicit destination-to-Host-asset-ID map. Missing images show alt text; URLs are never fetched.")),
        ("heading_roles".to_owned(), ObjectField::required(ValueSchema::Array { items: Box::new(ValueSchema::string()), max_items: Some(6) }).with_doc("Typography roles for heading levels 1 through 6; an explicit heading part typography overrides these.")),
        ("on_link".to_owned(), ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)).with_doc("Emits href and visible text. Does not open a browser.")),
    ]);
    for part in PARTS {
        props.insert(
            format!("{part}_style"),
            ObjectField::required(ValueSchema::Style)
                .with_doc(format!("Style of the Markdown {part} part.")),
        );
    }
    PrimitiveDescriptor {
        id: PrimitiveId::parse("gpui_rhai.markdown_viewer").expect("static primitive ID"),
        export: "MarkdownViewPrimitive".to_owned(),
        props,
        events: BTreeMap::from([(
            "link".to_owned(),
            EventSchema {
                payload: ValueSchema::object(BTreeMap::from([
                    (
                        "href".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    ),
                    (
                        "text".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    ),
                ])),
                doc: Some(
                    "An untrusted link activation proposal; Host decides whether to open it."
                        .to_owned(),
                ),
            },
        )]),
        state: crate::ComponentStateSchema::default(),
        lifecycle: true,
        effect: None,
    }
}
