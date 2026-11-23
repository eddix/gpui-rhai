// Project-owned Default Dark palette: warm graphite, paper ink, cobalt.
// Palettes own colors only; layout tokens live in tokens.rhai.
fn theme() {
    #{
        family: "Default",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface: 0x151412ff,
                surface_raised: 0x21201eff,
                surface_hover: 0x2e2d2bff,
                text_primary: 0xe8e4daff,
                text_muted: 0xbcb7aaff,
                accent: 0x3d5fe0ff,
                accent_hover: 0x3454d4ff,
                on_accent: 0xffffffff,
                danger: 0xef6b5eff,
                on_danger: 0x151412ff,
                warning: 0xe2a640ff,
                on_warning: 0x151412ff,
                success: 0x7cbd78ff,
                on_success: 0x151412ff,
                border: 0x50504eff,
                focus_ring: 0xe8e4daff,
                selection: 0x141e3bff,
                disabled: 0x737270ff,
            },
        },
    }
}
