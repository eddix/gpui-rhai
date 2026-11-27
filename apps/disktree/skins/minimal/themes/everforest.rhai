// Semantic adaptation of Omarchy Everforest (MIT).
// Source: https://github.com/basecamp/omarchy/tree/master/themes/everforest
// Adjusted for the design-system palette constraints (docs/design/themes.md):
// - text_muted raised to 4.5:1 on hover and selection surfaces.
// - selection tinted toward the accent so it differs from hover.
fn theme() {
    #{
        family: "Everforest",
        name: "Dark",
        mode: "dark",
        tokens: #{
            colors: #{
                surface_sunken: 0x242c32ff,
                surface: 0x2d353bff,
                surface_raised: 0x3b4248ff,
                surface_elevated: 0x464d53ff,
                surface_hover: 0x4a5157ff,
                text_primary: 0xfbeed1ff,
                text_muted: 0xcfd3c3ff,
                accent: 0x7fbbb3ff,
                accent_hover: 0x8ac6beff,
                on_accent: 0x21272cff,
                danger: 0xe67e80ff,
                on_danger: 0x21272cff,
                warning: 0xdbbc7fff,
                on_warning: 0x21272cff,
                success: 0xa7c080ff,
                on_success: 0x21272cff,
                border_subtle: 0x505557ff,
                border: 0x696f73ff,
                border_strong: 0x8f918dff,
                focus_ring: 0x83c092ff,
                selection: 0x314648ff,
                disabled: 0x949a9fff,
                "shadow.ambient": 0x00000052,
                "shadow.key": 0x0000007a,
                "palette.blue": 0x7fbbb3ff, "palette.blue_ink": 0x9edbd2ff, "palette.blue_soft": 0x495a5dff, "palette.blue_border": 0x587575ff,
                "palette.violet": 0xd699b6ff, "palette.violet_ink": 0xfbbbd9ff, "palette.violet_soft": 0x5a535eff, "palette.violet_border": 0x7c6776ff,
                "palette.teal": 0x83c092ff, "palette.teal_ink": 0xa0dfafff, "palette.teal_soft": 0x495b57ff, "palette.teal_border": 0x597767ff,
                "palette.green": 0xa7c080ff, "palette.green_ink": 0xc1db99ff, "palette.green_soft": 0x515b53ff, "palette.green_border": 0x687760ff,
                "palette.amber": 0xdbbc7fff, "palette.amber_ink": 0xefd092ff, "palette.amber_soft": 0x5b5a53ff, "palette.amber_border": 0x7e755fff,
                "palette.rose": 0xe67e80ff, "palette.rose_ink": 0xffb5b4ff, "palette.rose_soft": 0x5d4e53ff, "palette.rose_border": 0x835b60ff,
                "palette.orange": 0xe69875ff, "palette.orange_ink": 0xffbea1ff, "palette.orange_soft": 0x5d5351ff, "palette.orange_border": 0x83665bff,
                "palette.slate": 0x9da9a0ff, "palette.slate_ink": 0xc4d1c7ff, "palette.slate_soft": 0x4f575aff, "palette.slate_border": 0x646d6dff,
            },
        },
    }
}
