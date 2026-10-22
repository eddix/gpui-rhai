// disktree-rhai default theme — Tokyo Night based, matching tobi/disktree's
// dark fallback (Tokyo Night) with disktree's zero-radius Omarchy-style scale.
// Category palette constants live in core.rhai; they derive fills from these
// surface/accent tokens at render time.
fn theme() {
    #{
        family: "Disktree",
        name: "Minimal Gray",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x16161aff,
                surface_raised: 0x1e1e22ff,
                surface_hover: 0x242428ff,
                text_primary: 0xd4d4d8ff,
                text_muted: 0x8a8a94ff,
                accent: 0xa1a1aaff,
                accent_hover: 0xb4b4bcff,
                on_accent: 0x1a1b26ff,
                danger: 0xf7768eff,
                on_danger: 0x1a1b26ff,
                warning: 0xe0af68ff,
                on_warning: 0x1a1b26ff,
                success: 0x9ece6aff,
                on_success: 0x1a1b26ff,
                border: 0x2a2a30ff,
                focus_ring: 0x7dcfffff,
                selection: 0x292e42ff,
                disabled: 0x565f89ff,
                scrim: 0x10121bcc,
            },
        },
    }
}
