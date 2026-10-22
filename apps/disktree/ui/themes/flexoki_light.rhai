// disktree-rhai light palette — Flexoki Light (Steph Ango's Flexoki, light
// mode). The dark default lives in ../theme.rhai; both are variants of the
// one "Disktree" family, so the app can switch without re-mounting. Colors
// only: layout tokens come from the token base (../tokens.rhai).
fn theme() {
    #{
        family: "Disktree",
        name: "Flexoki Light",
        mode: "light",
        tokens: #{
            colors: #{
                surface: 0xfffcf0ff,
                surface_raised: 0xf2f0e5ff,
                surface_hover: 0xe6e4d9ff,
                text_primary: 0x100f0fff,
                text_muted: 0x6f6a63ff,
                accent: 0x4385beff,
                accent_hover: 0x36679bff,
                on_accent: 0xfffcf0ff,
                danger: 0xaf3029ff,
                on_danger: 0xfffcf0ff,
                warning: 0xbc5215ff,
                on_warning: 0xfffcf0ff,
                success: 0x66800bff,
                on_success: 0xfffcf0ff,
                border: 0xe0ddd0ff,
                focus_ring: 0x248a9aff,
                selection: 0xe6e4d9ff,
                disabled: 0x87837dff,
                scrim: 0x100f0f33,
            },
        },
    }
}
