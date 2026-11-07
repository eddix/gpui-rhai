// Embeds ui/, the built-in skin, into the binary; see src/embedded.rs.
fn main() {
    gpui_rhai_build::embed_ui("ui").expect("embed ui/");
}
