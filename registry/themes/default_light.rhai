// Project-owned Default Light palette: paper, ink, cobalt.
// Palettes own colors only; layout tokens live in tokens.rhai.
fn theme() {
    #{
        family: "Default",
        name: "Light",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xf3f1eaff,
                surface_raised: 0xfdfbf4ff,
                surface_hover: 0xdfddd7ff,
                text_primary: 0x1c1b18ff,
                text_muted: 0x514d44ff,
                accent: 0x1f45b8ff,
                accent_hover: 0x173aacff,
                on_accent: 0xffffffff,
                danger: 0xb3261eff,
                on_danger: 0xffffffff,
                warning: 0x855600ff,
                on_warning: 0xffffffff,
                success: 0x2e6b36ff,
                on_success: 0xffffffff,
                border: 0xadaca7ff,
                focus_ring: 0x1c1b18ff,
                selection: 0xbdcdeeff,
                disabled: 0x817f7bff,
            },
        },
    }
}
