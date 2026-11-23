//! Preparation-time SVG color inheritance analysis shared by assets and inline SVG.

pub(crate) fn svg_uses_external_current_color(source: &str) -> bool {
    if !source
        .as_bytes()
        .windows(b"currentColor".len())
        .any(|candidate| candidate.eq_ignore_ascii_case(b"currentColor"))
    {
        return false;
    }
    let Ok(document) = roxmltree::Document::parse(source) else {
        return true;
    };
    let mut found_attribute_use = false;
    for node in document.descendants().filter(roxmltree::Node::is_element) {
        let uses_current_color = node.attributes().any(|attribute| {
            attribute
                .value()
                .as_bytes()
                .windows(b"currentColor".len())
                .any(|candidate| candidate.eq_ignore_ascii_case(b"currentColor"))
        });
        if !uses_current_color {
            continue;
        }
        found_attribute_use = true;
        let mut ancestor = Some(node);
        let mut locally_resolved = false;
        while let Some(element) = ancestor {
            if element_has_explicit_color(element) {
                locally_resolved = true;
                break;
            }
            ancestor = element.parent_element();
        }
        if !locally_resolved {
            return true;
        }
    }
    // Asset SVG may use a style sheet; the SVG decoder remains its authority.
    !found_attribute_use
}

fn element_has_explicit_color(node: roxmltree::Node<'_, '_>) -> bool {
    if node
        .attribute("color")
        .is_some_and(is_explicit_svg_color_value)
    {
        return true;
    }
    node.attribute("style").is_some_and(|style| {
        style.split(';').any(|declaration| {
            let Some((name, value)) = declaration.split_once(':') else {
                return false;
            };
            name.trim().eq_ignore_ascii_case("color") && is_explicit_svg_color_value(value)
        })
    })
}

pub(crate) fn is_explicit_svg_color_value(value: &str) -> bool {
    let value = value.trim();
    !value.eq_ignore_ascii_case("inherit")
        && !value.eq_ignore_ascii_case("currentColor")
        && !value.eq_ignore_ascii_case("unset")
        && !value.eq_ignore_ascii_case("revert")
}
