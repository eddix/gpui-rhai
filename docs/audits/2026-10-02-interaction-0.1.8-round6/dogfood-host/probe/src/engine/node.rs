//! The scanned tree — ported from tobi/disktree tree.rs.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};

use rayon::prelude::*;
use rustc_hash::FxHashSet;

use super::classify::{Category, Reclaim};

/// What a node represents on disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeKind {
    #[default]
    Directory,
    File,
    Symlink,
    /// Sockets, fifos and devices: addressable, but not space.
    Other,
}

impl NodeKind {
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Directory)
    }
}

/// How a node's importance is measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Metric {
    /// Bytes, apparent or on-disk depending on the scan options.
    #[default]
    Bytes,
    /// Number of files at or beneath the node.
    Files,
}

impl Metric {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bytes => "size",
            Self::Files => "files",
        }
    }

    /// Size → Files → Age in tobi's `t` cycle; Age colours by mtime while
    /// area stays bytes, so the engine only knows two area metrics.
    pub const fn toggled(self) -> Self {
        match self {
            Self::Bytes => Self::Files,
            Self::Files => Self::Bytes,
        }
    }

    /// Parse the metric the UI sends across the capability boundary.
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "bytes" | "size" => Some(Self::Bytes),
            "files" => Some(Self::Files),
            _ => None,
        }
    }
}

/// One entry in the scanned tree.
///
/// Totals and direct figures are both kept: `own_bytes` and `own_files` are
/// what sits directly in a directory, `bytes` and `files` are the subtree
/// totals the treemap draws. Derived by [`aggregate`], never tracked
/// separately, so hardlink de-duplication rewrites stay consistent.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: Box<str>,
    pub kind: NodeKind,
    /// Subtree total: direct contents plus every descendant.
    pub bytes: u64,
    /// Bytes of the leaf entries directly in this directory, or this file's
    /// own size.
    pub own_bytes: u64,
    /// Files at or beneath this node; `1` for a file.
    pub files: u64,
    /// Files directly in this directory; `1` for a file.
    pub own_files: u64,
    /// Directories at or beneath this node; `1` for a directory.
    pub dirs: u64,
    /// `(device, inode)` for files, used to de-duplicate hardlinks.
    pub inode: Option<(u64, u64)>,
    /// The directory could not be read; its contents are unknown.
    pub read_error: bool,
    /// Newest write time at or beneath this node, in Unix seconds; `0` when
    /// unknown.
    pub modified: i64,
    pub category: Category,
    pub reclaim: Option<Reclaim>,
    /// Children, ordered by [`Metric`] value, descending.
    pub children: Vec<Self>,
}

impl Node {
    /// A directory with no children yet.
    pub fn directory(name: impl Into<Box<str>>) -> Self {
        Self {
            name: name.into(),
            kind: NodeKind::Directory,
            bytes: 0,
            own_bytes: 0,
            files: 0,
            own_files: 0,
            dirs: 1,
            inode: None,
            read_error: false,
            modified: 0,
            category: Category::Other,
            reclaim: None,
            children: Vec::new(),
        }
    }

    /// A leaf entry.
    pub fn entry(name: impl Into<Box<str>>, kind: NodeKind, bytes: u64) -> Self {
        Self {
            name: name.into(),
            kind,
            bytes,
            own_bytes: bytes,
            files: u64::from(kind == NodeKind::File),
            own_files: u64::from(kind == NodeKind::File),
            dirs: 0,
            inode: None,
            read_error: false,
            modified: 0,
            category: Category::Other,
            reclaim: None,
            children: Vec::new(),
        }
    }

    pub const fn is_dir(&self) -> bool {
        self.kind.is_dir()
    }

    /// The value a treemap should weight this node by.
    pub const fn value(&self, metric: Metric) -> u64 {
        match metric {
            Metric::Bytes => self.bytes,
            Metric::Files => self.files,
        }
    }

    pub fn child_named(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|child| &*child.name == name)
    }

    /// Follow `crumbs` (child indices) from this node.
    pub fn resolve(&self, crumbs: &[usize]) -> Option<&Self> {
        let mut node = self;
        for &index in crumbs {
            node = node.children.get(index)?;
        }
        Some(node)
    }

    /// The chain of nodes ending at `crumbs`, including this node.
    pub fn resolve_chain<'a>(&'a self, crumbs: &[usize]) -> Vec<&'a Self> {
        let mut chain = vec![self];
        let mut node = self;
        for &index in crumbs {
            match node.children.get(index) {
                Some(child) => {
                    chain.push(child);
                    node = child;
                }
                None => break,
            }
        }
        chain
    }

    /// Index of the largest child (children are sorted), for a useful
    /// descent target.
    pub const fn largest_child(&self) -> Option<usize> {
        if self.children.is_empty() {
            None
        } else {
            Some(0)
        }
    }
}

/// Recompute `bytes`, `files`, `dirs` and the direct totals bottom-up, then
/// order children by `metric`, largest first.
pub fn aggregate(node: &mut Node, metric: Metric) {
    aggregate_at(node, metric, 0, None);
}

/// [`aggregate`], charging a hardlinked file once: a leaf whose identity
/// `seen` already holds weighs nothing. Which of a file's names is charged
/// is whichever a worker reaches first; the totals are the same.
pub fn aggregate_deduped(node: &mut Node, metric: Metric, seen: &Seen) {
    aggregate_at(node, metric, 0, Some(seen));
}

/// Identities a finish pass has met.
///
/// Ported as-is: an 8 MiB lock-free bitmap for the first volume met covers
/// 64 million small inode numbers; everything else lands in 64 sharded hash
/// sets. Measured in tobi's source: 4.1M ids took 150 ms sharded vs 1.2 s
/// behind one lock.
pub struct Seen {
    volume: OnceLock<u64>,
    bits: LazyLock<Box<[AtomicU64]>>,
    rest: Box<[Shard]>,
}

type Shard = Mutex<FxHashSet<(u64, u64)>>;

impl Default for Seen {
    fn default() -> Self {
        Self::new()
    }
}

impl Seen {
    /// Numbers the bitmap covers: sixty-four million, more files than a
    /// desktop volume holds, for 8 MiB written once.
    const BITS: u64 = 1 << 26;
    const SHARD_BITS: u32 = 6;

    pub fn new() -> Self {
        Self {
            volume: OnceLock::new(),
            bits: LazyLock::new(|| {
                std::iter::repeat_with(|| AtomicU64::new(0))
                    .take((Self::BITS / 64) as usize)
                    .collect()
            }),
            rest: std::iter::repeat_with(Mutex::default)
                .take(1 << Self::SHARD_BITS)
                .collect(),
        }
    }

    /// Whether `key` is new.
    fn insert(&self, key: (u64, u64)) -> bool {
        let (volume, number) = key;
        if *self.volume.get_or_init(|| volume) == volume && number < Self::BITS {
            let bit = 1 << (number % 64);
            let word = &self.bits[(number / 64) as usize];
            return word.fetch_or(bit, Ordering::Relaxed) & bit == 0;
        }
        let shard = (number.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            >> (u64::BITS - Self::SHARD_BITS)) as usize;
        super::scan::lock(&self.rest[shard]).insert(key)
    }
}

/// Levels whose subtrees are aggregated in parallel.
pub const PARALLEL_LEVELS: usize = 4;

fn aggregate_at(node: &mut Node, metric: Metric, depth: usize, seen: Option<&Seen>) {
    if !node.is_dir() {
        // Every name of a file has the file's size, so one that weighs
        // nothing need not be remembered to be charged once.
        if node.own_bytes > 0
            && let Some(seen) = seen
            && let Some(key) = node.inode
            && !seen.insert(key)
        {
            node.own_bytes = 0;
        }
        node.bytes = node.own_bytes;
        node.files = node.own_files;
        node.dirs = 0;
        return;
    }

    let mut bytes = 0;
    let mut files = 0;
    let mut own_bytes = 0;
    let mut own_files = 0;
    let mut dirs: u64 = 1;
    let mut modified = 0;
    if depth < PARALLEL_LEVELS {
        node.children
            .par_iter_mut()
            .for_each(|child| aggregate_at(child, metric, depth + 1, seen));
    } else {
        for child in &mut node.children {
            aggregate_at(child, metric, depth + 1, seen);
        }
    }
    for child in &node.children {
        modified = modified.max(child.modified);
        // Saturating: a corrupt volume can claim any size.
        bytes = child.bytes.saturating_add(bytes);
        files += child.files;
        dirs += child.dirs;
        if !child.is_dir() {
            own_bytes = child.bytes.saturating_add(own_bytes);
            own_files += child.files;
        }
    }
    node.bytes = bytes;
    node.files = files;
    node.own_bytes = own_bytes;
    node.own_files = own_files;
    node.dirs = dirs;
    node.modified = modified;

    // Largest first, ties by name. Unstable: names in one directory are
    // distinct, so the stable sort's scratch allocation buys nothing.
    node.children.sort_unstable_by(|left, right| {
        right
            .value(metric)
            .cmp(&left.value(metric))
            .then_with(|| left.name.cmp(&right.name))
    });
}

/// Absolute path of the node at `crumbs` beneath a scanned root.
pub fn path_of(root_path: &Path, root: &Node, crumbs: &[usize]) -> PathBuf {
    let mut path = root_path.to_path_buf();
    let mut node = root;
    for &index in crumbs {
        match node.children.get(index) {
            Some(child) => {
                path.push(&*child.name);
                node = child;
            }
            None => break,
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(name: &str, bytes: u64) -> Node {
        Node::entry(name, NodeKind::File, bytes)
    }

    #[test]
    fn seen_charges_an_identity_once_in_either_store() {
        let seen = Seen::new();
        assert!(seen.insert((7, 42)));
        assert!(!seen.insert((7, 42)));
        assert!(seen.insert((7, 43)));
        let far = Seen::BITS + 42;
        assert!(seen.insert((7, far)));
        assert!(!seen.insert((7, far)));
        assert!(seen.insert((8, 42)), "another volume's 42 is another file");
        assert!(!seen.insert((8, 42)));
    }

    #[test]
    fn aggregate_derives_totals_and_orders_children() {
        let mut root = Node::directory("root");
        let mut nested = Node::directory("child");
        nested.children.push(leaf("deep", 7));
        root.children.push(nested);
        root.children.push(leaf("direct", 5));
        root.children.push(leaf("small", 9));

        aggregate(&mut root, Metric::Bytes);
        assert_eq!(root.own_bytes, 5 + 9, "direct leaves only");
        assert_eq!(root.bytes, 5 + 9 + 7);
        assert_eq!(root.files, 3);
        assert_eq!(root.own_files, 2);
        assert_eq!(root.dirs, 2);
        assert_eq!(root.child_named("child").unwrap().own_bytes, 7);
        assert_eq!(&*root.children[0].name, "small", "9 bytes, largest first");
        assert_eq!(&*root.children[2].name, "direct");
    }

    #[test]
    fn aggregate_can_rank_by_file_count() {
        let mut root = Node::directory("root");
        let mut many = Node::directory("many");
        for index in 0..5 {
            many.children.push(leaf(&format!("f{index}"), 1));
        }
        root.children.push(many);
        root.children.push(leaf("huge", 10_000));

        aggregate(&mut root, Metric::Bytes);
        assert_eq!(&*root.children[0].name, "huge");
        aggregate(&mut root, Metric::Files);
        assert_eq!(&*root.children[0].name, "many");
        assert_eq!(root.children[0].files, 5);
    }

    #[test]
    fn resolve_walks_child_indices() {
        let mut root = Node::directory("root");
        let mut nested = Node::directory("child");
        nested.children.push(leaf("deep", 1));
        root.children.push(nested);

        assert!(root.resolve(&[]).is_some());
        assert_eq!(root.resolve(&[0, 0]).map(|n| &*n.name), Some("deep"));
        assert!(root.resolve(&[0, 1]).is_none());
        assert_eq!(root.resolve_chain(&[0, 0]).len(), 3);
    }

    #[test]
    fn path_of_joins_names_beneath_the_root() {
        let mut root = Node::directory("root");
        let mut nested = Node::directory("child");
        nested.children.push(leaf("deep", 1));
        root.children.push(nested);

        let path = path_of(Path::new("/home/eddix"), &root, &[0, 0]);
        assert_eq!(path, PathBuf::from("/home/eddix/child/deep"));
    }
}
