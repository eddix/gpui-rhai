# Native text documents and read-only viewers

`MarkdownViewer`, `CodeViewer` and `DiffViewer` are source-owned Rhai components over public
native primitives. Rhai owns composition and policy; Rust owns text indexing,
syntax parsing, comparison, virtual rows, selection, search, and scrolling.
Neither component calls Rhai per line or during GPUI layout/paint.

Use [CodeViewer](components/code-viewer.md) for one source document and
[DiffViewer](components/diff-viewer.md) for a neutral two-way comparison.
Use MarkdownViewer for native CommonMark/GFM document presentation, including
streamed assistant replies. It is neither an editor nor a WebView.

## Native Markdown

```rhai
import "components/markdown_viewer" as markdown;
markdown::MarkdownViewer(#{
    key: "reply-42", label: "Assistant reply",
    source: ctx.get_native_text_document("reply-42"),
    on_link: Fn("propose_open_link"),
    code_block_actions: ["copy"],
})
```

The native parser supports CommonMark and GFM tables/alignment, task lists,
strikethrough, angle and bare autolinks, nested lists and block quotes. HTML
is displayed literally, never interpreted. Link activation emits
`#{ href, text }`; the application must validate the untrusted destination and
decide whether to open it. The viewer does not open URLs or initiate image
requests. `image_assets: #{ "markdown-destination": "app/asset-id" }` explicitly
maps images to Host asset IDs, not directly to provider loads. Native rendering
only reads the prepared image cache: it never calls `AssetProvider::load`, does
provider I/O, or decodes an image. The Host must preload declared assets during
setup (`AssetRegistry::preload_images` / `load_image`) or use the existing async
image-decoding workflow before publishing the prepared image and requesting a
repaint. Missing, failed, or not-yet-prepared images retain their alt text; an
explicit map alone does not authorize loading. `PrimitiveContext::image_asset`
has the same cache-only contract for Host primitives.

Selection is continuous across virtual blocks inside one viewer, never across
separate viewer/message instances. Cmd/Ctrl+C copies rendered plain text;
Alt+Cmd+C / Ctrl+Alt+C explicitly copies the original Markdown. Cmd/Ctrl+A
selects the viewer document. Code copy actions copy only code, without fences.
Arrow, Page Up/Down, Home and End scroll; links and code actions are native
tab stops. Headings, paragraphs, code, inline code, quotes, tables, links,
rules and loading/error states have component parts. Heading levels map to
typography roles, while code blocks share the existing document monospace
fallbacks and syntax registry, including Rhai and Host-added grammars.
`height: "content"` grows to measured content; optional `max_height` bounds it
and enables internal scrolling. The component accepts a logical-pixel integer
or an absolute-resolvable `Length` (`px`/`rem`), not a relative length.

Parsing runs on background typed immutable snapshots. Rapid revisions replace
the owned task and cancel obsolete work; a released/replaced viewer cannot
receive a late commit. Each parse preserves complete Markdown semantics:
appended reference definitions can change an earlier block, so the runtime
compares the full native block model and invalidates from the first changed
block instead of assuming only the final block can change. This intentionally
uses a complete semantic parse rather than an unsafe tail-only optimization.
GPUI measures and virtualizes block heights. Long paragraphs/code/cells are
split at UTF-8 boundaries into at most 4 KiB text segments; large tables use a
bounded nested measured list and retain horizontal scrolling.
Nested table/cell viewports share the native scrollbar's residual wheel policy;
unsupported axes are not converted. Fill and clamped-content viewers reserve
a stable logical scrollbar gutter so document and table thumbs cannot overlap.
This is presentation policy, not a second scroll position or a child layout
query into its parent's actively borrowed native list.
Oversized cells have their own bounded fragment viewport: fragments never
become logical table rows, and normal copy stays in logical row/cell order.
The default `DocumentLimits::max_markdown_table_columns` is 128; wider input
produces a visible resource error before native layout rather than allocating
unbounded cells. Hosts can explicitly configure this limit alongside byte/line
budgets. Inline images retain the heading, list, paragraph or cell that owns
their alt run; Host-provided asset presentation stays inside that same parent.

## String and Host-owned sources

Direct strings are the simplest path and remain first-class:

```rhai
code_viewer::CodeViewer(#{
    key: "config",
    label: "Service configuration",
    source: config_text,
    file_name: "service.toml",
    language: "toml",
})
```

Large or frequently replaced text can remain in Rust as an immutable
`NativeTextDocument`. Register it before initial render or on an already mounted
view and read it through the tracked context API:

```rust
use gpui_rhai::{NativeTextDocument, ScriptViewExtension, UiRuntimeState};

#[derive(Clone)]
struct Documents {
    config: NativeTextDocument,
}

impl ScriptViewExtension for Documents {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime.native_documents
            .register("config", self.config.clone())
            .map_err(|error| error.to_string())
    }
}
```

```rhai
let config = ctx.get_native_text_document("config");
code_viewer::CodeViewer(#{ key: "config", label: "Config", source: config })
```

Publish a later immutable revision on GPUI's foreground thread:

```rust
view.replace_native_text_document(
    "config",
    NativeTextDocument::new("service-config", 2, next_text)?,
    cx,
)?;
```

Only components that read the named document are invalidated. Revisions for the
same document identity must increase; changing identity starts a new reading
session. Inline strings use the retained primitive identity as their session,
so changing content does not reset scroll merely because its hash changed.

Mounted hosts can call `view.register_native_text_document(name, snapshot, cx)`
as messages arrive; no fixed document-slot pool is required. Registration rejects
duplicates and invalidates only readers that previously attempted that missing
name (a script may catch the lookup error to render a pending state). Use
`view.release_native_text_document(name, cx)` after removing the message to
release registry ownership. Existing immutable snapshots remain safe until
their readers/jobs release them. Replacement rejects stale revisions, and all
three handle operations reject disposed views. Documents and pending-reader
dependencies participate in the runtime's existing candidate rollback and
generation lifecycle.
Caught missing-name lookups share the bounded dependency policy used by native
collections: 64 names per reader, 4,096 reader/name pairs and 64 KiB of names per
registry. Rerender, contribution replacement and owner teardown remove stale
edges. Exceeding the budget reports an error without growing the index.

The tracked reader is the formal component whose `ctx` executes
`get_native_text_document`. Reading at the application root deliberately marks
the root dirty; for narrower invalidation, put the read and Viewer invocation in
the smallest source component that owns that document surface.

## Shared scrollbars

CodeViewer and DiffViewer share the runtime's themed native scrollbar with
ScrollArea and virtual collections. `scrollbar: "auto"` is the default;
`"always"` and `"hidden"` are available. Their `scrollbar_track`,
`scrollbar_thumb` and `scrollbar_thumb_hover` parts override the same theme
backgrounds. Split DiffViewer has one vertical position and independent
horizontal pane bars. The bars do not take focus or change native document
keyboard ownership.

## Language registry

The built-in pack includes Plain Text, Rhai, Rust, Go, JavaScript,
TypeScript/TSX, JSON/JSONC, TOML, YAML, Markdown, Bash/Shell, Python, HTML,
CSS, SQL, and Dockerfile. An explicit `language` takes precedence over
`file_name`; unknown languages fall back to Plain Text.

Host extensions may install an additional Sublime syntax before compilation:

```rust
use gpui_rhai::{RuntimeEngine, ScriptViewExtension};

struct LanguageExtension;

impl ScriptViewExtension for LanguageExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .syntax_registry()
            .register_sublime_syntax(include_str!("syntaxes/example.sublime-syntax"))
            .map_err(|error| error.to_string())
    }
}
```

Syntax registries and immutable text snapshots are `Send + Sync` host data and
may enter background jobs. Rhai `Dynamic`, `FnPtr`, `Engine`, and stored call
contexts remain on the foreground thread because this workspace pins Rhai
1.26 without its `sync` feature.

## Resource policy

Document parsing and diffing never run inside a Rhai native call. Retained
entities start cancellable background jobs, publish only complete matching
revisions, and discard obsolete results. A later calculation leaves the last
complete surface visible with an updating indicator.
Viewport/fixed-column wrap projection follows the same rule, so resizing a
large document does not synchronously rebuild every visual row on GPUI's
foreground thread.

Defaults allow 10 MiB and 500,000 lines per document, 20 MiB combined diff
input, 100,000 hunks, and two seconds for the diff algorithm. A Host can replace
these limits before preparation:

```rust
let config = engine.document_runtime_config();
config.set_limits(gpui_rhai::DocumentLimits {
    max_document_bytes: 2 * 1024 * 1024,
    max_document_lines: 100_000,
    ..config.limits()
})?;
```

Exceeding a hard budget produces an explicit native error surface. Syntax
highlighting failure is softer: the document remains selectable/searchable and
falls back to unstyled monospace text with a diagnostic.
To keep adversarial single-line inputs bounded, a replaced line pair above
256 KiB is marked as changed as a whole instead of running grapheme-level
refinement; line-level diff and original-text selection remain intact.

All bundled themes materialize semantic `syntax.*`, `document.*`, and `diff.*`
colors from their own palette. Explicit namespaced theme values override those
defaults and survive Theme Studio's canonical save format.

Document text and line-number gutters default to an installed platform
monospace family (Lilex/SF Mono/Menlo/Monaco on macOS, Cascadia/Consolas on
Windows, and the common DejaVu/Liberation/Noto/Ubuntu Mono families on Linux).
Applications can override either intended part without forking the component:

```rhai
let code_font = style().font_family("JetBrains Mono")
    .font_fallbacks(["SF Mono", "Menlo"]);
code_viewer::CodeViewer(#{
    key: "source",
    source: source,
    part_styles: #{ text: code_font, gutter: code_font },
})
```

The selected family must be installed or registered by the Host through
`FontSource`.
