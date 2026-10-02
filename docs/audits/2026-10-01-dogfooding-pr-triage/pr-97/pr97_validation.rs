// Exact helper functions from PR head 31df838797767811ee29fa2385ec17a298bdf53d.
fn is_valid_key_handler_name(key: &str) -> bool {
    !key.is_empty() && key.len() <= 64 && key.chars().all(|character| {
        character.is_ascii_alphanumeric() || character == '_' || {
            // Printable ASCII punctuation and symbols, minus the reserved
            // namespace separator `:`.
            character.is_ascii_graphic() && character != ':'
        }
    })
}

fn is_printable_key_character(key: &str) -> bool {
    key.chars().all(|character| {
        character.is_ascii_graphic() && character != ':'
    })
}
