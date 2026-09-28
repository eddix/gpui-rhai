//! Shared stable structural ordering for native collection projections.

use std::collections::BTreeMap;

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

#[cfg(test)]
mod tests {
    use super::stable_groups;

    #[test]
    fn stable_grouping_preserves_first_group_and_item_order() {
        let groups = stable_groups([("b", 1), ("a", 2), ("b", 3), ("c", 4), ("a", 5)]);
        assert_eq!(
            groups,
            vec![("b", vec![1, 3]), ("a", vec![2, 5]), ("c", vec![4])]
        );
    }
}
