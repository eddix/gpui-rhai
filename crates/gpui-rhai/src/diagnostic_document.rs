//! Format host-provided diagnostic content, never evaluate it as code.
use std::io::{self, Write};

use rhai::{Dynamic, Engine, EvalAltResult, ImmutableString, Map, Position};
use serde_json::Value;

const MAX_TEXT: usize = 128 * 1024;
const MAX_REPLAY_INPUT: usize = 8192;

struct BoundedDocument(Vec<u8>);

impl Write for BoundedDocument {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_TEXT - self.0.len() {
            return Err(io::Error::other("diagnostic document exceeds 128 KiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn failure(text: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(Dynamic::from(text.to_owned()), Position::NONE)
}

fn formatted(value: Dynamic) -> Result<Map, Box<EvalAltResult>> {
    let (text, language) = if value.is::<ImmutableString>() {
        let source = value.cast::<ImmutableString>();
        if source.len() > MAX_TEXT {
            return Ok(document("", "plain", true));
        }
        match serde_json::from_str::<Value>(source.as_str()) {
            Ok(parsed) => (parsed, "json"),
            Err(_) => return Ok(document(source.as_str(), "plain", false)),
        }
    } else {
        // Validate using the existing typed, bounded SDK boundary before
        // cloning a script value into serde's native JSON representation.
        crate::UiValue::from_dynamic(value.clone())
            .map_err(|_| failure("invalid diagnostic document input"))?;
        let json: Value = rhai::serde::from_dynamic(&value)
            .map_err(|_| failure("diagnostic document only accepts JSON values"))?;
        (json, "json")
    };
    let mut out = BoundedDocument(Vec::new());
    if serde_json::to_writer_pretty(&mut out, &text).is_err() {
        return Ok(document("", "plain", true));
    }
    let text =
        String::from_utf8(out.0).map_err(|_| failure("could not encode diagnostic document"))?;
    Ok(document(&text, language, false))
}

fn document(text: &str, language: &str, omitted: bool) -> Map {
    Map::from_iter([
        ("text".into(), Dynamic::from(text.to_owned())),
        ("language".into(), Dynamic::from(language.to_owned())),
        ("omitted".into(), Dynamic::from(omitted)),
    ])
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("diagnostic_document", formatted);
    engine.register_fn("validate_replay_input", valid_replay_input);
}

fn valid_replay_input(input: &str, format: &str) -> bool {
    if input.len() > MAX_REPLAY_INPUT {
        return false;
    }
    match format {
        "text" => true,
        "json" => serde_json::from_str::<Value>(input).is_ok(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_input_checks_format_and_bounds_without_interpreting_script() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            validate_replay_input("{\"path\": []}", "json") &&
            validate_replay_input("not JSON", "text") &&
            !validate_replay_input("{value: 1+2}", "json") &&
            !validate_replay_input("null", "unknown")
        "#
                )
                .unwrap()
        );
        assert!(valid_replay_input(&" ".repeat(MAX_REPLAY_INPUT), "text"));
        assert!(!valid_replay_input(
            &"x".repeat(MAX_REPLAY_INPUT + 1),
            "text"
        ));
    }

    #[test]
    fn diagnostic_document_distinguishes_plain_text_from_json() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            let doc = diagnostic_document(#{query:"\"quoted\"", ok:true, data:()});
            let decoded = diagnostic_document("{\"hello\":[1,2]}");
            let plain = diagnostic_document("not JSON {");
            doc.language == "json" && doc.text.contains("\"data\": null") &&
                doc.text.contains("\"ok\": true") && decoded.text.contains("\n") &&
                plain.language == "plain" && plain.text == "not JSON {"
        "#
                )
                .unwrap()
        );
    }

    #[test]
    fn diagnostic_document_is_bounded_without_partial_copy_content() {
        let source = String::from("🙂").repeat(MAX_TEXT);
        let result = formatted(Dynamic::from(source)).unwrap();
        assert!(result["omitted"].clone_cast::<bool>());
        assert_eq!(result["text"].clone_cast::<ImmutableString>().len(), 0);
        let spaces = Value::String(String::from("\n").repeat(MAX_TEXT));
        let mut output = BoundedDocument(Vec::new());
        assert!(serde_json::to_writer_pretty(&mut output, &spaces).is_err());
        assert!(output.0.len() <= MAX_TEXT);
    }
}
