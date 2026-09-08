use gpui_rhai::{ModuleId, extract_imports};
use rhai::{ASTNode, Engine, Expr, OptimizationLevel, Stmt, Token};
use std::collections::BTreeSet;

#[derive(Default)]
struct Frame {
    braces: usize,
    needs_text: bool,
}
// Characterization prototype only. Not installed in production source.
fn stack_extract(source: &str) -> Result<BTreeSet<ModuleId>, String> {
    let engine = Engine::new_raw();
    let scripts = [source];
    let (tokens, control) = engine.lex(&scripts);
    let mut frames: Vec<Frame> = Vec::new();
    let mut imports = BTreeSet::new();
    let mut expects_module = false;
    for (token, position) in tokens {
        if expects_module {
            if let Token::StringConstant(module) = token {
                imports.insert(ModuleId::parse(module.to_string()).map_err(|e| e.to_string())?);
                expects_module = false;
                continue;
            }
            return Err(format!("nonliteral import at {position}"));
        }
        match &token {
            Token::InterpolatedString(_) => {
                if let Some(frame) = frames.last_mut().filter(|f| f.needs_text) {
                    frame.needs_text = false;
                    frame.braces = 0;
                } else {
                    frames.push(Frame::default());
                }
            }
            Token::StringConstant(_) if frames.last().is_some_and(|f| f.needs_text) => {
                frames.pop();
            }
            Token::LeftBrace => {
                if let Some(frame) = frames.last_mut() {
                    frame.braces += 1;
                }
            }
            Token::RightBrace => {
                if let Some(frame) = frames.last_mut() {
                    frame.braces = frame.braces.saturating_sub(1);
                    if frame.braces == 0 {
                        frame.needs_text = true;
                        control.borrow_mut().is_within_text = true;
                    }
                }
            }
            _ => {}
        }
        match token {
            Token::Import => expects_module = true,
            Token::LexError(e) => return Err(format!("{e} at {position}")),
            Token::EOF => break,
            _ => {}
        }
    }
    if expects_module {
        return Err("missing module".into());
    }
    Ok(imports)
}
fn ast_extract(source: &str) -> Result<BTreeSet<ModuleId>, String> {
    let mut engine = Engine::new_raw();
    engine.set_optimization_level(OptimizationLevel::None);
    engine.set_max_expr_depths(64, 32);
    let ast = engine.compile(source).map_err(|e| e.to_string())?;
    let mut imports = BTreeSet::new();
    let mut error = None;
    ast.walk(&mut |path| {
        if let Some(ASTNode::Stmt(Stmt::Import(import, _))) = path.last() {
            if let Expr::StringConstant(module, _) = &import.0 {
                match ModuleId::parse(module.to_string()) {
                    Ok(id) => {
                        imports.insert(id);
                    }
                    Err(e) => error = Some(e.to_string()),
                }
            } else {
                error = Some("nonliteral import".into());
            }
        }
        error.is_none()
    });
    error.map_or(Ok(imports), Err)
}
fn main() {
    let cases = [
        (
            "issue42",
            r#"let x = `a${if true { "" } else { `b${1}` }}`;"#,
            vec![],
        ),
        (
            "after",
            r#"let x = `a${`b${1}`}`; import "components/real" as r;"#,
            vec!["components/real"],
        ),
        ("multi", r#"let x = `a${`b${1}c${2}`}d${3}`;"#, vec![]),
        ("triple", r#"let x = `a${`b${`c${1}`}`}`;"#, vec![]),
        (
            "inner_import",
            r#"let x = `a${{ import "components/real" as r; `b${1}` }}`;"#,
            vec!["components/real"],
        ),
        ("fake", r#"let x = `a${`import "../fake" ${1}`}`;"#, vec![]),
        (
            "object",
            r#"let x = `a${#{x: `b${#{y: 1}.y}`}.x}`;"#,
            vec![],
        ),
        (
            "comment",
            r#"/* a /* b */ import "../fake" */ let x = `a${1}`;"#,
            vec![],
        ),
        (
            "dead_import",
            r#"if false { import "components/real" as r; }"#,
            vec!["components/real"],
        ),
    ];
    for (name, src, expected) in cases {
        let expected = expected
            .into_iter()
            .map(|id| ModuleId::parse(id).unwrap())
            .collect::<BTreeSet<_>>();
        let stack = stack_extract(src);
        let ast = ast_extract(src);
        assert_eq!(stack.as_ref().unwrap(), &expected, "stack {name}");
        assert_eq!(ast.as_ref().unwrap(), &expected, "ast {name}");
        println!(
            "{name}: current={:?}; stack=ok; unoptimized_ast=ok",
            extract_imports(src)
        );
    }
    for src in [
        r#"let name = "components/real"; import name as r;"#,
        r#"import "../bad" as r;"#,
    ] {
        assert!(stack_extract(src).is_err());
        assert!(ast_extract(src).is_err());
    }
    let root = std::env::var("GPUI_RHAI_ACCEPT_REPO").unwrap();
    let mut scanned = 0;
    for entry in std::fs::read_dir(format!("{root}/registry/components")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rhai") {
            let src = std::fs::read_to_string(&path).unwrap();
            let current = extract_imports(&src).unwrap();
            assert_eq!(stack_extract(&src).unwrap(), current, "{}", path.display());
            assert_eq!(ast_extract(&src).unwrap(), current, "{}", path.display());
            scanned += 1;
        }
    }
    println!(
        "reference strategies agree with all {scanned} current registry components; literal-only negative cases rejected"
    );
}
