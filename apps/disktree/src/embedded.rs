//! The built-in default skin.
//!
//! The `ui/` directory is the editable source of truth; `build.rs` embeds it
//! into the binary so a stock `disktree-rhai` run needs no files on disk, and
//! Cargo embeds it again whenever a file under `ui/` changes. `--export-skin`
//! writes these same files out for user modification.
//!
//! Since the 0.2 migration the skin rides the official component set: the
//! token base (`tokens.rhai`) sits under the palettes, and the chrome mounts
//! the copied `components/` + `layouts/` + `patterns/` sources. Assets are the
//! SVGs official components declare (checkbox marks).

/// Every file of `ui/`, as `ScriptView::embedded` reads it.
pub static UI: gpui_rhai::EmbeddedUi =
    gpui_rhai::EmbeddedUi::new(include!(concat!(env!("OUT_DIR"), "/gpui_rhai_ui.rs")));

#[cfg(test)]
mod tests {
    const CORE: &str = include_str!("../ui/core.rhai");

    /// Run one pure function of the embedded core module.
    fn core_call_i64(name: &str, arg: i64) -> String {
        let mut engine = rhai::Engine::new();
        engine.set_max_expr_depths(1_000, 1_000);
        let ast = engine.compile(CORE).expect("core.rhai compiles");
        let mut scope = rhai::Scope::new();
        engine
            .call_fn(&mut scope, &ast, name, (arg,))
            .expect("core function call")
    }

    #[test]
    fn bytes_step_up_at_1023_5_of_a_unit() {
        // Below the boundary the value keeps its unit; at 10 and up no
        // decimal is shown, so 1023.499 KiB prints "1023 KiB".
        assert_eq!(core_call_i64("human_bytes", 1_048_063), "1023 KiB");
        assert_eq!(core_call_i64("human_bytes", 1_047_552), "1023 KiB");
        // From 1023.5 of a unit the next unit takes over: 1024 of a unit
        // would print instead of 1.0 of the next (tobi PR #70).
        assert_eq!(core_call_i64("human_bytes", 1_048_064), "1.0 MiB");
        assert_eq!(core_call_i64("human_bytes", 1_048_575), "1.0 MiB");
        assert_eq!(core_call_i64("human_bytes", 1_048_576), "1.0 MiB");
        // The short form follows the same boundary.
        assert_eq!(core_call_i64("human_bytes_short", 1_048_063), "1023KiB");
        assert_eq!(core_call_i64("human_bytes_short", 1_048_064), "1.0MiB");
        // A value under 10 keeps its one decimal.
        assert_eq!(core_call_i64("human_bytes", 1_281), "1.3 KiB");
        assert_eq!(core_call_i64("human_bytes", 1_229), "1.2 KiB");
    }

    #[test]
    fn counts_step_up_where_the_decimal_would_round() {
        assert_eq!(core_call_i64("human_count", 9_999), "9999");
        assert_eq!(core_call_i64("human_count", 10_000), "10.0k");
        assert_eq!(core_call_i64("human_count", 999_949), "999.9k");
        // 999,999 used to read "1000.0k"; it is 1.0M now.
        assert_eq!(core_call_i64("human_count", 999_950), "1.0M");
        assert_eq!(core_call_i64("human_count", 999_999), "1.0M");
    }
}
