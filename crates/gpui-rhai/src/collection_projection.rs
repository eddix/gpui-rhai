//! Shared stable structural ordering for native collection projections.

use std::collections::BTreeMap;

use rhai::{Array, Dynamic, Engine, EvalAltResult, FuncRegistration, Position};

use crate::UiValue;

/// Group values by key while preserving first-seen group order and item order.
pub(crate) fn stable_groups<K, V>(items: impl IntoIterator<Item = (K, V)>) -> Vec<(K, Vec<V>)>
where
    K: Clone + Ord,
{
    let mut order = Vec::new();
    let mut groups = BTreeMap::<K, Vec<V>>::new();
    for (key, value) in items {
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(value);
    }
    order
        .into_iter()
        .map(|key| {
            let values = groups
                .remove(&key)
                .expect("first-seen group order contains every bucket");
            (key, values)
        })
        .collect()
}

#[derive(Clone, Debug)]
struct OutlineSource {
    key: String,
    parent: Option<String>,
    label: String,
    disabled: bool,
    loading: bool,
}

pub(crate) fn register_collection_projection_api(engine: &mut Engine) {
    FuncRegistration::new("outline_projection")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |items: Array, expanded: Array| -> Result<Array, Box<EvalAltResult>> {
                let items = items
                    .into_iter()
                    .map(UiValue::from_dynamic)
                    .map(|value| {
                        value.map_err(|error| Box::new(outline_message(error.to_string())))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let expanded = expanded
                    .into_iter()
                    .map(|value| {
                        if value.is::<rhai::ImmutableString>() {
                            Ok(value.cast::<rhai::ImmutableString>().to_string())
                        } else {
                            Err(Box::new(outline_message("expanded keys must be strings")))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                outline_projection(&items, &expanded)
                    .map(|rows| rows.into_iter().map(UiValue::into_dynamic).collect())
                    .map_err(|message| Box::new(outline_message(message)))
            },
        );
}

fn outline_projection(items: &[UiValue], expanded: &[String]) -> Result<Vec<UiValue>, String> {
    if items.len() > 10_000 {
        return Err("outline items exceed 10000".to_owned());
    }
    let sources = items
        .iter()
        .map(parse_outline_source)
        .collect::<Result<Vec<_>, _>>()?;
    let mut by_key = BTreeMap::new();
    let mut children = BTreeMap::<Option<String>, Vec<String>>::new();
    for source in &sources {
        if by_key.insert(source.key.clone(), source.clone()).is_some() {
            return Err(format!("duplicate outline key `{}`", source.key));
        }
        children
            .entry(source.parent.clone())
            .or_default()
            .push(source.key.clone());
    }
    for source in &sources {
        if source
            .parent
            .as_ref()
            .is_some_and(|parent| !by_key.contains_key(parent))
        {
            return Err(format!(
                "outline item `{}` references missing parent",
                source.key
            ));
        }
        if source.parent.as_deref() == Some(source.key.as_str()) {
            return Err(format!(
                "outline item `{}` cannot parent itself",
                source.key
            ));
        }
        let mut ancestry = std::collections::BTreeSet::new();
        let mut cursor = Some(source.key.as_str());
        while let Some(key) = cursor {
            if !ancestry.insert(key) {
                return Err(format!("outline contains a cycle at `{key}`"));
            }
            cursor = by_key.get(key).and_then(|item| item.parent.as_deref());
        }
    }
    let expanded = expanded
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(key) = expanded.iter().find(|key| !by_key.contains_key(*key)) {
        return Err(format!("expanded outline key `{key}` is unknown"));
    }
    let mut rows = Vec::new();
    let mut visiting = std::collections::BTreeSet::new();
    let mut visited = std::collections::BTreeSet::new();
    for root in children.get(&None).into_iter().flatten() {
        flatten_outline(
            root,
            0,
            &by_key,
            &children,
            &expanded,
            &mut visiting,
            &mut visited,
            &mut rows,
        )?;
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
fn flatten_outline(
    key: &str,
    depth: usize,
    sources: &BTreeMap<String, OutlineSource>,
    children: &BTreeMap<Option<String>, Vec<String>>,
    expanded: &std::collections::BTreeSet<String>,
    visiting: &mut std::collections::BTreeSet<String>,
    visited: &mut std::collections::BTreeSet<String>,
    rows: &mut Vec<UiValue>,
) -> Result<(), String> {
    if depth > 256 || !visiting.insert(key.to_owned()) {
        return Err(format!("outline cycle or depth limit at `{key}`"));
    }
    let source = sources
        .get(key)
        .ok_or_else(|| format!("outline source `{key}` disappeared"))?;
    let descendants = children.get(&Some(key.to_owned()));
    let has_children = descendants.is_some_and(|children| !children.is_empty());
    rows.push(UiValue::Map(BTreeMap::from([
        ("key".to_owned(), UiValue::String(source.key.clone())),
        ("label".to_owned(), UiValue::String(source.label.clone())),
        (
            "parent".to_owned(),
            source
                .parent
                .as_ref()
                .map_or(UiValue::Null, |parent| UiValue::String(parent.clone())),
        ),
        (
            "depth".to_owned(),
            UiValue::Integer(i64::try_from(depth).unwrap_or(i64::MAX)),
        ),
        ("has_children".to_owned(), UiValue::Bool(has_children)),
        (
            "expanded".to_owned(),
            UiValue::Bool(has_children && expanded.contains(key)),
        ),
        ("disabled".to_owned(), UiValue::Bool(source.disabled)),
        ("loading".to_owned(), UiValue::Bool(source.loading)),
    ])));
    visited.insert(key.to_owned());
    if has_children && expanded.contains(key) {
        for child in descendants.into_iter().flatten() {
            flatten_outline(
                child,
                depth + 1,
                sources,
                children,
                expanded,
                visiting,
                visited,
                rows,
            )?;
        }
    }
    visiting.remove(key);
    Ok(())
}

fn parse_outline_source(value: &UiValue) -> Result<OutlineSource, String> {
    let UiValue::Map(value) = value else {
        return Err("outline item must be an object".to_owned());
    };
    let string = |name: &str| match value.get(name) {
        Some(UiValue::String(value)) if !value.is_empty() && value.len() <= 128 => {
            Ok(value.clone())
        }
        _ => Err(format!("outline item {name} must be a safe string")),
    };
    let parent = match value.get("parent") {
        None | Some(UiValue::Null) => None,
        Some(UiValue::String(value)) if !value.is_empty() && value.len() <= 128 => {
            Some(value.clone())
        }
        _ => return Err("outline item parent must be null or a safe string".to_owned()),
    };
    let boolean = |name: &str| match value.get(name) {
        Some(UiValue::Bool(value)) => *value,
        None | Some(_) => false,
    };
    Ok(OutlineSource {
        key: string("key")?,
        parent,
        label: string("label")?,
        disabled: boolean("disabled"),
        loading: boolean("loading"),
    })
}

fn outline_message(message: impl Into<String>) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(Dynamic::from(message.into()), Position::NONE)
}

#[cfg(test)]
mod tests {
    use super::{outline_projection, stable_groups};
    use crate::UiValue;
    use std::collections::BTreeMap;

    #[test]
    fn stable_grouping_preserves_first_group_and_item_order() {
        let groups = stable_groups([("b", 1), ("a", 2), ("b", 3), ("c", 4), ("a", 5)]);
        assert_eq!(
            groups,
            vec![("b", vec![1, 3]), ("a", vec![2, 5]), ("c", vec![4])]
        );
    }

    #[test]
    fn outline_projection_is_stable_and_rejects_cycles() {
        let item = |key: &str, parent: Option<&str>| {
            UiValue::Map(BTreeMap::from([
                ("key".to_owned(), UiValue::String(key.to_owned())),
                ("label".to_owned(), UiValue::String(key.to_owned())),
                (
                    "parent".to_owned(),
                    parent.map_or(UiValue::Null, |value| UiValue::String(value.to_owned())),
                ),
            ]))
        };
        let rows = outline_projection(
            &[
                item("root", None),
                item("a", Some("root")),
                item("b", Some("root")),
            ],
            &["root".to_owned()],
        )
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert!(outline_projection(&[item("a", Some("b")), item("b", Some("a"))], &[]).is_err());
    }
}
