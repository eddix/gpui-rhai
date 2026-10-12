//! Helpers shared across areas.

mod host;

pub(crate) use host::{
    KeyboardHost, SingleEmbeddedHost, click_palette_trigger, dispatch_script_button, node_texts,
    official_icon_assets, palette_texts, wait_for_view_text,
};

/// Builds fixture variants by replacing exact places in the source.
///
/// A variant built with `str::replace` silently becomes the base case when the fixture text
/// changes and the pattern no longer matches. These panic instead, and also when the pattern
/// occurs more often than the variant expects, so a variant always changes exactly the places
/// it names.
pub(crate) trait ReplaceOnce {
    /// Replaces the one occurrence of `from`.
    fn replace_once(&self, from: &str, to: &str) -> String;
    /// Replaces `from` where it occurs exactly `times` times.
    fn replace_exactly(&self, from: &str, to: &str, times: usize) -> String;
}

impl ReplaceOnce for str {
    #[track_caller]
    fn replace_once(&self, from: &str, to: &str) -> String {
        self.replace_exactly(from, to, 1)
    }

    #[track_caller]
    fn replace_exactly(&self, from: &str, to: &str, times: usize) -> String {
        let found = self.matches(from).count();
        assert!(
            found == times,
            "fixture variant: {from:?} occurs {found} times in the source, expected {times}"
        );
        self.replace(from, to)
    }
}
