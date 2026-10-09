//! Documentation of the native functions the runtime registers for scripts.
//!
//! Rhai's function metadata names each function and its parameter types but
//! carries no parameter names or comments, and its definitions printer cannot
//! read the return type of a fallible function. This table supplies one entry
//! per registered signature, keyed by Rhai's own signature string. The script
//! API reference and the language-server definitions are built from it; a test
//! requires an entry for every registered signature and rejects entries whose
//! signature no longer exists.

#[cfg(feature = "charts")]
mod charts;
mod context;
mod global;
mod node;
mod primitives;
mod style;
mod values;

use std::collections::BTreeSet;
use std::fmt::Write as _;

use rhai::Engine;
use serde_json::Value;

/// The documentation of one registered signature.
#[derive(Clone, Copy, Debug)]
pub struct ScriptFnDoc {
    /// The signature as `Engine::gen_fn_metadata_to_json` reports it.
    pub signature: &'static str,
    /// Names of the explicit parameters, after the receiver of a method.
    pub params: &'static [&'static str],
    /// One or two sentences.
    pub doc: &'static str,
}

/// Every documented signature of this build.
pub fn script_fn_docs() -> impl Iterator<Item = &'static ScriptFnDoc> {
    let tables: [&[ScriptFnDoc]; 6] = [
        style::DOCS,
        node::DOCS,
        context::DOCS,
        global::DOCS,
        values::DOCS,
        primitives::DOCS,
    ];
    let docs = tables.into_iter().flatten();
    #[cfg(feature = "charts")]
    let docs = docs.chain(charts::DOCS);
    docs
}

/// One parameter of a native function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptParam {
    pub name: String,
    /// A script-facing type name: `int`, `String`, `Style`, `?` for any value.
    pub type_name: String,
}

/// A native function as scripts see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptFn {
    /// The static module that holds it (`gpui_rhai` for primitives), or `None`.
    pub module: Option<String>,
    /// The receiver type of a method, when the first parameter is `&mut`.
    pub receiver: Option<String>,
    /// Rhai's name: `padding_x`, `get$revision` for a property getter, `*` for an operator.
    pub name: String,
    /// Every parameter, the receiver first for a method.
    pub params: Vec<ScriptParam>,
    pub return_type: String,
    pub doc: Option<&'static str>,
    pub signature: String,
}

impl ScriptFn {
    /// An operator such as `*` rather than a named function.
    #[must_use]
    pub fn is_operator(&self) -> bool {
        !self.name.contains('$')
            && !self
                .name
                .chars()
                .next()
                .is_some_and(|first| first.is_alphabetic() || first == '_')
    }

    /// How a script calls it: `style.padding_x(value: Length) -> Style`,
    /// `node.revision -> int`, `text(content: String) -> UiNode`.
    #[must_use]
    pub fn display(&self) -> String {
        let explicit = if self.receiver.is_some() {
            &self.params[1..]
        } else {
            &self.params[..]
        };
        let args = explicit
            .iter()
            .map(|param| format!("{}: {}", param.name, param.type_name))
            .collect::<Vec<_>>()
            .join(", ");
        let returns = if self.return_type == "()" {
            String::new()
        } else {
            format!(" -> {}", self.return_type)
        };
        let prefix = match (&self.module, &self.receiver) {
            (Some(module), _) => format!("{module}::"),
            (None, Some(receiver)) => format!("{}.", receiver_variable(receiver)),
            (None, None) => String::new(),
        };
        if let Some(property) = self.name.strip_prefix("get$") {
            format!("{prefix}{property}{returns}")
        } else if let Some(property) = self.name.strip_prefix("set$") {
            format!("{prefix}{property} = {args}")
        } else if self.is_operator() {
            let types = self
                .params
                .iter()
                .map(|param| param.type_name.as_str())
                .collect::<Vec<_>>();
            format!(
                "{} {} {}{returns}",
                types[0],
                self.name,
                types.get(1).unwrap_or(&"")
            )
        } else {
            format!("{prefix}{}({args}){returns}", self.name)
        }
    }
}

/// The variable a reader writes for a receiver type: `style`, `node`, `ctx`.
#[must_use]
pub fn receiver_variable(receiver: &str) -> String {
    match receiver {
        "UiContext" => "ctx".to_owned(),
        "UiNode" => "node".to_owned(),
        other => {
            let mut name = String::new();
            for (index, character) in other.chars().enumerate() {
                if character.is_uppercase() && index > 0 {
                    name.push('_');
                }
                name.extend(character.to_lowercase());
            }
            name
        }
    }
}

/// The native functions of an engine joined with their documentation.
#[derive(Clone, Debug, Default)]
pub struct ScriptApi {
    pub functions: Vec<ScriptFn>,
    /// Documented signatures the engine no longer registers.
    pub stale: Vec<&'static str>,
}

impl ScriptApi {
    /// Read an engine's registered native functions (standard packages excluded).
    ///
    /// # Errors
    ///
    /// Returns an error when Rhai's metadata cannot be serialized or read.
    pub fn from_engine(engine: &Engine) -> Result<Self, serde_json::Error> {
        let metadata: Value = serde_json::from_str(&engine.gen_fn_metadata_to_json(false)?)?;
        let docs = script_fn_docs()
            .map(|doc| (doc.signature, doc))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut functions = Vec::new();
        let mut seen = BTreeSet::new();
        let mut read = |entries: Option<&Vec<Value>>, module: Option<&str>| {
            for entry in entries.into_iter().flatten() {
                let signature = entry["signature"].as_str().unwrap_or_default().to_owned();
                let name = entry["name"].as_str().unwrap_or_default().to_owned();
                let raw_params = entry["params"].as_array().cloned().unwrap_or_default();
                let receiver = raw_params
                    .first()
                    .and_then(|param| param["type"].as_str())
                    .and_then(|kind| kind.strip_prefix("&mut "))
                    .map(|kind| type_name(kind.trim()));
                let doc = docs.get(signature.as_str()).copied();
                let names = doc.map_or(&[][..], |doc| doc.params);
                let offset = usize::from(receiver.is_some());
                let params = raw_params
                    .iter()
                    .enumerate()
                    .map(|(index, param)| ScriptParam {
                        name: if index < offset {
                            "this".to_owned()
                        } else {
                            names
                                .get(index - offset)
                                .filter(|name| !name.is_empty())
                                .map_or_else(|| "_".to_owned(), |name| (*name).to_owned())
                        },
                        type_name: param["type"]
                            .as_str()
                            .map_or_else(|| "?".to_owned(), type_name),
                    })
                    .collect();
                seen.insert(signature.clone());
                functions.push(ScriptFn {
                    module: module.map(str::to_owned),
                    receiver,
                    name,
                    params,
                    return_type: entry["returnType"]
                        .as_str()
                        .map_or_else(|| "()".to_owned(), type_name),
                    doc: doc.map(|doc| doc.doc).filter(|doc| !doc.is_empty()),
                    signature,
                });
            }
        };
        read(metadata["functions"].as_array(), None);
        if let Some(modules) = metadata["modules"].as_object() {
            for (module, contents) in modules {
                read(contents["functions"].as_array(), Some(module));
            }
        }
        functions.sort_by(|left, right| {
            (&left.module, &left.receiver, &left.name, &left.signature).cmp(&(
                &right.module,
                &right.receiver,
                &right.name,
                &right.signature,
            ))
        });
        let stale = docs
            .keys()
            .filter(|signature| !seen.contains(**signature))
            .copied()
            .collect();
        Ok(Self { functions, stale })
    }

    /// Signatures without a documentation entry, or with an empty doc or a
    /// missing parameter name.
    #[must_use]
    pub fn undocumented(&self) -> Vec<&str> {
        let documented = script_fn_docs()
            .filter(|doc| !doc.doc.is_empty() && doc.params.iter().all(|name| !name.is_empty()))
            .map(|doc| doc.signature)
            .collect::<BTreeSet<_>>();
        self.functions
            .iter()
            .filter(|function| !documented.contains(function.signature.as_str()))
            .map(|function| function.signature.as_str())
            .collect()
    }

    /// Rhai language-server definitions in the official `.d.rhai` format, with
    /// parameter names, doc comments and readable return types.
    #[must_use]
    pub fn definitions(&self) -> String {
        let mut global = String::new();
        let mut modules = std::collections::BTreeMap::<&str, String>::new();
        for function in &self.functions {
            let target = match function.module.as_deref() {
                Some(module) => modules.entry(module).or_default(),
                None => &mut global,
            };
            if !target.is_empty() {
                target.push('\n');
            }
            if let Some(doc) = function.doc {
                for line in wrap(doc, 96) {
                    let _ = writeln!(target, "/// {line}");
                }
            }
            let _ = writeln!(target, "{}", definition_line(function));
        }
        let mut file = format!("module static;\n\n{global}");
        for (module, body) in modules {
            let indented = body
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        String::new()
                    } else {
                        format!("    {line}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = write!(file, "\nmodule {module} {{\n{indented}\n}}\n");
        }
        file
    }
}

fn definition_line(function: &ScriptFn) -> String {
    let operator = function.is_operator();
    let params = function
        .params
        .iter()
        .map(|param| {
            if operator {
                param.type_name.clone()
            } else {
                format!("{}: {}", param.name, param.type_name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let keyword = if operator { "op" } else { "fn" };
    let name = if let Some(property) = function.name.strip_prefix("get$") {
        format!("get {property}")
    } else if let Some(property) = function.name.strip_prefix("set$") {
        format!("set {property}")
    } else {
        function.name.clone()
    };
    format!("{keyword} {name}({params}) -> {};", function.return_type)
}

/// A script-facing type name for a Rust type path from Rhai's metadata.
#[must_use]
pub fn type_name(raw: &str) -> String {
    let raw = raw.trim();
    let raw = raw.strip_prefix("&mut ").unwrap_or(raw).trim();
    let raw = raw.strip_prefix('&').unwrap_or(raw).trim();
    // A fallible function returns `Result<T, Box<EvalAltResult>>`: scripts see `T`.
    if let Some(inner) = raw
        .strip_prefix("core::result::Result<")
        .or_else(|| raw.strip_prefix("Result<"))
        .or_else(|| raw.strip_prefix("RhaiResultOf<"))
        .and_then(|inner| inner.strip_suffix('>'))
    {
        return type_name(first_generic(inner));
    }
    if let Some(inner) = raw
        .strip_prefix("core::option::Option<")
        .or_else(|| raw.strip_prefix("Option<"))
        .and_then(|inner| inner.strip_suffix('>'))
    {
        return format!("{}?", type_name(inner));
    }
    let last = raw.rsplit("::").next().unwrap_or(raw);
    match last {
        "" | "_" | "?" | "Dynamic" => "?".to_owned(),
        "i64" | "INT" | "i32" | "u32" | "usize" | "u64" => "int".to_owned(),
        "f64" | "FLOAT" | "f32" => "float".to_owned(),
        "str" | "string" | "String" | "ImmutableString" => "String".to_owned(),
        "array" | "Array" | "Vec<Dynamic>" => "Array".to_owned(),
        "map" | "Map" => "Map".to_owned(),
        "FnPtr" | "Fn" => "FnPtr".to_owned(),
        "bool" => "bool".to_owned(),
        "()" => "()".to_owned(),
        other => other.to_owned(),
    }
}

/// The first generic argument of `T, U` at nesting depth zero.
fn first_generic(arguments: &str) -> &str {
    let mut depth = 0usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return arguments[..index].trim(),
            _ => {}
        }
    }
    arguments.trim()
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_types_read_as_script_types() {
        assert_eq!(
            type_name(
                "core::result::Result<gpui_rhai::style::Length,alloc::boxed::Box<rhai::types::error::EvalAltResult>>"
            ),
            "Length"
        );
        assert_eq!(type_name("&mut Style"), "Style");
        assert_eq!(type_name("types::dynamic::Dynamic"), "?");
        assert_eq!(type_name("i64"), "int");
        assert_eq!(type_name("string"), "String");
        assert_eq!(
            type_name(
                "core::result::Result<(),alloc::boxed::Box<rhai::types::error::EvalAltResult>>"
            ),
            "()"
        );
    }

    #[test]
    fn every_builtin_primitive_documents_its_props_and_events() {
        let runtime = crate::RuntimeEngine::new();
        let mut missing = Vec::new();
        for descriptor in runtime.primitive_registry().descriptors() {
            let id = descriptor.id.as_str().to_owned();
            for (name, field) in &descriptor.props {
                if field.doc.is_none() {
                    missing.push(format!("{id}: prop `{name}`"));
                }
            }
            for (name, event) in &descriptor.events {
                if event.doc.is_none() {
                    missing.push(format!("{id}: event `{name}`"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "{} primitive props and events lack a `doc`:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    #[test]
    fn every_registered_native_function_is_documented() {
        let runtime = crate::RuntimeEngine::new();
        let api = ScriptApi::from_engine(runtime.engine()).unwrap();
        assert!(api.stale.is_empty(), "stale script docs: {:#?}", api.stale);
        let undocumented = api.undocumented();
        assert!(
            undocumented.is_empty(),
            "{} native functions lack a doc or parameter names (crates/gpui-rhai/src/script_docs):\n{}",
            undocumented.len(),
            undocumented.join("\n")
        );
    }
}
