// disktree-rhai default palette — Tokyo Night, matching tobi/disktree's dark
// fallback. gpui-rhai 0.2: palettes own colors only; typography, spacing,
// metrics and radii come from the token base (ui/tokens.rhai), so switching
// palette never changes layout. Category palette constants live in core.rhai;
// they derive fills from these surface/accent tokens at render time.
fn theme() {
    #{
        family: "Disktree",
        name: "Tokyo Night",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x1a1b26ff,
                surface_raised: 0x24283bff,
                surface_hover: 0x292e42ff,
                text_primary: 0xc0caf5ff,
                text_muted: 0xa9b1d6ff,
                accent: 0x7aa2f7ff,
                accent_hover: 0x89b4faff,
                on_accent: 0x1a1b26ff,
                danger: 0xf7768eff,
                on_danger: 0x1a1b26ff,
                warning: 0xe0af68ff,
                on_warning: 0x1a1b26ff,
                success: 0x9ece6aff,
                on_success: 0x1a1b26ff,
                border: 0x3b4261ff,
                focus_ring: 0x7dcfffff,
                selection: 0x292e42ff,
                disabled: 0x565f89ff,
                scrim: 0x10121bcc,
            },
        },
    }
}
