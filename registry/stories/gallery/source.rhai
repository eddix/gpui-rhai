import "components/code_viewer" as code_viewer;

fn view(ctx) {
    code_viewer::CodeViewer(#{ key: "gallery-source", label: "Current running Rhai source",
        file_name: "story.rhai", language: "rhai",
        source: ctx.get_native_text_document("gallery_source"),
        style: style().width(relative(1.0)).height(relative(1.0)) })
}
