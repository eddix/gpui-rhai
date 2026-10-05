// Shared by the visual examples through `include!`: the registry icon set, so every
// asset a component declares is available to preparation.

fn registry_icons() -> Vec<(String, gpui_rhai::AssetData)> {
    [
        ("calendar", include_bytes!("../../../../registry/assets/icons/calendar.svg").as_slice()),
        ("check", include_bytes!("../../../../registry/assets/icons/check.svg").as_slice()),
        ("chevron_down", include_bytes!("../../../../registry/assets/icons/chevron_down.svg").as_slice()),
        ("chevron_left", include_bytes!("../../../../registry/assets/icons/chevron_left.svg").as_slice()),
        ("chevron_right", include_bytes!("../../../../registry/assets/icons/chevron_right.svg").as_slice()),
        ("chevron_up", include_bytes!("../../../../registry/assets/icons/chevron_up.svg").as_slice()),
        ("close", include_bytes!("../../../../registry/assets/icons/close.svg").as_slice()),
        ("date_next", include_bytes!("../../../../registry/assets/icons/date_next.svg").as_slice()),
        ("date_previous", include_bytes!("../../../../registry/assets/icons/date_previous.svg").as_slice()),
        ("disclosure_down", include_bytes!("../../../../registry/assets/icons/disclosure_down.svg").as_slice()),
        ("help", include_bytes!("../../../../registry/assets/icons/help.svg").as_slice()),
        ("info", include_bytes!("../../../../registry/assets/icons/info.svg").as_slice()),
        ("minus", include_bytes!("../../../../registry/assets/icons/minus.svg").as_slice()),
        ("plus", include_bytes!("../../../../registry/assets/icons/plus.svg").as_slice()),
        ("search", include_bytes!("../../../../registry/assets/icons/search.svg").as_slice()),
        ("sort_ascending", include_bytes!("../../../../registry/assets/icons/sort_ascending.svg").as_slice()),
        ("sort_descending", include_bytes!("../../../../registry/assets/icons/sort_descending.svg").as_slice()),
        ("warning", include_bytes!("../../../../registry/assets/icons/warning.svg").as_slice()),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        (
            format!("icons/{name}"),
            gpui_rhai::AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: bytes.to_vec(),
            },
        )
    })
    .collect()
}
