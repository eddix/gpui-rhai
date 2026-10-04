// A bring-your-own-design theme: its own color vocabulary and its own length
// namespace. It loads without the gpui-rhai token base, because no official
// component is mounted and nothing requires the design language's tokens.
fn theme() {
    #{
        family: "Treemap",
        name: "Night",
        mode: "dark",
        tokens: #{
            colors: #{
                ground: 0x101418ff,
                panel: 0x161c22ff,
                ink: 0xe6edf3ff,
                faint: 0x8b98a5ff,
                rule: 0x2a333cff,
                pick: 0xf2c94cff,
                "tile.code": 0x2f5d50ff,
                "tile.media": 0x5b3f6bff,
                "tile.data": 0x31506bff,
                "tile.other": 0x4a4a3aff,
            },
            treemap: #{
                gutter: px(2),
                pad: px(6),
                bar: px(36),
            },
        },
    }
}
