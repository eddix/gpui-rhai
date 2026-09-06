//! Bundled, application-owned source distribution for GPUI Rhai.
//!
//! Most users install these sources through `gpui-rhai-cli`. The constants are
//! public so alternative tooling can consume the exact same release snapshot.

pub const BUTTON_SOURCE: &str = include_str!("../components/button.rhai");
pub const LABEL_SOURCE: &str = include_str!("../components/label.rhai");
pub const ICON_SOURCE: &str = include_str!("../components/icon.rhai");
pub const INPUT_SOURCE: &str = include_str!("../components/input.rhai");
pub const TEXTAREA_SOURCE: &str = include_str!("../components/textarea.rhai");
pub const DIVIDER_SOURCE: &str = include_str!("../components/divider.rhai");
pub const POPOVER_SOURCE: &str = include_str!("../components/popover.rhai");
pub const DIALOG_SOURCE: &str = include_str!("../components/dialog.rhai");
pub const COMBOBOX_SOURCE: &str = include_str!("../components/combobox.rhai");
pub const SELECT_SOURCE: &str = include_str!("../components/select.rhai");
pub const DATE_PICKER_SOURCE: &str = include_str!("../components/date_picker.rhai");
pub const TABLE_SOURCE: &str = include_str!("../components/table.rhai");
pub const PAGINATION_SOURCE: &str = include_str!("../components/pagination.rhai");
pub const CHECKBOX_SOURCE: &str = include_str!("../components/checkbox.rhai");
pub const RADIO_SOURCE: &str = include_str!("../components/radio.rhai");
pub const RADIO_GROUP_SOURCE: &str = include_str!("../components/radio_group.rhai");
pub const SWITCH_SOURCE: &str = include_str!("../components/switch.rhai");
pub const TAG_SOURCE: &str = include_str!("../components/tag.rhai");
pub const AVATAR_SOURCE: &str = include_str!("../components/avatar.rhai");
pub const PROGRESS_SOURCE: &str = include_str!("../components/progress.rhai");
pub const SKELETON_SOURCE: &str = include_str!("../components/skeleton.rhai");
pub const FORM_FIELD_SOURCE: &str = include_str!("../components/form_field.rhai");
pub const COLLAPSIBLE_SOURCE: &str = include_str!("../components/collapsible.rhai");
pub const ACCORDION_SOURCE: &str = include_str!("../components/accordion.rhai");
pub const TABS_SOURCE: &str = include_str!("../components/tabs.rhai");
pub const TOOLTIP_SOURCE: &str = include_str!("../components/tooltip.rhai");
pub const MENU_SOURCE: &str = include_str!("../components/menu.rhai");
pub const TOAST_SOURCE: &str = include_str!("../components/toast.rhai");
pub const ALERT_SOURCE: &str = include_str!("../components/alert.rhai");
pub const ALERT_DIALOG_SOURCE: &str = include_str!("../components/alert_dialog.rhai");
pub const BADGE_SOURCE: &str = include_str!("../components/badge.rhai");
pub const BUTTON_GROUP_SOURCE: &str = include_str!("../components/button_group.rhai");
pub const CARD_SOURCE: &str = include_str!("../components/card.rhai");
pub const EMPTY_SOURCE: &str = include_str!("../components/empty.rhai");
pub const GROUP_BOX_SOURCE: &str = include_str!("../components/group_box.rhai");
pub const INPUT_GROUP_SOURCE: &str = include_str!("../components/input_group.rhai");
pub const KBD_SOURCE: &str = include_str!("../components/kbd.rhai");
pub const TOGGLE_SOURCE: &str = include_str!("../components/toggle.rhai");
pub const TOGGLE_GROUP_SOURCE: &str = include_str!("../components/toggle_group.rhai");
pub const SLIDER_SOURCE: &str = include_str!("../components/slider.rhai");
pub const CONTEXT_MENU_SOURCE: &str = include_str!("../components/context_menu.rhai");
pub const SHEET_SOURCE: &str = include_str!("../components/sheet.rhai");
pub const COMMAND_SOURCE: &str = include_str!("../components/command.rhai");
pub const COMMAND_DIALOG_SOURCE: &str = include_str!("../components/command_dialog.rhai");
pub const SPINNER_SOURCE: &str = include_str!("../components/spinner.rhai");
pub const SCROLL_AREA_SOURCE: &str = include_str!("../components/scroll_area.rhai");
pub const TITLE_BAR_SOURCE: &str = include_str!("../components/title_bar.rhai");
pub const STATUS_BAR_SOURCE: &str = include_str!("../components/status_bar.rhai");
pub const CODE_VIEWER_SOURCE: &str = include_str!("../components/code_viewer.rhai");
pub const DIFF_VIEWER_SOURCE: &str = include_str!("../components/diff_viewer.rhai");

/// Every official component source in deterministic installation order.
pub const BUNDLED_COMPONENT_SOURCES: &[&str] = &[
    BUTTON_SOURCE,
    LABEL_SOURCE,
    ICON_SOURCE,
    INPUT_SOURCE,
    TEXTAREA_SOURCE,
    DIVIDER_SOURCE,
    POPOVER_SOURCE,
    DIALOG_SOURCE,
    COMBOBOX_SOURCE,
    SELECT_SOURCE,
    DATE_PICKER_SOURCE,
    TABLE_SOURCE,
    PAGINATION_SOURCE,
    CHECKBOX_SOURCE,
    RADIO_SOURCE,
    RADIO_GROUP_SOURCE,
    SWITCH_SOURCE,
    TAG_SOURCE,
    AVATAR_SOURCE,
    PROGRESS_SOURCE,
    SKELETON_SOURCE,
    FORM_FIELD_SOURCE,
    COLLAPSIBLE_SOURCE,
    ACCORDION_SOURCE,
    TABS_SOURCE,
    TOOLTIP_SOURCE,
    MENU_SOURCE,
    TOAST_SOURCE,
    ALERT_SOURCE,
    ALERT_DIALOG_SOURCE,
    BADGE_SOURCE,
    BUTTON_GROUP_SOURCE,
    CARD_SOURCE,
    EMPTY_SOURCE,
    GROUP_BOX_SOURCE,
    INPUT_GROUP_SOURCE,
    KBD_SOURCE,
    TOGGLE_SOURCE,
    TOGGLE_GROUP_SOURCE,
    SLIDER_SOURCE,
    CONTEXT_MENU_SOURCE,
    SHEET_SOURCE,
    COMMAND_SOURCE,
    COMMAND_DIALOG_SOURCE,
    SPINNER_SOURCE,
    SCROLL_AREA_SOURCE,
    TITLE_BAR_SOURCE,
    STATUS_BAR_SOURCE,
    CODE_VIEWER_SOURCE,
    DIFF_VIEWER_SOURCE,
];

pub const CHECK_SVG: &str = include_str!("../assets/icons/check.svg");
pub const CLOSE_SVG: &str = include_str!("../assets/icons/close.svg");
pub const CHEVRON_LEFT_SVG: &str = include_str!("../assets/icons/chevron_left.svg");
pub const CHEVRON_RIGHT_SVG: &str = include_str!("../assets/icons/chevron_right.svg");
pub const CALENDAR_SVG: &str = include_str!("../assets/icons/calendar.svg");
pub const DATE_PREVIOUS_SVG: &str = include_str!("../assets/icons/date_previous.svg");
pub const DATE_NEXT_SVG: &str = include_str!("../assets/icons/date_next.svg");
pub const DISCLOSURE_DOWN_SVG: &str = include_str!("../assets/icons/disclosure_down.svg");
pub const SORT_ASCENDING_SVG: &str = include_str!("../assets/icons/sort_ascending.svg");
pub const SORT_DESCENDING_SVG: &str = include_str!("../assets/icons/sort_descending.svg");
pub const CHEVRON_DOWN_SVG: &str = include_str!("../assets/icons/chevron_down.svg");
pub const CHEVRON_UP_SVG: &str = include_str!("../assets/icons/chevron_up.svg");
pub const MINUS_SVG: &str = include_str!("../assets/icons/minus.svg");
pub const PLUS_SVG: &str = include_str!("../assets/icons/plus.svg");
pub const SEARCH_SVG: &str = include_str!("../assets/icons/search.svg");
pub const INFO_SVG: &str = include_str!("../assets/icons/info.svg");
pub const WARNING_SVG: &str = include_str!("../assets/icons/warning.svg");
pub const HELP_SVG: &str = include_str!("../assets/icons/help.svg");

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
        assert_eq!(BUNDLED_COMPONENT_SOURCES.len(), 50);
        assert_eq!(BUNDLED_THEME_SOURCES.len(), 15);
        assert!(
            BUNDLED_COMPONENT_SOURCES
                .iter()
                .all(|source| !source.is_empty())
        );
        assert!(
            BUNDLED_THEME_SOURCES
                .iter()
                .all(|(_, source)| !source.is_empty())
        );
    }
}
