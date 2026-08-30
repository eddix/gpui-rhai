use std::ops::Range;
use std::time::{Duration, Instant};

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextBuffer {
    pub(crate) content: String,
    pub(crate) selected: Range<usize>,
    pub(crate) selection_reversed: bool,
    pub(crate) marked: Option<Range<usize>>,
    max_length: Option<usize>,
    undo: Vec<TextRevision>,
    redo: Vec<TextRevision>,
    composition_base: Option<TextRevision>,
    last_edit: Option<LastEdit>,
}

const HISTORY_LIMIT: usize = 100;
const COALESCE_INTERVAL: Duration = Duration::from_millis(750);

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextRevision {
    content: String,
    selected: Range<usize>,
    selection_reversed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditKind {
    Insert,
    Delete,
    Replace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LastEdit {
    kind: EditKind,
    at: Instant,
    cursor: usize,
}

impl TextBuffer {
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        let content = content.into();
        let cursor = content.len();
        Self {
            content,
            selected: cursor..cursor,
            selection_reversed: false,
            marked: None,
            max_length: None,
            undo: Vec::new(),
            redo: Vec::new(),
            composition_base: None,
            last_edit: None,
        }
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    #[must_use]
    pub fn selection(&self) -> &Range<usize> {
        &self.selected
    }

    #[must_use]
    pub fn marked(&self) -> Option<&Range<usize>> {
        self.marked.as_ref()
    }

    #[must_use]
    pub fn grapheme_count(&self) -> usize {
        self.content.graphemes(true).count()
    }

    pub(crate) fn set_max_length(&mut self, max_length: Option<usize>) {
        self.max_length = max_length;
    }

    #[must_use]
    pub(crate) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected.start
        } else {
            self.selected.end
        }
    }

    pub(crate) fn move_to(&mut self, offset: usize) {
        let offset = self.clamp_boundary(offset);
        self.selected = offset..offset;
        self.selection_reversed = false;
        self.last_edit = None;
    }

    pub(crate) fn select_to(&mut self, offset: usize) {
        let offset = self.clamp_boundary(offset);
        if self.selection_reversed {
            self.selected.start = offset;
        } else {
            self.selected.end = offset;
        }
        if self.selected.end < self.selected.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected = self.selected.end..self.selected.start;
        }
        self.last_edit = None;
    }

    #[must_use]
    pub(crate) fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    #[must_use]
    pub(crate) fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.content.len())
    }

    #[must_use]
    pub(crate) fn line_start(&self, offset: usize) -> usize {
        let offset = self.clamp_boundary(offset);
        self.content[..offset]
            .rfind('\n')
            .map_or(0, |index| index + 1)
    }

    #[must_use]
    pub(crate) fn line_end(&self, offset: usize) -> usize {
        let offset = self.clamp_boundary(offset);
        self.content[offset..]
            .find('\n')
            .map_or(self.content.len(), |index| offset + index)
    }

    #[must_use]
    pub(crate) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for character in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += character.len_utf16();
            utf8_offset += character.len_utf8();
        }
        self.clamp_boundary(utf8_offset)
    }

    #[must_use]
    pub(crate) fn offset_to_utf16(&self, offset: usize) -> usize {
        let offset = self.clamp_boundary(offset);
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for character in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += character.len_utf8();
            utf16_offset += character.len_utf16();
        }
        utf16_offset
    }

    #[must_use]
    pub(crate) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    #[must_use]
    pub(crate) fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }

    pub(crate) fn replace(&mut self, range_utf16: Option<&Range<usize>>, text: &str) {
        let range = range_utf16
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selected.clone());
        let insertion = self.fit_replacement(&range, text);
        if self.content.get(range.clone()) == Some(insertion.as_str()) {
            self.move_to(range.start + insertion.len());
            self.marked = None;
            self.composition_base = None;
            return;
        }
        let kind = if insertion.is_empty() {
            EditKind::Delete
        } else if range.is_empty() {
            EditKind::Insert
        } else {
            EditKind::Replace
        };
        let coalescible = match kind {
            EditKind::Insert => {
                insertion.graphemes(true).count() == 1
                    && !insertion.chars().any(char::is_whitespace)
            }
            EditKind::Delete => true,
            EditKind::Replace => false,
        };
        self.record_before_edit(kind, coalescible);
        self.content.replace_range(range.clone(), &insertion);
        self.set_cursor_after_edit(range.start + insertion.len(), kind, coalescible);
        self.marked = None;
    }

    pub(crate) fn replace_and_mark(
        &mut self,
        range_utf16: Option<&Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
    ) {
        if self.composition_base.is_none() {
            self.composition_base = Some(self.revision());
            self.last_edit = None;
        }
        let range = range_utf16
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.marked.clone())
            .unwrap_or_else(|| self.selected.clone());
        self.content.replace_range(range.clone(), text);
        self.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        self.selected = selected_utf16.map_or_else(
            || range.start + text.len()..range.start + text.len(),
            |selection| {
                let selection = utf16_range_in(text, &selection);
                range.start + selection.start..range.start + selection.end
            },
        );
    }

    /// Commit marked IME text and enforce `max_length` only inside the marked
    /// replacement, preserving text after the composition range.
    pub(crate) fn unmark(&mut self) -> bool {
        let Some(marked) = self.marked.take() else {
            return false;
        };
        let mut changed = false;
        if let Some(max_length) = self.max_length {
            let outside = self.content[..marked.start].graphemes(true).count()
                + self.content[marked.end..].graphemes(true).count();
            let capacity = max_length.saturating_sub(outside);
            let marked_text = self.content[marked.clone()].to_owned();
            let fitted = take_graphemes(&marked_text, capacity);
            if fitted != marked_text {
                self.content.replace_range(marked.clone(), &fitted);
                self.set_cursor_without_history(marked.start + fitted.len());
                changed = true;
            }
        }
        self.commit_composition_history();
        changed
    }

    pub(crate) fn set_controlled(&mut self, content: &str) {
        if self.content != content {
            content.clone_into(&mut self.content);
            self.move_to(self.content.len());
            self.marked = None;
            self.undo.clear();
            self.redo.clear();
            self.composition_base = None;
            self.last_edit = None;
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.composition_base.is_some()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(base) = self.composition_base.take() {
            self.marked = None;
            self.restore_revision(base);
            self.last_edit = None;
            return true;
        }
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.revision());
        self.restore_revision(previous);
        self.last_edit = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.push_undo(self.revision());
        self.restore_revision(next);
        self.last_edit = None;
        true
    }

    fn fit_replacement(&self, range: &Range<usize>, text: &str) -> String {
        let Some(max_length) = self.max_length else {
            return text.to_owned();
        };
        let outside = self.content[..range.start].graphemes(true).count()
            + self.content[range.end..].graphemes(true).count();
        take_graphemes(text, max_length.saturating_sub(outside))
    }

    fn clamp_boundary(&self, offset: usize) -> usize {
        let offset = offset.min(self.content.len());
        if offset == self.content.len() {
            return offset;
        }
        self.content
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .take_while(|index| *index <= offset)
            .last()
            .unwrap_or(0)
    }

    fn revision(&self) -> TextRevision {
        TextRevision {
            content: self.content.clone(),
            selected: self.selected.clone(),
            selection_reversed: self.selection_reversed,
        }
    }

    fn restore_revision(&mut self, revision: TextRevision) {
        self.content = revision.content;
        self.selected = revision.selected;
        self.selection_reversed = revision.selection_reversed;
        self.marked = None;
        self.composition_base = None;
    }

    fn record_before_edit(&mut self, kind: EditKind, coalescible: bool) {
        if let Some(base) = self.composition_base.take() {
            self.push_undo(base);
            self.redo.clear();
            return;
        }
        let now = Instant::now();
        let coalesced = coalescible
            && self.selected.is_empty()
            && self.last_edit.is_some_and(|last| {
                last.kind == kind
                    && last.cursor == self.cursor_offset()
                    && now.saturating_duration_since(last.at) <= COALESCE_INTERVAL
            });
        if !coalesced {
            self.push_undo(self.revision());
        }
        self.redo.clear();
    }

    fn set_cursor_after_edit(&mut self, offset: usize, kind: EditKind, coalescible: bool) {
        self.set_cursor_without_history(offset);
        self.last_edit = coalescible.then(|| LastEdit {
            kind,
            at: Instant::now(),
            cursor: offset,
        });
    }

    fn set_cursor_without_history(&mut self, offset: usize) {
        let offset = self.clamp_boundary(offset);
        self.selected = offset..offset;
        self.selection_reversed = false;
    }

    fn commit_composition_history(&mut self) {
        let Some(base) = self.composition_base.take() else {
            return;
        };
        if base.content != self.content {
            self.push_undo(base);
            self.redo.clear();
        }
        self.last_edit = None;
    }

    fn push_undo(&mut self, revision: TextRevision) {
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(revision);
    }
}

fn take_graphemes(value: &str, count: usize) -> String {
    value.graphemes(true).take(count).collect()
}

fn utf16_range_in(value: &str, range: &Range<usize>) -> Range<usize> {
    fn offset(value: &str, target: usize) -> usize {
        let mut utf8 = 0;
        let mut utf16 = 0;
        for character in value.chars() {
            if utf16 >= target {
                break;
            }
            utf16 += character.len_utf16();
            utf8 += character.len_utf8();
        }
        utf8
    }
    offset(value, range.start)..offset(value, range.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_length_counts_extended_graphemes_and_credits_selection() {
        let mut buffer = TextBuffer::new("a👨‍👩‍👧‍👦b");
        buffer.set_max_length(Some(4));
        assert_eq!(buffer.grapheme_count(), 3);
        buffer.move_to(1);
        buffer.select_to("a👨‍👩‍👧‍👦".len());
        buffer.replace(None, "🇨🇳x");
        assert_eq!(buffer.content(), "a🇨🇳xb");
        assert_eq!(buffer.grapheme_count(), 4);
    }

    #[test]
    fn oversized_paste_truncates_at_grapheme_boundary() {
        let mut buffer = TextBuffer::new("ab");
        buffer.set_max_length(Some(4));
        buffer.replace(None, "👩🏽‍💻cd");
        assert_eq!(buffer.content(), "ab👩🏽‍💻c");
        assert_eq!(buffer.grapheme_count(), 4);
    }

    #[test]
    fn ime_can_temporarily_exceed_then_clamps_only_marked_text() {
        let mut buffer = TextBuffer::new("ab-tail");
        buffer.set_max_length(Some(8));
        buffer.move_to(2);
        buffer.replace_and_mark(None, "中文输入", Some(4..4));
        assert_eq!(buffer.content(), "ab中文输入-tail");
        assert!(buffer.unmark());
        assert_eq!(buffer.content(), "ab中-tail");
        assert_eq!(buffer.grapheme_count(), 8);
    }

    #[test]
    fn physical_line_boundaries_are_stable() {
        let buffer = TextBuffer::new("one\ntwo\nthree");
        assert_eq!(buffer.line_start(6), 4);
        assert_eq!(buffer.line_end(6), 7);
    }

    #[test]
    fn utf16_offsets_never_split_extended_graphemes() {
        let buffer = TextBuffer::new("e\u{301}👩🏽‍💻");
        assert_eq!(buffer.offset_from_utf16(1), 0);
        assert_eq!(buffer.offset_from_utf16(2), "e\u{301}".len());
        assert!(
            buffer
                .content()
                .grapheme_indices(true)
                .any(|(index, _)| index == buffer.offset_from_utf16(4))
        );
    }

    #[test]
    fn undo_redo_coalesces_typing_and_restores_selection() {
        let mut buffer = TextBuffer::new("");
        buffer.replace(None, "a");
        buffer.replace(None, "b");
        buffer.replace(None, "c");
        assert_eq!(buffer.content(), "abc");
        assert!(buffer.undo());
        assert_eq!(buffer.content(), "");
        assert_eq!(buffer.selection(), &(0..0));
        assert!(buffer.redo());
        assert_eq!(buffer.content(), "abc");
        assert_eq!(buffer.selection(), &(3..3));
    }

    #[test]
    fn ime_composition_is_one_revision_and_external_control_resets_history() {
        let mut buffer = TextBuffer::new("start");
        buffer.move_to(buffer.content().len());
        buffer.replace_and_mark(None, "に", Some(1..1));
        buffer.replace_and_mark(None, "日本", Some(2..2));
        buffer.unmark();
        assert_eq!(buffer.content(), "start日本");
        assert!(buffer.undo());
        assert_eq!(buffer.content(), "start");
        assert!(buffer.redo());
        assert_eq!(buffer.content(), "start日本");
        buffer.set_controlled("external");
        assert!(!buffer.can_undo());
        assert!(!buffer.can_redo());
    }
}
