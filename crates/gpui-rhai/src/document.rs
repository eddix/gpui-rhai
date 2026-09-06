//! Native immutable text documents, syntax tokenization, and two-way diff models.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::str::FromStr as _;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use rhai::{CustomType, Dynamic, Engine, FuncRegistration, ImmutableString, TypeBuilder};
use similar::{Algorithm, DiffOp, capture_diff_slices_deadline};
use syntect::easy::ScopeRangeIterator;
use syntect::highlighting::ScopeSelectors;
use syntect::parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxSet};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation as _;

use crate::ComponentInstancePath;

pub const DEFAULT_DOCUMENT_MAX_BYTES: usize = 10 * 1024 * 1024;
pub const DEFAULT_DOCUMENT_MAX_LINES: usize = 500_000;
pub const DEFAULT_DIFF_MAX_HUNKS: usize = 100_000;
pub const DEFAULT_DIFF_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_INLINE_DIFF_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentLimits {
    pub max_document_bytes: usize,
    pub max_document_lines: usize,
    pub max_diff_total_bytes: usize,
    pub max_diff_hunks: usize,
    pub diff_timeout: Duration,
}

impl Default for DocumentLimits {
    fn default() -> Self {
        Self {
            max_document_bytes: DEFAULT_DOCUMENT_MAX_BYTES,
            max_document_lines: DEFAULT_DOCUMENT_MAX_LINES,
            max_diff_total_bytes: DEFAULT_DOCUMENT_MAX_BYTES * 2,
            max_diff_hunks: DEFAULT_DIFF_MAX_HUNKS,
            diff_timeout: DEFAULT_DIFF_TIMEOUT,
        }
    }
}

impl DocumentLimits {
    /// Validate Host-configured resource limits.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::InvalidLimits`] when any limit is zero.
    pub fn validate(self) -> Result<(), DocumentError> {
        if self.max_document_bytes == 0
            || self.max_document_lines == 0
            || self.max_diff_total_bytes == 0
            || self.max_diff_hunks == 0
            || self.diff_timeout.is_zero()
        {
            Err(DocumentError::InvalidLimits)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct DocumentRuntimeConfig {
    limits: Arc<RwLock<DocumentLimits>>,
}

impl Default for DocumentRuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentRuntimeConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(DocumentLimits::default())),
        }
    }

    #[must_use]
    pub fn limits(&self) -> DocumentLimits {
        self.limits
            .read()
            .map_or_else(|poisoned| *poisoned.into_inner(), |limits| *limits)
    }

    /// Replace limits used by future document and diff jobs.
    ///
    /// # Errors
    ///
    /// Returns an invalid-limit or poisoned-registry error.
    pub fn set_limits(&self, limits: DocumentLimits) -> Result<(), DocumentError> {
        limits.validate()?;
        *self
            .limits
            .write()
            .map_err(|_| DocumentError::DocumentConfigPoisoned)? = limits;
        Ok(())
    }
}

const RHAI_SYNTAX: &str = r#"%YAML 1.2
---
name: Rhai
file_extensions: [rhai]
scope: source.rhai
contexts:
  main:
    - include: comments
    - match: '\b(fn|let|const|if|else|switch|for|in|while|loop|do|until|break|continue|return|throw|try|catch|import|export|as|private)\b'
      scope: keyword.control.rhai
    - match: '\b(true|false)\b'
      scope: constant.language.rhai
    - match: '\b[0-9]+(?:\.[0-9]+)?\b'
      scope: constant.numeric.rhai
    - match: '"'
      push: double-quoted-string
    - match: "'"
      push: single-quoted-string
    - match: '`'
      push: template-string
    - match: '\b[A-Za-z_][A-Za-z0-9_]*(?=\s*\()'
      scope: entity.name.function.rhai
    - match: '[+\-*/%!=<>|&^~?:]+'
      scope: keyword.operator.rhai
  comments:
    - match: '//.*$'
      scope: comment.line.double-slash.rhai
    - match: '/\*'
      scope: punctuation.definition.comment.begin.rhai
      push:
        - meta_scope: comment.block.rhai
        - match: '\*/'
          scope: punctuation.definition.comment.end.rhai
          pop: true
  double-quoted-string:
    - meta_scope: string.quoted.double.rhai
    - match: '\\.'
      scope: constant.character.escape.rhai
    - match: '"'
      pop: true
  single-quoted-string:
    - meta_scope: string.quoted.single.rhai
    - match: '\\.'
      scope: constant.character.escape.rhai
    - match: "'"
      pop: true
  template-string:
    - meta_scope: string.quoted.other.rhai
    - match: '\\.'
      scope: constant.character.escape.rhai
    - match: '`'
      pop: true
"#;

const TYPESCRIPT_SYNTAX: &str = r#"%YAML 1.2
---
name: TypeScript
file_extensions: [ts, tsx]
scope: source.ts
contexts:
  main:
    - match: '//.*$'
      scope: comment.line.double-slash.ts
    - match: '/\*'
      push:
        - meta_scope: comment.block.ts
        - match: '\*/'
          pop: true
    - match: '\b(async|await|break|case|catch|class|const|continue|debugger|default|delete|do|else|enum|export|extends|finally|for|from|function|get|if|implements|import|in|instanceof|interface|keyof|let|namespace|new|of|private|protected|public|readonly|return|set|static|super|switch|throw|try|type|typeof|var|void|while|with|yield)\b'
      scope: keyword.control.ts
    - match: '\b(any|bigint|boolean|never|number|object|string|symbol|unknown|undefined|null|true|false)\b'
      scope: storage.type.ts
    - match: '\b[0-9]+(?:\.[0-9]+)?\b'
      scope: constant.numeric.ts
    - match: '"'
      push: double-string
    - match: "'"
      push: single-string
    - match: '`'
      push: template-string
    - match: '\b[A-Za-z_$][A-Za-z0-9_$]*(?=\s*\()'
      scope: entity.name.function.ts
    - match: '</?[A-Za-z][A-Za-z0-9:.-]*'
      scope: entity.name.tag.tsx
    - match: '[+\-*/%!=<>|&^~?:]+'
      scope: keyword.operator.ts
  double-string:
    - meta_scope: string.quoted.double.ts
    - match: '\\.'
      scope: constant.character.escape.ts
    - match: '"'
      pop: true
  single-string:
    - meta_scope: string.quoted.single.ts
    - match: '\\.'
      scope: constant.character.escape.ts
    - match: "'"
      pop: true
  template-string:
    - meta_scope: string.quoted.other.ts
    - match: '\\.'
      scope: constant.character.escape.ts
    - match: '`'
      pop: true
"#;

const JSONC_SYNTAX: &str = r#"%YAML 1.2
---
name: JSON with Comments
file_extensions: [jsonc]
scope: source.json.comments
contexts:
  main:
    - match: '//.*$'
      scope: comment.line.double-slash.json
    - match: '/\*'
      push:
        - meta_scope: comment.block.json
        - match: '\*/'
          pop: true
    - match: '"'
      push: string
    - match: '-?\b[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?\b'
      scope: constant.numeric.json
    - match: '\b(true|false|null)\b'
      scope: constant.language.json
    - match: '[{}\[\],:]'
      scope: punctuation.separator.json
  string:
    - meta_scope: string.quoted.double.json
    - match: '\\(?:["\\/bfnrt]|u[0-9A-Fa-f]{4})'
      scope: constant.character.escape.json
    - match: '"'
      pop: true
"#;

const TOML_SYNTAX: &str = r#"%YAML 1.2
---
name: TOML
file_extensions: [toml]
scope: source.toml
contexts:
  main:
    - match: '#.*$'
      scope: comment.line.number-sign.toml
    - match: '^\s*\[\[?[^\]]+\]\]?'
      scope: entity.name.section.toml
    - match: '^[A-Za-z0-9_.-]+(?=\s*=)'
      scope: variable.other.key.toml
    - match: '"""'
      push: multiline-string
    - match: "'''"
      push: multiline-literal
    - match: '"'
      push: string
    - match: "'"
      push: literal
    - match: '\b(true|false)\b'
      scope: constant.language.toml
    - match: '[+-]?\b(?:0x[0-9A-Fa-f_]+|0o[0-7_]+|0b[01_]+|[0-9][0-9_]*(?:\.[0-9_]+)?)\b'
      scope: constant.numeric.toml
    - match: '[=,.{}\[\]]'
      scope: punctuation.separator.toml
  string:
    - meta_scope: string.quoted.double.toml
    - match: '\\.'
      scope: constant.character.escape.toml
    - match: '"'
      pop: true
  literal:
    - meta_scope: string.quoted.single.toml
    - match: "'"
      pop: true
  multiline-string:
    - meta_scope: string.quoted.triple.toml
    - match: '"""'
      pop: true
  multiline-literal:
    - meta_scope: string.quoted.triple.toml
    - match: "'''"
      pop: true
"#;

const DOCKERFILE_SYNTAX: &str = r#"%YAML 1.2
---
name: Dockerfile
file_extensions: [Dockerfile, dockerfile]
scope: source.dockerfile
contexts:
  main:
    - match: '#.*$'
      scope: comment.line.number-sign.dockerfile
    - match: '(?i)^\s*(ADD|ARG|CMD|COPY|ENTRYPOINT|ENV|EXPOSE|FROM|HEALTHCHECK|LABEL|MAINTAINER|ONBUILD|RUN|SHELL|STOPSIGNAL|USER|VOLUME|WORKDIR)(?=\s)'
      scope: keyword.control.dockerfile
    - match: '\$\{?[A-Za-z_][A-Za-z0-9_]*\}?'
      scope: variable.other.dockerfile
    - match: '"'
      push: double-string
    - match: "'"
      push: single-string
  double-string:
    - meta_scope: string.quoted.double.dockerfile
    - match: '\\.'
      scope: constant.character.escape.dockerfile
    - match: '"'
      pop: true
  single-string:
    - meta_scope: string.quoted.single.dockerfile
    - match: "'"
      pop: true
"#;

const EXTRA_SYNTAXES: &[&str] = &[
    RHAI_SYNTAX,
    TYPESCRIPT_SYNTAX,
    JSONC_SYNTAX,
    TOML_SYNTAX,
    DOCKERFILE_SYNTAX,
];

#[derive(Clone)]
pub struct NativeTextDocument {
    snapshot: Arc<NativeTextSnapshot>,
}

#[derive(Debug, Eq, PartialEq)]
struct NativeTextSnapshot {
    identity: String,
    revision: u64,
    text: Arc<str>,
}

impl NativeTextDocument {
    /// Construct one immutable Host-owned text revision.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::InvalidIdentity`] for an unsafe identity.
    pub fn new(
        identity: impl Into<String>,
        revision: u64,
        text: impl Into<Arc<str>>,
    ) -> Result<Self, DocumentError> {
        let identity = identity.into();
        validate_identity(&identity)?;
        Ok(Self {
            snapshot: Arc::new(NativeTextSnapshot {
                identity,
                revision,
                text: text.into(),
            }),
        })
    }

    #[must_use]
    pub fn identity(&self) -> &str {
        &self.snapshot.identity
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.snapshot.revision
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.snapshot.text
    }

    #[must_use]
    pub fn text_arc(&self) -> Arc<str> {
        Arc::clone(&self.snapshot.text)
    }
}

impl PartialEq for NativeTextDocument {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.snapshot, &other.snapshot) || self.snapshot == other.snapshot
    }
}

impl Eq for NativeTextDocument {}

impl fmt::Debug for NativeTextDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeTextDocument")
            .field("identity", &self.identity())
            .field("revision", &self.revision())
            .field("bytes", &self.text().len())
            .finish()
    }
}

impl CustomType for NativeTextDocument {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("NativeTextDocument")
            .with_get("identity", |document: &mut Self| {
                ImmutableString::from(document.identity().to_owned())
            })
            .with_get("revision", |document: &mut Self| {
                i64::try_from(document.revision()).unwrap_or(i64::MAX)
            })
            .with_get("len", |document: &mut Self| {
                i64::try_from(document.text().len()).unwrap_or(i64::MAX)
            })
            .with_fn("to_string", |document: &mut Self| {
                format!(
                    "NativeTextDocument({}, revision={})",
                    document.identity(),
                    document.revision()
                )
            });
    }
}

pub(crate) fn register_document_api(engine: &mut Engine) {
    engine.build_type::<NativeTextDocument>();
    FuncRegistration::new("is_native_text_document")
        .in_global_namespace()
        .register_into_engine(engine, |value: Dynamic| value.is::<NativeTextDocument>());
}

#[derive(Clone, Debug, Default)]
pub struct NativeTextDocumentRegistry {
    documents: BTreeMap<String, NativeTextDocument>,
    readers: BTreeMap<String, BTreeSet<ComponentInstancePath>>,
}

impl NativeTextDocumentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one Host-owned document before the first render.
    ///
    /// # Errors
    ///
    /// Returns an identity or duplicate-name error.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        document: NativeTextDocument,
    ) -> Result<(), DocumentError> {
        let name = name.into();
        validate_identity(&name)?;
        if self.documents.contains_key(&name) {
            return Err(DocumentError::DuplicateDocument(name));
        }
        self.documents.insert(name, document);
        Ok(())
    }

    /// Replace one document revision and return exact subscribed components.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError::UnknownDocument`] for an unregistered name.
    pub fn replace(
        &mut self,
        name: &str,
        document: NativeTextDocument,
    ) -> Result<BTreeSet<ComponentInstancePath>, DocumentError> {
        let current = self
            .documents
            .get_mut(name)
            .ok_or_else(|| DocumentError::UnknownDocument(name.to_owned()))?;
        if current == &document {
            return Ok(BTreeSet::new());
        }
        if current.identity() == document.identity() && document.revision() <= current.revision() {
            return Err(DocumentError::NonMonotonicRevision {
                identity: document.identity().to_owned(),
                current: current.revision(),
                next: document.revision(),
            });
        }
        *current = document;
        Ok(self.readers.get(name).cloned().unwrap_or_default())
    }

    pub(crate) fn read_tracked(
        &mut self,
        reader: &ComponentInstancePath,
        name: &str,
    ) -> Result<NativeTextDocument, DocumentError> {
        let document = self
            .documents
            .get(name)
            .cloned()
            .ok_or_else(|| DocumentError::UnknownDocument(name.to_owned()))?;
        self.readers
            .entry(name.to_owned())
            .or_default()
            .insert(reader.clone());
        Ok(document)
    }

    pub(crate) fn reset_reader(&mut self, reader: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.remove(reader);
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn retain_reader_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| {
                !reader.is_within(root) || reader == root || active.contains(reader)
            });
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn remove_reader_scope(&mut self, root: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| !reader.is_within(root));
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }
}

#[derive(Clone)]
pub struct SyntaxRegistry {
    syntax_set: Arc<RwLock<Arc<SyntaxSet>>>,
}

impl fmt::Debug for SyntaxRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SyntaxRegistry")
            .field("syntaxes", &self.snapshot().syntaxes().len())
            .finish()
    }
}

impl Default for SyntaxRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            syntax_set: Arc::new(RwLock::new(Arc::clone(default_syntax_set()))),
        }
    }

    /// Register a Sublime `.sublime-syntax` definition for this Engine.
    ///
    /// # Errors
    ///
    /// Returns a syntax parse or poisoned-registry error.
    pub fn register_sublime_syntax(&self, source: &str) -> Result<(), DocumentError> {
        let syntax = SyntaxDefinition::load_from_str(source, true, None)
            .map_err(|error| DocumentError::Syntax(error.to_string()))?;
        let current = self
            .syntax_set
            .read()
            .map_err(|_| DocumentError::SyntaxRegistryPoisoned)?
            .clone();
        let mut builder = current.as_ref().clone().into_builder();
        builder.add(syntax);
        let next = Arc::new(builder.build());
        *self
            .syntax_set
            .write()
            .map_err(|_| DocumentError::SyntaxRegistryPoisoned)? = next;
        Ok(())
    }

    #[must_use]
    pub fn snapshot(&self) -> Arc<SyntaxSet> {
        self.syntax_set
            .read()
            .map_or_else(|poisoned| poisoned.into_inner().clone(), |set| set.clone())
    }
}

fn default_syntax_set() -> &'static Arc<SyntaxSet> {
    static SYNTAXES: OnceLock<Arc<SyntaxSet>> = OnceLock::new();
    SYNTAXES.get_or_init(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        for source in EXTRA_SYNTAXES {
            builder.add(
                SyntaxDefinition::load_from_str(source, true, None)
                    .expect("built-in syntax is valid"),
            );
        }
        Arc::new(builder.build())
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentSource {
    Inline(Arc<str>),
    Native(NativeTextDocument),
}

impl DocumentSource {
    #[must_use]
    pub fn identity(&self) -> Cow<'_, str> {
        match self {
            Self::Inline(_) => Cow::Borrowed("inline"),
            Self::Native(document) => Cow::Borrowed(document.identity()),
        }
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        match self {
            Self::Inline(text) => stable_text_hash(text),
            Self::Native(document) => document.revision(),
        }
    }

    #[must_use]
    pub fn text(&self) -> Arc<str> {
        match self {
            Self::Inline(text) => Arc::clone(text),
            Self::Native(document) => document.text_arc(),
        }
    }
}

impl From<String> for DocumentSource {
    fn from(value: String) -> Self {
        Self::Inline(Arc::from(value))
    }
}

impl From<NativeTextDocument> for DocumentSource {
    fn from(value: NativeTextDocument) -> Self {
        Self::Native(value)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DocumentWrap {
    #[default]
    None,
    Viewport,
    Column(usize),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DiffViewMode {
    #[default]
    Unified,
    Split,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DiffWhitespace {
    #[default]
    Exact,
    IgnoreChanges,
    IgnoreAll,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentDescriptor {
    pub source: DocumentSource,
    pub label: String,
    pub file_name: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxTokenKind {
    Comment,
    String,
    Number,
    Keyword,
    Function,
    Type,
    Variable,
    Constant,
    Operator,
    Punctuation,
    Tag,
    Attribute,
}

impl SyntaxTokenKind {
    #[must_use]
    pub const fn theme_token(self) -> &'static str {
        match self {
            Self::Comment => "syntax.comment",
            Self::String => "syntax.string",
            Self::Number => "syntax.number",
            Self::Keyword => "syntax.keyword",
            Self::Function => "syntax.function",
            Self::Type => "syntax.type",
            Self::Variable => "syntax.variable",
            Self::Constant => "syntax.constant",
            Self::Operator => "syntax.operator",
            Self::Punctuation => "syntax.punctuation",
            Self::Tag => "syntax.tag",
            Self::Attribute => "syntax.attribute",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxSpan {
    pub range: Range<usize>,
    pub kind: SyntaxTokenKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentLine {
    pub range: Range<usize>,
    pub syntax: Vec<SyntaxSpan>,
}

#[derive(Clone, Debug)]
pub struct PreparedDocument {
    identity: String,
    revision: u64,
    text: Arc<str>,
    lines: Arc<[DocumentLine]>,
    language: String,
    max_line_chars: usize,
    ends_with_newline: bool,
    highlight_error: Option<String>,
}

impl PreparedDocument {
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn text_arc(&self) -> Arc<str> {
        Arc::clone(&self.text)
    }

    #[must_use]
    pub fn lines(&self) -> &[DocumentLine] {
        &self.lines
    }

    #[must_use]
    pub fn line_text(&self, index: usize) -> Option<&str> {
        let line = self.lines.get(index)?;
        self.text.get(line.range.clone())
    }

    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    #[must_use]
    pub const fn max_line_chars(&self) -> usize {
        self.max_line_chars
    }

    #[must_use]
    pub const fn ends_with_newline(&self) -> bool {
        self.ends_with_newline
    }

    #[must_use]
    pub fn highlight_error(&self) -> Option<&str> {
        self.highlight_error.as_deref()
    }
}

/// Prepare one immutable text snapshot with default resource limits.
///
/// # Errors
///
/// Returns document budget errors. Syntax highlighting failures degrade to
/// plain text and are recorded on the returned snapshot.
pub fn prepare_document(
    descriptor: &DocumentDescriptor,
    syntaxes: &SyntaxRegistry,
) -> Result<PreparedDocument, DocumentError> {
    prepare_document_with_limits(descriptor, syntaxes, DocumentLimits::default())
}

/// Prepare one immutable text snapshot with Host-provided resource limits.
///
/// # Errors
///
/// Returns when byte or line budgets are exceeded.
pub fn prepare_document_with_limits(
    descriptor: &DocumentDescriptor,
    syntaxes: &SyntaxRegistry,
    limits: DocumentLimits,
) -> Result<PreparedDocument, DocumentError> {
    let text = descriptor.source.text();
    if text.len() > limits.max_document_bytes {
        return Err(DocumentError::TooManyBytes {
            actual: text.len(),
            limit: limits.max_document_bytes,
        });
    }
    let ranges = document_line_ranges(&text);
    if ranges.len() > limits.max_document_lines {
        return Err(DocumentError::TooManyLines {
            actual: ranges.len(),
            limit: limits.max_document_lines,
        });
    }
    let syntax_set = syntaxes.snapshot();
    let syntax = resolve_syntax(
        &syntax_set,
        descriptor.language.as_deref(),
        descriptor.file_name.as_deref(),
    );
    let language = syntax.name.clone();
    let mut parser = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let selectors = syntax_selectors();
    let mut lines = Vec::with_capacity(ranges.len());
    let mut max_line_chars = 0usize;
    let mut highlight_error = None;
    let mut highlighting = true;
    for range in ranges {
        let line = &text[range.clone()];
        max_line_chars = max_line_chars.max(line.chars().count());
        let parse_end = text[range.end..]
            .find('\n')
            .map_or(range.end, |offset| range.end + offset + 1);
        let parse_start = range.start;
        let parse_line = &text[parse_start..parse_end];
        let mut spans: Vec<SyntaxSpan> = Vec::new();
        if highlighting {
            match parser.parse_line(parse_line, &syntax_set) {
                Ok(operations) => {
                    for (token_range, operation) in ScopeRangeIterator::new(&operations, parse_line)
                    {
                        if let Err(error) = stack.apply(operation) {
                            highlight_error.get_or_insert_with(|| error.to_string());
                            highlighting = false;
                            spans.clear();
                            break;
                        }
                        let start = token_range.start.min(line.len());
                        let end = token_range.end.min(line.len());
                        if start >= end {
                            continue;
                        }
                        if let Some(kind) = selectors.kind(stack.as_slice()) {
                            if let Some(previous) = spans.last_mut()
                                && previous.kind == kind
                                && previous.range.end == start
                            {
                                previous.range.end = end;
                            } else {
                                spans.push(SyntaxSpan {
                                    range: start..end,
                                    kind,
                                });
                            }
                        }
                    }
                }
                Err(error) => {
                    highlight_error.get_or_insert_with(|| error.to_string());
                    highlighting = false;
                }
            }
        }
        lines.push(DocumentLine {
            range,
            syntax: spans,
        });
    }
    Ok(PreparedDocument {
        identity: descriptor.source.identity().into_owned(),
        revision: descriptor.source.revision(),
        text,
        lines: lines.into(),
        language,
        max_line_chars,
        ends_with_newline: descriptor.source.text().ends_with('\n'),
        highlight_error,
    })
}

fn document_line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            let mut end = index;
            if text.as_bytes().get(index.wrapping_sub(1)) == Some(&b'\r') && index > start {
                end -= 1;
            }
            ranges.push(start..end);
            start = index + 1;
        }
    }
    if start < text.len() || text.is_empty() || text.ends_with('\n') {
        ranges.push(start..text.len());
    }
    ranges
}

fn resolve_syntax<'a>(
    syntaxes: &'a SyntaxSet,
    language: Option<&str>,
    file_name: Option<&str>,
) -> &'a syntect::parsing::SyntaxReference {
    language
        .and_then(|language| {
            let token = language_alias(language);
            syntaxes.find_syntax_by_token(token.as_ref())
        })
        .or_else(|| {
            file_name.and_then(|file_name| {
                syntaxes.find_syntax_by_extension(file_name).or_else(|| {
                    file_name
                        .rsplit_once('.')
                        .and_then(|(_, ext)| syntaxes.find_syntax_by_extension(ext))
                })
            })
        })
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

fn language_alias(language: &str) -> Cow<'_, str> {
    let normalized = language.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "plaintext" | "plain_text" | "text" => Cow::Borrowed("txt"),
        "javascript" => Cow::Borrowed("js"),
        "typescript" => Cow::Borrowed("ts"),
        "shell" | "bash" => Cow::Borrowed("sh"),
        "python" => Cow::Borrowed("py"),
        "markdown" => Cow::Borrowed("md"),
        "rust" => Cow::Borrowed("rs"),
        "golang" => Cow::Borrowed("go"),
        _ => Cow::Owned(normalized),
    }
}

struct SyntaxSelectors {
    comment: ScopeSelectors,
    string: ScopeSelectors,
    number: ScopeSelectors,
    keyword: ScopeSelectors,
    function: ScopeSelectors,
    type_name: ScopeSelectors,
    variable: ScopeSelectors,
    constant: ScopeSelectors,
    operator: ScopeSelectors,
    punctuation: ScopeSelectors,
    tag: ScopeSelectors,
    attribute: ScopeSelectors,
}

impl SyntaxSelectors {
    fn new() -> Self {
        let parse = |value: &str| ScopeSelectors::from_str(value).expect("static scope selector");
        Self {
            comment: parse("comment"),
            string: parse("string"),
            number: parse("constant.numeric"),
            keyword: parse("keyword, storage.modifier"),
            function: parse("entity.name.function, support.function"),
            type_name: parse("entity.name.type, support.type, storage.type"),
            variable: parse("variable"),
            constant: parse("constant"),
            operator: parse("keyword.operator"),
            punctuation: parse("punctuation"),
            tag: parse("entity.name.tag"),
            attribute: parse("entity.other.attribute-name"),
        }
    }

    fn kind(&self, stack: &[syntect::parsing::Scope]) -> Option<SyntaxTokenKind> {
        [
            (&self.comment, SyntaxTokenKind::Comment),
            (&self.string, SyntaxTokenKind::String),
            (&self.number, SyntaxTokenKind::Number),
            (&self.function, SyntaxTokenKind::Function),
            (&self.type_name, SyntaxTokenKind::Type),
            (&self.tag, SyntaxTokenKind::Tag),
            (&self.attribute, SyntaxTokenKind::Attribute),
            (&self.operator, SyntaxTokenKind::Operator),
            (&self.keyword, SyntaxTokenKind::Keyword),
            (&self.variable, SyntaxTokenKind::Variable),
            (&self.constant, SyntaxTokenKind::Constant),
            (&self.punctuation, SyntaxTokenKind::Punctuation),
        ]
        .into_iter()
        .find_map(|(selector, kind)| selector.does_match(stack).map(|_| kind))
    }
}

fn syntax_selectors() -> &'static SyntaxSelectors {
    static SELECTORS: OnceLock<SyntaxSelectors> = OnceLock::new();
    SELECTORS.get_or_init(SyntaxSelectors::new)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffRowKind {
    Equal,
    LeftOnly,
    RightOnly,
    Modified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffAlignedRow {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub kind: DiffRowKind,
    pub hunk: Option<usize>,
    pub left_inline: Vec<Range<usize>>,
    pub right_inline: Vec<Range<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffDisplayRow {
    Content(usize),
    Fold {
        id: usize,
        full_range: Range<usize>,
        hidden_rows: usize,
    },
}

#[derive(Clone, Debug)]
pub struct PreparedDiff {
    pub left: PreparedDocument,
    pub right: PreparedDocument,
    pub rows: Arc<[DiffAlignedRow]>,
    pub collapsed: Arc<[DiffDisplayRow]>,
    pub hunk_count: usize,
}

/// Prepare a two-way comparison with default resource limits.
///
/// # Errors
///
/// Returns document or diff budget errors.
pub fn prepare_diff(
    left: &DocumentDescriptor,
    right: &DocumentDescriptor,
    whitespace: DiffWhitespace,
    context_lines: Option<usize>,
    syntaxes: &SyntaxRegistry,
) -> Result<PreparedDiff, DocumentError> {
    prepare_diff_with_limits(
        left,
        right,
        whitespace,
        context_lines,
        syntaxes,
        DocumentLimits::default(),
    )
}

/// Prepare a two-way comparison with Host-provided resource limits.
///
/// # Errors
///
/// Returns when document, combined-input, or hunk budgets are exceeded.
#[allow(clippy::too_many_lines)]
pub fn prepare_diff_with_limits(
    left: &DocumentDescriptor,
    right: &DocumentDescriptor,
    whitespace: DiffWhitespace,
    context_lines: Option<usize>,
    syntaxes: &SyntaxRegistry,
    limits: DocumentLimits,
) -> Result<PreparedDiff, DocumentError> {
    let total_bytes = left
        .source
        .text()
        .len()
        .saturating_add(right.source.text().len());
    if total_bytes > limits.max_diff_total_bytes {
        return Err(DocumentError::TooManyDiffBytes {
            actual: total_bytes,
            limit: limits.max_diff_total_bytes,
        });
    }
    let left = prepare_document_with_limits(left, syntaxes, limits)?;
    let right = prepare_document_with_limits(right, syntaxes, limits)?;
    let left_keys = diff_keys(&left, whitespace);
    let right_keys = diff_keys(&right, whitespace);
    let deadline = Instant::now().checked_add(limits.diff_timeout);
    let ops = capture_diff_slices_deadline(Algorithm::Patience, &left_keys, &right_keys, deadline);
    let mut rows = Vec::with_capacity(left.lines().len().max(right.lines().len()));
    let mut hunk = 0usize;
    let mut in_change = false;
    for operation in ops {
        match operation {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                in_change = false;
                rows.extend((0..len).map(|offset| DiffAlignedRow {
                    left: Some(old_index + offset),
                    right: Some(new_index + offset),
                    kind: DiffRowKind::Equal,
                    hunk: None,
                    left_inline: Vec::new(),
                    right_inline: Vec::new(),
                }));
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                let id = next_hunk(&mut hunk, &mut in_change, limits.max_diff_hunks)?;
                rows.extend((0..old_len).map(|offset| DiffAlignedRow {
                    left: Some(old_index + offset),
                    right: None,
                    kind: DiffRowKind::LeftOnly,
                    hunk: Some(id),
                    left_inline: Vec::new(),
                    right_inline: Vec::new(),
                }));
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                let id = next_hunk(&mut hunk, &mut in_change, limits.max_diff_hunks)?;
                rows.extend((0..new_len).map(|offset| DiffAlignedRow {
                    left: None,
                    right: Some(new_index + offset),
                    kind: DiffRowKind::RightOnly,
                    hunk: Some(id),
                    left_inline: Vec::new(),
                    right_inline: Vec::new(),
                }));
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let id = next_hunk(&mut hunk, &mut in_change, limits.max_diff_hunks)?;
                let aligned = old_len.max(new_len);
                for offset in 0..aligned {
                    let left_index = (offset < old_len).then_some(old_index + offset);
                    let right_index = (offset < new_len).then_some(new_index + offset);
                    let (left_inline, right_inline) = match (left_index, right_index) {
                        (Some(left_index), Some(right_index)) => inline_ranges(
                            left.line_text(left_index).unwrap_or_default(),
                            right.line_text(right_index).unwrap_or_default(),
                            deadline,
                        ),
                        _ => (Vec::new(), Vec::new()),
                    };
                    rows.push(DiffAlignedRow {
                        left: left_index,
                        right: right_index,
                        kind: match (left_index, right_index) {
                            (Some(_), Some(_)) => DiffRowKind::Modified,
                            (Some(_), None) => DiffRowKind::LeftOnly,
                            (None, Some(_)) => DiffRowKind::RightOnly,
                            (None, None) => unreachable!(),
                        },
                        hunk: Some(id),
                        left_inline,
                        right_inline,
                    });
                }
            }
        }
    }
    let collapsed = collapse_rows(&rows, context_lines);
    Ok(PreparedDiff {
        left,
        right,
        rows: rows.into(),
        collapsed: collapsed.into(),
        hunk_count: hunk,
    })
}

fn next_hunk(
    hunks: &mut usize,
    in_change: &mut bool,
    limit: usize,
) -> Result<usize, DocumentError> {
    if !*in_change {
        *hunks = hunks.checked_add(1).ok_or(DocumentError::TooManyHunks {
            actual: usize::MAX,
            limit,
        })?;
        *in_change = true;
    }
    if *hunks > limit {
        return Err(DocumentError::TooManyHunks {
            actual: *hunks,
            limit,
        });
    }
    Ok(*hunks - 1)
}

fn diff_keys(document: &PreparedDocument, whitespace: DiffWhitespace) -> Vec<Cow<'_, str>> {
    document
        .lines()
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let line = document.line_text(index).unwrap_or_default();
            match whitespace {
                DiffWhitespace::Exact => Cow::Borrowed(line),
                DiffWhitespace::IgnoreChanges => {
                    Cow::Owned(line.split_whitespace().collect::<Vec<_>>().join(" "))
                }
                DiffWhitespace::IgnoreAll => Cow::Owned(
                    line.chars()
                        .filter(|character| !character.is_whitespace())
                        .collect(),
                ),
            }
        })
        .collect()
}

fn inline_ranges(
    left: &str,
    right: &str,
    deadline: Option<Instant>,
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if left.len().saturating_add(right.len()) > MAX_INLINE_DIFF_BYTES {
        return (
            (!left.is_empty())
                .then_some(0..left.len())
                .into_iter()
                .collect(),
            (!right.is_empty())
                .then_some(0..right.len())
                .into_iter()
                .collect(),
        );
    }
    let left_graphemes = left.graphemes(true).collect::<Vec<_>>();
    let right_graphemes = right.graphemes(true).collect::<Vec<_>>();
    let operations = capture_diff_slices_deadline(
        Algorithm::Patience,
        &left_graphemes,
        &right_graphemes,
        deadline,
    );
    let mut left_offset = 0usize;
    let mut right_offset = 0usize;
    let mut left_ranges = Vec::new();
    let mut right_ranges = Vec::new();
    for operation in operations {
        for change in operation.iter_changes(&left_graphemes, &right_graphemes) {
            let len = change.value().len();
            match change.tag() {
                similar::ChangeTag::Equal => {
                    left_offset += len;
                    right_offset += len;
                }
                similar::ChangeTag::Delete => {
                    push_range(&mut left_ranges, left_offset..left_offset + len);
                    left_offset += len;
                }
                similar::ChangeTag::Insert => {
                    push_range(&mut right_ranges, right_offset..right_offset + len);
                    right_offset += len;
                }
            }
        }
    }
    (left_ranges, right_ranges)
}

fn push_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if range.is_empty() {
        return;
    }
    if let Some(previous) = ranges.last_mut()
        && previous.end == range.start
    {
        previous.end = range.end;
    } else {
        ranges.push(range);
    }
}

fn collapse_rows(rows: &[DiffAlignedRow], context: Option<usize>) -> Vec<DiffDisplayRow> {
    let Some(context) = context else {
        return (0..rows.len()).map(DiffDisplayRow::Content).collect();
    };
    if rows.iter().all(|row| row.kind == DiffRowKind::Equal) {
        return if rows.is_empty() {
            Vec::new()
        } else {
            vec![DiffDisplayRow::Fold {
                id: 0,
                full_range: 0..rows.len(),
                hidden_rows: rows.len(),
            }]
        };
    }
    let mut keep = vec![false; rows.len()];
    for (index, row) in rows.iter().enumerate() {
        if row.kind != DiffRowKind::Equal {
            let start = index.saturating_sub(context);
            let end = index
                .saturating_add(context)
                .saturating_add(1)
                .min(rows.len());
            keep[start..end].fill(true);
        }
    }
    let mut display = Vec::new();
    let mut index = 0usize;
    let mut fold = 0usize;
    while index < rows.len() {
        if keep[index] {
            display.push(DiffDisplayRow::Content(index));
            index += 1;
            continue;
        }
        let start = index;
        while index < rows.len() && !keep[index] {
            index += 1;
        }
        display.push(DiffDisplayRow::Fold {
            id: fold,
            full_range: start..index,
            hidden_rows: index - start,
        });
        fold += 1;
    }
    display
}

fn stable_text_hash(text: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    text.bytes().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(PRIME)
    })
}

fn validate_identity(value: &str) -> Result<(), DocumentError> {
    if value.is_empty()
        || value.len() > 256
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/' | ':')
        })
    {
        Err(DocumentError::InvalidIdentity(value.to_owned()))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DocumentError {
    #[error("document identity `{0}` must contain 1-256 safe ASCII characters")]
    InvalidIdentity(String),
    #[error("native text document `{0}` is already registered")]
    DuplicateDocument(String),
    #[error("native text document `{0}` is not registered")]
    UnknownDocument(String),
    #[error(
        "native text document `{identity}` revision must increase: current {current}, next {next}"
    )]
    NonMonotonicRevision {
        identity: String,
        current: u64,
        next: u64,
    },
    #[error("document contains {actual} bytes, exceeding the {limit}-byte budget")]
    TooManyBytes { actual: usize, limit: usize },
    #[error("diff inputs contain {actual} bytes, exceeding the {limit}-byte budget")]
    TooManyDiffBytes { actual: usize, limit: usize },
    #[error("document contains {actual} lines, exceeding the {limit}-line budget")]
    TooManyLines { actual: usize, limit: usize },
    #[error("diff contains {actual} hunks, exceeding the {limit}-hunk budget")]
    TooManyHunks { actual: usize, limit: usize },
    #[error("syntax definition or parse failed: {0}")]
    Syntax(String),
    #[error("syntax registry lock was poisoned")]
    SyntaxRegistryPoisoned,
    #[error("document resource limits must all be greater than zero")]
    InvalidLimits,
    #[error("document runtime configuration lock was poisoned")]
    DocumentConfigPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(text: &str, language: &str) -> DocumentDescriptor {
        DocumentDescriptor {
            source: DocumentSource::from(text.to_owned()),
            label: language.to_owned(),
            file_name: None,
            language: Some(language.to_owned()),
        }
    }

    #[test]
    fn native_documents_are_revisioned_and_registry_reads_are_exact() {
        let first = NativeTextDocument::new("server/a", 1, Arc::<str>::from("port=80\n")).unwrap();
        let second =
            NativeTextDocument::new("server/a", 2, Arc::<str>::from("port=443\n")).unwrap();
        let reader = ComponentInstancePath::root("View", "main");
        let mut registry = NativeTextDocumentRegistry::new();
        registry.register("config", first).unwrap();
        assert_eq!(
            registry.read_tracked(&reader, "config").unwrap().revision(),
            1
        );
        assert_eq!(
            registry.replace("config", second).unwrap(),
            BTreeSet::from([reader])
        );
        let stale = NativeTextDocument::new("server/a", 2, Arc::<str>::from("stale")).unwrap();
        assert!(matches!(
            registry.replace("config", stale),
            Err(DocumentError::NonMonotonicRevision { .. })
        ));
    }

    #[test]
    fn rhai_and_go_are_built_in_and_unknown_languages_fall_back_to_plain_text() {
        let registry = SyntaxRegistry::new();
        let rhai = prepare_document(&descriptor("fn view() { 42 }\n", "rhai"), &registry).unwrap();
        assert_eq!(rhai.language(), "Rhai");
        assert!(
            rhai.lines()[0]
                .syntax
                .iter()
                .any(|span| span.kind == SyntaxTokenKind::Keyword),
            "{:?}",
            rhai.lines()[0].syntax
        );

        let go = prepare_document(
            &descriptor("package main\nfunc main() {}\n", "go"),
            &registry,
        )
        .unwrap();
        assert_eq!(go.language(), "Go");
        let plain = prepare_document(&descriptor("hello\n", "not-a-language"), &registry).unwrap();
        assert_eq!(plain.language(), "Plain Text");
    }

    #[test]
    fn public_launch_language_pack_resolves_every_declared_language() {
        let registry = SyntaxRegistry::new();
        let mut missing = Vec::new();
        for language in [
            "rhai",
            "rust",
            "go",
            "javascript",
            "typescript",
            "tsx",
            "json",
            "jsonc",
            "toml",
            "yaml",
            "markdown",
            "bash",
            "python",
            "html",
            "css",
            "sql",
            "dockerfile",
            "RUST",
        ] {
            let document = prepare_document(&descriptor("value = 1\n", language), &registry)
                .unwrap_or_else(|error| panic!("{language}: {error}"));
            if document.language() == "Plain Text" {
                missing.push(language);
            }
        }
        assert!(missing.is_empty(), "missing {missing:?}");
    }

    #[test]
    fn line_ranges_normalize_crlf_but_retain_terminal_empty_line() {
        let registry = SyntaxRegistry::new();
        let document = prepare_document(&descriptor("one\r\ntwo\n", "text"), &registry).unwrap();
        assert_eq!(document.lines().len(), 3);
        assert_eq!(document.line_text(0), Some("one"));
        assert_eq!(document.line_text(1), Some("two"));
        assert_eq!(document.line_text(2), Some(""));
        assert!(document.ends_with_newline());
    }

    #[test]
    fn diff_is_direction_neutral_refines_unicode_and_collapses_context() {
        let registry = SyntaxRegistry::new();
        let left = descriptor(
            "same\n城市 = 东京\nunchanged 1\nunchanged 2\nunchanged 3\nunchanged 4\n",
            "rhai",
        );
        let right = descriptor(
            "same\n城市 = 上海\nunchanged 1\nunchanged 2\nunchanged 3\nunchanged 4\n",
            "rhai",
        );
        let diff = prepare_diff(&left, &right, DiffWhitespace::Exact, Some(1), &registry).unwrap();
        assert_eq!(diff.hunk_count, 1);
        let modified = diff
            .rows
            .iter()
            .find(|row| row.kind == DiffRowKind::Modified)
            .unwrap();
        assert!(!modified.left_inline.is_empty());
        assert!(!modified.right_inline.is_empty());
        assert!(
            diff.collapsed
                .iter()
                .any(|row| matches!(row, DiffDisplayRow::Fold { .. }))
        );
    }

    #[test]
    fn whitespace_policy_is_explicit() {
        let registry = SyntaxRegistry::new();
        let left = descriptor("value = 1\n", "rhai");
        let right = descriptor("value   =   1\n", "rhai");
        assert_eq!(
            prepare_diff(&left, &right, DiffWhitespace::Exact, None, &registry)
                .unwrap()
                .hunk_count,
            1
        );
        assert_eq!(
            prepare_diff(
                &left,
                &right,
                DiffWhitespace::IgnoreChanges,
                None,
                &registry,
            )
            .unwrap()
            .hunk_count,
            0
        );
    }

    #[test]
    fn host_limits_fail_explicitly_before_unbounded_work() {
        let registry = SyntaxRegistry::new();
        let descriptor = descriptor("12345", "text");
        let limits = DocumentLimits {
            max_document_bytes: 4,
            ..DocumentLimits::default()
        };
        assert!(matches!(
            prepare_document_with_limits(&descriptor, &registry, limits),
            Err(DocumentError::TooManyBytes {
                actual: 5,
                limit: 4
            })
        ));
        let runtime = DocumentRuntimeConfig::new();
        assert_eq!(runtime.limits(), DocumentLimits::default());
        assert_eq!(
            runtime.set_limits(DocumentLimits {
                max_diff_hunks: 0,
                ..DocumentLimits::default()
            }),
            Err(DocumentError::InvalidLimits)
        );
    }

    #[test]
    fn host_registered_syntax_participates_in_background_safe_snapshots() {
        let registry = SyntaxRegistry::new();
        registry
            .register_sublime_syntax(
                r"%YAML 1.2
---
name: Probe
file_extensions: [probe]
scope: source.probe
contexts:
  main:
    - match: '\bprobe\b'
      scope: keyword.control.probe
",
            )
            .unwrap();
        let document = prepare_document(&descriptor("probe value\n", "probe"), &registry).unwrap();
        assert_eq!(document.language(), "Probe");
        assert!(
            document.lines()[0]
                .syntax
                .iter()
                .any(|span| span.kind == SyntaxTokenKind::Keyword)
        );
    }
}
