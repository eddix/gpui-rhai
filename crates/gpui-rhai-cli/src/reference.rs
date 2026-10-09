//! Reference pages for the official modules, generated from their schemas and
//! header comments.
//!
//! The schema is the source of truth for props, events, slots and parts; the
//! header comment above `define_component` supplies the purpose and the example.
//! `docs/reference/` holds the generated pages; a test fails when they are stale
//! and rewrites them when `GPUI_RHAI_UPDATE_REFERENCE` is set.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use gpui_rhai::{ComponentDefinition, ModuleId, ObjectField, UiValue, ValueSchema};

use crate::{BundledRegistry, ProjectError};

/// Module families with reference pages, in index order.
pub const REFERENCE_FAMILIES: [(&str, &str); 5] = [
    ("components", "Components"),
    ("layouts", "Layouts"),
    ("patterns", "Patterns"),
    ("motion", "Motion"),
    ("charts", "Charts"),
];

/// Labels of header-comment blocks that the schema now states; the reference
/// takes those facts from the schema instead.
const SCHEMA_LABELS: [&str; 5] = ["Props:", "Events:", "Events/state:", "Parts:", "Slots:"];

/// The prose and example of a module's header comment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeaderNotes {
    /// Paragraphs describing the module, in order.
    pub paragraphs: Vec<String>,
    /// The example call, without its `Example:` label.
    pub example: Option<String>,
}

impl HeaderNotes {
    /// The first sentence of the description, for indexes.
    #[must_use]
    pub fn summary(&self) -> String {
        let Some(first) = self.paragraphs.first() else {
            return String::new();
        };
        match first.find(". ") {
            Some(end) => first[..=end].to_owned(),
            None => first.clone(),
        }
    }
}

/// Read the comment block between a module's metadata header and its code.
#[must_use]
pub fn header_notes(source: &str) -> HeaderNotes {
    let body = source.split_once("*/").map_or(source, |(_, rest)| rest);
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(text) = trimmed.strip_prefix("//") {
            lines.push(text.strip_prefix(' ').unwrap_or(text).to_owned());
        } else if !trimmed.is_empty() {
            break;
        }
    }
    let mut notes = HeaderNotes::default();
    let mut paragraph: Vec<String> = Vec::new();
    // The block a line belongs to: None for prose.
    let mut block: Option<&str> = None;
    let mut example: Vec<String> = Vec::new();
    let flush = |paragraph: &mut Vec<String>, notes: &mut HeaderNotes| {
        if !paragraph.is_empty() {
            notes.paragraphs.push(paragraph.join(" "));
            paragraph.clear();
        }
    };
    for line in lines {
        let text = line.trim_end();
        if text.trim().is_empty() {
            flush(&mut paragraph, &mut notes);
            block = None;
            continue;
        }
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Example:") {
            flush(&mut paragraph, &mut notes);
            block = Some("Example:");
            example.push(rest.trim().to_owned());
            continue;
        }
        if let Some(label) = SCHEMA_LABELS
            .iter()
            .find(|label| trimmed.starts_with(**label))
        {
            flush(&mut paragraph, &mut notes);
            block = Some(label);
            continue;
        }
        match block {
            Some("Example:") => example.push(text.to_owned()),
            Some(_) => {}
            None => paragraph.push(trimmed.to_owned()),
        }
    }
    flush(&mut paragraph, &mut notes);
    if !example.is_empty() {
        notes.example = Some(dedent_continuation(&example));
    }
    notes
}

/// Join an example whose continuation lines are indented under its first line.
fn dedent_continuation(lines: &[String]) -> String {
    let indent = lines[1..]
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut joined = lines[0].clone();
    for line in &lines[1..] {
        joined.push('\n');
        joined.push_str("    ");
        joined.push_str(line.get(indent..).unwrap_or(line.trim_start()));
    }
    joined
}

/// Generate every reference page, keyed by its path below `docs/reference/`.
///
/// # Errors
///
/// Returns compile or export errors of the bundled modules.
pub fn reference_documents(
    registry: &BundledRegistry,
) -> Result<BTreeMap<PathBuf, String>, ProjectError> {
    let definitions = registry.compiled_definitions()?;
    let mut documents = BTreeMap::new();
    let mut index = String::from(
        "# Module reference\n\n\
         Generated from the schemas and header comments of the official modules; do not\n\
         edit. Regenerate with\n\
         `GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli reference`.\n",
    );
    for (family, title) in REFERENCE_FAMILIES {
        let members = definitions
            .iter()
            .filter(|(id, _)| id.as_str().split('/').next() == Some(family))
            .collect::<Vec<_>>();
        if members.is_empty() {
            continue;
        }
        let _ = write!(
            index,
            "\n## {title}\n\n| Module | Export | Purpose |\n|---|---|---|\n"
        );
        for (id, (definition, source)) in members {
            let notes = header_notes(source);
            let page = module_page(id, definition, &notes);
            let path = PathBuf::from(format!("{id}.md"));
            let _ = writeln!(
                index,
                "| [`{id}`]({}) | `{}` | {} |",
                path.display(),
                definition.metadata.export,
                cell(&notes.summary())
            );
            documents.insert(path, page);
        }
    }
    documents.insert(PathBuf::from("README.md"), index);
    Ok(documents)
}

fn module_page(id: &ModuleId, definition: &ComponentDefinition, notes: &HeaderNotes) -> String {
    let metadata = &definition.metadata;
    let schema = &definition.schema;
    let alias = id.as_str().rsplit('/').next().unwrap_or(id.as_str());
    let mut page = format!(
        "# {export}\n\n`{id}` · export `{export}` · version {version}. Generated from\n\
         [`registry/{id}.rhai`](../../../registry/{id}.rhai); do not edit.\n",
        export = metadata.export,
        version = metadata.version,
    );
    for paragraph in &notes.paragraphs {
        let _ = write!(page, "\n{paragraph}\n");
    }
    let example = notes.example.as_deref().map(|example| {
        let call = format!("{}(", metadata.export);
        match example.strip_prefix(&call) {
            Some(rest) => format!("{alias}::{call}{rest}"),
            None => example.to_owned(),
        }
    });
    let _ = write!(page, "\n```rhai\nimport \"{id}\" as {alias};\n");
    if let Some(example) = example {
        let _ = write!(page, "\n{example}\n");
    }
    page.push_str("```\n");

    props_section(&mut page, schema);
    contract_section(&mut page, schema);
    theme_section(&mut page, metadata);
    page
}

fn props_section(page: &mut String, schema: &gpui_rhai::ComponentSchema) {
    page.push_str(
        "\n## Props\n\n| Prop | Type | Required or default | Description |\n|---|---|---|---|\n",
    );
    for (name, field) in &schema.props {
        let _ = writeln!(page, "{}", field_row(name, field));
    }
    let mut nested = Vec::new();
    for (name, field) in &schema.props {
        collect_objects(name, &field.schema, &mut nested);
    }
    for (path, fields) in nested {
        let _ = write!(
            page,
            "\n### `{path}` fields\n\n| Field | Type | Required or default | Description |\n|---|---|---|---|\n"
        );
        for (name, field) in fields {
            let _ = writeln!(page, "{}", field_row(name, field));
        }
    }
}

/// Events, slots and parts.
fn contract_section(page: &mut String, schema: &gpui_rhai::ComponentSchema) {
    if !schema.events.is_empty() {
        page.push_str(
            "\n## Events\n\n| Event | Callback prop | Payload | Description |\n|---|---|---|---|\n",
        );
        for (name, event) in &schema.events {
            let callback = format!("on_{name}");
            let callback = if schema.props.contains_key(&callback) {
                format!("`{callback}`")
            } else {
                "—".to_owned()
            };
            let _ = writeln!(
                page,
                "| `{name}` | {callback} | {} | {} |",
                cell(&type_name(&event.payload)),
                cell(event.doc.as_deref().unwrap_or("—"))
            );
        }
    }
    if !schema.slots.is_empty() {
        page.push_str(
            "\n## Slots\n\n| Slot | Required | Multiple | Description |\n|---|---|---|---|\n",
        );
        for (name, slot) in &schema.slots {
            let _ = writeln!(
                page,
                "| `{name}` | {} | {} | {} |",
                yes_no(slot.required),
                yes_no(slot.multiple),
                cell(slot.doc.as_deref().unwrap_or("—"))
            );
        }
    }
    if !schema.parts.is_empty() {
        let parts = schema
            .parts
            .iter()
            .map(|part| format!("`{part}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(
            page,
            "\n## Parts\n\nStyle a part with `part_styles` or in `ui/styles.rhai`: {parts}.\n"
        );
    }
}

/// Tokens, environment and dependencies.
fn theme_section(page: &mut String, metadata: &gpui_rhai::ComponentMetadata) {
    let lists = [
        ("Tokens", &metadata.tokens),
        ("Environment", &metadata.environment),
    ];
    let mut theme = String::new();
    for (label, values) in lists {
        if !values.is_empty() {
            let values = values
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(theme, "- {label}: {values}");
        }
    }
    if !theme.is_empty() {
        let _ = write!(page, "\n## Theme\n\n{theme}");
    }
    if !metadata.dependencies.is_empty() {
        let dependencies = metadata
            .dependencies
            .iter()
            .map(|dependency| format!("[`{dependency}`](../{dependency}.md)"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(page, "\n## Dependencies\n\n{dependencies}\n");
    }
}

fn field_row(name: &str, field: &ObjectField) -> String {
    let requirement = if field.required {
        "required".to_owned()
    } else {
        field
            .default
            .as_ref()
            .map_or_else(|| "—".to_owned(), |value| format!("`{}`", literal(value)))
    };
    format!(
        "| `{name}` | {} | {} | {} |",
        cell(&type_name(&field.schema)),
        cell(&requirement),
        cell(field.doc.as_deref().unwrap_or("—"))
    )
}

/// Object schemas reachable from a prop, with the path a reader writes to reach them.
fn collect_objects<'a>(
    path: &str,
    schema: &'a ValueSchema,
    out: &mut Vec<(String, &'a BTreeMap<String, ObjectField>)>,
) {
    match schema {
        ValueSchema::Object { fields, .. } => {
            out.push((path.to_owned(), fields));
            for (name, field) in fields {
                collect_objects(&format!("{path}.{name}"), &field.schema, out);
            }
        }
        ValueSchema::Array { items, .. } => collect_objects(&format!("{path}[]"), items, out),
        ValueSchema::Map { values } => collect_objects(&format!("{path}.<key>"), values, out),
        ValueSchema::Optional { value } => collect_objects(path, value, out),
        ValueSchema::OneOf { variants } => {
            for variant in variants {
                collect_objects(path, variant, out);
            }
        }
        _ => {}
    }
}

/// A readable type: `bool`, `integer 1–10`, `"start" or "end"`, `array of object`.
#[must_use]
pub fn type_name(schema: &ValueSchema) -> String {
    match schema {
        ValueSchema::Null => "none".to_owned(),
        ValueSchema::Bool => "bool".to_owned(),
        ValueSchema::Integer { min, max } => with_range(
            "integer",
            min.map(|value| value.to_string()),
            max.map(|value| value.to_string()),
            false,
            false,
        ),
        ValueSchema::Float {
            min,
            max,
            exclusive_min,
            exclusive_max,
        }
        | ValueSchema::Number {
            min,
            max,
            exclusive_min,
            exclusive_max,
        } => with_range(
            "number",
            exclusive_min.or(*min).map(bound),
            exclusive_max.or(*max).map(bound),
            exclusive_min.is_some(),
            exclusive_max.is_some(),
        ),
        ValueSchema::String { allowed } if allowed.is_empty() => "string".to_owned(),
        ValueSchema::String { allowed } => allowed
            .iter()
            .map(|value| format!("`\"{value}\"`"))
            .collect::<Vec<_>>()
            .join(" or "),
        ValueSchema::Array { items, max_items } => {
            let items = type_name(items);
            match max_items {
                Some(max) => format!("array of {items} (at most {max})"),
                None => format!("array of {items}"),
            }
        }
        ValueSchema::Map { values } => format!("map of {}", type_name(values)),
        ValueSchema::Object { .. } => "object".to_owned(),
        ValueSchema::Optional { value } => format!("{} or `()`", type_name(value)),
        ValueSchema::OneOf { variants } => variants
            .iter()
            .map(type_name)
            .collect::<Vec<_>>()
            .join(" or "),
        ValueSchema::Node => "node".to_owned(),
        ValueSchema::Callback => "callback".to_owned(),
        ValueSchema::Style => "style".to_owned(),
        ValueSchema::Length => "length".to_owned(),
        ValueSchema::UiValue => "any value".to_owned(),
        ValueSchema::Asset => "asset".to_owned(),
        ValueSchema::Signal => "signal".to_owned(),
        ValueSchema::Collection => "native collection".to_owned(),
        ValueSchema::Document => "native document".to_owned(),
        #[allow(unreachable_patterns)]
        _ => "chart data".to_owned(),
    }
}

fn with_range(
    name: &str,
    min: Option<String>,
    max: Option<String>,
    exclusive_min: bool,
    exclusive_max: bool,
) -> String {
    match (min, max) {
        (Some(min), Some(max)) if !exclusive_min && !exclusive_max => {
            format!("{name} {min}–{max}")
        }
        (min, max) => {
            let mut bounds = Vec::new();
            if let Some(min) = min {
                bounds.push(format!("{} {min}", if exclusive_min { ">" } else { "≥" }));
            }
            if let Some(max) = max {
                bounds.push(format!("{} {max}", if exclusive_max { "<" } else { "≤" }));
            }
            if bounds.is_empty() {
                name.to_owned()
            } else {
                format!("{name} {}", bounds.join(", "))
            }
        }
    }
}

/// A range bound: whole numbers without a fraction.
fn bound(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

/// A float literal as Rhai writes it.
fn number(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

/// A Rhai literal for a default value.
fn literal(value: &UiValue) -> String {
    match value {
        UiValue::Null => "()".to_owned(),
        UiValue::Bool(value) => value.to_string(),
        UiValue::Integer(value) => value.to_string(),
        UiValue::Float(value) => number(*value),
        UiValue::String(value) => format!("{value:?}"),
        UiValue::Array(values) => format!(
            "[{}]",
            values.iter().map(literal).collect::<Vec<_>>().join(", ")
        ),
        UiValue::Map(values) if values.is_empty() => "#{}".to_owned(),
        UiValue::Map(values) => format!(
            "#{{ {} }}",
            values
                .iter()
                .map(|(key, value)| format!("{key}: {}", literal(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        UiValue::Handle(_) => "handle".to_owned(),
    }
}

/// Escape a Markdown table cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// Props, nested fields, events and slots of official modules that lack a `doc`,
/// as `module: item` lines.
#[must_use]
pub fn missing_docs(definitions: &BTreeMap<ModuleId, (ComponentDefinition, &str)>) -> Vec<String> {
    let mut missing = Vec::new();
    for (id, (definition, _)) in definitions {
        let schema = &definition.schema;
        for (name, field) in &schema.props {
            if field.doc.is_none() {
                missing.push(format!("{id}: prop `{name}`"));
            }
            let mut nested = Vec::new();
            collect_objects(name, &field.schema, &mut nested);
            for (path, fields) in nested {
                for (field_name, field) in fields {
                    if field.doc.is_none() {
                        missing.push(format!("{id}: field `{path}.{field_name}`"));
                    }
                }
            }
        }
        for (name, event) in &schema.events {
            if event.doc.is_none() {
                missing.push(format!("{id}: event `{name}`"));
            }
        }
        for (name, slot) in &schema.slots {
            if slot.doc.is_none() {
                missing.push(format!("{id}: slot `{name}`"));
            }
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn reference_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/reference")
    }

    #[test]
    fn header_notes_keep_prose_and_example_and_drop_schema_lists() {
        let source = "/* gpui-rhai\n{}\n*/\n\
            // Region is one working area.\n\
            // It fills its column.\n\
            //\n\
            // Props: label, body,\n\
            //        fill.\n\
            // State: the caller owns `open`.\n\
            // Example: Region(#{ label: \"Hosts\",\n\
            //              body: table_node })\n\
            //\n\
            // Content starts at metrics.inset.\n\
            define_component(#{})\n";
        let notes = header_notes(source);
        assert_eq!(
            notes.paragraphs,
            vec![
                "Region is one working area. It fills its column.".to_owned(),
                "Content starts at metrics.inset.".to_owned(),
            ]
        );
        assert_eq!(
            notes.example.as_deref(),
            Some("Region(#{ label: \"Hosts\",\n    body: table_node })")
        );
        assert_eq!(notes.summary(), "Region is one working area.");
    }

    #[test]
    fn types_read_as_prose() {
        assert_eq!(
            type_name(&ValueSchema::Integer {
                min: Some(1),
                max: Some(24)
            }),
            "integer 1–24"
        );
        assert_eq!(
            type_name(&ValueSchema::Optional {
                value: Box::new(ValueSchema::String {
                    allowed: vec!["start".to_owned(), "end".to_owned()]
                })
            }),
            "`\"start\"` or `\"end\"` or `()`"
        );
    }

    #[test]
    fn official_modules_document_every_prop_event_and_slot() {
        let registry = BundledRegistry::load().unwrap();
        let definitions = registry.compiled_definitions().unwrap();
        let missing = missing_docs(&definitions);
        assert!(
            missing.is_empty(),
            "{} items lack a `doc`:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    #[test]
    fn reference_documents_are_current() {
        let generated = reference_documents(&BundledRegistry::load().unwrap()).unwrap();
        let root = reference_root();
        if std::env::var_os("GPUI_RHAI_UPDATE_REFERENCE").is_some() {
            if root.exists() {
                std::fs::remove_dir_all(&root).unwrap();
            }
            for (path, text) in &generated {
                let path = root.join(path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text).unwrap();
            }
            return;
        }
        let mut stale = Vec::new();
        for (path, text) in &generated {
            match std::fs::read_to_string(root.join(path)) {
                Ok(current) if &current == text => {}
                _ => stale.push(path.display().to_string()),
            }
        }
        assert!(
            stale.is_empty(),
            "docs/reference is stale ({} pages): {stale:?}; run \
             GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli reference",
            stale.len()
        );
    }
}
