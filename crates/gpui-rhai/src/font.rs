use std::hash::{Hash, Hasher};

use thiserror::Error;

const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct FontSource {
    label: String,
    bytes: Vec<u8>,
    fingerprint: u64,
}

impl FontSource {
    /// Validate one in-memory OpenType/TrueType collection supplied by the Host.
    ///
    /// # Errors
    ///
    /// Returns [`FontError`] for an unsafe label, unsupported header, empty
    /// payload, or a payload larger than 16 MiB.
    pub fn new(label: impl Into<String>, bytes: Vec<u8>) -> Result<Self, FontError> {
        let label = label.into();
        if label.trim().is_empty() || label.len() > 256 {
            return Err(FontError::InvalidLabel(label));
        }
        if bytes.is_empty() || bytes.len() > MAX_FONT_BYTES {
            return Err(FontError::InvalidSize {
                label,
                size: bytes.len(),
            });
        }
        let header = bytes.get(..4).ok_or_else(|| FontError::UnsupportedFormat {
            label: label.clone(),
        })?;
        if header != [0x00, 0x01, 0x00, 0x00]
            && header != b"OTTO"
            && header != b"true"
            && header != b"ttcf"
        {
            return Err(FontError::UnsupportedFormat { label });
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        Ok(Self {
            label,
            bytes,
            fingerprint: hasher.finish(),
        })
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Validate a bounded font source set and reject duplicate payloads.
///
/// # Errors
///
/// Returns [`FontError`] above 64 sources or when the same bytes are declared
/// more than once.
pub fn validate_font_sources(sources: &[FontSource]) -> Result<(), FontError> {
    if sources.len() > 64 {
        return Err(FontError::TooMany(sources.len()));
    }
    let mut fingerprints = std::collections::BTreeSet::new();
    for source in sources {
        if !fingerprints.insert(source.fingerprint) {
            return Err(FontError::Duplicate(source.label.clone()));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FontError {
    #[error("font label `{0}` must be non-empty and no longer than 256 bytes")]
    InvalidLabel(String),
    #[error("font `{label}` must contain 1..=16777216 bytes, got {size}")]
    InvalidSize { label: String, size: usize },
    #[error("font `{label}` is not a supported TrueType/OpenType font or collection")]
    UnsupportedFormat { label: String },
    #[error("at most 64 font sources may be declared, got {0}")]
    TooMany(usize),
    #[error("font `{0}` duplicates an already declared payload")]
    Duplicate(String),
    #[error("GPUI failed to load declared fonts: {0}")]
    Load(String),
    #[error("failed to read font `{path}`: {message}")]
    Io { path: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_sources_validate_format_size_and_duplicates() {
        let font = FontSource::new("Art", [b"OTTO".as_slice(), &[0, 1, 2]].concat()).unwrap();
        validate_font_sources(std::slice::from_ref(&font)).unwrap();
        assert!(matches!(
            validate_font_sources(&[font.clone(), font]),
            Err(FontError::Duplicate(_))
        ));
        assert!(matches!(
            FontSource::new("bad", b"woff".to_vec()),
            Err(FontError::UnsupportedFormat { .. })
        ));
    }
}
