//! Text and documents: code typography, CodeViewer and DiffViewer, selectable text.

mod code_typography;
mod code_viewer_height;
mod document_viewers;
mod markdown_preparation;
mod markdown_tables;
mod markdown_viewer;
mod modern_clean_documents;
mod selectable_text;
// Requires the vendor's transform/shaping probe.
#[cfg(feature = "vendor-gpui")]
mod text_geometry;
