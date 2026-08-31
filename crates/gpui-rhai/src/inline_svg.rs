use thiserror::Error;

const MAX_INLINE_SVG_BYTES: usize = 64 * 1024;
const MAX_INLINE_SVG_ELEMENTS: usize = 2_048;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InlineSvg(String);

impl InlineSvg {
    /// Validate bounded self-contained SVG markup.
    ///
    /// # Errors
    ///
    /// Returns [`InlineSvgError`] for oversized, non-SVG, active, or external
    /// content. Rendering still uses GPUI's SVG parser as the syntax authority.
    pub fn new(source: impl Into<String>) -> Result<Self, InlineSvgError> {
        let source = source.into();
        let trimmed = source.trim();
        if trimmed.is_empty() || source.len() > MAX_INLINE_SVG_BYTES {
            return Err(InlineSvgError::InvalidSize(source.len()));
        }
        let lowercase = source.to_ascii_lowercase();
        if !lowercase.contains("<svg") {
            return Err(InlineSvgError::MissingRoot);
        }
        let element_count = source
            .as_bytes()
            .windows(2)
            .filter(|pair| pair[0] == b'<' && pair[1].is_ascii_alphabetic())
            .count();
        if element_count > MAX_INLINE_SVG_ELEMENTS {
            return Err(InlineSvgError::TooManyElements(element_count));
        }
        for forbidden in [
            "<!doctype",
            "<!entity",
            "<script",
            "<foreignobject",
            "<iframe",
            "<object",
            "<embed",
            "<style",
            "@import",
        ] {
            if lowercase.contains(forbidden) {
                return Err(InlineSvgError::ForbiddenContent(forbidden));
            }
        }
        if contains_event_attribute(&lowercase) {
            return Err(InlineSvgError::EventAttribute);
        }
        for external in ["http:", "https:", "file:", "data:", "javascript:", "//"] {
            if lowercase.contains(external) {
                return Err(InlineSvgError::ExternalReference(external));
            }
        }
        if contains_non_fragment_href(&lowercase) {
            return Err(InlineSvgError::ExternalReference("href"));
        }
        usvg::Tree::from_str(trimmed, &usvg::Options::default())
            .map_err(|error| InlineSvgError::InvalidMarkup(error.to_string()))?;
        Ok(Self(source))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn contains_event_attribute(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if !bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        index += 1;
        if bytes.get(index..index + 2) != Some(b"on") {
            continue;
        }
        index += 2;
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
            index += 1;
        }
        if index == start {
            continue;
        }
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if bytes.get(index) == Some(&b'=') {
            return true;
        }
    }
    false
}

fn contains_non_fragment_href(source: &str) -> bool {
    let mut remaining = source;
    while let Some(index) = remaining.find("href") {
        remaining = &remaining[index + 4..];
        let after = remaining.trim_start();
        let Some(after) = after.strip_prefix('=') else {
            continue;
        };
        let value = after.trim_start();
        let Some(quote) = value
            .chars()
            .next()
            .filter(|quote| matches!(quote, '\'' | '"'))
        else {
            return true;
        };
        let value = &value[quote.len_utf8()..];
        let Some(end) = value.find(quote) else {
            return true;
        };
        if !value[..end].starts_with('#') {
            return true;
        }
        remaining = &value[end + quote.len_utf8()..];
    }
    false
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum InlineSvgError {
    #[error("inline SVG must be 1-{MAX_INLINE_SVG_BYTES} UTF-8 bytes; got {0}")]
    InvalidSize(usize),
    #[error("inline SVG source is missing an `<svg>` root")]
    MissingRoot,
    #[error("inline SVG markup is invalid: {0}")]
    InvalidMarkup(String),
    #[error("inline SVG exceeds {MAX_INLINE_SVG_ELEMENTS} elements; got {0}")]
    TooManyElements(usize),
    #[error("inline SVG contains forbidden active content `{0}`")]
    ForbiddenContent(&'static str),
    #[error("inline SVG event-handler attributes are forbidden")]
    EventAttribute,
    #[error("inline SVG external reference `{0}` is forbidden")]
    ExternalReference(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_svg_accepts_local_gradients_and_current_color() {
        let source = r#"<svg viewBox="0 0 10 10"><defs><linearGradient id="g"/></defs><path fill="url(#g)" stroke="currentColor" d="M0 0L10 10"/></svg>"#;
        assert_eq!(InlineSvg::new(source).unwrap().as_str(), source);
    }

    #[test]
    fn inline_svg_rejects_active_and_external_content() {
        assert!(matches!(
            InlineSvg::new("<svg onload='run()'/>"),
            Err(InlineSvgError::EventAttribute)
        ));
        assert!(matches!(
            InlineSvg::new("<svg><image href='https://example.com/a.png'/></svg>"),
            Err(InlineSvgError::ExternalReference(_))
        ));
        assert!(matches!(
            InlineSvg::new("<svg><script>run()</script></svg>"),
            Err(InlineSvgError::ForbiddenContent("<script"))
        ));
    }
}
