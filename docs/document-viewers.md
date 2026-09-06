# Native text documents and read-only viewers

`CodeViewer` and `DiffViewer` are source-owned Rhai components over public
native primitives. Rhai owns composition and policy; Rust owns text indexing,
syntax parsing, comparison, virtual rows, selection, search, and scrolling.
Neither component calls Rhai per line or during GPUI layout/paint.

Use [CodeViewer](components/code-viewer.md) for one source document and
[DiffViewer](components/diff-viewer.md) for a neutral two-way comparison.

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
`NativeTextDocument`. Register it before initial render and read it through the
tracked context API:

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

The tracked reader is the formal component whose `ctx` executes
`get_native_text_document`. Reading at the application root deliberately marks
the root dirty; for narrower invalidation, put the read and Viewer invocation in
the smallest source component that owns that document surface.

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
