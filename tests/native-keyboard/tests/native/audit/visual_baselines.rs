//! The committed macOS visual baselines match the capture tools' case list: every case is a
//! PNG under `tests/visual/macos/` that decodes and has its size, and no other PNG is there.

use std::collections::BTreeSet;
use std::path::Path;

#[allow(dead_code)]
#[path = "../../../src/baseline_cases.rs"]
mod baseline_cases;

/// Every `.png` under `directory`, as `/`-separated paths relative to `root`.
fn pngs(root: &Path, directory: &Path, found: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            pngs(root, &path, found);
        } else if path.extension().is_some_and(|extension| extension == "png") {
            let relative = path.strip_prefix(root).unwrap();
            let parts = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>();
            found.insert(parts.join("/"));
        }
    }
}

#[test]
fn every_case_is_a_png_of_its_size_and_no_other_png_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../visual/macos");
    let baselines = baseline_cases::baselines();
    let expected = baselines
        .iter()
        .map(|baseline| baseline.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        expected.len(),
        baselines.len(),
        "the case list repeats a file"
    );

    let mut problems = Vec::new();
    for baseline in &baselines {
        let path = root.join(&baseline.path);
        match image::open(&path) {
            Err(error) => problems.push(format!("{}: {error}", baseline.path)),
            Ok(image) => {
                let size = (image.width(), image.height());
                if size != baseline.size {
                    problems.push(format!(
                        "{} is {}x{}, expected {}x{}",
                        baseline.path, size.0, size.1, baseline.size.0, baseline.size.1
                    ));
                }
            }
        }
    }
    let mut found = BTreeSet::new();
    pngs(&root, &root, &mut found);
    for unexpected in found.difference(&expected) {
        problems.push(format!(
            "{unexpected} is not in the case list (src/baseline_cases.rs)"
        ));
    }
    assert!(
        problems.is_empty(),
        "{} visual baselines expected:\n{}",
        baselines.len(),
        problems.join("\n")
    );
}
