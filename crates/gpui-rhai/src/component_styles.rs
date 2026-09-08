use std::collections::BTreeMap;

use rhai::{Dynamic, Engine, Map, Scope};
use thiserror::Error;

use crate::{ComponentRegistry, ModuleId, Style};

const MAX_STYLED_COMPONENTS: usize = 256;
const MAX_STYLED_PARTS: usize = 4_096;

/// Validated application-wide Style overrides keyed by formal component and part.
///
/// Rules contain the same typed [`Style`] values accepted by per-instance
/// `style`/`part_styles`. They are merged after component source defaults and
/// before explicit instance overrides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentStyleSheet {
    rules: BTreeMap<ModuleId, BTreeMap<String, Style>>,
}

impl ComponentStyleSheet {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.values().map(BTreeMap::len).sum()
    }

    #[must_use]
    pub fn component(&self, id: &ModuleId) -> Option<&BTreeMap<String, Style>> {
        self.rules.get(id)
    }

    fn new(
        rules: BTreeMap<ModuleId, BTreeMap<String, Style>>,
        components: &ComponentRegistry,
    ) -> Result<Self, ComponentStyleError> {
        if rules.len() > MAX_STYLED_COMPONENTS {
            return Err(ComponentStyleError::TooManyComponents(rules.len()));
        }
        let part_count = rules.values().map(BTreeMap::len).sum::<usize>();
        if part_count > MAX_STYLED_PARTS {
            return Err(ComponentStyleError::TooManyParts(part_count));
        }
        for (id, parts) in &rules {
            let component = components
                .get(id)
                .ok_or_else(|| ComponentStyleError::UnknownComponent(id.clone()))?;
            for part in parts.keys() {
                if !component.schema.parts.contains(part) {
                    return Err(ComponentStyleError::UnknownPart {
                        component: id.clone(),
                        part: part.clone(),
                    });
                }
            }
        }
        Ok(Self { rules })
    }
}

/// Compile and evaluate `component_styles() -> map`, then validate every
/// component ID and named part against the active component registry.
///
/// # Errors
///
/// Returns compile/evaluation errors, shape/type errors, resource-limit errors,
/// or an unknown component/part diagnostic.
pub fn load_component_styles(
    engine: &Engine,
    source_name: &str,
    source: &str,
    components: &ComponentRegistry,
) -> Result<ComponentStyleSheet, ComponentStyleError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| ComponentStyleError::Script(error.to_string()))?;
    crate::engine::validate_assignment_targets(&ast)
        .map_err(|error| ComponentStyleError::Script(error.to_string()))?;
    ast.set_source(source_name);
    let raw: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "component_styles", ())
        .map_err(|error| ComponentStyleError::Script(error.to_string()))?;
    let root = raw
        .try_cast::<Map>()
        .ok_or(ComponentStyleError::RootNotMap)?;
    let mut rules = BTreeMap::new();
    for (raw_id, raw_parts) in root {
        let id = ModuleId::parse(raw_id.as_str()).map_err(|source| {
            ComponentStyleError::InvalidComponentId {
                id: raw_id.to_string(),
                source,
            }
        })?;
        let parts = raw_parts
            .try_cast::<Map>()
            .ok_or_else(|| ComponentStyleError::ComponentNotMap(id.clone()))?;
        let mut decoded = BTreeMap::new();
        for (part, value) in parts {
            if !value.is::<Style>() {
                return Err(ComponentStyleError::PartNotStyle {
                    component: id,
                    part: part.to_string(),
                });
            }
            decoded.insert(part.to_string(), value.cast::<Style>());
        }
        rules.insert(id, decoded);
    }
    ComponentStyleSheet::new(rules, components)
}

#[derive(Debug, Error)]
pub enum ComponentStyleError {
    #[error("component stylesheet script failed: {0}")]
    Script(String),
    #[error("component_styles() must return a map")]
    RootNotMap,
    #[error("component stylesheet ID `{id}` is invalid: {source}")]
    InvalidComponentId {
        id: String,
        source: crate::ModuleIdError,
    },
    #[error("component stylesheet rule `{0}` must be a map of named Style values")]
    ComponentNotMap(ModuleId),
    #[error("component stylesheet rule `{component}.{part}` must be a Style")]
    PartNotStyle { component: ModuleId, part: String },
    #[error("component stylesheet references unavailable component `{0}`")]
    UnknownComponent(ModuleId),
    #[error("component stylesheet references unknown part `{component}.{part}`")]
    UnknownPart { component: ModuleId, part: String },
    #[error("component stylesheet has {0} components; the limit is {MAX_STYLED_COMPONENTS}")]
    TooManyComponents(usize),
    #[error("component stylesheet has {0} part rules; the limit is {MAX_STYLED_PARTS}")]
    TooManyParts(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentDefinition, ComponentInstancePath, ComponentMetadata, ComponentSchema,
        ExecutionPhase, RuntimeApiRange, UiContext, UiRuntimeState,
    };
    use semver::Version;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn registry() -> ComponentRegistry {
        let mut registry = ComponentRegistry::new();
        registry
            .register(
                ComponentDefinition::new(
                    ComponentMetadata {
                        id: ModuleId::parse("components/button").unwrap(),
                        export: "Button".to_owned(),
                        version: Version::new(0, 1, 0),
                        runtime_api: RuntimeApiRange::new(1, 2),
                        dependencies: std::collections::BTreeSet::default(),
                        capabilities: BTreeMap::default(),
                        assets: std::collections::BTreeSet::default(),
                    },
                    ComponentSchema {
                        parts: ["root".to_owned(), "label".to_owned()]
                            .into_iter()
                            .collect(),
                        ..ComponentSchema::default()
                    },
                )
                .unwrap(),
                crate::RUNTIME_API_VERSION,
            )
            .unwrap();
        registry
    }

    #[test]
    fn source_decodes_typed_styles_and_validates_parts() {
        let engine = crate::RuntimeEngine::new();
        let sheet = load_component_styles(
            engine.engine(),
            "styles.rhai",
            r#"
                fn component_styles() {
                    #{ "components/button": #{
                        root: style().height(px(34)).radius(theme_radius("md")),
                        label: style().typography("body").font_weight(600),
                    } }
                }
            "#,
            &registry(),
        )
        .unwrap();
        assert_eq!(sheet.len(), 2);
        assert_eq!(
            sheet
                .component(&ModuleId::parse("components/button").unwrap())
                .unwrap()["root"]
                .base
                .height,
            Some(crate::LayoutLength::Definite(crate::Length::Pixels(34.0)))
        );
    }

    #[test]
    fn source_rejects_unknown_components_parts_and_non_styles() {
        let engine = crate::RuntimeEngine::new();
        for (source, expected) in [
            (
                r#"fn component_styles() { #{ "components/missing": #{ root: style() } } }"#,
                "unavailable component",
            ),
            (
                r#"fn component_styles() { #{ "components/button": #{ icon: style() } } }"#,
                "unknown part",
            ),
            (
                r#"fn component_styles() { #{ "components/button": #{ root: 12 } } }"#,
                "must be a Style",
            ),
        ] {
            let error = load_component_styles(engine.engine(), "styles.rhai", source, &registry())
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn stylesheet_precedes_instance_overrides_during_component_render() {
        let mut engine = crate::RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    define_component(#{
                        metadata: #{
                            id: "components/button", "export": "Button", version: "0.1.0",
                            runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
                            dependencies: [], capabilities: #{}
                        },
                        schema: #{ props: #{}, state: #{ fields: #{} }, events: #{},
                            slots: #{}, parts: ["root", "label"] },
                        render: Fn("render_Button")
                    });
                    fn Button(props) { render_component("components/button", props) }
                    fn render_Button(ctx, props) {
                        text("button").with_style(ctx.component_style("root",
                            style().height(px(24)).radius(px(0))))
                    }
                    fn view(ctx) {
                        column([
                            Button(#{ key: "global" }),
                            Button(#{ key: "instance", style: style().height(px(40)) }),
                        ])
                    }
                "#,
            )
            .unwrap();
        let sheet = load_component_styles(
            engine.engine(),
            "styles.rhai",
            r#"
                fn component_styles() {
                    #{ "components/button": #{
                        root: style().height(px(34)).radius(px(6)),
                    } }
                }
            "#,
            &registry(),
        )
        .unwrap();
        let mut state = UiRuntimeState::new();
        state.replace_component_styles_from_host(sheet);
        let state = Rc::new(RefCell::new(state));
        let root_path = ComponentInstancePath::root("App", "styles");
        let context = UiContext::new(
            Rc::clone(&state),
            root_path,
            None,
            ExecutionPhase::Render,
            BTreeMap::new(),
        )
        .with_generation(compiled.generation());
        let root = engine.render_with_context(&compiled, context).unwrap();
        let crate::UiNodeKind::Box { children } = root.kind() else {
            panic!("view must return a column");
        };
        assert_eq!(
            children[0].style().base.height,
            Some(crate::LayoutLength::Definite(crate::Length::Pixels(34.0)))
        );
        assert_eq!(
            children[1].style().base.height,
            Some(crate::LayoutLength::Definite(crate::Length::Pixels(40.0)))
        );
        assert_eq!(
            children[0].style().base.radii.top_left,
            Some(crate::Length::Pixels(6.0))
        );
    }
}
