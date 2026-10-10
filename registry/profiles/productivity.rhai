// gpui-rhai productivity profile.
//
// The reference policy for keyboard-first productivity tools: the design
// language defaults (ui/tokens.rhai), comfortable density, square corners,
// and every composition rule. See docs/design/composition.md#10-profiles.
fn profile() {
    #{
        name: "productivity",
        audit: [
            "row-height-mismatch",
            "text-edge-misaligned",
            "spacing-not-nested",
            "multiple-solid-actions",
            "mixed-type-in-row",
            "low-contrast-text",
            "unresolved-font",
        ],
        check: ["literal-geometry"],
    }
}
