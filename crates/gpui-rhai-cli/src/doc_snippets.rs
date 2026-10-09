//! Runs the Rhai snippets the documentation marks for checking.
//!
//! A snippet is a ```` ```rhai ```` block preceded by a marker line:
//!
//! - `<!-- check: pass -->`: a complete entry script that passes `gpui-rhai check`
//!   (known calls, then one headless first frame) without warnings;
//! - `<!-- check: error "text" -->`: `check` fails with a message containing `text`;
//! - `<!-- check: warning "text" -->`: `check` passes with a warning containing `text`.
//!
//! Each snippet runs in a fresh project with the official modules it imports.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{BundledRegistry, Project};

#[derive(Debug)]
struct Snippet {
    file: PathBuf,
    line: usize,
    expectation: Expectation,
    source: String,
}

#[derive(Debug)]
enum Expectation {
    Pass,
    Error(String),
    Warning(String),
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn documentation_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![root.join("README.md"), root.join("USER_GUIDE.md")];
    let mut pending = vec![root.join("docs")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn parse_marker(line: &str) -> Option<Expectation> {
    let body = line
        .trim()
        .strip_prefix("<!-- check:")?
        .strip_suffix("-->")?
        .trim();
    let quoted = |rest: &str| rest.trim().trim_matches('"').to_owned();
    if body == "pass" {
        Some(Expectation::Pass)
    } else if let Some(rest) = body.strip_prefix("error") {
        Some(Expectation::Error(quoted(rest)))
    } else {
        body.strip_prefix("warning")
            .map(|rest| Expectation::Warning(quoted(rest)))
    }
}

fn snippets(file: &Path) -> Vec<Snippet> {
    let text = fs::read_to_string(file).unwrap();
    let lines = text.lines().collect::<Vec<_>>();
    let mut found = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(expectation) = parse_marker(lines[index]) {
            let fence = index + 1;
            assert!(
                lines
                    .get(fence)
                    .is_some_and(|line| line.trim() == "```rhai"),
                "{}:{}: a check marker must be followed by a ```rhai block",
                file.display(),
                index + 1
            );
            let end = (fence + 1..lines.len())
                .find(|&line| lines[line].trim() == "```")
                .unwrap_or_else(|| panic!("{}:{}: unclosed block", file.display(), fence + 1));
            found.push(Snippet {
                file: file.to_owned(),
                line: fence + 2,
                expectation,
                source: lines[fence + 1..end].join("\n") + "\n",
            });
            index = end;
        }
        index += 1;
    }
    found
}

/// The official modules a snippet imports, as `add` arguments.
fn imports(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import \""))
        .filter_map(|rest| rest.split('"').next())
        .map(|id| id.rsplit('/').next().unwrap_or(id).to_owned())
        .collect()
}

/// Run one snippet; `Err` describes how the outcome differed from the marker.
fn run(snippet: &Snippet, registry: &BundledRegistry) -> Result<(), String> {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir_all(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname = \"snippet\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n",
    )
    .unwrap();
    let project = Project::new(directory.path());
    project.plan_init().unwrap().apply().unwrap();
    let modules = imports(&snippet.source);
    if !modules.is_empty() {
        project
            .plan_add(registry, &modules)
            .map_err(|error| format!("could not add {modules:?}: {error}"))?
            .apply()
            .unwrap();
    }
    fs::write(directory.path().join("ui/main.rhai"), &snippet.source).unwrap();
    let outcome = project.check();
    match (&snippet.expectation, outcome) {
        (Expectation::Pass, Ok(report)) if report.warnings.is_empty() => Ok(()),
        (Expectation::Pass, Ok(report)) => {
            Err(format!("passed with warnings {:?}", report.warnings))
        }
        (Expectation::Pass | Expectation::Warning(_), Err(error)) => {
            Err(format!("failed: {error}"))
        }
        (Expectation::Error(text), Err(error)) if error.to_string().contains(text.as_str()) => {
            Ok(())
        }
        (Expectation::Error(text), Err(error)) => Err(format!("failed without {text:?}: {error}")),
        (Expectation::Error(text), Ok(_)) => {
            Err(format!("passed; expected an error with {text:?}"))
        }
        (Expectation::Warning(text), Ok(report))
            if report
                .warnings
                .iter()
                .any(|warning| warning.contains(text.as_str())) =>
        {
            Ok(())
        }
        (Expectation::Warning(text), Ok(report)) => Err(format!(
            "passed without a warning containing {text:?}: {:?}",
            report.warnings
        )),
    }
}

#[test]
fn marked_documentation_snippets_behave_as_documented() {
    let root = repository_root();
    let registry = BundledRegistry::load().unwrap();
    let all = documentation_files(&root)
        .iter()
        .flat_map(|file| snippets(file))
        .collect::<Vec<_>>();
    assert!(!all.is_empty(), "no marked snippets found");
    let failures = all
        .iter()
        .filter_map(|snippet| {
            run(snippet, &registry).err().map(|reason| {
                format!(
                    "{}:{}: {reason}",
                    snippet
                        .file
                        .strip_prefix(&root)
                        .unwrap_or(&snippet.file)
                        .display(),
                    snippet.line
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "{} of {} documentation snippets differ from their markers:\n{}",
        failures.len(),
        all.len(),
        failures.join("\n")
    );
}

#[test]
fn markers_parse() {
    assert!(matches!(
        parse_marker("<!-- check: pass -->"),
        Some(Expectation::Pass)
    ));
    assert!(matches!(
        parse_marker("<!-- check: error \"Variable not found\" -->"),
        Some(Expectation::Error(text)) if text == "Variable not found"
    ));
    assert!(parse_marker("<!-- not a check -->").is_none());
    assert_eq!(
        imports("import \"components/button\" as button;\nimport \"layouts/region\" as r;"),
        vec!["button".to_owned(), "region".to_owned()]
    );
}
