// Interactive final Gallery module. Host streaming fixtures are supplied by the
// Gallery host; this standalone source specimen also exercises keyboard copy.
import "gallery/kit" as kit;
import "components/markdown_viewer" as markdown;
import "components/button" as button;

fn pages() { [#{ id: "markdown_viewer", title: "MarkdownViewer", group: "Display" }] }

fn sample() {
    "# Native Markdown\n\nSelect from this paragraph through the next block, then use **Copy**. Alt+Cmd+C copies the original Markdown.\n\n> A quote with *emphasis*, ~~removed text~~ and an [untrusted link](https://example.com/a_(b)).\n\n- [x] Rust-owned parsing and text\n- [ ] No network fetching\n  - Nested content keeps its indentation\n\n```rhai\nfn view(ctx) {\n    text(\"你好, native Markdown\")\n}\n```\n\n| Feature | Policy |\n| :--- | ---: |\n| Copy | Visible text |\n| Images | Host assets only |\n\n![Image alt fallback](https://no-fetch.invalid/image.png)\n\nBare links work too: https://example.com/a_(b).\n\n<script>HTML is literal, never executed</script>\n"
}

fn view(ctx, h, v, id) {
    if id != "markdown_viewer" { return (); }
    let large = kit::val(v, "markdown.large", false);
    let source = ctx.get_native_text_document(if large { "gallery.markdown.large" } else { "gallery.markdown.reference" });
    let link = kit::val(v, "markdown.link", #{ href: "No link activated" });
    kit::page(h, v, id, "MarkdownViewer", "Native CommonMark and GFM with cross-block selection. Links only emit callbacks; the Host owns opening them.", [
        kit::sec("Source", kit::line([
            button::Button(#{ key: "markdown-reference", text: "Reference article", variant: "secondary", on_click: kit::put(h, "markdown.large", false) }),
            button::Button(#{ key: "markdown-large", text: "Large native document", variant: "secondary", on_click: kit::put(h, "markdown.large", true) }),
            kit::note(`${source.len} bytes, held in Rust`),
        ])),
        kit::sec("Document", column([
            markdown::MarkdownViewer(#{ key: "markdown-gallery", label: "Markdown specimen", source: source, on_link: kit::store(h, "markdown.link") })
        ]).with_style(style().height(px(440)))),
        kit::note("Select across blocks; Cmd/Ctrl+C copies visible text. Alt+Cmd+C / Ctrl+Alt+C copies Markdown."),
        kit::note(`Last link proposal: ${link.href}`),
    ])
}
