use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use rhai::{
    AST, ASTFlags, Dynamic, Engine, EvalAltResult, Expr, Module, ModuleResolver, Position, Scope,
    Shared, Stmt,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ModuleCompileCache, ScriptSource, ScriptSourceError};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModuleId(String);

impl ModuleId {
    /// Parse and validate a logical module identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ModuleIdError`] for absolute paths, traversal, empty segments,
    /// platform paths, or unsupported characters.
    pub fn parse(value: impl Into<String>) -> Result<Self, ModuleIdError> {
        let value = value.into();
        validate_module_id(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TryFrom<String> for ModuleId {
    type Error = ModuleIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<ModuleId> for String {
    fn from(value: ModuleId) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ModuleIdError {
    #[error("module id cannot be empty")]
    Empty,
    #[error("module id `{0}` must be relative and use `/` separators")]
    AbsoluteOrPlatformPath(String),
    #[error("module id `{0}` contains an empty, `.` or `..` segment")]
    InvalidSegment(String),
    #[error("module id `{0}` contains unsupported characters")]
    UnsupportedCharacters(String),
}

fn validate_module_id(value: &str) -> Result<(), ModuleIdError> {
    if value.is_empty() {
        return Err(ModuleIdError::Empty);
    }
    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains(':')
    {
        return Err(ModuleIdError::AbsoluteOrPlatformPath(value.to_owned()));
    }
    if value
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(ModuleIdError::InvalidSegment(value.to_owned()));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-'))
    {
        return Err(ModuleIdError::UnsupportedCharacters(value.to_owned()));
    }
    Ok(())
}

/// A resolver that can only load explicitly registered, logical module IDs.
///
/// It never reads the filesystem and rejects path traversal, absolute paths,
/// platform path syntax, and cyclic imports.
#[derive(Debug, Default)]
pub struct RestrictedModuleResolver {
    sources: BTreeMap<ModuleId, String>,
    compiled: BTreeMap<ModuleId, AST>,
    resolving: Mutex<Vec<ModuleId>>,
}

impl RestrictedModuleResolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot every module from a file or embedded script source.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptSourceError`] when any declared module cannot be loaded.
    pub fn from_source(source: &impl ScriptSource) -> Result<Self, ScriptSourceError> {
        let mut resolver = Self::new();
        for id in source.module_ids() {
            let asset = source.load(&id)?;
            // `ModuleId` has already been validated by the source.
            resolver.sources.insert(id, asset.source);
        }
        Ok(resolver)
    }

    /// Snapshot source while reusing transactionally compiled module ASTs.
    ///
    /// # Errors
    ///
    /// Returns source errors for declared modules that cannot load.
    pub fn from_source_with_cache(
        source: &impl ScriptSource,
        cache: &ModuleCompileCache,
    ) -> Result<Self, ScriptSourceError> {
        let mut resolver = Self::from_source(source)?;
        for id in source.module_ids() {
            let asset = source.load(&id)?;
            if cache.content_hash(&id) == Some(asset.content_hash)
                && let Some(ast) = cache.ast(&id)
            {
                resolver.compiled.insert(id, ast.clone());
            }
        }
        Ok(resolver)
    }

    /// Register source under a validated logical module identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ModuleIdError`] when `id` violates the resolver's path rules.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<(), ModuleIdError> {
        self.sources.insert(ModuleId::parse(id)?, source.into());
        Ok(())
    }

    #[must_use]
    pub fn contains(&self, id: &ModuleId) -> bool {
        self.sources.contains_key(id)
    }

    fn begin_resolution(
        &self,
        id: &ModuleId,
        position: Position,
    ) -> Result<(), Box<EvalAltResult>> {
        let mut stack = self.resolving.lock().map_err(|_| {
            Box::new(runtime_error(
                "module resolver lock is poisoned".to_owned(),
                position,
            ))
        })?;
        if let Some(cycle_start) = stack.iter().position(|active| active == id) {
            let mut cycle = stack[cycle_start..]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            cycle.push(id.to_string());
            return Err(Box::new(runtime_error(
                format!("cyclic module import: {}", cycle.join(" -> ")),
                position,
            )));
        }
        stack.push(id.clone());
        Ok(())
    }

    fn end_resolution(&self, id: &ModuleId) {
        if let Ok(mut stack) = self.resolving.lock() {
            if stack.last() == Some(id) {
                stack.pop();
            } else if let Some(index) = stack.iter().rposition(|active| active == id) {
                stack.remove(index);
            }
        }
    }
}

impl ModuleResolver for RestrictedModuleResolver {
    fn resolve(
        &self,
        engine: &Engine,
        _source: Option<&str>,
        path: &str,
        position: Position,
    ) -> Result<Shared<Module>, Box<EvalAltResult>> {
        let id = ModuleId::parse(path)
            .map_err(|error| Box::new(runtime_error(error.to_string(), position)))?;
        let source = self.sources.get(&id).ok_or_else(|| {
            Box::new(EvalAltResult::ErrorModuleNotFound(
                path.to_owned(),
                position,
            ))
        })?;

        self.begin_resolution(&id, position)?;
        let result = (|| {
            crate::extract_imports(source).map_err(|error| {
                Box::new(EvalAltResult::ErrorInModule(
                    path.to_owned(),
                    Box::new(runtime_error(error.to_string(), position)),
                    position,
                ))
            })?;
            let ast = if let Some(ast) = self.compiled.get(&id) {
                ast.clone()
            } else {
                let mut ast = engine.compile(source).map_err(|error| {
                    Box::new(EvalAltResult::ErrorInModule(
                        path.to_owned(),
                        error.into(),
                        position,
                    ))
                })?;
                crate::engine::validate_assignment_targets(&ast).map_err(|error| {
                    Box::new(EvalAltResult::ErrorInModule(
                        path.to_owned(),
                        Box::new(runtime_error(error.to_string(), position)),
                        position,
                    ))
                })?;
                ast.set_source(path);
                ast
            };
            validate_module_init(&id, &ast, position)?;
            Module::eval_ast_as_new(Scope::new(), &ast, engine)
                .map(Into::into)
                .map_err(|error| {
                    Box::new(EvalAltResult::ErrorInModule(
                        path.to_owned(),
                        error,
                        position,
                    ))
                })
        })();
        self.end_resolution(&id);
        result
    }
}

fn validate_module_init(
    id: &ModuleId,
    ast: &AST,
    import_position: Position,
) -> Result<(), Box<EvalAltResult>> {
    let mut definitions = 0_usize;
    for statement in ast.statements() {
        let accepted = match statement {
            Stmt::Noop(_) | Stmt::Import(..) | Stmt::Export(..) => true,
            Stmt::Var(variable, options, ..) => {
                options.contains(ASTFlags::CONSTANT) && variable.1.get_literal_value(None).is_some()
            }
            Stmt::FnCall(call, ..) if call.name == "define_component" && !call.is_qualified() => {
                definitions = definitions.saturating_add(1);
                call.args.len() == 1 && pure_component_definition(&call.args[0])
            }
            _ => false,
        };
        if !accepted {
            let position = statement.position();
            let location = if position.is_none() {
                String::new()
            } else {
                format!(" at {position}")
            };
            return Err(Box::new(runtime_error(
                format!(
                    "module `{id}` init must contain only imports, literal const values, exports, and one direct define_component declaration{location}"
                ),
                if position.is_none() {
                    import_position
                } else {
                    position
                },
            )));
        }
    }
    if definitions > 1 {
        return Err(Box::new(runtime_error(
            format!("module `{id}` declares {definitions} components; exactly one is allowed"),
            import_position,
        )));
    }
    Ok(())
}

fn pure_component_definition(expression: &Expr) -> bool {
    if expression.get_literal_value(None).is_some() {
        return true;
    }
    match expression {
        Expr::Array(values, ..) => values.iter().all(pure_component_definition),
        Expr::Map(entries, ..) => entries
            .0
            .iter()
            .all(|(_, value)| pure_component_definition(value)),
        Expr::FnCall(call, ..)
            if call.name == "Fn"
                && !call.is_qualified()
                && call.args.len() == 1
                && matches!(call.args[0], Expr::StringConstant(..)) =>
        {
            true
        }
        _ => false,
    }
}

fn runtime_error(message: String, position: Position) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(Dynamic::from(message), position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn module_ids_reject_escape_paths() {
        for invalid in [
            "",
            "/absolute",
            "../outside",
            "a/../b",
            "C:/ui",
            "a\\b",
            "a//b",
        ] {
            assert!(ModuleId::parse(invalid).is_err(), "accepted `{invalid}`");
        }
        assert_eq!(
            ModuleId::parse("components/button").unwrap().as_str(),
            "components/button"
        );
    }

    #[test]
    fn registered_modules_can_be_imported() {
        let mut resolver = RestrictedModuleResolver::new();
        resolver
            .insert("components/greeting", "fn greeting() { \"hello\" }")
            .unwrap();

        let mut engine = Engine::new();
        engine.set_module_resolver(resolver);
        let value: String = engine
            .eval(
                r#"
                    import "components/greeting" as greeting;
                    greeting::greeting()
                "#,
            )
            .unwrap();
        assert_eq!(value, "hello");
    }

    #[test]
    fn module_init_rejects_effectful_calls_before_evaluation() {
        let touched = Rc::new(Cell::new(false));
        let observer = Rc::clone(&touched);
        let mut resolver = RestrictedModuleResolver::new();
        resolver
            .insert(
                "components/effectful",
                "touch(); fn greeting() { \"hello\" }",
            )
            .unwrap();
        let mut engine = Engine::new();
        engine.register_fn("touch", move || observer.set(true));
        engine.set_module_resolver(resolver);
        let error = engine
            .eval::<Dynamic>("import \"components/effectful\" as effectful;")
            .unwrap_err();
        assert!(
            error.to_string().contains("init must contain only"),
            "{error}"
        );
        assert!(!touched.get());
    }

    #[test]
    fn module_init_accepts_literal_consts_and_direct_pure_definition() {
        let engine = Engine::new();
        let id = ModuleId::parse("components/pure").unwrap();
        let ast = engine
            .compile(
                r#"
                    const LIMITS = #{ min: 1, values: [2, 3] };
                    define_component(#{
                        metadata: #{ id: "components/pure" },
                        render: Fn("render_Pure")
                    });
                    fn render_Pure(ctx, props) { () }
                "#,
            )
            .unwrap();
        validate_module_init(&id, &ast, Position::NONE).unwrap();
    }

    #[test]
    fn module_init_rejects_mutable_globals_and_computed_definitions() {
        let engine = Engine::new();
        let id = ModuleId::parse("components/impure").unwrap();
        for source in [
            "let count = 0; fn read() { count }",
            "fn build() { #{} } define_component(build());",
            "define_component(#{}); define_component(#{});",
        ] {
            let ast = engine.compile(source).unwrap();
            assert!(validate_module_init(&id, &ast, Position::NONE).is_err());
        }
    }

    #[test]
    fn resolver_reuses_only_content_matching_cached_asts() {
        let id = ModuleId::parse("components/greeting").unwrap();
        let source = crate::EmbeddedScriptSource::new(BTreeMap::from([(
            id.clone(),
            "fn greeting() { \"hello\" }".to_owned(),
        )]));
        let engine = Engine::new();
        let mut cache = ModuleCompileCache::new();
        cache.refresh(&engine, &source, [id.clone()]).unwrap();
        let resolver = RestrictedModuleResolver::from_source_with_cache(&source, &cache).unwrap();
        assert!(resolver.compiled.contains_key(&id));

        let changed = crate::EmbeddedScriptSource::new(BTreeMap::from([(
            id.clone(),
            "fn greeting() { \"changed\" }".to_owned(),
        )]));
        let resolver = RestrictedModuleResolver::from_source_with_cache(&changed, &cache).unwrap();
        assert!(!resolver.compiled.contains_key(&id));
    }

    #[test]
    fn missing_modules_are_diagnostic_errors() {
        let mut engine = Engine::new();
        engine.set_module_resolver(RestrictedModuleResolver::new());
        let error = engine
            .eval::<Dynamic>("import \"components/missing\" as missing;")
            .unwrap_err();
        assert!(error.to_string().contains("components/missing"));
    }

    #[test]
    fn cyclic_imports_report_the_cycle() {
        let mut resolver = RestrictedModuleResolver::new();
        resolver
            .insert("components/a", "import \"components/b\" as b;")
            .unwrap();
        resolver
            .insert("components/b", "import \"components/a\" as a;")
            .unwrap();

        let mut engine = Engine::new();
        engine.set_module_resolver(resolver);
        let error = engine
            .eval::<Dynamic>("import \"components/a\" as a;")
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("cyclic module import"), "{message}");
        assert!(message.contains("components/a -> components/b -> components/a"));
    }
}
