use std::collections::{BTreeMap, BTreeSet};

use crate::ComponentInstancePath;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocaleReader {
    window: Option<String>,
    component: ComponentInstancePath,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EnvironmentDependencyRegistry {
    locale: BTreeSet<LocaleReader>,
    viewport: BTreeMap<String, BTreeSet<ComponentInstancePath>>,
}

impl EnvironmentDependencyRegistry {
    pub fn reset_reader(&mut self, component: &ComponentInstancePath) {
        self.locale.retain(|reader| &reader.component != component);
        for readers in self.viewport.values_mut() {
            readers.remove(component);
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }

    pub fn track_locale(&mut self, window: Option<&str>, component: &ComponentInstancePath) {
        self.locale.insert(LocaleReader {
            window: window.map(ToOwned::to_owned),
            component: component.clone(),
        });
    }

    pub fn track_viewport(&mut self, window: &str, component: &ComponentInstancePath) {
        self.viewport
            .entry(window.to_owned())
            .or_default()
            .insert(component.clone());
    }

    pub fn invalidate_locale_app(&self) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .map(|reader| reader.component.clone())
            .collect()
    }

    pub fn invalidate_locale_window(&self, window: &str) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .filter(|reader| reader.window.as_deref() == Some(window))
            .map(|reader| reader.component.clone())
            .collect()
    }

    pub fn invalidate_locale_scope(
        &self,
        scope: &ComponentInstancePath,
    ) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .filter(|reader| reader.component.is_within(scope))
            .map(|reader| reader.component.clone())
            .collect()
    }

    pub fn invalidate_viewport(&self, window: &str) -> BTreeSet<ComponentInstancePath> {
        self.viewport.get(window).cloned().unwrap_or_default()
    }

    pub fn remove_window(&mut self, window: &str) {
        self.locale
            .retain(|reader| reader.window.as_deref() != Some(window));
        self.viewport.remove(window);
    }

    pub fn remove_scope(&mut self, scope: &ComponentInstancePath) {
        self.locale
            .retain(|reader| !reader.component.is_within(scope));
        for readers in self.viewport.values_mut() {
            readers.retain(|component| !component.is_within(scope));
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }

    pub fn retain_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        let retain = |component: &ComponentInstancePath| {
            !component.is_within(root) || component == root || active.contains(component)
        };
        self.locale.retain(|reader| retain(&reader.component));
        for readers in self.viewport.values_mut() {
            readers.retain(&retain);
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_dependencies_invalidate_only_matching_domains() {
        let mut dependencies = EnvironmentDependencyRegistry::default();
        let first = ComponentInstancePath::root("View", "first");
        let second = ComponentInstancePath::root("View", "second");
        let first_child = first.child("Panel", "panel");
        dependencies.track_locale(Some("main"), &first);
        dependencies.track_locale(Some("main"), &first_child);
        dependencies.track_locale(Some("settings"), &second);
        dependencies.track_viewport("main", &first_child);
        dependencies.track_viewport("settings", &second);

        assert_eq!(
            dependencies.invalidate_locale_window("main"),
            BTreeSet::from([first.clone(), first_child.clone()])
        );
        assert_eq!(
            dependencies.invalidate_locale_scope(&first),
            BTreeSet::from([first.clone(), first_child.clone()])
        );
        assert_eq!(
            dependencies.invalidate_viewport("main"),
            BTreeSet::from([first_child.clone()])
        );

        dependencies.retain_scope(&first, &BTreeSet::from([first.clone()]));
        assert!(dependencies.invalidate_viewport("main").is_empty());
        assert_eq!(
            dependencies.invalidate_locale_window("main"),
            BTreeSet::from([first])
        );
    }
}
