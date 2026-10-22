// disktree-rhai productivity profile: the design-system audit rules.
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
