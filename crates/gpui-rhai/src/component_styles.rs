use std::collections::BTreeMap;

use rhai::{Dynamic, Engine, Map, Scope};
use serde::Serialize;
use thiserror::Error;

use crate::token::{Environment, EnvironmentDeclarations, Variable};
use crate::{ComponentRegistry, ModuleId, Style};

const MAX_STYLED_COMPONENTS: usize = 256;
const MAX_STYLED_PARTS: usize = 4_096;

/// Validated application-wide Style overrides keyed by formal component and part.
///
/// Rules contain the same typed [`Style`] values accepted by per-instance
/// `style`/`part_styles` (or environment tables via `by_env(...)`). They are
/// merged after component source defaults and before explicit instance overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ComponentStyleSheet {
    rules: BTreeMap<ModuleId, BTreeMap<String, Variable<Style>>>,
}

fn merge_style_variable(base: &Variable<Style>, overlay: &Variable<Style>) -> Variable<Style> {
    match (base, overlay) {
        (Variable::Fixed(a), Variable::Fixed(b)) => Variable::Fixed(a.clone().merged(b)),
        (Variable::ByEnv(a), Variable::Fixed(b)) => {
            Variable::ByEnv(a.map(|style| style.clone().merged(b)))
        }
        (Variable::Fixed(a), Variable::ByEnv(b)) => {
            Variable::ByEnv(b.map(|style| a.clone().merged(style)))
        }
        (Variable::ByEnv(a), Variable::ByEnv(b)) => {
            if a.keys() == b.keys() {
                let mut entries = BTreeMap::new();
                for (path, style_a) in a.entries() {
                    let merged = match b.get(path) {
                        Some(style_b) => style_a.clone().merged(style_b),
                        None => style_a.clone(),
                    };
                    entries.insert(path.to_vec(), merged);
                }
                for (path, style_b) in b.entries() {
                    entries
                        .entry(path.to_vec())
                        .or_insert_with(|| style_b.clone());
                }
                crate::token::EnvTable::new(a.keys().to_vec(), entries)
                    .map_or_else(|_| overlay.clone(), Variable::ByEnv)
            } else {
                // Combine distinct environment axes into their Cartesian product.
                let mut keys = a.keys().to_vec();
                let mut b_positions = Vec::with_capacity(b.keys().len());
                for key in b.keys() {
                    if let Some(pos) = keys.iter().position(|existing| existing == key) {
                        b_positions.push((pos, false));
                    } else {
                        b_positions.push((keys.len(), true));
                        keys.push(*key);
                    }
                }
                let mut entries = BTreeMap::new();
                for (path_a, style_a) in a.entries() {
                    for (path_b, style_b) in b.entries() {
                        let mut compatible = true;
                        let mut combined = path_a.to_vec();
                        for (index, &(target_pos, is_new)) in b_positions.iter().enumerate() {
                            if is_new {
                                combined.push(path_b[index]);
                            } else if combined[target_pos] != path_b[index] {
                                compatible = false;
                                break;
                            }
                        }
                        if compatible {
                            entries.insert(combined, style_a.clone().merged(style_b));
                        }
                    }
                }
                crate::token::EnvTable::new(keys, entries)
                    .map_or_else(|_| overlay.clone(), Variable::ByEnv)
            }
        }
    }
}

impl ComponentStyleSheet {
    pub(crate) fn validate_parts(
        &self,
        component: &ModuleId,
        declared: &std::collections::BTreeSet<String>,
    ) -> Result<(), ComponentStyleError> {
        if let Some(parts) = self.rules.get(component) {
            for part in parts.keys() {
                if !declared.contains(part) {
                    return Err(ComponentStyleError::UnknownPart {
                        component: component.clone(),
                        part: part.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Overlay rules part by part, preserving unspecified declarations.
    #[must_use]
    pub fn merged(mut self, overlay: &Self) -> Self {
        for (id, parts) in &overlay.rules {
            let target = self.rules.entry(id.clone()).or_default();
            for (part, style) in parts {
                match target.get(part) {
                    Some(existing) => {
                        let combined = merge_style_variable(existing, style);
                        target.insert(part.clone(), combined);
                    }
                    None => {
                        target.insert(part.clone(), style.clone());
                    }
                }
            }
        }
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.values().map(BTreeMap::len).sum()
    }

    /// Raw component part rules, which may be fixed styles or `by_env(...)` tables.
    #[must_use]
    pub fn component_rules(&self, id: &ModuleId) -> Option<&BTreeMap<String, Variable<Style>>> {
        self.rules.get(id)
    }

    /// Resolve every part rule of `id` for `environment` using declared defaults.
    #[must_use]
    pub fn resolve_component(
        &self,
        id: &ModuleId,
        environment: &Environment,
        declarations: &EnvironmentDeclarations,
    ) -> Option<BTreeMap<String, Style>> {
        let parts = self.rules.get(id)?;
        let resolved = parts
            .iter()
            .filter_map(|(part, value)| {
                value
                    .resolve(environment, declarations)
                    .cloned()
                    .map(|style| (part.clone(), style))
            })
            .collect();
        Some(resolved)
    }

    /// Resolve every fixed part rule of `id` without an environment table.
    /// Rules that depend on an environment without defaults are omitted; prefer
    /// [`Self::resolve_component`] when a theme environment is available.
    #[must_use]
    pub fn component(&self, id: &ModuleId) -> Option<BTreeMap<String, Style>> {
        self.resolve_component(id, &Environment::EMPTY, &EnvironmentDeclarations::new())
    }

    /// Iterate raw validated component/part rules (including `by_env(...)` tables).
    pub fn iter(&self) -> impl Iterator<Item = (&ModuleId, &BTreeMap<String, Variable<Style>>)> {
        self.rules.iter()
    }

    fn new(
        rules: BTreeMap<ModuleId, BTreeMap<String, Variable<Style>>>,
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
    crate::engine::validate_runtime_ast(&ast)
        .map_err(|error| ComponentStyleError::Script(error.to_string()))?;
    ast.set_source(source_name);
    let raw: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "component_styles", ())
        .map_err(|error| ComponentStyleError::Script(error.to_string()))?;
    let sheet = decode_component_styles(raw)?;
    ComponentStyleSheet::new(sheet.rules, components)
}

/// Theme presets can describe modules not imported by a particular app. Decode
/// their bounded, typed rules here; the active module validates its parts when
/// the preset is used, rather than requiring every module to be imported.
pub(crate) fn decode_component_styles(
    raw: Dynamic,
) -> Result<ComponentStyleSheet, ComponentStyleError> {
    let root = raw
        .try_cast::<Map>()
        .ok_or(ComponentStyleError::RootNotMap)?;
    if root.len() > MAX_STYLED_COMPONENTS {
        return Err(ComponentStyleError::TooManyComponents(root.len()));
    }
    let mut rules = BTreeMap::new();
    let mut part_count = 0;
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
            part_count += 1;
            if part_count > MAX_STYLED_PARTS {
                return Err(ComponentStyleError::TooManyParts(part_count));
            }
            if !crate::token::valid_token_segment(&part) {
                return Err(ComponentStyleError::InvalidPartName(part.to_string()));
            }
            if let Some(style) = value.read_lock::<Style>().map(|style| style.clone()) {
                decoded.insert(part.to_string(), Variable::Fixed(style));
            } else if let Some(table) =
                crate::theme_source::try_decode_style_env_table(&value, &format!("{id}.{part}"))
                    .map_err(|reason| ComponentStyleError::Script(reason))?
            {
                decoded.insert(part.to_string(), Variable::ByEnv(table));
            } else {
                return Err(ComponentStyleError::PartNotStyle {
                    component: id,
                    part: part.to_string(),
                });
            }
        }
        rules.insert(id, decoded);
    }
    Ok(ComponentStyleSheet { rules })
}

#[derive(Debug, Error)]
pub enum ComponentStyleError {
    #[error("component style part `{0}` must be a snake_case name")]
    InvalidPartName(String),
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
                        runtime_api: RuntimeApiRange::new(3, 4),
                        dependencies: std::collections::BTreeSet::default(),
                        capabilities: BTreeMap::default(),
                        assets: std::collections::BTreeSet::default(),
                        tokens: std::collections::BTreeSet::new(),
                        environment: std::collections::BTreeSet::new(),
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
    fn multipart_state_keeps_the_cascade_global_to_instance_order() {
        let mut engine = crate::RuntimeEngine::new();
        let compiled = engine.compile(r#"
            define_component(#{metadata:#{id:"components/button","export":"Button",version:"0.1.0",
                runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
                schema:#{props:#{},state:#{fields:#{}},events:#{},slots:#{},parts:["root","label"]},render:Fn("render_Button")});
            fn Button(props){render_component("components/button",props)}
            fn render_Button(ctx,props){text("Button").with_style(ctx.component_style(["root","label"],
                style().height(px(24)).radius(px(0)),style().height(px(20)).radius(px(0))))}
            fn view(ctx){column([Button(#{key:"app"}),Button(#{key:"instance",style:style().height(px(40))}),
                Button(#{key:"parts",style:style().height(px(40)),part_styles:#{label:style().height(px(42))}})])}
        "#).unwrap();
        let theme = crate::load_theme_source(engine.engine(), "preset", r#"fn theme(){#{family:"Test",name:"Dark",mode:"dark",tokens:#{
            component_styles:#{"components/button":#{root:style().height(px(28)).radius(px(8)),label:style().height(px(29))}}
        }}}"#).unwrap();
        let sheet = load_component_styles(
            engine.engine(),
            "styles",
            r#"fn component_styles(){#{"components/button":#{
            root:style().height(px(34)),label:style().radius(px(6))}}}"#,
            &registry(),
        )
        .unwrap();
        let mut runtime = UiRuntimeState::new();
        runtime.theme = Some(
            crate::ThemeManager::from_variants([theme], crate::ThemeSelection::new("Test", "Dark"))
                .unwrap(),
        );
        runtime.replace_component_styles_from_host(sheet);
        let context = UiContext::new(
            Rc::new(RefCell::new(runtime)),
            ComponentInstancePath::root("App", "multi"),
            None,
            ExecutionPhase::Render,
            BTreeMap::new(),
        )
        .with_generation(compiled.generation());
        let root = engine.render_with_context(&compiled, context).unwrap();
        let crate::UiNodeKind::Box { children } = root.kind() else {
            panic!("column");
        };
        for (child, height) in children.iter().zip([34.0, 40.0, 42.0]) {
            assert_eq!(
                child.style().base.height,
                Some(crate::Length::Pixels(height).into())
            );
            assert_eq!(
                child.style().base.radii.top_left,
                Some(crate::Length::Pixels(6.0))
            );
        }
    }

    #[test]
    fn multipart_names_are_bounded_typed_unique_and_nonempty() {
        let engine = crate::RuntimeEngine::new();
        let context = UiContext::new(
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "parts"),
            None,
            ExecutionPhase::Event,
            BTreeMap::new(),
        );
        let mut scope = Scope::new();
        scope.push("ctx", context);
        for parts in ["[]", "[0]", "[\"root\",\"root\"]", "[\"bad-name\"]"] {
            assert!(
                engine
                    .engine()
                    .eval_with_scope::<Style>(
                        &mut scope,
                        &format!("ctx.component_style({parts},style(),style())")
                    )
                    .is_err()
            );
        }
        let many = (0..33)
            .map(|i| format!("\"part_{i}\""))
            .collect::<Vec<_>>()
            .join(",");
        assert!(
            engine
                .engine()
                .eval_with_scope::<Style>(
                    &mut scope,
                    &format!("ctx.component_style([{many}],style())")
                )
                .is_err()
        );
        assert_eq!(
            engine
                .engine()
                .eval_with_scope::<Style>(
                    &mut scope,
                    "ctx.component_style([\"root\"],style().height(px(10)))"
                )
                .unwrap()
                .base
                .height,
            Some(crate::Length::Pixels(10.0).into())
        );
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
                            runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
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
