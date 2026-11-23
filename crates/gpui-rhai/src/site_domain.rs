//! Validation for a site-address field, not navigation or network policy.

use rhai::{Dynamic, Engine, FuncRegistration};

fn site_domain(input: &str) -> Option<String> {
    // Bound native work before Unicode mapping.
    if input.len() > 2048 || input.chars().any(char::is_control) {
        return None;
    }
    let mut name = input.trim();
    if name.is_empty() {
        return Some(String::new());
    }
    for prefix in ["https://", "http://"] {
        if name
            .get(..prefix.len())
            .is_some_and(|prefix_text| prefix_text.eq_ignore_ascii_case(prefix))
        {
            name = &name[prefix.len()..];
            break;
        }
    }
    name = name.trim_end_matches('/');
    if name.is_empty()
        || name.contains(['/', '\\', '@', ':', '?', '#', '%', '[', ']'])
        || name.chars().any(char::is_whitespace)
    {
        return None;
    }
    let ascii = idna::domain_to_ascii_strict(name).ok()?;
    let (prefix, tail) = ascii.rsplit_once('.')?;
    if prefix.is_empty()
        || tail.len() < 2
        || !(tail.starts_with("xn--") || tail.bytes().all(|byte| byte.is_ascii_alphabetic()))
    {
        return None;
    }
    let valid = ascii.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    });
    (valid && ascii.len() <= 253).then_some(ascii)
}

pub(crate) fn register(engine: &mut Engine) {
    FuncRegistration::new("normalize_site_domain")
        .in_global_namespace()
        .register_into_engine(engine, |text: &str| {
            site_domain(text).map_or(Dynamic::UNIT, Into::into)
        });
}

#[cfg(test)]
mod tests {
    use super::site_domain;

    #[test]
    fn normalizes_bare_hostnames_and_idna_without_navigation() {
        assert_eq!(
            site_domain(" HTTPS://München.de// ").as_deref(),
            Some("xn--mnchen-3ya.de")
        );
        assert_eq!(
            site_domain("alpha-7.Example.COM").as_deref(),
            Some("alpha-7.example.com")
        );
        assert_eq!(site_domain(" ").as_deref(), Some(""));
        assert_eq!(
            site_domain("你好.中国").as_deref(),
            Some("xn--6qq79v.xn--fiqs8s")
        );
    }

    #[test]
    fn rejects_non_hostname_address_components_and_malformed_labels() {
        for input in [
            "https://",
            "https:///example.com",
            "example.com:80",
            "u@example.com",
            "example.com/path",
            "example.com?query=1",
            "example.com#part",
            "ex%61mple.com",
            "example.com\\path",
            "local",
            "127.0.0.1",
            "-x.example",
            "x-.example",
            "x..example",
            ".example.com",
            "example.com.",
            "line\n.example",
        ] {
            assert_eq!(site_domain(input), None, "{input}");
        }
    }
}
