//! Bundled, application-owned source distribution for GPUI Rhai.
//!
//! Most users install these sources through `gpui-rhai-cli`. The constants are
//! public so alternative tooling can consume the exact same release snapshot.

macro_rules! bundled_components {
    ($(($constant:ident, $id:literal, $path:literal)),+ $(,)?) => {
        $(pub const $constant: &str = include_str!($path);)+

        /// Every official component keyed by its canonical module ID.
        pub const BUNDLED_COMPONENT_SOURCES_BY_ID: &[(&str, &str)] = &[
            $(($id, $constant),)+
        ];

        /// Every official component source in deterministic installation order.
        pub const BUNDLED_COMPONENT_SOURCES: &[&str] = &[
            $($constant,)+
        ];
    };
}

bundled_components!(
    (
        BUTTON_SOURCE,
        "components/button",
        "../components/button.rhai"
    ),
    (
        ICON_BUTTON_SOURCE,
        "components/icon_button",
        "../components/icon_button.rhai"
    ),
    (LABEL_SOURCE, "components/label", "../components/label.rhai"),
    (ICON_SOURCE, "components/icon", "../components/icon.rhai"),
    (INPUT_SOURCE, "components/input", "../components/input.rhai"),
    (
        TEXTAREA_SOURCE,
        "components/textarea",
        "../components/textarea.rhai"
    ),
    (
        DIVIDER_SOURCE,
        "components/divider",
        "../components/divider.rhai"
    ),
    (
        POPOVER_SOURCE,
        "components/popover",
        "../components/popover.rhai"
    ),
    (
        DIALOG_SOURCE,
        "components/dialog",
        "../components/dialog.rhai"
    ),
    (
        COMBOBOX_SOURCE,
        "components/combobox",
        "../components/combobox.rhai"
    ),
    (
        SELECT_SOURCE,
        "components/select",
        "../components/select.rhai"
    ),
    (
        DATE_PICKER_SOURCE,
        "components/date_picker",
        "../components/date_picker.rhai"
    ),
    (TABLE_SOURCE, "components/table", "../components/table.rhai"),
    (
        PAGINATION_SOURCE,
        "components/pagination",
        "../components/pagination.rhai"
    ),
    (
        CHECKBOX_SOURCE,
        "components/checkbox",
        "../components/checkbox.rhai"
    ),
    (RADIO_SOURCE, "components/radio", "../components/radio.rhai"),
    (
        RADIO_GROUP_SOURCE,
        "components/radio_group",
        "../components/radio_group.rhai"
    ),
    (
        SWITCH_SOURCE,
        "components/switch",
        "../components/switch.rhai"
    ),
    (TAG_SOURCE, "components/tag", "../components/tag.rhai"),
    (
        AVATAR_SOURCE,
        "components/avatar",
        "../components/avatar.rhai"
    ),
    (
        PROGRESS_SOURCE,
        "components/progress",
        "../components/progress.rhai"
    ),
    (
        SKELETON_SOURCE,
        "components/skeleton",
        "../components/skeleton.rhai"
    ),
    (
        FORM_FIELD_SOURCE,
        "components/form_field",
        "../components/form_field.rhai"
    ),
    (
        COLLAPSIBLE_SOURCE,
        "components/collapsible",
        "../components/collapsible.rhai"
    ),
    (
        ACCORDION_SOURCE,
        "components/accordion",
        "../components/accordion.rhai"
    ),
    (TABS_SOURCE, "components/tabs", "../components/tabs.rhai"),
    (
        TOOLTIP_SOURCE,
        "components/tooltip",
        "../components/tooltip.rhai"
    ),
    (MENU_SOURCE, "components/menu", "../components/menu.rhai"),
    (TOAST_SOURCE, "components/toast", "../components/toast.rhai"),
    (ALERT_SOURCE, "components/alert", "../components/alert.rhai"),
    (
        ALERT_DIALOG_SOURCE,
        "components/alert_dialog",
        "../components/alert_dialog.rhai"
    ),
    (BADGE_SOURCE, "components/badge", "../components/badge.rhai"),
    (
        BUTTON_GROUP_SOURCE,
        "components/button_group",
        "../components/button_group.rhai"
    ),
    (CARD_SOURCE, "components/card", "../components/card.rhai"),
    (EMPTY_SOURCE, "components/empty", "../components/empty.rhai"),
    (
        GROUP_BOX_SOURCE,
        "components/group_box",
        "../components/group_box.rhai"
    ),
    (
        INPUT_GROUP_SOURCE,
        "components/input_group",
        "../components/input_group.rhai"
    ),
    (KBD_SOURCE, "components/kbd", "../components/kbd.rhai"),
    (
        TOGGLE_SOURCE,
        "components/toggle",
        "../components/toggle.rhai"
    ),
    (
        TOGGLE_GROUP_SOURCE,
        "components/toggle_group",
        "../components/toggle_group.rhai"
    ),
    (
        SLIDER_SOURCE,
        "components/slider",
        "../components/slider.rhai"
    ),
    (
        CONTEXT_MENU_SOURCE,
        "components/context_menu",
        "../components/context_menu.rhai"
    ),
    (SHEET_SOURCE, "components/sheet", "../components/sheet.rhai"),
    (
        COMMAND_SOURCE,
        "components/command",
        "../components/command.rhai"
    ),
    (
        COMMAND_DIALOG_SOURCE,
        "components/command_dialog",
        "../components/command_dialog.rhai"
    ),
    (
        SPINNER_SOURCE,
        "components/spinner",
        "../components/spinner.rhai"
    ),
    (
        SCROLL_AREA_SOURCE,
        "components/scroll_area",
        "../components/scroll_area.rhai"
    ),
    (
        TITLE_BAR_SOURCE,
        "components/title_bar",
        "../components/title_bar.rhai"
    ),
    (
        STATUS_BAR_SOURCE,
        "components/status_bar",
        "../components/status_bar.rhai"
    ),
    (
        CODE_VIEWER_SOURCE,
        "components/code_viewer",
        "../components/code_viewer.rhai"
    ),
    (
        DIFF_VIEWER_SOURCE,
        "components/diff_viewer",
        "../components/diff_viewer.rhai"
    ),
);

macro_rules! bundled_motion {
    ($(($constant:ident, $id:literal, $path:literal)),+ $(,)?) => {
        $(pub const $constant: &str = include_str!($path);)+

        /// Optional first-party motion components, built only from the public
        /// Runtime API 2 substrate.
        pub const BUNDLED_MOTION_SOURCES_BY_ID: &[(&str, &str)] = &[
            $(($id, $constant),)+
        ];

        pub const BUNDLED_MOTION_SOURCES: &[&str] = &[
            $($constant,)+
        ];
    };
}

bundled_motion!(
    (
        TEXT_REVEAL_SOURCE,
        "motion/text_reveal",
        "../motion/text_reveal.rhai"
    ),
    (
        NUMBER_TICKER_SOURCE,
        "motion/number_ticker",
        "../motion/number_ticker.rhai"
    ),
    (MARQUEE_SOURCE, "motion/marquee", "../motion/marquee.rhai"),
    (SHIMMER_SOURCE, "motion/shimmer", "../motion/shimmer.rhai"),
    (
        BORDER_BEAM_SOURCE,
        "motion/border_beam",
        "../motion/border_beam.rhai"
    ),
    (ORBIT_SOURCE, "motion/orbit", "../motion/orbit.rhai"),
    (
        PARTICLES_SOURCE,
        "motion/particles",
        "../motion/particles.rhai"
    ),
    (
        ANIMATED_TABS_SOURCE,
        "motion/animated_tabs",
        "../motion/animated_tabs.rhai"
    ),
    (
        REORDER_LIST_SOURCE,
        "motion/reorder_list",
        "../motion/reorder_list.rhai"
    ),
    (
        SHARED_LAYOUT_CARDS_SOURCE,
        "motion/shared_layout_cards",
        "../motion/shared_layout_cards.rhai"
    ),
);

/// Optional visualization components backed by the native `charts` feature.
pub const CHART_SOURCE: &str = include_str!("../charts/chart.rhai");
pub const BAR_CHART_SOURCE: &str = include_str!("../charts/bar_chart.rhai");
pub const LINE_CHART_SOURCE: &str = include_str!("../charts/line_chart.rhai");
pub const PIE_CHART_SOURCE: &str = include_str!("../charts/pie_chart.rhai");
pub const MAP_CHART_SOURCE: &str = include_str!("../charts/map_chart.rhai");
pub const BUNDLED_CHART_SOURCES_BY_ID: &[(&str, &str)] = &[
    ("charts/chart", CHART_SOURCE),
    ("charts/bar_chart", BAR_CHART_SOURCE),
    ("charts/line_chart", LINE_CHART_SOURCE),
    ("charts/pie_chart", PIE_CHART_SOURCE),
    ("charts/map_chart", MAP_CHART_SOURCE),
];
pub const BUNDLED_CHART_SOURCES: &[&str] = &[
    CHART_SOURCE,
    BAR_CHART_SOURCE,
    LINE_CHART_SOURCE,
    PIE_CHART_SOURCE,
    MAP_CHART_SOURCE,
];

macro_rules! bundled_assets {
    ($(($constant:ident, $id:literal, $path:literal)),+ $(,)?) => {
        $(pub const $constant: &str = include_str!($path);)+

        /// Every bundled component asset keyed exactly as component metadata declares it.
        pub const BUNDLED_ASSET_SOURCES: &[(&str, &str)] = &[
            $(($id, $constant),)+
        ];
    };
}

bundled_assets!(
    (CHECK_SVG, "icons/check.svg", "../assets/icons/check.svg"),
    (CLOSE_SVG, "icons/close.svg", "../assets/icons/close.svg"),
    (
        CHEVRON_LEFT_SVG,
        "icons/chevron_left.svg",
        "../assets/icons/chevron_left.svg"
    ),
    (
        CHEVRON_RIGHT_SVG,
        "icons/chevron_right.svg",
        "../assets/icons/chevron_right.svg"
    ),
    (
        CALENDAR_SVG,
        "icons/calendar.svg",
        "../assets/icons/calendar.svg"
    ),
    (
        DATE_PREVIOUS_SVG,
        "icons/date_previous.svg",
        "../assets/icons/date_previous.svg"
    ),
    (
        DATE_NEXT_SVG,
        "icons/date_next.svg",
        "../assets/icons/date_next.svg"
    ),
    (
        DISCLOSURE_DOWN_SVG,
        "icons/disclosure_down.svg",
        "../assets/icons/disclosure_down.svg"
    ),
    (
        SORT_ASCENDING_SVG,
        "icons/sort_ascending.svg",
        "../assets/icons/sort_ascending.svg"
    ),
    (
        SORT_DESCENDING_SVG,
        "icons/sort_descending.svg",
        "../assets/icons/sort_descending.svg"
    ),
    (
        CHEVRON_DOWN_SVG,
        "icons/chevron_down.svg",
        "../assets/icons/chevron_down.svg"
    ),
    (
        CHEVRON_UP_SVG,
        "icons/chevron_up.svg",
        "../assets/icons/chevron_up.svg"
    ),
    (MINUS_SVG, "icons/minus.svg", "../assets/icons/minus.svg"),
    (PLUS_SVG, "icons/plus.svg", "../assets/icons/plus.svg"),
    (SEARCH_SVG, "icons/search.svg", "../assets/icons/search.svg"),
    (INFO_SVG, "icons/info.svg", "../assets/icons/info.svg"),
    (
        WARNING_SVG,
        "icons/warning.svg",
        "../assets/icons/warning.svg"
    ),
    (HELP_SVG, "icons/help.svg", "../assets/icons/help.svg"),
);

pub const DEFAULT_THEME: &str = include_str!("../themes/default_dark.rhai");
pub const DEFAULT_LIGHT_THEME: &str = include_str!("../themes/default_light.rhai");
pub const TOKYO_NIGHT_THEME: &str = include_str!("../themes/tokyo_night.rhai");
pub const TOKYO_STORM_THEME: &str = include_str!("../themes/tokyo_storm.rhai");
pub const CATPPUCCIN_LATTE_THEME: &str = include_str!("../themes/catppuccin_latte.rhai");
pub const CATPPUCCIN_MOCHA_THEME: &str = include_str!("../themes/catppuccin_mocha.rhai");
pub const ETHEREAL_THEME: &str = include_str!("../themes/ethereal.rhai");
pub const EVERFOREST_THEME: &str = include_str!("../themes/everforest.rhai");
pub const GRUVBOX_THEME: &str = include_str!("../themes/gruvbox.rhai");
pub const HACKERMAN_THEME: &str = include_str!("../themes/hackerman.rhai");
pub const NORD_THEME: &str = include_str!("../themes/nord.rhai");
pub const RETRO_82_THEME: &str = include_str!("../themes/retro_82.rhai");
pub const HERMARCHY_THEME: &str = include_str!("../themes/hermarchy.rhai");
pub const FUTURISM_THEME: &str = include_str!("../themes/futurism.rhai");
pub const AETHERIA_THEME: &str = include_str!("../themes/aetheria.rhai");

pub const EN_LOCALE: &str = include_str!("../locales/en.rhai");
pub const ZH_CN_LOCALE: &str = include_str!("../locales/zh_cn.rhai");
pub const AR_LOCALE: &str = include_str!("../locales/ar.rhai");
pub const STUDIO_SOURCE: &str = include_str!("../studio/theme_studio.rhai");

/// One deterministic, source-backed case exposed by the first-party Gallery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoryCase {
    pub id: &'static str,
    pub title: &'static str,
    pub purpose: &'static str,
}

/// Static first-party story metadata shared by Gallery, Theme Studio, and tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoryDefinition {
    pub id: &'static str,
    pub title: &'static str,
    pub purpose: &'static str,
    pub category: &'static str,
    pub keywords: &'static [&'static str],
    pub module_ids: &'static [&'static str],
    pub source_module: &'static str,
    pub source: &'static str,
    pub cases: &'static [StoryCase],
    pub fixture: Option<&'static str>,
    pub documentation: &'static str,
    pub theme_studio: bool,
}

pub const BUTTON_STORY_SOURCE: &str = include_str!("../stories/components/button.rhai");
pub const TABS_STORY_SOURCE: &str = include_str!("../stories/components/tabs.rhai");
pub const INPUT_STORY_SOURCE: &str = include_str!("../stories/components/input.rhai");
pub const TABLE_STORY_SOURCE: &str = include_str!("../stories/components/table.rhai");
pub const CHART_INTERACTION_STORY_SOURCE: &str = include_str!("../stories/charts/interaction.rhai");
pub const OPERATIONS_STORY_SOURCE: &str = include_str!("../stories/apps/operations.rhai");
pub const GALLERY_NAVIGATION_SOURCE: &str = include_str!("../stories/gallery/navigation.rhai");
pub const GALLERY_SOURCE_VIEW_SOURCE: &str = include_str!("../stories/gallery/source.rhai");

const BASIC_CASE: &[StoryCase] = &[StoryCase {
    id: "basic",
    title: "Basic",
    purpose: "Exercise the normal controlled interaction path.",
}];

const OPERATIONS_CASES: &[StoryCase] = &[
    StoryCase {
        id: "basic",
        title: "Dashboard",
        purpose: "Enter the complete Operations Workbench on its dashboard.",
    },
    StoryCase {
        id: "config-diff",
        title: "Configuration diff",
        purpose: "Start at the cross-host configuration comparison and deployment flow.",
    },
    StoryCase {
        id: "theme-overrides",
        title: "Host theme overrides",
        purpose: "Apply nonzero Host radii uniformly and preserve Workbench state across themes.",
    },
];

pub const BUNDLED_STORIES: &[StoryDefinition] = &[
    StoryDefinition {
        id: "components/button",
        title: "Button, Badge, and Tag",
        purpose: "Compare action, status, and metadata density with real activation.",
        category: "actions",
        keywords: &["action", "badge", "tag", "density", "button"],
        module_ids: &["components/button", "components/badge", "components/tag"],
        source_module: "stories/components/button",
        source: BUTTON_STORY_SOURCE,
        cases: BASIC_CASE,
        fixture: None,
        documentation: "docs/components/catalog.md#actions-choices-and-forms",
        theme_studio: true,
    },
    StoryDefinition {
        id: "components/input",
        title: "Input and Textarea",
        purpose: "Exercise controlled single-line and multiline native editing.",
        category: "forms",
        keywords: &["form", "ime", "text", "input", "textarea"],
        module_ids: &["components/input", "components/textarea"],
        source_module: "stories/components/input",
        source: INPUT_STORY_SOURCE,
        cases: BASIC_CASE,
        fixture: None,
        documentation: "docs/components/catalog.md#actions-choices-and-forms",
        theme_studio: true,
    },
    StoryDefinition {
        id: "components/tabs",
        title: "Tabs",
        purpose: "Exercise controlled selection, content layout, and disabled navigation.",
        category: "navigation",
        keywords: &["navigation", "selection", "panel", "tabs"],
        module_ids: &["components/tabs"],
        source_module: "stories/components/tabs",
        source: TABS_STORY_SOURCE,
        cases: BASIC_CASE,
        fixture: None,
        documentation: "docs/components/catalog.md#tabs",
        theme_studio: true,
    },
    StoryDefinition {
        id: "components/table",
        title: "Table",
        purpose: "Exercise controlled sorting, selection, and semantic cell adornments.",
        category: "data",
        keywords: &["data", "virtual", "selection", "badge", "table"],
        module_ids: &["components/table", "components/badge"],
        source_module: "stories/components/table",
        source: TABLE_STORY_SOURCE,
        cases: BASIC_CASE,
        fixture: None,
        documentation: "docs/components/catalog.md#navigation-and-data",
        theme_studio: true,
    },
    StoryDefinition {
        id: "charts/interaction",
        title: "Chart titles and wheel interaction",
        purpose: "Compare absent and explicit titles while preserving parent scrolling.",
        category: "charts",
        keywords: &["chart", "scroll", "wheel", "zoom", "title"],
        module_ids: &["charts/chart", "components/scroll_area"],
        source_module: "stories/charts/interaction",
        source: CHART_INTERACTION_STORY_SOURCE,
        cases: BASIC_CASE,
        fixture: None,
        documentation: "docs/charts.md",
        theme_studio: false,
    },
    StoryDefinition {
        id: "apps/operations",
        title: "Operations Workbench",
        purpose: "Complete a cross-page host inspection, configuration, and deployment task.",
        category: "applications",
        keywords: &["application", "operations", "hosts", "config", "deployment"],
        module_ids: &[
            "components/button",
            "components/badge",
            "components/command",
            "components/dialog",
            "components/input",
            "components/table",
            "components/code_viewer",
            "components/diff_viewer",
            "components/progress",
            "components/toast",
            "components/title_bar",
            "components/status_bar",
            "charts/chart",
        ],
        source_module: "stories/apps/operations",
        source: OPERATIONS_STORY_SOURCE,
        cases: OPERATIONS_CASES,
        fixture: Some("operations"),
        documentation: "docs/gallery.md#operations-workbench",
        theme_studio: false,
    },
];

pub const BUNDLED_THEME_SOURCES: &[(&str, &str)] = &[
    ("default_dark.rhai", DEFAULT_THEME),
    ("default_light.rhai", DEFAULT_LIGHT_THEME),
    ("tokyo_night.rhai", TOKYO_NIGHT_THEME),
    ("tokyo_storm.rhai", TOKYO_STORM_THEME),
    ("catppuccin_latte.rhai", CATPPUCCIN_LATTE_THEME),
    ("catppuccin_mocha.rhai", CATPPUCCIN_MOCHA_THEME),
    ("ethereal.rhai", ETHEREAL_THEME),
    ("everforest.rhai", EVERFOREST_THEME),
    ("gruvbox.rhai", GRUVBOX_THEME),
    ("hackerman.rhai", HACKERMAN_THEME),
    ("nord.rhai", NORD_THEME),
    ("retro_82.rhai", RETRO_82_THEME),
    ("hermarchy.rhai", HERMARCHY_THEME),
    ("futurism.rhai", FUTURISM_THEME),
    ("aetheria.rhai", AETHERIA_THEME),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_snapshot_has_the_expected_catalog_size() {
        assert_eq!(BUNDLED_COMPONENT_SOURCES.len(), 51);
        assert_eq!(BUNDLED_COMPONENT_SOURCES_BY_ID.len(), 51);
        assert_eq!(BUNDLED_MOTION_SOURCES.len(), 10);
        assert_eq!(BUNDLED_MOTION_SOURCES_BY_ID.len(), 10);
        assert_eq!(BUNDLED_CHART_SOURCES.len(), 5);
        assert_eq!(BUNDLED_CHART_SOURCES_BY_ID.len(), 5);
        assert_eq!(BUNDLED_ASSET_SOURCES.len(), 18);
        assert_eq!(BUNDLED_THEME_SOURCES.len(), 15);
        assert_eq!(BUNDLED_STORIES.len(), 6);
        assert!(
            BUNDLED_COMPONENT_SOURCES
                .iter()
                .all(|source| !source.is_empty())
        );
        assert!(
            BUNDLED_MOTION_SOURCES
                .iter()
                .all(|source| !source.is_empty())
        );
        assert!(
            BUNDLED_CHART_SOURCES
                .iter()
                .all(|source| !source.is_empty())
        );
        assert!(
            BUNDLED_THEME_SOURCES
                .iter()
                .all(|(_, source)| !source.is_empty())
        );
        assert_eq!(
            BUNDLED_COMPONENT_SOURCES_BY_ID
                .iter()
                .map(|(id, _)| *id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            BUNDLED_COMPONENT_SOURCES_BY_ID.len()
        );
        assert_eq!(
            BUNDLED_ASSET_SOURCES
                .iter()
                .map(|(id, _)| *id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            BUNDLED_ASSET_SOURCES.len()
        );
        let story_ids = BUNDLED_STORIES
            .iter()
            .map(|story| story.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(story_ids.len(), BUNDLED_STORIES.len());
        for story in BUNDLED_STORIES {
            assert!(!story.source.is_empty(), "{} source is empty", story.id);
            assert!(!story.cases.is_empty(), "{} has no cases", story.id);
            assert!(story.source_module.starts_with("stories/"));
            let case_ids = story
                .cases
                .iter()
                .map(|case| case.id)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(case_ids.len(), story.cases.len(), "{} case IDs", story.id);
            for module in story.module_ids {
                assert!(
                    BUNDLED_COMPONENT_SOURCES_BY_ID
                        .iter()
                        .chain(BUNDLED_MOTION_SOURCES_BY_ID)
                        .chain(BUNDLED_CHART_SOURCES_BY_ID)
                        .any(|(id, _)| id == module),
                    "{} references unknown module {module}",
                    story.id
                );
            }
        }
        assert!(!CHART_SOURCE.contains("title: prop_or(props, \"title\", \"\")"));
        for source in [
            BAR_CHART_SOURCE,
            LINE_CHART_SOURCE,
            PIE_CHART_SOURCE,
            MAP_CHART_SOURCE,
        ] {
            assert!(!source.contains("title: #{ schema: #{ type: \"string\" }, required: false, sensitive: false, \"default\""));
        }
    }
}
