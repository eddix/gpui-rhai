use std::collections::BTreeMap;

use crate::{ComponentInstancePath, ScriptCallback, UiValue};

#[derive(Clone, Debug, PartialEq)]
pub struct UiEvent {
    pub name: String,
    pub payload: UiValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPropagation {
    Handled,
    Propagate,
}

#[derive(Clone, Debug, Default)]
pub struct EventRouter {
    handlers: BTreeMap<ComponentInstancePath, BTreeMap<String, ScriptCallback>>,
}

impl EventRouter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(
        &mut self,
        target: ComponentInstancePath,
        event: impl Into<String>,
        callback: ScriptCallback,
    ) {
        self.handlers
            .entry(target)
            .or_default()
            .insert(event.into(), callback);
    }

    pub fn unbind_component(&mut self, target: &ComponentInstancePath) {
        self.handlers.remove(target);
    }

    /// Bubble a normalized event from its target toward the root.
    ///
    /// # Errors
    ///
    /// Propagates errors returned by the callback invoker and stops dispatch.
    pub fn dispatch<E>(
        &self,
        target: &ComponentInstancePath,
        event: &UiEvent,
        mut invoke: impl FnMut(
            &ComponentInstancePath,
            &ScriptCallback,
            &UiEvent,
        ) -> Result<EventPropagation, E>,
    ) -> Result<EventDispatchReport, E> {
        let mut current = Some(target.clone());
        let mut visited = Vec::new();
        while let Some(path) = current {
            visited.push(path.clone());
            if let Some(callback) = self
                .handlers
                .get(&path)
                .and_then(|handlers| handlers.get(&event.name))
                && invoke(&path, callback, event)? == EventPropagation::Handled
            {
                return Ok(EventDispatchReport {
                    visited,
                    handled_by: Some(path),
                });
            }
            current = path.parent();
        }
        Ok(EventDispatchReport {
            visited,
            handled_by: None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDispatchReport {
    pub visited: Vec<ComponentInstancePath>,
    pub handled_by: Option<ComponentInstancePath>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    #[test]
    fn events_bubble_until_explicitly_handled() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("event") }
                    fn child_click(payload) { payload }
                    fn root_click(payload) { payload }
                "#,
            )
            .unwrap();
        let root = ComponentInstancePath::root("App", "root");
        let child = root.child("Button", "save");
        let mut router = EventRouter::new();
        router.bind(
            child.clone(),
            "click",
            runtime.callback(&compiled, "child_click").unwrap(),
        );
        router.bind(
            root.clone(),
            "click",
            runtime.callback(&compiled, "root_click").unwrap(),
        );
        let event = UiEvent {
            name: "click".to_owned(),
            payload: UiValue::Null,
        };
        let mut order = Vec::new();
        let report = router
            .dispatch(&child, &event, |path, _, _| {
                order.push(path.clone());
                Ok::<_, ()>(if path == &root {
                    EventPropagation::Handled
                } else {
                    EventPropagation::Propagate
                })
            })
            .unwrap();

        assert_eq!(order, vec![child, root.clone()]);
        assert_eq!(report.handled_by, Some(root));
    }
}
