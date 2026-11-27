//! Helpers shared across areas: mounting a view on the official registry (`Mount`, `Host`,
//! `bundled_assets`), letting it settle (`settle` with a `Step`), reading it (`nodes`,
//! `node_by_test_id`, `text_of`, `bounds`, `last_error`), acting on it (`click_center`,
//! `dispatch`) and reading the painted quads in logical pixels (`painted_quads`, `filled`).

mod differential;
mod host;
mod mount;
pub(crate) mod paint;
mod preset_risks;
mod query;
mod settle;

pub(crate) use differential::Lockstep;
pub(crate) use host::{
    KeyboardHost, click_palette_trigger, dispatch_script_button, node_texts, palette_texts,
    wait_for_view_text,
};
pub(crate) use mount::{ActiveHost, Host, Mount, bundled_assets, registry_source};
pub(crate) use paint::{
    PaintedQuad, Rect, filled, filled_including_rounded_clips, glyph_rects_with_ink,
    glyphs_with_ink, live_fills, logical_paint_rect, painted_quads,
    painted_quads_including_rounded_clips, walk_live_paint,
};
pub(crate) use preset_risks::PresetRisks;
pub(crate) use query::{
    bounds, by_role, by_test_id, center, click_center, dispatch, last_error, node_by_test_id,
    nodes, snapshot, text_of,
};
pub(crate) use settle::{Step, settle};

/// Translate GPUI's platform-neutral spelling before passing explicit
/// keystrokes to the native test context.
pub(crate) fn primary_keys(chords: &str) -> String {
    chords.replace(
        "secondary-",
        if cfg!(target_os = "macos") {
            "cmd-"
        } else {
            "ctrl-"
        },
    )
}

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
