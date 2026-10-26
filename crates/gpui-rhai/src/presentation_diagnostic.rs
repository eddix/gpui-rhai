//! Bounded, foreground presentation diagnostics, not script transaction errors.
use std::{
    cell::RefCell,
    collections::BTreeMap,
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{NodeId, ScriptGeneration};

/// A recoverable native image presentation failure. No image bytes or script
/// state are included. The normal last-good script transaction remains valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationDiagnostic {
    pub generation: ScriptGeneration,
    pub node_id: Option<NodeId>,
    pub node_path: String,
    pub asset_id: Option<String>,
    pub message: String,
}

const CAPACITY: usize = 256;
type Key = (Option<NodeId>, u64);

#[derive(Default)]
struct State {
    generation: Option<ScriptGeneration>,
    entries: BTreeMap<Key, (PresentationDiagnostic, bool)>,
}

/// One view's bounded active-error set. Deterministic admission avoids an LRU
/// eviction/repaint feedback loop when a scene contains more than CAPACITY errors.
#[derive(Clone, Default)]
pub(crate) struct PresentationDiagnostics(Rc<RefCell<State>>);

#[derive(Clone)]
pub(crate) struct PresentationSink {
    registry: PresentationDiagnostics,
    generation: ScriptGeneration,
}

impl PresentationDiagnostics {
    pub(crate) fn sink(&self, generation: ScriptGeneration) -> PresentationSink {
        let mut state = self.0.borrow_mut();
        if state.generation != Some(generation) {
            state.generation = Some(generation);
            state.entries.clear();
        }
        PresentationSink {
            registry: self.clone(),
            generation,
        }
    }

    pub(crate) fn drain(&self) -> Vec<PresentationDiagnostic> {
        self.0
            .borrow_mut()
            .entries
            .values_mut()
            .filter_map(|(report, delivered)| {
                if *delivered {
                    None
                } else {
                    *delivered = true;
                    Some(report.clone())
                }
            })
            .collect()
    }

    pub(crate) fn clear(&self) {
        *self.0.borrow_mut() = State::default();
    }

    pub(crate) fn retain_nodes(&self, mut alive: impl FnMut(NodeId) -> bool) {
        self.0
            .borrow_mut()
            .entries
            .retain(|(node, _), _| node.is_none_or(&mut alive));
    }
}

impl PresentationSink {
    pub(crate) fn image_result(
        &self,
        path: &str,
        node_id: Option<NodeId>,
        asset_id: Option<&str>,
        error: Option<String>,
    ) {
        let mut state = self.registry.0.borrow_mut();
        if state.generation != Some(self.generation) {
            return;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hash);
        let key = (node_id, hash.finish());
        let Some(message) = error else {
            state.entries.remove(&key);
            return;
        };
        let report = PresentationDiagnostic {
            generation: self.generation,
            node_id,
            node_path: path.chars().take(512).collect(),
            asset_id: asset_id.map(|id| id.chars().take(512).collect()),
            message: message.chars().take(1024).collect(),
        };
        if state
            .entries
            .get(&key)
            .is_some_and(|(previous, _)| *previous == report)
        {
            return;
        }
        if state.entries.len() == CAPACITY && !state.entries.contains_key(&key) {
            if state
                .entries
                .last_key_value()
                .is_some_and(|(last, _)| key > *last)
            {
                return;
            }
            state.entries.pop_last();
        }
        state.entries.insert(key, (report, false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_are_bounded_deduplicated_and_generation_scoped() {
        let registry = PresentationDiagnostics::default();
        let generation = ScriptGeneration::default();
        let sink = registry.sink(generation);
        for _ in 0..3 {
            for index in 0..1000 {
                sink.image_result(&index.to_string(), None, None, Some("missing".into()));
            }
        }
        assert_eq!(registry.drain().len(), CAPACITY);
        for index in 0..1000 {
            sink.image_result(&index.to_string(), None, None, Some("missing".into()));
        }
        assert!(
            registry.drain().is_empty(),
            "no repaint/eviction reporting loop"
        );
        let key = registry
            .0
            .borrow()
            .entries
            .values()
            .next()
            .unwrap()
            .0
            .node_path
            .clone();
        sink.image_result(&key, None, None, None);
        sink.image_result(&key, None, None, Some("missing".into()));
        assert_eq!(
            registry.drain().len(),
            1,
            "recovery permits a later failure"
        );
        registry.clear();
        sink.image_result("stale", None, None, Some("obsolete".into()));
        assert!(registry.drain().is_empty());
        let next = registry.sink(generation.next());
        sink.image_result("stale", None, None, Some("obsolete".into()));
        next.image_result("new", None, None, Some("current".into()));
        assert_eq!(registry.drain().len(), 1);
    }
}
