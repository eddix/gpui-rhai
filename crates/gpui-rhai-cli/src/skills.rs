//! Agent skills built from the documentation, the generated reference and the
//! tested examples.
//!
//! Two skills, in the layout `npx skills` and agents read
//! (`<name>/SKILL.md` plus `references/`): `gpui-rhai` (building with the
//! runtime and the official modules) and `gpui-rhai-design` (the design rules).
//! Each `SKILL.md` is written by hand; every reference file is generated:
//! documents are copied with their links rewritten (to the copy inside the skill
//! when it is there, otherwise to the repository on GitHub), the module and
//! script API references are rendered, and the recipes are the Rhai views of
//! the tested examples. The registry crate bundles the result for
//! `gpui-rhai skills` (`BUNDLED_SKILL_FILES`); the repository keeps an
//! identical copy in `skills/` for `npx skills add`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use crate::{BundledRegistry, ProjectError};

/// Where links to files outside a skill point.
const REPOSITORY_URL: &str = "https://github.com/eddix/gpui-rhai/blob/main";

/// Documents copied into a skill: (repository path, path inside the skill).
const COPIED: &[(&str, &str, &str)] = &[
    ("gpui-rhai", "USER_GUIDE.md", "references/user-guide.md"),
    ("gpui-rhai", "docs/rhai.md", "references/rhai.md"),
    (
        "gpui-rhai",
        "docs/component-authoring-guide.md",
        "references/component-authoring.md",
    ),
    ("gpui-rhai", "docs/embedding.md", "references/embedding.md"),
    (
        "gpui-rhai-design",
        "docs/design/principles.md",
        "references/principles.md",
    ),
    (
        "gpui-rhai-design",
        "docs/design/atoms.md",
        "references/atoms.md",
    ),
    (
        "gpui-rhai-design",
        "docs/design/composition.md",
        "references/composition.md",
    ),
    (
        "gpui-rhai-design",
        "docs/design/themes.md",
        "references/themes.md",
    ),
    (
        "gpui-rhai-design",
        "docs/design/decisions.md",
        "references/decisions.md",
    ),
];

/// Recipes: complete, tested views from the examples, (title, example, note).
const RECIPES: &[(&str, &str, &str)] = &[
    (
        "Settings panel",
        "settings_panel",
        "A Region with a FormLayout of settings: Switch, RadioGroup, ToggleGroup and a \
         searchable Combobox, plus an Accordion and a Popover.",
    ),
    (
        "Form with validation, dialog and toast",
        "form_showcase",
        "A FormLayout of Input, Textarea, Select, DatePicker, Checkbox, RadioGroup and \
         Switch fields, validated on submit, with a Dialog and a Toast.",
    ),
    (
        "Data view with a table",
        "data_table",
        "A DataView around a Table with a filter, sorting, selection, paging, and \
         loading and empty states.",
    ),
    (
        "Dashboard",
        "dashboard_layout",
        "Sections of Stat figures, DescriptionLists, Tabs, Tags and Progress.",
    ),
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Normalize `a/b/../c` without touching the file system.
fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<Component<'_>> = Vec::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir if matches!(parts.last(), Some(Component::Normal(_))) => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

/// The relative path from directory `from` to file `to`, both inside one tree.
fn relative(from: &Path, to: &Path) -> String {
    let from = from.components().collect::<Vec<_>>();
    let to_parts = to.components().collect::<Vec<_>>();
    let common = from
        .iter()
        .zip(&to_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut path = PathBuf::new();
    for _ in common..from.len() {
        path.push("..");
    }
    for part in &to_parts[common..] {
        path.push(part);
    }
    path.to_string_lossy().replace('\\', "/")
}

/// Rewrite the relative Markdown links of a document copied from `source`
/// (a repository path) to `destination` (a path inside the skills tree), given
/// where each repository file landed in that tree.
fn rewrite_links(
    text: &str,
    source: &str,
    destination: &Path,
    placed: &BTreeMap<PathBuf, PathBuf>,
) -> String {
    let source_directory = Path::new(source).parent().unwrap_or(Path::new(""));
    let destination_directory = destination.parent().unwrap_or(Path::new(""));
    let mut out = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        // Leave code blocks alone.
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if in_fence || line.trim_start().starts_with("```") {
            out.push_str(line);
            continue;
        }
        let mut cursor = 0;
        while let Some(start) = line[cursor..].find("](") {
            let open = cursor + start + 2;
            let Some(length) = line[open..].find(')') else {
                break;
            };
            let target = &line[open..open + length];
            out.push_str(&line[cursor..open]);
            out.push_str(&rewrite_target(
                target,
                source_directory,
                destination_directory,
                placed,
            ));
            cursor = open + length;
        }
        out.push_str(&line[cursor..]);
    }
    out
}

fn rewrite_target(
    target: &str,
    source_directory: &Path,
    destination_directory: &Path,
    placed: &BTreeMap<PathBuf, PathBuf>,
) -> String {
    if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
        return target.to_owned();
    }
    let (path, anchor) = target
        .split_once('#')
        .map_or((target, None), |(path, anchor)| (path, Some(anchor)));
    let repository_path = normalize(&source_directory.join(path));
    let anchor = anchor
        .map(|anchor| format!("#{anchor}"))
        .unwrap_or_default();
    if let Some(copy) = placed.get(&repository_path) {
        return format!("{}{anchor}", relative(destination_directory, copy));
    }
    let kind = if path.ends_with('/') || Path::new(path).extension().is_none() {
        "tree"
    } else {
        "blob"
    };
    let url = REPOSITORY_URL.replace("/blob/", &format!("/{kind}/"));
    format!(
        "{url}/{}{anchor}",
        repository_path.to_string_lossy().replace('\\', "/")
    )
}

/// The Rhai view an example embeds as `const MAIN: &str = r#"..."#;`.
fn example_view(root: &Path, example: &str) -> Result<String, ProjectError> {
    let path = root.join(format!("crates/gpui-rhai/examples/{example}.rs"));
    let source = crate::read(&path)?;
    let start = source
        .find("const MAIN: &str = r#\"")
        .map(|index| index + "const MAIN: &str = r#\"".len())
        .ok_or_else(|| ProjectError::MissingExampleView(path.clone()))?;
    let end = source[start..]
        .find("\"#;")
        .ok_or_else(|| ProjectError::MissingExampleView(path.clone()))?;
    Ok(source[start..start + end]
        .trim_start_matches('\n')
        .to_owned())
}

fn recipes(root: &Path) -> Result<String, ProjectError> {
    let mut page = String::from(
        "# Recipes\n\n\
         Complete views copied from the examples in `crates/gpui-rhai/examples/`, which\n\
         are built, rendered into checked-in baselines and audited by the test suite.\n\
         Start from the closest one; the module reference has every prop they use.\n",
    );
    for (title, example, note) in RECIPES {
        let view = example_view(root, example)?;
        let _ = write!(
            page,
            "\n## {title}\n\n{note} From [`{example}.rs`]({REPOSITORY_URL}/crates/gpui-rhai/examples/{example}.rs).\n\n```rhai\n{view}```\n"
        );
    }
    Ok(page)
}

/// The two skills, by directory name.
#[cfg(test)]
const SKILLS: [&str; 2] = ["gpui-rhai", "gpui-rhai-design"];

/// The registry crate's `src/skills.rs`, which bundles `files` (paths below
/// `registry/skills`).
#[cfg(test)]
fn bundle_source(files: &[String]) -> String {
    let mut source = String::from(
        "//! The agent skills `gpui-rhai skills` writes into a project, from this\n\
         //! crate's `skills/`. Generated by the `skills_are_current` test of\n\
         //! `gpui-rhai-cli`; do not edit. Run\n\
         //! `GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli` instead.\n\n\
         /// Every skill file: (path below a skills directory, contents).\n\
         #[rustfmt::skip]\n\
         pub const BUNDLED_SKILL_FILES: &[(&str, &str)] = &[\n",
    );
    for file in files {
        let _ = writeln!(
            source,
            "    (\"{file}\", include_str!(\"../skills/{file}\")),"
        );
    }
    source.push_str("];\n");
    source
}

/// Every generated skill file, keyed by its path below a skills directory
/// (`gpui-rhai/references/rhai.md`). `SKILL.md` files are not generated.
///
/// # Errors
///
/// Returns read, compile or export errors.
pub fn skill_files(registry: &BundledRegistry) -> Result<BTreeMap<PathBuf, String>, ProjectError> {
    let root = repository_root();
    let reference = crate::reference::reference_documents(registry)?;
    // Where each repository file lands inside the skills tree.
    let mut placed = BTreeMap::new();
    for (skill, source, destination) in COPIED {
        placed.insert(PathBuf::from(source), Path::new(skill).join(destination));
    }
    for path in reference.keys() {
        let target = if path == Path::new("script-api.md") {
            PathBuf::from("gpui-rhai/references/script-api.md")
        } else {
            Path::new("gpui-rhai/references/modules").join(path)
        };
        placed.insert(Path::new("docs/reference").join(path), target);
    }
    let mut files = BTreeMap::new();
    for (skill, source, destination) in COPIED {
        let text = crate::read(&root.join(source))?;
        let destination = Path::new(skill).join(destination);
        files.insert(
            destination.clone(),
            rewrite_links(&text, source, &destination, &placed),
        );
    }
    for (path, text) in &reference {
        let source = format!("docs/reference/{}", path.to_string_lossy());
        let destination = placed[&PathBuf::from(&source)].clone();
        files.insert(
            destination.clone(),
            rewrite_links(text, &source, &destination, &placed),
        );
    }
    files.insert(
        PathBuf::from("gpui-rhai/references/recipes.md"),
        recipes(&root)?,
    );
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_point_into_the_skill_or_at_the_repository() {
        let placed = BTreeMap::from([
            (
                PathBuf::from("docs/design/atoms.md"),
                PathBuf::from("gpui-rhai-design/references/atoms.md"),
            ),
            (
                PathBuf::from("docs/design/themes.md"),
                PathBuf::from("gpui-rhai-design/references/themes.md"),
            ),
        ]);
        let text = "See [atoms](atoms.md#5-markers), [themes](themes.md), \
            [embedding](../embedding.md) and [site](https://rhai.rs).\n\
            ```rhai\nlet x = [a](b);\n```\n";
        let rewritten = rewrite_links(
            text,
            "docs/design/composition.md",
            Path::new("gpui-rhai-design/references/composition.md"),
            &placed,
        );
        assert!(
            rewritten.contains("[atoms](atoms.md#5-markers)"),
            "{rewritten}"
        );
        assert!(rewritten.contains("[themes](themes.md)"), "{rewritten}");
        assert!(
            rewritten.contains(&format!("[embedding]({REPOSITORY_URL}/docs/embedding.md)")),
            "{rewritten}"
        );
        assert!(rewritten.contains("[site](https://rhai.rs)"));
        assert!(
            rewritten.contains("let x = [a](b);"),
            "code blocks are untouched"
        );
    }

    /// The bundled copy in the registry crate and the repository's `skills/`.
    fn skill_directories() -> [PathBuf; 2] {
        let root = repository_root();
        [root.join("registry/skills"), root.join("skills")]
    }

    /// Every file below `directory`, as `/`-separated relative paths.
    fn files_below(directory: &Path) -> Vec<String> {
        let mut files = Vec::new();
        let mut pending = vec![directory.to_path_buf()];
        while let Some(current) = pending.pop() {
            for entry in std::fs::read_dir(&current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.push(relative(directory, &path));
                }
            }
        }
        files.sort();
        files
    }

    #[test]
    fn skills_are_current() {
        let generated = skill_files(&BundledRegistry::load().unwrap()).unwrap();
        let mut expected = generated
            .keys()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .chain(SKILLS.iter().map(|skill| format!("{skill}/SKILL.md")))
            .collect::<Vec<_>>();
        expected.sort();
        let bundle = repository_root().join("registry/src/skills.rs");
        let update = std::env::var_os("GPUI_RHAI_UPDATE_REFERENCE").is_some();
        let mut stale = Vec::new();
        for directory in skill_directories() {
            if update {
                for skill in SKILLS {
                    let references = directory.join(skill).join("references");
                    if references.exists() {
                        std::fs::remove_dir_all(&references).unwrap();
                    }
                    // `skills/<name>/SKILL.md` is written by hand; the bundle copies it.
                    let source = repository_root()
                        .join("skills")
                        .join(skill)
                        .join("SKILL.md");
                    let copy = directory.join(skill).join("SKILL.md");
                    if copy != source {
                        std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
                        std::fs::copy(&source, &copy).unwrap();
                    }
                }
                for (path, text) in &generated {
                    let path = directory.join(path);
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, text).unwrap();
                }
                continue;
            }
            for (path, text) in &generated {
                match std::fs::read_to_string(directory.join(path)) {
                    Ok(current) if &current == text => {}
                    _ => stale.push(directory.join(path).display().to_string()),
                }
            }
            let present = files_below(&directory);
            for file in present.iter().filter(|file| !expected.contains(file)) {
                stale.push(format!(
                    "{} (not generated)",
                    directory.join(file).display()
                ));
            }
        }
        if update {
            std::fs::write(&bundle, bundle_source(&expected)).unwrap();
        } else if crate::read(&bundle).ok().as_deref() != Some(bundle_source(&expected).as_str()) {
            stale.push(bundle.display().to_string());
        }
        assert!(
            stale.is_empty(),
            "skills are stale ({} files): {stale:?}; run \
             GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli",
            stale.len()
        );
    }

    #[test]
    fn both_skill_copies_keep_the_same_skill_md() {
        let [bundled, repository] = skill_directories();
        for skill in SKILLS {
            let bundled = std::fs::read_to_string(bundled.join(skill).join("SKILL.md")).unwrap();
            let repository =
                std::fs::read_to_string(repository.join(skill).join("SKILL.md")).unwrap();
            assert_eq!(
                bundled, repository,
                "{skill}/SKILL.md differs between the copies"
            );
        }
    }
}
