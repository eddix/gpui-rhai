//! Typeahead filtering: which parts of a tree match a name — port of
//! tobi/disktree filter.rs.
//!
//! A node whose name contains the needle (ignoring ASCII case) matches, and
//! is kept whole: everything under a matching directory goes with it. Its
//! ancestors are kept only partly, and sized by what matched beneath them,
//! so a filtered treemap shows exactly the matches, at their true relative
//! sizes, in the places they live.

use rustc_hash::FxHashMap;

use super::node::{Metric, Node};

/// How a node takes part in a filtered view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// It matches: drawn as usual, with everything beneath it.
    Whole,
    /// It holds matches: drawn with only those, at their size.
    Partial { bytes: u64, files: u64 },
}

/// The outcome of filtering a subtree by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Matches {
    /// The needle, lowercased.
    pub needle: String,
    /// Absolute crumbs of the subtree that was searched.
    pub base: Vec<usize>,
    /// Keyed by absolute crumbs. Only matches and their ancestors appear;
    /// a match's own descendants are implied.
    keep: FxHashMap<Vec<usize>, Keep>,
    /// Topmost matches: a match inside a match is not counted again.
    pub count: usize,
    pub bytes: u64,
    pub files: u64,
}

impl Matches {
    /// How the node at `crumbs` takes part: `None` when it is filtered out.
    /// Anything outside the searched subtree, and anything beneath a match,
    /// is kept whole.
    pub fn keep(&self, crumbs: &[usize]) -> Option<Keep> {
        if !crumbs.starts_with(&self.base) {
            return Some(Keep::Whole);
        }
        for length in self.base.len()..=crumbs.len() {
            match self.keep.get(&crumbs[..length]) {
                Some(Keep::Whole) => return Some(Keep::Whole),
                Some(partial) if length == crumbs.len() => {
                    return Some(*partial);
                }
                // Only the base may be absent: it holds the matches but is
                // not recorded, being where the search started.
                None if length > self.base.len() => return None,
                _ => {}
            }
        }
        // The base itself holds the matches.
        Some(Keep::Partial {
            bytes: self.bytes,
            files: self.files,
        })
    }

    /// The value a kept node is laid out by.
    #[must_use]
    pub const fn value(keep: Keep, node: &Node, metric: Metric) -> u64 {
        match (keep, metric) {
            (Keep::Whole, _) => node.value(metric),
            (Keep::Partial { bytes, .. }, Metric::Bytes) => bytes,
            (Keep::Partial { files, .. }, Metric::Files) => files,
        }
    }
}

/// Search `node`, found at absolute `base`, for names containing `needle`.
/// `None` for an empty needle: nothing is filtered.
#[must_use]
pub fn filter(node: &Node, base: &[usize], needle: &str) -> Option<Matches> {
    let needle = needle.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    let mut matches = Matches {
        needle,
        base: base.to_vec(),
        ..Matches::default()
    };
    let mut crumbs = base.to_vec();
    let (bytes, files) = visit(node, &mut crumbs, &mut matches);
    matches.bytes = bytes;
    matches.files = files;
    Some(matches)
}

/// Returns the bytes and files that matched at or beneath `node`'s
/// children, recording what to keep.
fn visit(node: &Node, crumbs: &mut Vec<usize>, matches: &mut Matches) -> (u64, u64) {
    let mut total = (0_u64, 0_u64);
    for (index, child) in node.children.iter().enumerate() {
        crumbs.push(index);
        if contains_ignoring_case(&child.name, &matches.needle) {
            matches.keep.insert(crumbs.clone(), Keep::Whole);
            matches.count += 1;
            total.0 = total.0.saturating_add(child.bytes);
            total.1 = total.1.saturating_add(child.files);
        } else if !child.children.is_empty() {
            let (bytes, files) = visit(child, crumbs, matches);
            if bytes > 0 || files > 0 {
                matches.keep.insert(
                    crumbs.clone(),
                    Keep::Partial { bytes, files },
                );
                total.0 = total.0.saturating_add(bytes);
                total.1 = total.1.saturating_add(files);
            }
        }
        crumbs.pop();
    }
    total
}

/// Substring search ignoring ASCII case, without allocating: a filter runs
/// over every name in view on every keystroke.
#[must_use]
pub fn contains_ignoring_case(haystack: &str, lower_needle: &str) -> bool {
    let (hay, needle) = (haystack.as_bytes(), lower_needle.as_bytes());
    if needle.is_empty() {
        return true;
    }
    if needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.to_ascii_lowercase() == *right)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::node::NodeKind;

    fn file(name: &str, bytes: u64) -> Node {
        Node::entry(name, NodeKind::File, bytes)
    }

    fn dir(name: &str, children: Vec<Node>) -> Node {
        let mut node = Node::directory(name);
        node.children = children;
        node
    }

    fn tree() -> Node {
        dir(
            "root",
            vec![
                dir(
                    "code",
                    vec![
                        file("main.rs", 100),
                        file("lib.rs", 40),
                        dir("target", vec![file("app.d", 900)]),
                    ],
                ),
                dir("docs", vec![file("notes.md", 10), file("readme.md", 5)]),
                file("Cargo.toml", 20),
            ],
        )
    }

    #[test]
    fn matching_directory_is_kept_whole() {
        let mut root = tree();
        crate::engine::node::aggregate(&mut root, Metric::Bytes);
        let matches = filter(&root, &[], "code").expect("non-empty needle matches");
        assert_eq!(matches.count, 1);
        assert_eq!(matches.bytes, 1040);
        // The match itself, whole.
        assert_eq!(matches.keep(&[0]), Some(Keep::Whole));
        // Its contents ride along unrecorded but kept whole.
        assert_eq!(matches.keep(&[0, 1]), Some(Keep::Whole));
        // The parent holds the matches, sized by them.
        assert_eq!(
            matches.keep(&[]),
            Some(Keep::Partial {
                bytes: 1040,
                files: 3
            })
        );
        // Siblings are filtered out.
        assert_eq!(matches.keep(&[1]), None);
    }

    #[test]
    fn match_inside_directory_keeps_ancestors_partially() {
        let mut root = tree();
        crate::engine::node::aggregate(&mut root, Metric::Bytes);
        let matches = filter(&root, &[], "md").expect("non-empty needle matches");
        assert_eq!(matches.count, 2);
        // Children are ranked by size; find docs and code wherever they
        // landed.
        let docs = root
            .children
            .iter()
            .position(|child| child.name.as_ref() == "docs")
            .expect("docs present");
        let code = root
            .children
            .iter()
            .position(|child| child.name.as_ref() == "code")
            .expect("code present");
        // docs holds both matches.
        assert_eq!(
            matches.keep(&[docs]),
            Some(Keep::Partial { bytes: 15, files: 2 })
        );
        // code does not.
        assert_eq!(matches.keep(&[code]), None);
        assert_eq!(matches.keep(&[docs, 0]), Some(Keep::Whole));
    }

    #[test]
    fn search_scopes_to_the_requested_base() {
        let mut root = tree();
        crate::engine::node::aggregate(&mut root, Metric::Bytes);
        let base = [0];
        let code = root.resolve(&base).expect("child exists");
        let matches = filter(code, &base, "rs").expect("non-empty needle matches");
        assert_eq!(matches.count, 2);
        assert_eq!(matches.bytes, 140);
        // Outside the base everything is whole.
        assert_eq!(matches.keep(&[1, 0]), Some(Keep::Whole));
    }

    #[test]
    fn needle_ignores_ascii_case() {
        assert!(contains_ignoring_case("Cargo.TOML", "toml"));
        assert!(!contains_ignoring_case("ab", "abc"));
        assert!(contains_ignoring_case("x", ""));
    }

    #[test]
    fn empty_or_whitespace_needle_disables_filtering() {
        assert!(filter(&tree(), &[], "").is_none());
        assert!(filter(&tree(), &[], "   ").is_none());
    }
}
#[test]
    #[allow(unused)]
fn debug_map() {
    let mut root = crate::engine::node::Node::directory("root");
    let mut docs = crate::engine::node::Node::directory("docs");
    docs.children.push(crate::engine::node::Node::entry("notes.md", crate::engine::node::NodeKind::File, 10));
    docs.children.push(crate::engine::node::Node::entry("readme.md", crate::engine::node::NodeKind::File, 5));
    root.children.push(docs);
    crate::engine::node::aggregate(&mut root, crate::engine::node::Metric::Bytes);
    let m = crate::engine::filter::filter(&root, &[], "md").unwrap();
    println!("count={} bytes={} files={}", m.count, m.bytes, m.files);
    println!("keep(&[0]) = {:?}", m.keep(&[0]));
    println!("keep(&[0,0]) = {:?}", m.keep(&[0, 0]));

}
