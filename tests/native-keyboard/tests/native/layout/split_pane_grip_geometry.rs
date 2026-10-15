//! A decorative grip stays on the separator when its styled thickness changes (#159).

use gpui::TestAppContext;
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

use crate::support::{Mount, Step, bounds, filled, settle};

const MARK: u32 = 0xff00_ffff;

#[gpui::test]
fn grip_follows_separator_thickness_in_both_orientations_and_directions(cx: &mut TestAppContext) {
    for orientation in ["horizontal", "vertical"] {
        for locale in ["en", "ar"] {
            for thickness in [8, 1, 16] {
                let dimension = if orientation == "horizontal" {
                    "width"
                } else {
                    "height"
                };
                let main = format!(
                    r#"import "components/split_pane" as split;
fn init(ctx) {{ ctx.set_locale("{locale}"); }}
fn view(ctx) {{
    split::SplitPane(#{{ key: "layout", label: "Panels", size: 0.5,
        orientation: "{orientation}", min_start: 0.0, min_end: 0.0,
        start: text("Start"), end: text("End"), line: false,
        handle: box([]).with_style(style().width(px(4)).height(px(4)).background(rgba({MARK:#x}))),
        part_styles: #{{ handle: style().{dimension}(px({thickness})) }}
    }}).with_style(style().width(px(420)).height(px(180)))
}}"#
                );
                let case = format!("{orientation}-{locale}-{thickness}");
                let (mut visual, view) = Mount::components(main)
                    .locale("en.rhai", EN_LOCALE)
                    .locale("ar.rhai", AR_LOCALE)
                    .id(&case)
                    .mount(cx);
                settle(&mut visual, Step::Tick, 4);
                let separator = bounds(&mut visual, &view, "separator", "Panels");
                let quads = filled(&mut visual, MARK);
                assert_eq!(quads.len(), 1, "{case}: exactly one grip must paint");
                let grip = quads[0].bounds;
                let (expected, actual) = if orientation == "horizontal" {
                    (
                        separator.x + separator.width / 2.0,
                        f64::from(grip.x + grip.width / 2.0),
                    )
                } else {
                    (
                        separator.y + separator.height / 2.0,
                        f64::from(grip.y + grip.height / 2.0),
                    )
                };
                assert!(
                    (expected - actual).abs() < 0.5,
                    "{case}: separator center {expected}, grip center {actual}"
                );
            }
        }
    }
}
