//! Native CommonMark/GFM preparation. The source and model never enter Rhai.

use std::ops::Range;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::{
    DocumentDescriptor, DocumentLimits, DocumentSource, SyntaxRegistry, SyntaxTokenKind,
    prepare_document_with_limits,
};

/// Maximum UTF-8 bytes shaped in one ordinary Markdown virtual row.
pub const MARKDOWN_SEGMENT_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Mark {
    Strong,
    Emphasis,
    Strike,
    Code,
    Link {
        href: String,
        label: String,
        first: bool,
    },
    Image(String),
    Syntax(SyntaxTokenKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InlineMark {
    pub range: Range<usize>,
    pub mark: Mark,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MarkdownText {
    pub text: String,
    pub marks: Vec<InlineMark>,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TableCell {
    pub text: MarkdownText,
    pub fragments: Arc<Vec<MarkdownText>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BlockKind {
    Paragraph,
    Heading(u8),
    Code {
        language: String,
        source: Arc<str>,
        first: bool,
    },
    Rule,
    Table {
        alignments: Vec<Alignment>,
        rows: Arc<Vec<Vec<TableCell>>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MarkdownBlock {
    pub kind: BlockKind,
    pub text: MarkdownText,
    pub indent: usize,
    pub quote: usize,
    pub continuation: bool,
    pub list_item: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedMarkdown {
    pub source: Arc<str>,
    pub plain: Arc<str>,
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Default)]
struct Builder {
    blocks: Vec<MarkdownBlock>,
    current: MarkdownText,
    kind: Option<BlockKind>,
    marks: Vec<Mark>,
    indent: usize,
    quote: usize,
    lists: Vec<Option<u64>>,
    table: Option<(Vec<Alignment>, Vec<Vec<MarkdownText>>)>,
    row: Vec<MarkdownText>,
    image: Option<(String, String)>,
    links: Vec<(String, usize)>,
}

impl Builder {
    fn append(&mut self, value: &str) {
        if let Some((_, alt)) = &mut self.image {
            alt.push_str(value);
            return;
        }
        let start = self.current.text.len();
        self.current.text.push_str(value);
        let end = self.current.text.len();
        for mark in &self.marks {
            self.current.marks.push(InlineMark {
                range: start..end,
                mark: mark.clone(),
            });
        }
    }

    fn flush(&mut self) {
        if self.current.text.is_empty() {
            self.kind = None;
            return;
        }
        let kind = self.kind.take().unwrap_or(BlockKind::Paragraph);
        let current = std::mem::take(&mut self.current);
        for range in segments(&current.text) {
            self.blocks.push(MarkdownBlock {
                kind: match &kind {
                    BlockKind::Code {
                        language,
                        source,
                        first,
                    } => BlockKind::Code {
                        language: language.clone(),
                        source: source.clone(),
                        first: *first && range.start == 0,
                    },
                    other => other.clone(),
                },
                text: slice_text(&current, range.clone()),
                indent: self.indent,
                quote: self.quote,
                continuation: range.start > 0,
                list_item: !self.lists.is_empty(),
            });
        }
    }
}

fn segments(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + MARKDOWN_SEGMENT_BYTES).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end < text.len()
            && let Some(newline) = text[start..end].rfind('\n')
            && newline > 0
        {
            end = start + newline + 1;
        }
        result.push(start..end);
        start = end;
    }
    result
}

fn slice_text(text: &MarkdownText, range: Range<usize>) -> MarkdownText {
    MarkdownText {
        text: text.text[range.clone()].to_owned(),
        marks: text
            .marks
            .iter()
            .filter_map(|mark| {
                let start = mark.range.start.max(range.start);
                let end = mark.range.end.min(range.end);
                (start < end).then(|| InlineMark {
                    range: start - range.start..end - range.start,
                    mark: match &mark.mark {
                        Mark::Link { href, label, first } => Mark::Link {
                            href: href.clone(),
                            label: label.clone(),
                            first: *first && start == mark.range.start,
                        },
                        other => other.clone(),
                    },
                })
            })
            .collect(),
        range: 0..0,
    }
}

/// Parse complete semantics rather than assuming an append changes only the
/// final block: reference definitions, setext headings and list tightness can
/// alter earlier blocks. The native consumer reuses equal presentation blocks.
#[allow(clippy::too_many_lines)] // Exhaustive parser event-to-native-model translation.
pub(crate) fn prepare_markdown(
    source: &DocumentSource,
    syntaxes: &SyntaxRegistry,
    limits: DocumentLimits,
    canceled: &AtomicBool,
    alive: &AtomicBool,
) -> Result<PreparedMarkdown, String> {
    if canceled.load(Ordering::Acquire) || !alive.load(Ordering::Acquire) {
        return Err("Markdown preparation canceled".to_owned());
    }
    let source = source.text();
    if source.len() > limits.max_document_bytes {
        return Err(format!(
            "Markdown exceeds {} byte limit",
            limits.max_document_bytes
        ));
    }
    if source.lines().count() > limits.max_document_lines {
        return Err(format!(
            "Markdown exceeds {} line limit",
            limits.max_document_lines
        ));
    }
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    let mut b = Builder::default();
    for event in Parser::new_ext(&source, options) {
        if canceled.load(Ordering::Acquire) || !alive.load(Ordering::Acquire) {
            return Err("Markdown preparation canceled".to_owned());
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if b.table.is_none() && b.kind.is_some() {
                        b.flush();
                    }
                    b.kind = Some(BlockKind::Paragraph);
                }
                Tag::Heading { level, .. } => {
                    b.flush();
                    b.kind = Some(BlockKind::Heading(level as u8));
                }
                Tag::BlockQuote(_) => {
                    b.flush();
                    b.quote += 1;
                }
                Tag::List(start) => {
                    b.flush();
                    b.lists.push(start);
                    b.indent = b.lists.len().saturating_sub(1);
                }
                Tag::Item => {
                    b.flush();
                    let prefix = if let Some(Some(number)) = b.lists.last_mut() {
                        let prefix = format!("{number}. ");
                        *number += 1;
                        prefix
                    } else {
                        "• ".to_owned()
                    };
                    b.append(&prefix);
                }
                Tag::CodeBlock(kind) => {
                    b.flush();
                    let language = match kind {
                        CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    b.kind = Some(BlockKind::Code {
                        language,
                        source: Arc::from(""),
                        first: true,
                    });
                }
                Tag::HtmlBlock => {
                    b.flush();
                    b.kind = Some(BlockKind::Paragraph);
                }
                Tag::Strong => b.marks.push(Mark::Strong),
                Tag::Emphasis => b.marks.push(Mark::Emphasis),
                Tag::Strikethrough => b.marks.push(Mark::Strike),
                Tag::Link { dest_url, .. } => {
                    b.links.push((dest_url.into_string(), b.current.text.len()));
                }
                Tag::Image { dest_url, .. } => {
                    b.image = Some((dest_url.into_string(), String::new()));
                }
                Tag::Table(alignments) => {
                    if alignments.len() > limits.max_markdown_table_columns {
                        return Err(format!(
                            "Markdown table has {} columns, exceeding the {}-column budget",
                            alignments.len(),
                            limits.max_markdown_table_columns
                        ));
                    }
                    b.flush();
                    b.table = Some((alignments, Vec::new()));
                }
                Tag::TableCell => {
                    b.current = MarkdownText::default();
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::HtmlBlock | TagEnd::Item
                    if b.table.is_none() =>
                {
                    b.flush();
                }
                TagEnd::BlockQuote(_) => {
                    b.flush();
                    b.quote = b.quote.saturating_sub(1);
                }
                TagEnd::List(_) => {
                    b.flush();
                    b.lists.pop();
                    b.indent = b.lists.len().saturating_sub(1);
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                    b.marks.pop();
                }
                TagEnd::Link => {
                    if let Some((href, start)) = b.links.pop() {
                        b.current.marks.push(InlineMark {
                            range: start..b.current.text.len(),
                            mark: Mark::Link {
                                href,
                                label: b.current.text[start..].to_owned(),
                                first: true,
                            },
                        });
                    }
                }
                TagEnd::CodeBlock => {
                    let code: Arc<str> = Arc::from(b.current.text.as_str());
                    if let Some(BlockKind::Code {
                        language, source, ..
                    }) = &mut b.kind
                    {
                        *source = code.clone();
                        let descriptor = DocumentDescriptor {
                            source: DocumentSource::from(code.to_string()),
                            label: "Code block".to_owned(),
                            file_name: None,
                            language: Some(language.clone()),
                        };
                        // Shared language registry (including Host grammars and Rhai).
                        if let Ok(prepared) =
                            prepare_document_with_limits(&descriptor, syntaxes, limits)
                        {
                            for line in prepared.lines() {
                                for syntax in &line.syntax {
                                    b.current.marks.push(InlineMark {
                                        range: line.range.start + syntax.range.start
                                            ..line.range.start + syntax.range.end,
                                        mark: Mark::Syntax(syntax.kind),
                                    });
                                }
                            }
                        }
                    }
                    b.flush();
                }
                TagEnd::Image => {
                    if let Some((destination, alt)) = b.image.take() {
                        let start = b.current.text.len();
                        b.append(if alt.is_empty() { "Image" } else { &alt });
                        b.current.marks.push(InlineMark {
                            range: start..b.current.text.len(),
                            mark: Mark::Image(destination),
                        });
                    }
                }
                TagEnd::TableCell => {
                    b.row.push(std::mem::take(&mut b.current));
                }
                TagEnd::TableHead | TagEnd::TableRow => {
                    if let Some((_, rows)) = &mut b.table {
                        rows.push(std::mem::take(&mut b.row));
                    }
                }
                TagEnd::Table => {
                    if let Some((alignments, rows)) = b.table.take() {
                        let rows = rows
                            .into_iter()
                            .map(|row| {
                                row.into_iter()
                                    .map(|text| TableCell {
                                        text,
                                        fragments: Arc::new(Vec::new()),
                                    })
                                    .collect()
                            })
                            .collect();
                        b.blocks.push(MarkdownBlock {
                            kind: BlockKind::Table {
                                alignments,
                                rows: Arc::new(rows),
                            },
                            text: MarkdownText::default(),
                            indent: b.indent,
                            quote: b.quote,
                            continuation: false,
                            list_item: !b.lists.is_empty(),
                        });
                    }
                }
                _ => {}
            },
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => b.append(&text),
            Event::Code(text) => {
                b.marks.push(Mark::Code);
                b.append(&text);
                b.marks.pop();
            }
            Event::SoftBreak => b.append(" "),
            Event::HardBreak => b.append("\n"),
            Event::Rule => {
                b.flush();
                b.blocks.push(MarkdownBlock {
                    kind: BlockKind::Rule,
                    text: MarkdownText::default(),
                    indent: b.indent,
                    quote: b.quote,
                    continuation: false,
                    list_item: !b.lists.is_empty(),
                });
            }
            Event::TaskListMarker(checked) => {
                // Replace the list marker with the visible read-only task marker.
                if b.current.text == "• " {
                    b.current.text.clear();
                }
                b.append(if checked { "☑ " } else { "☐ " });
            }
            _ => {}
        }
    }
    b.flush();
    let mut plain = String::new();
    for block in &mut b.blocks {
        if !block.continuation && !plain.is_empty() {
            if !plain.ends_with('\n') {
                plain.push('\n');
            }
            plain.push('\n');
        }
        if let BlockKind::Table { rows, .. } = &mut block.kind {
            for row in Arc::make_mut(rows) {
                for (column, cell) in row.iter_mut().enumerate() {
                    if column > 0 {
                        plain.push('\t');
                    }
                    add_autolinks(&mut cell.text);
                    append_plain(&mut cell.text, &mut plain);
                    cell.fragments = Arc::new(
                        segments(&cell.text.text)
                            .into_iter()
                            .map(|range| {
                                let mut fragment = slice_text(&cell.text, range.clone());
                                fragment.range = cell.text.range.start + range.start
                                    ..cell.text.range.start + range.end;
                                fragment
                            })
                            .collect(),
                    );
                }
                plain.push('\n');
            }
        } else {
            if !matches!(block.kind, BlockKind::Code { .. }) {
                add_autolinks(&mut block.text);
            }
            append_plain(&mut block.text, &mut plain);
        }
    }
    Ok(PreparedMarkdown {
        source,
        plain: Arc::from(plain),
        blocks: b.blocks,
    })
}

fn append_plain(text: &mut MarkdownText, plain: &mut String) {
    text.range = plain.len()..plain.len() + text.text.len();
    plain.push_str(&text.text);
}

fn add_autolinks(text: &mut MarkdownText) {
    // GFM bare URL/email extension, distinct from CommonMark's angle autolinks.
    static AUTOLINK: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let regex = AUTOLINK.get_or_init(|| regex::Regex::new(r"(?:https?://|www\.)[^\s<>]+|[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+").expect("static autolink regex"));
    for found in regex.find_iter(&text.text) {
        if text.marks.iter().any(|mark| {
            mark.range.start < found.end()
                && mark.range.end > found.start()
                && matches!(mark.mark, Mark::Link { .. } | Mark::Code | Mark::Image(_))
        }) {
            continue;
        }
        let mut value = found
            .as_str()
            .trim_end_matches(['.', ',', ':', ';', '!', '?', '*', '_', '~']);
        while value.ends_with(')') && value.matches(')').count() > value.matches('(').count() {
            value = &value[..value.len() - 1];
        }
        let href = if value.starts_with("www.") {
            format!("https://{value}")
        } else if !value.contains("://") {
            format!("mailto:{value}")
        } else {
            value.to_owned()
        };
        text.marks.push(InlineMark {
            range: found.start()..found.start() + value.len(),
            mark: Mark::Link {
                href,
                label: value.to_owned(),
                first: true,
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> PreparedMarkdown {
        prepare_markdown(
            &text.to_owned().into(),
            &SyntaxRegistry::new(),
            DocumentLimits::default(),
            &AtomicBool::new(false),
            &AtomicBool::new(true),
        )
        .unwrap()
    }

    #[test]
    fn gfm_and_commonmark_stay_native_and_visible_copy_is_not_source() {
        let result = parse(
            "# Heading\n\n~~old~~ **new** [link](https://example.com/a_(b))\n\n- [x] done\n  - nested\n\n> quote\n\n| Left | Right |\n| :--- | ---: |\n| 1 | 2 |\n",
        );
        assert!(result.plain.contains("old new link"));
        assert!(!result.plain.contains("**"));
        assert!(result.blocks.iter().any(|block| block.indent == 1));
        assert!(result.blocks.iter().any(|block| block.quote == 1));
        assert!(result.blocks.iter().any(|block| matches!(&block.kind, BlockKind::Table { alignments, .. } if alignments == &[Alignment::Left, Alignment::Right])));
        assert!(result.blocks.iter().flat_map(|block| &block.text.marks).any(|mark| matches!(&mark.mark, Mark::Link { href, .. } if href == "https://example.com/a_(b)")));
    }

    #[test]
    fn append_reference_definition_changes_earlier_block() {
        let before = parse("[earlier][ref]\n\nlast paragraph\n");
        let after = parse("[earlier][ref]\n\nlast paragraph\n\n[ref]: https://example.com\n");
        assert_ne!(before.blocks[0], after.blocks[0]);
        assert!(after.plain.starts_with("earlier"));
    }

    #[test]
    fn enormous_unicode_and_code_are_bounded_and_syntax_is_shared() {
        let code = format!("```rhai\nfn view() {{ 42 }}\n{}\n```", "世界".repeat(9000));
        let result = parse(&code);
        assert!(
            result
                .blocks
                .iter()
                .all(|block| block.text.text.len() <= MARKDOWN_SEGMENT_BYTES)
        );
        assert!(
            result
                .blocks
                .iter()
                .flat_map(|block| &block.text.marks)
                .any(|mark| matches!(mark.mark, Mark::Syntax(SyntaxTokenKind::Keyword)))
        );
        assert!(
            matches!(&result.blocks[0].kind, BlockKind::Code { source, .. } if !source.contains("```"))
        );
    }

    #[test]
    fn html_is_literal_images_are_alt_and_autolinks_balance_parentheses() {
        let result = parse(
            "<script>alert(1)</script>\n\n![Alt](https://no-fetch.invalid/image.png)\n\nhttps://example.com/a_(b). me@example.com\n",
        );
        assert!(result.plain.contains("<script>alert(1)</script>"));
        assert!(result.plain.contains("Alt"));
        assert!(result.blocks.iter().flat_map(|block| &block.text.marks).any(|mark| matches!(&mark.mark, Mark::Link { href, .. } if href == "https://example.com/a_(b)")));
    }

    #[test]
    fn cancellation_and_limits_are_checked() {
        assert!(
            prepare_markdown(
                &"hello".to_owned().into(),
                &SyntaxRegistry::new(),
                DocumentLimits::default(),
                &AtomicBool::new(true),
                &AtomicBool::new(true)
            )
            .is_err()
        );
        assert!(
            prepare_markdown(
                &"hello".to_owned().into(),
                &SyntaxRegistry::new(),
                DocumentLimits {
                    max_document_bytes: 2,
                    ..DocumentLimits::default()
                },
                &AtomicBool::new(false),
                &AtomicBool::new(true)
            )
            .is_err()
        );
    }

    #[test]
    fn formatted_link_is_one_semantic_span_with_its_complete_label() {
        let result = parse("[**A** B](https://example.com)\n");
        let links = result.blocks[0]
            .text
            .marks
            .iter()
            .filter(|mark| matches!(mark.mark, Mark::Link { .. }))
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].range, 0..3);
        assert!(matches!(&links[0].mark, Mark::Link { label, first: true, .. } if label == "A B"));
    }

    #[test]
    fn inline_images_preserve_heading_and_table_cell_structure() {
        let result = parse(
            "# A ![B](host-image) C\n\n| Left | Right |\n| --- | --- |\n| A ![B](host-image) C | D |\n",
        );
        assert!(matches!(result.blocks[0].kind, BlockKind::Heading(1)));
        assert_eq!(result.blocks[0].text.text, "A B C");
        assert_eq!(result.blocks.len(), 2);
        let BlockKind::Table { rows, .. } = &result.blocks[1].kind else {
            panic!("table");
        };
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][0].text.text, "A B C");
        assert_eq!(rows[1][1].text.text, "D");
        assert!(rows[1][0].text.marks.iter().any(
            |mark| matches!(&mark.mark, Mark::Image(destination) if destination == "host-image")
        ));
    }

    #[test]
    fn huge_cells_keep_logical_rows_and_row_major_plain_copy() {
        let long = "世界".repeat(3000);
        let result = parse(&format!(
            "| Left | Right |\n| --- | --- |\n| {long} | right-cell |\n"
        ));
        let BlockKind::Table { rows, .. } = &result.blocks[0].kind else {
            panic!("table");
        };
        assert_eq!(rows.len(), 2);
        assert!(rows[1][0].fragments.len() > 1);
        assert!(
            rows[1][0]
                .fragments
                .iter()
                .all(|fragment| fragment.text.len() <= MARKDOWN_SEGMENT_BYTES)
        );
        assert!(result.plain.contains(&format!("{long}\tright-cell")));
        for fragment in rows[1][0].fragments.iter() {
            assert_eq!(&result.plain[fragment.range.clone()], fragment.text);
        }
    }

    #[test]
    fn arbitrarily_wide_tables_fail_before_native_layout() {
        let columns = 129;
        let source = format!(
            "|{}|\n|{}|\n",
            vec!["cell"; columns].join("|"),
            vec!["---"; columns].join("|")
        );
        let error = prepare_markdown(
            &source.into(),
            &SyntaxRegistry::new(),
            DocumentLimits::default(),
            &AtomicBool::new(false),
            &AtomicBool::new(true),
        )
        .unwrap_err();
        assert!(error.contains("129 columns"));
        assert!(
            prepare_markdown(
                &"content".to_owned().into(),
                &SyntaxRegistry::new(),
                DocumentLimits::default(),
                &AtomicBool::new(false),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}
