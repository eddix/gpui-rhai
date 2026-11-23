//! Fast projection of an explicitly supplied memory snapshot.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, ImmutableString, Map, Position};
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation as _;

const MAX_MEMORIES: usize = 128;
const MAX_BYTES: usize = 8192;
const MAX_GRAPHEMES: usize = 2048;

fn invalid(msg: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(msg.into(), Position::NONE)
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn text(v: &Map, key: &str) -> Result<ImmutableString, EvalAltResult> {
    v.get(key)
        .and_then(|v| v.clone().try_cast())
        .ok_or_else(|| invalid("memory snapshot is missing a string field"))
}

fn category(cat: ImmutableString) -> Result<ImmutableString, EvalAltResult> {
    if cat.len() > 128 || cat.trim().is_empty() || cat.chars().any(char::is_control) {
        return Err(invalid(
            "memory category needs a nonempty label up to 128 bytes",
        ));
    }
    Ok(cat)
}

#[derive(Default)]
struct Group {
    count: INT,
    visible: Array,
}

fn projection(
    items: Array,
    categories: Array,
    active: &str,
    query: &str,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if items.len() > MAX_MEMORIES || categories.len() > 32 || query.len() > 8192 {
        return Err(invalid("memory panel collection exceeds its bound").into());
    }
    let needle = query.trim().to_lowercase();
    let mut counts = BTreeMap::<ImmutableString, Group>::new();
    let mut order = Vec::new();
    for cat in categories {
        let cat = category(
            cat.try_cast()
                .ok_or_else(|| invalid("categories must be strings"))?,
        )?;
        if counts.insert(cat.clone(), Group::default()).is_some() {
            return Err(invalid("duplicate memory category").into());
        }
        order.push(cat);
    }
    let mut ids = BTreeSet::new();
    let mut paused = 0;
    let mut shown = 0;
    let total = INT::try_from(items.len()).expect("bounded memory array");
    for (i, entry) in items.into_iter().enumerate() {
        let entry = entry
            .try_cast::<Map>()
            .ok_or_else(|| invalid("memories must be maps"))?;
        let id = text(&entry, "id")?;
        if id.trim().is_empty()
            || id.len() > 96
            || id.chars().any(char::is_control)
            || !ids.insert(id)
        {
            return Err(invalid("memory IDs must be unique and bounded").into());
        }
        let words = text(&entry, "text")?;
        if words.trim().is_empty()
            || words.len() > MAX_BYTES
            || words.graphemes(true).count() > MAX_GRAPHEMES
        {
            return Err(invalid("memory text is empty or exceeds its bound").into());
        }
        let cat = category(text(&entry, "category")?)?;
        let v = counts.entry(cat.clone()).or_insert_with(|| {
            order.push(cat.clone());
            Group::default()
        });
        v.count += 1;
        let is_paused = entry
            .get("paused")
            .filter(|p| !p.is_unit())
            .map(Dynamic::as_bool)
            .transpose()
            .map_err(|_| invalid("memory paused must be boolean"))?
            .unwrap_or(false);
        paused += i64::from(is_paused);
        if (active.is_empty() || active == cat.as_str())
            && (needle.is_empty()
                || words.to_lowercase().contains(&needle)
                || cat.to_lowercase().contains(&needle))
        {
            shown += 1;
            v.visible
                .push(Dynamic::from_int(INT::try_from(i).expect("bounded index")));
        }
    }
    let groups = order
        .into_iter()
        .map(|name| {
            let group = counts.remove(&name).expect("registered category");
            value([
                ("name", Dynamic::from(name)),
                ("count", Dynamic::from_int(group.count)),
                ("visible", Dynamic::from_array(group.visible)),
            ])
        })
        .collect();
    Ok(value([
        ("groups", Dynamic::from_array(groups)),
        ("total", Dynamic::from_int(total)),
        ("paused", Dynamic::from_int(paused)),
        ("shown", Dynamic::from_int(shown)),
    ]))
}

fn normalize(raw: &str) -> Result<String, Box<EvalAltResult>> {
    if raw.len() > MAX_BYTES || raw.graphemes(true).count() > MAX_GRAPHEMES {
        return Err(invalid("memory draft exceeds its bound").into());
    }
    Ok(raw.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("memory_panel_projection", projection);
    engine.register_fn("memory_panel_normalize", normalize);
}

#[cfg(test)]
mod tests {
    use super::register;
    use rhai::{Array, Engine, Map};

    #[test]
    fn memory_panel_groups_keep_host_order_and_search_without_changing_counts() {
        let mut engine = Engine::new();
        register(&mut engine);
        let r = engine
            .eval::<Map>(
                r#"memory_panel_projection([
            #{id:"a",text:"ÁLVARO uses Rust",category:"Work"},
            #{id:"b/旧",text:"Prefers tea",category:"Preferences",paused:true}
        ],["Preferences","Projects"],"Work"," álvaro ")"#,
            )
            .unwrap();
        assert_eq!(r["total"].as_int().unwrap(), 2);
        assert_eq!(r["paused"].as_int().unwrap(), 1);
        assert_eq!(r["shown"].as_int().unwrap(), 1);
        let groups = r["groups"].clone().cast::<Array>();
        assert_eq!(
            groups[0].clone().cast::<Map>()["name"]
                .clone()
                .into_string()
                .unwrap(),
            "Preferences"
        );
        assert_eq!(
            groups[2].clone().cast::<Map>()["visible"]
                .clone()
                .cast::<Array>()[0]
                .as_int()
                .unwrap(),
            0
        );
        assert_eq!(
            engine
                .eval::<String>(r#"memory_panel_normalize("  one\n two　three " )"#)
                .unwrap(),
            "one two three"
        );
    }

    #[test]
    fn memory_panel_rejects_duplicate_identity_and_empty_categories() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(engine.eval::<Map>(r#"let e=#{id:"a",text:"A",category:"Work"}; memory_panel_projection([e,e],[],"","")"#).is_err());
        assert!(
            engine
                .eval::<Map>(r#"memory_panel_projection([],["  "],"","")"#)
                .is_err()
        );
    }
}
