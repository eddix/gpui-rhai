//! Immutable Markdown preparation independent of GPUI element lifetime.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, atomic::AtomicBool};

use syntect::parsing::SyntaxSet;

use crate::markdown::{BlockKind, MarkdownBlock, PreparedMarkdown as MarkdownModel};
use crate::{DocumentError, DocumentLimits, DocumentSource, SyntaxRegistry};

/// Opaque, process-local identity of one complete Markdown preparation.
///
/// Equality includes source kind, native identity/revision, exact source bytes,
/// the immutable syntax snapshot and resource limits. A text hash accelerates
/// lookup but is never sufficient for a hit. Keys retain the source/syntax
/// snapshots; they are not persistent cache IDs or permission to skip limits.
#[derive(Clone)]
pub struct MarkdownPreparationKey {
    source: DocumentSource,
    syntax: Arc<SyntaxSet>,
    limits: DocumentLimits,
    source_hash: u64,
}

impl PartialEq for MarkdownPreparationKey {
    fn eq(&self, other: &Self) -> bool {
        self.source_hash == other.source_hash
            && self.limits == other.limits
            && Arc::ptr_eq(&self.syntax, &other.syntax)
            && self.source == other.source
    }
}
impl Eq for MarkdownPreparationKey {}
impl Hash for MarkdownPreparationKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_hash.hash(state);
        self.limits.hash(state);
        Arc::as_ptr(&self.syntax).hash(state);
    }
}
impl fmt::Debug for MarkdownPreparationKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Source text, links and syntax contents may contain sensitive data.
        formatter
            .debug_struct("MarkdownPreparationKey")
            .field("bytes", &self.source.text().len())
            .finish_non_exhaustive()
    }
}

/// A shareable prepared model, not a GPUI Entity, layout or rendered texture.
///
/// It contains no callbacks, focus/selection state, measurement or window data.
/// Dropping a Viewer therefore need not discard this model. Applications may
/// retain it in their own cache and return it to another preparation request.
#[derive(Clone)]
pub struct PreparedMarkdown {
    key: MarkdownPreparationKey,
    model: Arc<MarkdownModel>,
    estimated_bytes: usize,
}

impl fmt::Debug for PreparedMarkdown {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedMarkdown")
            .field("key", &self.key)
            .field("estimated_bytes", &self.estimated_bytes)
            .finish_non_exhaustive()
    }
}

impl PreparedMarkdown {
    #[must_use]
    pub fn key(&self) -> &MarkdownPreparationKey {
        &self.key
    }

    /// Conservative model storage estimate for Host budget accounting.
    ///
    /// Counts source/text, block buffers, marks and table fragments. Shared code
    /// text may be counted more than once. Excludes allocator overhead, shared
    /// syntax definitions and live Viewer/UI resources; this is not process RSS.
    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        self.estimated_bytes
    }

    /// The original source. Callers choose explicitly whether to expose it.
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.model.source
    }

    /// Plain text produced by the Markdown parser.
    #[must_use]
    pub fn plain_text(&self) -> &str {
        &self.model.plain
    }

    pub(crate) fn model(&self) -> Arc<MarkdownModel> {
        self.model.clone()
    }
}

/// Host-owned storage policy for immutable Markdown preparations.
///
/// SDK Viewers call these methods on the foreground, so implementations must be
/// fast/nonblocking (no parsing, disk/network I/O or GPUI updates). Public
/// [`MarkdownPreparation::prepare`] can run on a background executor for Host
/// prefetch. Use internal synchronization if both paths share a cache.
///
/// Capacity, byte accounting, admission, eviction, sharing and clearing belong
/// to the application. The SDK neither creates a global cache nor retains an
/// offscreen Entity. An oversized entry may simply be declined by `insert`.
/// `get` must not remove the entry; cloning a handle does not clone its model.
/// A mismatched result is treated as a miss by the official Viewer.
pub trait MarkdownPreparationCache: Send + Sync + 'static {
    fn get(&self, key: &MarkdownPreparationKey) -> Option<PreparedMarkdown>;
    fn insert(&self, prepared: PreparedMarkdown);
}

/// One frozen source/configuration request, usable without a Viewer or GPUI.
#[derive(Clone, Debug)]
pub struct MarkdownPreparation {
    key: MarkdownPreparationKey,
}

impl MarkdownPreparation {
    /// Capture a fixed source, syntax snapshot and validated limits for this
    /// request. Later registry/config updates cannot change its meaning.
    ///
    /// # Errors
    /// Returns [`DocumentError::InvalidLimits`] when any resource limit is zero.
    pub fn new(
        source: DocumentSource,
        syntaxes: &SyntaxRegistry,
        limits: DocumentLimits,
    ) -> Result<Self, DocumentError> {
        limits.validate()?;
        let mut hasher = DefaultHasher::new();
        match &source {
            DocumentSource::Inline(_) => 0_u8.hash(&mut hasher),
            DocumentSource::Native(document) => {
                1_u8.hash(&mut hasher);
                document.identity().hash(&mut hasher);
                document.revision().hash(&mut hasher);
            }
        }
        source.text().hash(&mut hasher);
        Ok(Self {
            key: MarkdownPreparationKey {
                source,
                syntax: syntaxes.snapshot(),
                limits,
                source_hash: hasher.finish(),
            },
        })
    }

    #[must_use]
    pub fn key(&self) -> &MarkdownPreparationKey {
        &self.key
    }

    /// Prepare without touching GPUI, Rhai or an implicit cache. May run on a
    /// background executor. A Host may insert the returned handle in its cache.
    ///
    /// # Errors
    /// Returns a cancellation, resource-limit or parser error.
    pub fn prepare(&self, canceled: &AtomicBool) -> Result<PreparedMarkdown, String> {
        self.prepare_with_liveness(canceled, &AtomicBool::new(true))
    }

    pub(crate) fn prepare_with_liveness(
        &self,
        canceled: &AtomicBool,
        alive: &AtomicBool,
    ) -> Result<PreparedMarkdown, String> {
        let model = crate::markdown::prepare_markdown(
            &self.key.source,
            &SyntaxRegistry::from_snapshot(self.key.syntax.clone()),
            self.key.limits,
            canceled,
            alive,
        )?;
        let estimated_bytes = model_bytes(&model)
            .saturating_add(std::mem::size_of::<PreparedMarkdown>())
            .saturating_add(self.key.source.identity().len());
        Ok(PreparedMarkdown {
            key: self.key.clone(),
            model: Arc::new(model),
            estimated_bytes,
        })
    }
}

fn text_bytes(text: &crate::markdown::MarkdownText) -> usize {
    text.text
        .capacity()
        .saturating_add(
            text.marks
                .capacity()
                .saturating_mul(std::mem::size_of::<crate::markdown::InlineMark>()),
        )
        .saturating_add(
            text.marks
                .iter()
                .map(|mark| match &mark.mark {
                    crate::markdown::Mark::Link { href, label, .. } => {
                        href.capacity().saturating_add(label.capacity())
                    }
                    crate::markdown::Mark::Image(href) => href.capacity(),
                    _ => 0,
                })
                .sum::<usize>(),
        )
}

fn model_bytes(model: &MarkdownModel) -> usize {
    let mut total = std::mem::size_of::<MarkdownModel>()
        .saturating_add(model.source.len())
        .saturating_add(model.plain.len())
        .saturating_add(
            model
                .blocks
                .capacity()
                .saturating_mul(std::mem::size_of::<MarkdownBlock>()),
        );
    for block in &model.blocks {
        total = total.saturating_add(text_bytes(&block.text));
        match &block.kind {
            BlockKind::Code {
                language, source, ..
            } => {
                total = total
                    .saturating_add(language.capacity())
                    .saturating_add(source.len());
            }
            BlockKind::Table { alignments, rows } => {
                total = total.saturating_add(
                    alignments
                        .capacity()
                        .saturating_mul(std::mem::size_of::<pulldown_cmark::Alignment>()),
                );
                total = total.saturating_add(
                    rows.capacity()
                        .saturating_mul(std::mem::size_of::<Vec<crate::markdown::TableCell>>()),
                );
                for row in rows.iter() {
                    total = total.saturating_add(
                        row.capacity()
                            .saturating_mul(std::mem::size_of::<crate::markdown::TableCell>()),
                    );
                    for cell in row {
                        total = total.saturating_add(text_bytes(&cell.text));
                        total =
                            total.saturating_add(cell.fragments.capacity().saturating_mul(
                                std::mem::size_of::<crate::markdown::MarkdownText>(),
                            ));
                        for fragment in cell.fragments.iter() {
                            total = total.saturating_add(text_bytes(fragment));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NativeTextDocument;

    fn request(source: impl Into<DocumentSource>) -> MarkdownPreparation {
        MarkdownPreparation::new(
            source.into(),
            &SyntaxRegistry::new(),
            DocumentLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn models_are_background_safe_and_independent_of_viewers() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<MarkdownPreparation>();
        send_sync::<PreparedMarkdown>();
        send_sync::<MarkdownPreparationKey>();
        let request = request("# Heading\n\n**Hello**\n\n```rhai\nlet a = 1;\n```\n".to_owned());
        let prepared = std::thread::spawn(move || request.prepare(&AtomicBool::new(false)))
            .join()
            .unwrap()
            .unwrap();
        assert!(prepared.plain_text().contains("Heading"));
        assert!(prepared.plain_text().contains("Hello"));
        assert!(
            prepared.estimated_bytes()
                >= prepared.source_text().len() + prepared.plain_text().len()
        );
        assert!(Arc::ptr_eq(&prepared.model(), &prepared.clone().model()));
    }

    #[test]
    fn complete_source_not_only_identity_or_revision_controls_reuse() {
        let native = |text: &str, revision| {
            NativeTextDocument::new("reply", revision, Arc::<str>::from(text)).unwrap()
        };
        let a = request(native("one", 7));
        assert_eq!(a.key(), request(native("one", 7)).key());
        assert_ne!(a.key(), request(native("two", 7)).key());
        assert_ne!(a.key(), request(native("one", 8)).key());
        assert_ne!(a.key(), request("one".to_owned()).key());
        assert_ne!(
            request("one".to_owned()).key(),
            request("two".to_owned()).key()
        );
        assert_eq!(
            request("one".to_owned()).key(),
            request("one".to_owned()).key()
        );
    }

    #[test]
    fn a_hash_collision_is_not_a_cache_hit() {
        let a = request("one".to_owned());
        let mut b = request("two".to_owned());
        b.key.source_hash = a.key.source_hash;
        assert_ne!(a.key(), b.key());
    }

    #[test]
    fn syntax_is_frozen_for_the_entire_preparation() {
        let syntax = SyntaxRegistry::new();
        let old = MarkdownPreparation::new(
            "```special\nhello\n```".to_owned().into(),
            &syntax,
            DocumentLimits::default(),
        )
        .unwrap();
        syntax.register_sublime_syntax("%YAML 1.2\n---\nname: Special\nfile_extensions: [special]\nscope: source.special\ncontexts:\n  main:\n    - match: hello\n      scope: keyword.control.special\n").unwrap();
        let new =
            MarkdownPreparation::new(old.key.source.clone(), &syntax, DocumentLimits::default())
                .unwrap();
        assert_ne!(old.key(), new.key());
        let old_model = old.prepare(&AtomicBool::new(false)).unwrap();
        let new_model = new.prepare(&AtomicBool::new(false)).unwrap();
        let syntax_marks = |model: &PreparedMarkdown| {
            model
                .model
                .blocks
                .iter()
                .flat_map(|b| &b.text.marks)
                .filter(|m| matches!(m.mark, crate::markdown::Mark::Syntax(_)))
                .count()
        };
        assert_eq!(syntax_marks(&old_model), 0);
        assert!(syntax_marks(&new_model) > 0);
    }

    #[test]
    fn stricter_limits_invalidate_and_cannot_be_bypassed_by_cached_models() {
        let old = request("# Heading".to_owned());
        let limits = DocumentLimits {
            max_document_bytes: 2,
            ..DocumentLimits::default()
        };
        let strict =
            MarkdownPreparation::new(old.key.source.clone(), &SyntaxRegistry::new(), limits)
                .unwrap();
        assert_ne!(old.key(), strict.key());
        assert!(strict.prepare(&AtomicBool::new(false)).is_err());
        assert!(
            old.prepare(&AtomicBool::new(true))
                .unwrap_err()
                .contains("canceled")
        );
        assert!(
            MarkdownPreparation::new(
                old.key.source.clone(),
                &SyntaxRegistry::new(),
                DocumentLimits {
                    max_markdown_table_columns: 0,
                    ..limits
                }
            )
            .is_err()
        );
    }

    #[test]
    fn debug_does_not_expose_source_text() {
        let prepared = request("secret-token\n\nhttps://private.example".to_owned())
            .prepare(&AtomicBool::new(false))
            .unwrap();
        let debug = format!("{prepared:?}");
        assert!(!debug.contains("secret-token"));
        assert!(!debug.contains("private.example"));
    }
}
