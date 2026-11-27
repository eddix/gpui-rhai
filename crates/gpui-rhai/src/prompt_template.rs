//! Bounded literal tokenization for template previews and their editors.

use std::collections::BTreeSet;

use rhai::{Array, Dynamic, Engine, EvalAltResult, FuncRegistration, Map};

#[derive(Debug, Eq, PartialEq)]
enum Part<'a> {
    Text(&'a str),
    Variable(&'a str),
}

fn tokenize(source: &str) -> Result<Vec<Part<'_>>, &'static str> {
    if source.len() > 32_768 {
        return Err("prompt template exceeds 32 KiB");
    }
    let bytes = source.as_bytes();
    let mut cursor = 0;
    let mut last = 0;
    let mut parts = Vec::new();
    let mut names = BTreeSet::new();
    let mut occurrences = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'{' {
            cursor += 1;
            continue;
        }
        let start = cursor + 1;
        let mut end = start;
        while end < bytes.len()
            && end - start <= 64
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'-'))
        {
            end += 1;
        }
        if end == start || end - start > 64 || bytes.get(end) != Some(&b'}') {
            cursor += 1;
            continue;
        }
        let name = &source[start..end];
        occurrences += 1;
        names.insert(name);
        if occurrences > 128 || names.len() > 32 {
            return Err("prompt template exceeds 32 variable names or 128 uses");
        }
        if cursor > last {
            parts.push(Part::Text(&source[last..cursor]));
        }
        parts.push(Part::Variable(name));
        cursor = end + 1;
        last = cursor;
    }
    if last < source.len() {
        parts.push(Part::Text(&source[last..]));
    }
    Ok(parts)
}

fn prompt_template_parts(source: &str) -> Result<Array, Box<EvalAltResult>> {
    Ok(tokenize(source)?
        .into_iter()
        .map(|part| {
            let (key, value) = match part {
                Part::Text(text) => ("text", text),
                Part::Variable(name) => ("name", name),
            };
            Dynamic::from_map(Map::from_iter([(key.into(), value.to_owned().into())]))
        })
        .collect())
}

fn replace_recipient_name(
    source: &str,
    previous: &str,
    next: &str,
) -> Result<String, &'static str> {
    const MAX: usize = 64_000;
    if source.len() > MAX || previous.len() > 512 || next.len() > 512 {
        return Err("recipient name substitution exceeds its input bounds");
    }
    let previous = previous.trim();
    let next = next.trim();
    let first = previous.split_whitespace().next().unwrap_or_default();
    if first.is_empty() || previous == next {
        return Ok(source.to_owned());
    }
    let first_next = next.split_whitespace().next().unwrap_or_default();
    let pattern = regex::Regex::new(&format!(
        "{}|\\b{}\\b",
        regex::escape(previous),
        regex::escape(first),
    ))
    .map_err(|_| "recipient name substitution could not build its literal matcher")?;
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for matched in pattern.find_iter(source) {
        let replacement = if matched.as_str() == previous {
            next
        } else {
            first_next
        };
        if result.len() + (matched.start() - cursor) + replacement.len() > MAX {
            return Err("recipient name substitution exceeds its output bounds");
        }
        result.push_str(&source[cursor..matched.start()]);
        result.push_str(replacement);
        cursor = matched.end();
    }
    if result.len() + (source.len() - cursor) > MAX {
        return Err("recipient name substitution exceeds its output bounds");
    }
    result.push_str(&source[cursor..]);
    Ok(result)
}

pub(crate) fn register(engine: &mut Engine) {
    FuncRegistration::new("prompt_template_parts")
        .in_global_namespace()
        .register_into_engine(engine, prompt_template_parts);
    FuncRegistration::new("replace_recipient_name")
        .in_global_namespace()
        .register_into_engine(engine, |source: &str, old: &str, new: &str| {
            replace_recipient_name(source, old, new)
                .map_err(|message| -> Box<EvalAltResult> { message.into() })
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_substitution_is_literal_bounded_and_non_recursive() {
        assert_eq!(
            replace_recipient_name(
                "Hi Ada [L],\nAda checked Adaline's draft.",
                "Ada [L]",
                "乐 L.+"
            )
            .unwrap(),
            "Hi 乐 L.+,\n乐 checked Adaline's draft."
        );
        assert_eq!(
            replace_recipient_name("Ada Lovelace; Ada", "Ada Lovelace", "Ada Lovelace II").unwrap(),
            "Ada Lovelace II; Ada"
        );
        assert!(replace_recipient_name(&"X ".repeat(2_000), "X", &"Z".repeat(100)).is_err());
    }

    #[test]
    fn preserves_utf8_literal_text_and_repeated_names() {
        assert_eq!(
            tokenize("用{format}写給{audience}，再按{format}校对。{bad name}").unwrap(),
            [
                Part::Text("用"),
                Part::Variable("format"),
                Part::Text("写給"),
                Part::Variable("audience"),
                Part::Text("，再按"),
                Part::Variable("format"),
                Part::Text("校对。{bad name}"),
            ]
        );
        assert_eq!(
            tokenize("{} { incomplete").unwrap(),
            [Part::Text("{} { incomplete")]
        );
    }

    #[test]
    fn bounds_template_size_and_uses() {
        assert!(tokenize(&"界".repeat(11_000)).is_err());
        assert!(tokenize(&"{x}".repeat(129)).is_err());
        assert_eq!(tokenize(&"{".repeat(16_000)).unwrap().len(), 1);
    }
}
