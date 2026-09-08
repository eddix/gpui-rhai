use std::collections::BTreeMap;

use gpui_rhai::{
    ComponentInstancePath, ComponentStateSchema, StateField, StateStore, StoreId, UiRuntimeState,
    UiValue, ValueSchema,
};

#[test]
fn five_thousand_component_mounts_and_large_store_rollback_remain_structural() {
    let schema = ComponentStateSchema::default();
    let mut components = StateStore::new();
    for index in 0..5_000 {
        components
            .mount_instance(
                ComponentInstancePath::root("Row", index.to_string()),
                &schema,
            )
            .unwrap();
    }
    assert_eq!(components.instance_count(), 5_000);

    let rows = UiValue::Array(
        (0..10_000)
            .map(|index| {
                UiValue::Map(BTreeMap::from([
                    ("id".to_owned(), UiValue::Integer(index)),
                    (
                        "label".to_owned(),
                        UiValue::String(format!("row-{index}")),
                    ),
                ]))
            })
            .collect(),
    );
    let store_schema = ComponentStateSchema::new(BTreeMap::from([(
        "rows".to_owned(),
        StateField::new(ValueSchema::UiValue, rows.clone()),
    )]))
    .unwrap();
    let store = StoreId::app("large");
    let mut runtime = UiRuntimeState::new();
    runtime.stores.declare(store.clone(), store_schema).unwrap();
    let checkpoint = runtime.begin_transaction().unwrap();
    runtime
        .stores
        .write(&store, "rows", UiValue::Array(Vec::new()))
        .unwrap();
    runtime.rollback_transaction(checkpoint).unwrap();
    assert_eq!(
        runtime
            .stores
            .read_tracked(
                &ComponentInstancePath::root("Probe", "reader"),
                &store,
                "rows",
            )
            .unwrap(),
        rows
    );
}
