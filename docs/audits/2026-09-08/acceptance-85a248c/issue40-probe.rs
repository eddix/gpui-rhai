use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
fn find_list(node: &UiNode) -> Option<&VirtualCollectionNodeSpec> {
    match node.kind() {
        UiNodeKind::VirtualCollection { spec } => Some(spec),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(find_list)
        }
        _ => None,
    }
}
// For this fixture NativeCollection data keys are exactly a/b/c. Its fuzzy
// projection prefixes realized option node keys with item:, while data.key(index)
// (used by collection_item_key) returns the original a/b/c key. Group keys agree.
fn data_key(native: bool, node: &UiNode) -> String {
    let key = node.key().unwrap().as_str();
    if native && node.attributes().get("role") == Some(&UiValue::String("option".into())) {
        key.strip_prefix("item:").unwrap().to_owned()
    } else {
        key.to_owned()
    }
}
fn main() {
    let repo = std::env::var("GPUI_RHAI_ACCEPT_REPO").unwrap();
    for native in [false, true] {
        for grouped in [false, true] {
            for disabled_first in [false, true] {
                let mut engine = RuntimeEngine::new();
                let mut resolver = RestrictedModuleResolver::new();
                for name in ["command", "input", "kbd"] {
                    resolver
                        .insert(
                            format!("components/{name}"),
                            std::fs::read_to_string(format!(
                                "{repo}/registry/components/{name}.rhai"
                            ))
                            .unwrap(),
                        )
                        .unwrap();
                }
                engine.set_module_resolver(resolver);
                let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
                let groups = if grouped {
                    ["G1", "G1", "G2"]
                } else {
                    ["", "", ""]
                };
                let rows = ["a", "b", "c"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, key)| {
                        BTreeMap::from([
                            ("value".into(), UiValue::String(key.into())),
                            ("label".into(), UiValue::String(key.to_uppercase())),
                            ("group".into(), UiValue::String(groups[i].into())),
                            ("disabled".into(), UiValue::Bool(i == 0 && disabled_first)),
                            ("keywords".into(), UiValue::Array(vec![])),
                            ("shortcut".into(), UiValue::String(String::new())),
                        ])
                    })
                    .collect::<Vec<_>>();
                runtime
                    .borrow_mut()
                    .native_collections
                    .register("commands", NativeCollection::new("value", rows).unwrap())
                    .unwrap();
                let items = if native {
                    "ctx.get_native_collection(\"commands\")".to_owned()
                } else {
                    format!(
                        "[#{{value:\"a\",label:\"A\",group:\"{}\",disabled:{}}},#{{value:\"b\",label:\"B\",group:\"{}\"}},#{{value:\"c\",label:\"C\",group:\"{}\"}}]",
                        groups[0], disabled_first, groups[1], groups[2]
                    )
                };
                let source = format!(
                    "import \"components/command\" as command; fn view(ctx) {{ command::Command(#{{key:\"palette\",label:\"Palette\",query:\"\",active_value:\"b\",items:{items},max_visible:8}}) }}"
                );
                let compiled = engine
                    .compile_self_contained_named("issue40", &source)
                    .unwrap();
                let mut lifecycle = ScriptLifecycle::new(
                    compiled,
                    runtime,
                    ComponentInstancePath::root("App", "issue40"),
                    None,
                    BTreeMap::new(),
                    &ComponentStateSchema::default(),
                )
                .unwrap();
                let root = lifecycle.start(&mut engine).unwrap();
                let spec = find_list(root).unwrap();
                assert_eq!(
                    spec.realized.len(),
                    spec.data.len(),
                    "the small fixture must be fully realized"
                );
                // Replay install_keys' exact inclusion predicate using the real source-produced snapshots.
                // This uses the public VirtualListState; it does not claim a new GPU screenshot.
                let keys = spec
                    .realized
                    .iter()
                    .filter(|(i, _)| !spec.sticky_headers.contains(i))
                    .map(|(_, n)| data_key(native, n))
                    .collect();
                let mut state = VirtualListState::default();
                state.set_keys(keys).unwrap();
                state.focus_first();
                let first = state.focused().unwrap().to_owned();
                let first_node = spec
                    .realized
                    .values()
                    .find(|n| data_key(native, n) == first)
                    .unwrap();
                let selected = spec
                    .realized
                    .values()
                    .filter(|n| n.attributes().get("checked") == Some(&UiValue::Bool(true)))
                    .map(|n| data_key(native, n))
                    .collect::<Vec<_>>();
                let mut sequence = Vec::new();
                for _ in 0..spec.data.len() {
                    sequence.push(state.focused().unwrap().to_owned());
                    state.focus_next();
                }
                println!(
                    "backend={} grouped={} disabled_first={} sticky={:?} reveal={:?} first={} first_role={:?} first_disabled={:?} source_active={:?} roving={:?}",
                    if native { "native" } else { "array" },
                    grouped,
                    disabled_first,
                    spec.sticky_headers,
                    spec.reveal_key,
                    first,
                    first_node.attributes().get("role"),
                    first_node.attributes().get("disabled"),
                    selected,
                    sequence
                );
            }
        }
    }
}
