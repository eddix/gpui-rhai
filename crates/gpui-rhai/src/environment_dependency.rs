use std::collections::{BTreeMap, BTreeSet};

use crate::ComponentInstancePath;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocaleReader {
    window: Option<String>,
    dependency: crate::read_dependency::ReadDependency,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EnvironmentDependencyRegistry {
    locale: BTreeSet<LocaleReader>,
    theme: BTreeSet<LocaleReader>,
    viewport: BTreeMap<String, BTreeSet<crate::read_dependency::ReadDependency>>,
}

impl EnvironmentDependencyRegistry {
    pub fn locale_reader_count(&self) -> usize {
        self.locale.len()
    }

    pub fn viewport_reader_count(&self) -> usize {
        self.viewport.values().map(BTreeSet::len).sum()
    }

    pub fn theme_reader_count(&self) -> usize {
        self.theme.len()
    }

    pub fn reset_reader(&mut self, component: &ComponentInstancePath) {
        self.reset_contribution(&crate::read_dependency::ReadDependency::component(
            component,
        ));
    }
    pub(crate) fn reset_contribution(
        &mut self,
        component: &crate::read_dependency::ReadDependency,
    ) {
        self.locale.retain(|reader| &reader.dependency != component);
        self.theme.retain(|reader| &reader.dependency != component);
        for readers in self.viewport.values_mut() {
            readers.remove(component);
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }

    pub fn track_locale(
        &mut self,
        window: Option<&str>,
        component: impl Into<crate::read_dependency::ReadDependency>,
    ) {
        self.locale.insert(LocaleReader {
            window: window.map(ToOwned::to_owned),
            dependency: component.into(),
        });
    }

    pub fn track_theme(
        &mut self,
        window: Option<&str>,
        component: impl Into<crate::read_dependency::ReadDependency>,
    ) {
        self.theme.insert(LocaleReader {
            window: window.map(ToOwned::to_owned),
            dependency: component.into(),
        });
    }

    pub fn invalidate_theme_app(&self) -> BTreeSet<ComponentInstancePath> {
        self.theme
            .iter()
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn invalidate_theme_window(&self, window: &str) -> BTreeSet<ComponentInstancePath> {
        self.theme
            .iter()
            .filter(|reader| reader.window.as_deref() == Some(window))
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn invalidate_theme_scope(
        &self,
        scope: &ComponentInstancePath,
    ) -> BTreeSet<ComponentInstancePath> {
        self.theme
            .iter()
            .filter(|reader| reader.dependency.owner.is_within(scope))
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn track_viewport(
        &mut self,
        window: &str,
        component: impl Into<crate::read_dependency::ReadDependency>,
    ) {
        self.viewport
            .entry(window.to_owned())
            .or_default()
            .insert(component.into());
    }

    pub fn invalidate_locale_app(&self) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn invalidate_locale_window(&self, window: &str) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .filter(|reader| reader.window.as_deref() == Some(window))
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn invalidate_locale_scope(
        &self,
        scope: &ComponentInstancePath,
    ) -> BTreeSet<ComponentInstancePath> {
        self.locale
            .iter()
            .filter(|reader| reader.dependency.owner.is_within(scope))
            .map(|reader| reader.dependency.owner.clone())
            .collect()
    }

    pub fn invalidate_viewport(&self, window: &str) -> BTreeSet<ComponentInstancePath> {
        crate::read_dependency::owners(self.viewport.get(window).cloned().unwrap_or_default())
    }

    pub fn remove_window(&mut self, window: &str) {
        self.locale
            .retain(|reader| reader.window.as_deref() != Some(window));
        self.theme
            .retain(|reader| reader.window.as_deref() != Some(window));
        self.viewport.remove(window);
    }

    pub fn remove_scope(&mut self, scope: &ComponentInstancePath) {
        self.locale
            .retain(|reader| !reader.dependency.owner.is_within(scope));
        self.theme
            .retain(|reader| !reader.dependency.owner.is_within(scope));
        for readers in self.viewport.values_mut() {
            readers.retain(|component| !component.owner.is_within(scope));
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }

    pub fn retain_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        let retain = |component: &crate::read_dependency::ReadDependency| {
            component.retained_in_owner_scope(root, active)
        };
        self.locale.retain(|reader| retain(&reader.dependency));
        self.theme.retain(|reader| retain(&reader.dependency));
        for readers in self.viewport.values_mut() {
            readers.retain(&retain);
        }
        self.viewport.retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn retain_contributions(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<crate::read_dependency::ReadContribution>,
    ) {
        self.locale.retain(|reader| {
            reader
                .dependency
                .retained_in_contribution_scope(scope, active)
        });
        self.theme.retain(|reader| {
            reader
                .dependency
                .retained_in_contribution_scope(scope, active)
        });
        crate::read_dependency::retain_readers(&mut self.viewport, |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
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
