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
                surface_raised: 0x1d1c19ff,
                surface_hover: 0x2e2d28ff,
                text_primary: 0xe8e4daff,
                text_muted: 0xa19c90ff,
                accent: 0x3d5fe0ff,
                accent_hover: 0x5474eaff,
                on_accent: 0xffffffff,
                danger: 0xef6b5eff,
                on_danger: 0x151412ff,
                warning: 0xe2a640ff,
                on_warning: 0x151412ff,
                success: 0x7cbd78ff,
                on_success: 0x151412ff,
                border: 0x3a3833ff,
                focus_ring: 0xe8e4daff,
                selection: 0x212a50ff,
                disabled: 0x6b675eff,
            },
        },
    }
}
