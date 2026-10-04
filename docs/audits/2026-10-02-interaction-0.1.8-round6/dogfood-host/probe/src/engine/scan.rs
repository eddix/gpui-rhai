//! Parallel filesystem scanning — ported from tobi/disktree scan.rs
//! (Linux paths only; the original's Windows MFT reader, macOS dataless
//! checks, and widening memoization are out of scope for v1).
//!
//! The shape follows dust, because that shape is the reason dust is fast:
//! one `rayon::scope` per root (recursion depth O(1)), every directory a
//! `PendingDir` with an atomic completion counter and a `+1` sentinel, and
//! one bottom-up aggregation pass where hardlinks are de-duplicated.

use std::fs::{self, DirEntry, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError, OnceLock};
use std::thread;

use rayon::Scope;
use rustc_hash::FxHashSet;

use super::classify;
use super::node::{Metric, Node, NodeKind, Seen, aggregate, aggregate_deduped};

/// Errors kept verbatim before the list is truncated; the count keeps rising.
const MAX_ERROR_DETAIL: usize = 50;

/// How a scan measures and filters the tree.
#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// Measure apparent length instead of allocated blocks.
    pub apparent_size: bool,
    /// Follow symlinks. Off by default: a home directory is full of links,
    /// and following them double-counts.
    pub follow_links: bool,
    /// Include dotfiles and dot-directories. On by default, like dust and
    /// `du`: `~/.cache` is frequently the largest directory in a home.
    pub include_hidden: bool,
    /// Stay on the root's volume: skip other disks, pseudo filesystems,
    /// network shares and automount points, but keep subvolumes of the same
    /// disk. See [`super::space::foreign_mounts`].
    pub one_filesystem: bool,
    /// Stop descending past this depth. Totals below are then unknown.
    pub max_depth: Option<usize>,
    /// Count a hardlinked file once instead of once per link.
    pub dedup_hardlinks: bool,
    /// Whether children are ranked by bytes or by file count.
    pub metric: Metric,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            apparent_size: false,
            follow_links: false,
            include_hidden: true,
            one_filesystem: true,
            max_depth: None,
            dedup_hardlinks: true,
            metric: Metric::Bytes,
        }
    }
}

/// Counters a running scan publishes for the UI.
///
/// Plain relaxed atomics: this is a progress meter, not a synchronisation
/// point, and the UI reads a snapshot that is allowed to be a few entries
/// stale.
#[derive(Debug, Default)]
pub struct ScanProgress {
    files: AtomicU64,
    dirs: AtomicU64,
    bytes: AtomicU64,
    errors: AtomicU64,
    finished: AtomicBool,
    cancelled: AtomicBool,
    messages: Mutex<Vec<String>>,
}

/// A point-in-time view of [`ScanProgress`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanSnapshot {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub finished: bool,
    pub cancelled: bool,
    /// Up to [`MAX_ERROR_DETAIL`] unreadable paths, most recent last.
    pub messages: Vec<String>,
}

impl ScanProgress {
    pub(crate) fn add(&self, files: u64, dirs: u64, bytes: u64) {
        self.files.fetch_add(files, Ordering::Relaxed);
        self.dirs.fetch_add(dirs, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn record_error(&self, path: &Path, error: &io::Error) {
        self.errors.fetch_add(1, Ordering::Relaxed);
        // A scan without permission can refuse thousands of paths; only the
        // ones that will be kept are worth formatting.
        let mut messages = lock(&self.messages);
        if messages.len() < MAX_ERROR_DETAIL {
            messages.push(format!("{}: {error}", path.display()));
        }
        drop(messages);
    }

    fn finish(&self) {
        self.finished.store(true, Ordering::Relaxed);
    }

    /// Ask the walk to stop at the next directory boundary.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> ScanSnapshot {
        ScanSnapshot {
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            finished: self.finished.load(Ordering::Relaxed),
            cancelled: self.is_cancelled(),
            messages: lock(&self.messages).clone(),
        }
    }
}

/// A scan running on its own thread.
#[derive(Debug)]
pub struct ScanHandle {
    pub progress: Arc<ScanProgress>,
    result: Receiver<io::Result<Node>>,
}

impl ScanHandle {
    pub fn audit_pending_handle() -> (Self, mpsc::Sender<io::Result<Node>>) {
        let (sender,result)=mpsc::channel();
        (Self {progress:Arc::new(ScanProgress::default()),result},sender)
    }

    /// Start walking `root` on a worker thread.
    pub fn spawn(root: PathBuf, options: ScanOptions) -> Self {
        let progress = Arc::new(ScanProgress::default());
        let context = Arc::new(WalkContext {
            options,
            progress: Arc::clone(&progress),
            root_device: Mutex::new(None),
            foreign_mounts: OnceLock::new(),
            never_scanned: OnceLock::new(),
            visited_dirs: Mutex::new(FxHashSet::default()),
            root: Mutex::new(None),
        });
        let (sender, result) = mpsc::channel();
        // The closure owns the sender, so a failed spawn drops it and the
        // receiver reports the failure.
        let worker = thread::Builder::new()
            .name("disktree-rhai-scan".into())
            .spawn(move || {
                let outcome = scan_blocking(&root, &context);
                context.progress.finish();
                let _ = sender.send(outcome);
            });
        if let Err(error) = worker {
            progress.record_error(Path::new("scan worker"), &error);
            progress.finish();
        }
        Self { progress, result }
    }

    /// Take the finished tree, if the walk is done.
    pub fn poll(&self) -> Option<io::Result<Node>> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(io::Error::other(
                "the scan worker stopped without a result",
            ))),
        }
    }

    /// Abandon the walk; the worker still finishes the directory it is in.
    pub fn cancel(&self) {
        self.progress.cancel();
    }
}

/// Walk `root` and return the aggregated tree. Blocking.
pub fn scan(root: &Path, options: ScanOptions) -> io::Result<Node> {
    let progress = Arc::new(ScanProgress::default());
    let context = Arc::new(WalkContext {
        options,
        progress: Arc::clone(&progress),
        root_device: Mutex::new(None),
        foreign_mounts: OnceLock::new(),
        never_scanned: OnceLock::new(),
        visited_dirs: Mutex::new(FxHashSet::default()),
        root: Mutex::new(None),
    });
    let node = scan_blocking(root, &context)?;
    progress.finish();
    Ok(node)
}

struct WalkContext {
    options: ScanOptions,
    progress: Arc<ScanProgress>,
    /// Device of the root, resolved once: the `one_filesystem` fallback when
    /// the mount table cannot be read.
    root_device: Mutex<Option<u64>>,
    /// Mount points `one_filesystem` keeps out, from the mount table. Unset
    /// when the table cannot be read, and devices are compared instead.
    foreign_mounts: OnceLock<FxHashSet<PathBuf>>,
    /// Directories no scan enters, on any volume setting.
    never_scanned: OnceLock<FxHashSet<PathBuf>>,
    /// Directories already entered, so followed symlinks cannot loop.
    visited_dirs: Mutex<FxHashSet<(u64, u64)>>,
    /// Set by the root's `complete`, read after the scope joins.
    root: Mutex<Option<Node>>,
}

impl WalkContext {
    fn root_device(&self, path: &Path) -> Option<u64> {
        let mut cached = lock(&self.root_device);
        if let Some(device) = *cached {
            return Some(device);
        }
        let device = fs::metadata(path).map(|meta| device_of(&meta)).ok();
        *cached = device;
        device
    }

    fn cancelled(&self) -> bool {
        self.progress.is_cancelled()
    }

    /// Decide what to do with an entry listed in `dir`.
    ///
    /// The entry's path is built only where it is used: most entries are
    /// files, and a disk has millions of them.
    fn classify(&self, dir: &Path, entry: &mut Named) -> Classified {
        let listing = match entry.listing() {
            Ok(listing) => listing,
            Err(error) => {
                self.progress.record_error(&entry.entry_path(dir), &error);
                return Classified::Skipped;
            }
        };

        if !self.options.include_hidden && entry.name().starts_with('.') {
            return Classified::Skipped;
        }

        match listing {
            Listing::Symlink => {
                let path = entry.entry_path(dir);
                self.classify_symlink(&path, entry.take_name())
            }
            Listing::Directory => {
                let path = entry.entry_path(dir);
                self.classify_dir(entry, path)
            }
            Listing::Leaf(kind) => match entry.facts(self.options.apparent_size) {
                Ok(facts) => self.leaf(entry.take_name(), kind, &facts),
                Err(error) => {
                    self.progress.record_error(&entry.entry_path(dir), &error);
                    Classified::Skipped
                }
            },
        }
    }

    fn classify_dir(&self, entry: &mut Named, path: PathBuf) -> Classified {
        if self
            .never_scanned
            .get()
            .is_some_and(|never| never.contains(&path))
        {
            return Classified::Skipped;
        }
        // The device check runs wherever the mount table cannot be read.
        let directory_cell = std::cell::OnceCell::new();
        let directory = || directory_cell.get_or_init(|| entry.directory());
        if self.options.one_filesystem
            && let Some(foreign) = self.foreign_mounts.get()
        {
            // Checked by path before anything reads the directory, so an
            // automount point is never triggered.
            if foreign.contains(&path) {
                return Classified::Skipped;
            }
        } else if self.options.one_filesystem {
            match (directory(), self.root_device(&path)) {
                (Ok(directory), Some(root_device)) if *directory != root_device => {
                    return Classified::Skipped;
                }
                (Err(error), _) => {
                    self.progress.record_error(&path, error);
                    return Classified::Skipped;
                }
                _ => {}
            }
        }
        Classified::Subdirectory {
            path,
            name: entry.take_name(),
        }
    }

    fn classify_symlink(&self, path: &Path, name: Box<str>) -> Classified {
        if !self.options.follow_links {
            // Not followed: the link occupies only its target string, which
            // `du` reports as a handful of bytes or nothing at all.
            return match fs::symlink_metadata(path) {
                Ok(meta) => {
                    let facts = Facts::of(&meta, self.options.apparent_size);
                    Classified::Entry(leaf_node(
                        name,
                        NodeKind::Symlink,
                        &facts,
                        facts.shared,
                    ))
                }
                Err(error) => {
                    self.progress.record_error(path, &error);
                    Classified::Skipped
                }
            };
        }

        let meta = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(error) => {
                // A dangling link is normal, not an error worth counting.
                if error.kind() != io::ErrorKind::NotFound {
                    self.progress.record_error(path, &error);
                }
                return Classified::Skipped;
            }
        };

        if meta.is_dir() {
            if let Some(key) = identity_of(&meta)
                && !lock(&self.visited_dirs).insert(key)
            {
                return Classified::Skipped;
            }
            return Classified::Subdirectory {
                path: path.to_path_buf(),
                name,
            };
        }

        let mut facts = Facts::of(&meta, self.options.apparent_size);
        // What the link leads to, so a file also reached directly is
        // charged once.
        facts.identity = identity_of(&meta);
        self.leaf(name, kind_of(&meta, meta.file_type()), &facts)
    }

    fn leaf(&self, name: Box<str>, kind: NodeKind, facts: &Facts) -> Classified {
        // A followed link reaches a file a second way, whatever its link
        // count says.
        let track = self.options.follow_links || facts.shared;
        Classified::Entry(leaf_node(name, kind, facts, track))
    }
}

/// What an entry is, as far as the walk is concerned.
enum Listing {
    Directory,
    Symlink,
    Leaf(NodeKind),
}

/// What a leaf contributes.
struct Facts {
    size: u64,
    identity: Option<(u64, u64)>,
    modified: i64,
    /// The file may have another name the walk could meet: its identity is
    /// worth keeping for hardlink de-duplication.
    shared: bool,
}

impl Facts {
    fn of(meta: &Metadata, apparent_size: bool) -> Self {
        Self {
            size: measure(meta, apparent_size),
            identity: file_identity(meta),
            modified: modified_seconds(meta),
            shared: shares_inode(meta),
        }
    }
}

/// A standard directory entry with its name decoded once, up front.
struct Named {
    entry: DirEntry,
    name: Box<str>,
}

impl Named {
    fn entry_path(&self, _dir: &Path) -> PathBuf {
        self.entry.path()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn take_name(&mut self) -> Box<str> {
        std::mem::take(&mut self.name)
    }

    fn listing(&self) -> io::Result<Listing> {
        let file_type = self.entry.file_type()?;
        Ok(if file_type.is_symlink() {
            Listing::Symlink
        } else if file_type.is_dir() {
            Listing::Directory
        } else if file_type.is_file() {
            Listing::Leaf(NodeKind::File)
        } else {
            Listing::Leaf(NodeKind::Other)
        })
    }

    fn facts(&self, apparent_size: bool) -> io::Result<Facts> {
        self.entry
            .metadata()
            .map(|meta| Facts::of(&meta, apparent_size))
    }

    fn directory(&self) -> io::Result<u64> {
        self.entry.metadata().map(|meta| device_of(&meta))
    }
}

/// List a directory.
fn list(path: &Path) -> io::Result<impl Iterator<Item = io::Result<Named>>> {
    Ok(fs::read_dir(path)?.map(|entry| {
        entry.map(|entry| Named {
            name: entry.file_name().into_string().map_or_else(
                |raw| raw.to_string_lossy().into(),
                String::into_boxed_str,
            ),
            entry,
        })
    }))
}

/// What a directory entry turned out to be.
enum Classified {
    /// Descend into this directory on a new task.
    Subdirectory { path: PathBuf, name: Box<str> },
    /// A leaf that contributes size.
    Entry(Node),
    /// Filtered out, unreadable, or a symlink we chose not to follow.
    Skipped,
}

/// One directory being walked, plus the counter that decides when it is done.
#[derive(Debug)]
struct PendingDir {
    path: PathBuf,
    parent: Option<Arc<Self>>,
    /// Starts at 1 for the directory itself; one more per subdirectory task.
    /// When it reaches zero the directory is complete.
    pending: AtomicUsize,
    /// The node so far, moved out whole once the directory is complete.
    partial: Mutex<Partial>,
    read_error: AtomicBool,
    depth: usize,
}

#[derive(Debug, Default)]
struct Partial {
    name: Box<str>,
    children: Vec<Node>,
}

impl PendingDir {
    const fn new(
        path: PathBuf,
        name: Box<str>,
        parent: Option<Arc<Self>>,
        depth: usize,
    ) -> Self {
        Self {
            path,
            parent,
            pending: AtomicUsize::new(1),
            partial: Mutex::new(Partial {
                name,
                children: Vec::new(),
            }),
            read_error: AtomicBool::new(false),
            depth,
        }
    }

    /// Turn a completed directory into a node. `bytes`/`own_bytes` are left
    /// at zero on purpose: [`aggregate`] derives them from the children.
    fn build(&self) -> Node {
        let partial = std::mem::take(&mut *lock(&self.partial));
        Node {
            name: partial.name,
            kind: NodeKind::Directory,
            bytes: 0,
            own_bytes: 0,
            files: 0,
            own_files: 0,
            dirs: 1,
            inode: None,
            read_error: self.read_error.load(Ordering::Relaxed),
            modified: 0,
            category: classify::Category::Other,
            reclaim: None,
            children: partial.children,
        }
    }
}

fn scan_blocking(root: &Path, context: &Arc<WalkContext>) -> io::Result<Node> {
    let root_meta = fs::metadata(root)?;
    if !root_meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a directory", root.display()),
        ));
    }
    let resolved = root.canonicalize().ok();
    let canonical = resolved.as_deref().unwrap_or(root);
    // Hidden-entry and depth filters can leave an alias as the only route to
    // files. Keep every mount view when either filter is active.
    let mut never = super::space::never_scanned(root, canonical);
    if context.options.include_hidden && context.options.max_depth.is_none() {
        never.extend(super::space::repeated_mounts_for(
            root,
            canonical,
            context.options.one_filesystem,
        ));
    }
    let _ = context.never_scanned.set(never.into_iter().collect());
    if context.options.one_filesystem {
        *lock(&context.root_device) = Some(device_of(&root_meta));
        if let Some(foreign) = super::space::foreign_mounts_for(canonical) {
            let _ = context.foreign_mounts.set(foreign.into_iter().collect());
        }
    }
    if context.options.follow_links
        && let Some(key) = identity_of(&root_meta)
    {
        lock(&context.visited_dirs).insert(key);
    }

    let root_dir = Arc::new(PendingDir::new(
        root.to_path_buf(),
        file_name(root),
        None,
        0,
    ));
    let context: &WalkContext = context;
    WALK_POOL.install(|| rayon::scope(|scope| walk(scope, &root_dir, context)));

    let node = lock(&context.root).take();
    let node = node.ok_or_else(|| {
        io::Error::other(format!("{} produced no tree", root.display()))
    })?;
    Ok(finish_tree(node, &context.options))
}

/// The walk's workers. Listing directories contends in the file system; a
/// pool of its own keeps work the UI hands the global pool from queueing
/// behind a scan's blocking reads.
static WALK_POOL: LazyLock<rayon::ThreadPool> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(rayon::current_num_threads())
        .thread_name(|index| format!("disktree-rhai-walk-{index}"))
        .build()
        .expect("a thread pool")
});

/// Entries counted before the shared progress counters are touched, so one
/// cache line is not bounced between cores per entry.
const TALLY_EVERY: u64 = 1024;

#[derive(Default)]
struct Tally {
    files: u64,
    dirs: u64,
    bytes: u64,
}

impl Tally {
    const fn add(&mut self, node: &Node) {
        if node.is_dir() {
            self.files += node.files;
            self.dirs += node.dirs;
        } else {
            // A link counts as a file on the meter, though the tree does
            // not count it.
            self.files += 1;
        }
        self.bytes += node.bytes;
    }

    fn flush(&mut self, progress: &ScanProgress) {
        progress.add(self.files, self.dirs, self.bytes);
        *self = Self::default();
    }
}

/// Read one directory, spawn a task per subdirectory, then report completion.
///
/// Flat tasks inside the scope: the worker stack never grows with tree depth.
fn walk<'scope>(scope: &Scope<'scope>, dir: &Arc<PendingDir>, context: &'scope WalkContext) {
    let mut subdirs: Vec<Arc<PendingDir>> = Vec::new();
    let mut leaves: Vec<Node> = Vec::new();
    let mut tally = Tally::default();

    match list(&dir.path) {
        Ok(entries) => {
            for entry in entries {
                if context.cancelled() {
                    break;
                }
                match entry {
                    Ok(mut entry) => {
                        match context.classify(&dir.path, &mut entry) {
                            Classified::Subdirectory { path, name } => {
                                tally.dirs += 1;
                                subdirs.push(Arc::new(PendingDir::new(
                                    path,
                                    name,
                                    Some(Arc::clone(dir)),
                                    dir.depth + 1,
                                )));
                            }
                            Classified::Entry(node) => {
                                tally.add(&node);
                                leaves.push(node);
                            }
                            Classified::Skipped => {}
                        }
                        if tally.files + tally.dirs >= TALLY_EVERY {
                            tally.flush(&context.progress);
                        }
                    }
                    Err(error) => {
                        context.progress.record_error(&dir.path, &error);
                    }
                }
            }
        }
        Err(error) => {
            context.progress.record_error(&dir.path, &error);
            dir.read_error.store(true, Ordering::Relaxed);
        }
    }
    tally.flush(&context.progress);

    // A depth-limited scan still measures what is directly in the
    // directory, it just does not descend further.
    let descend = context
        .options
        .max_depth
        .is_none_or(|max_depth| dir.depth < max_depth);
    let spawned = if descend { subdirs.len() } else { 0 };
    {
        let mut partial = lock(&dir.partial);
        partial.children.reserve_exact(leaves.len() + spawned);
        partial.children.extend(leaves);
    }
    if descend {
        for subdir in subdirs {
            if context.cancelled() {
                break;
            }
            dir.pending.fetch_add(1, Ordering::AcqRel);
            scope.spawn(move |scope| walk(scope, &subdir, context));
        }
    }

    signal_done(dir, context);
}

/// Report that one task for `dir` is done: either its own scan, or one of its
/// children. The last one to report builds the node and bubbles up.
///
/// This is the whole reason for the `+1` sentinel: a directory is only built
/// once its own scan *and* every subdirectory task has finished, no matter
/// which of them lands last.
fn signal_done(dir: &Arc<PendingDir>, context: &WalkContext) {
    if dir.pending.fetch_sub(1, Ordering::AcqRel) != 1 {
        return;
    }
    let node = dir.build();
    let Some(parent) = &dir.parent else {
        *lock(&context.root) = Some(node);
        return;
    };
    lock(&parent.partial).children.push(node);
    signal_done(parent, context);
}

/// Charge a hardlinked file once, derive every aggregate from the result,
/// then classify. On the walk's pool: the UI's own work on the global pool
/// must not queue behind a scan finishing.
fn finish_tree(mut node: Node, options: &ScanOptions) -> Node {
    WALK_POOL.install(|| {
        if options.dedup_hardlinks {
            aggregate_deduped(&mut node, options.metric, &Seen::new());
        } else {
            aggregate(&mut node, options.metric);
        }
        classify::classify(&mut node);
    });
    node
}

/// A leaf. Its identity is kept only when `track` says the walk could meet
/// the same file again: a dedup set of every file on a disk is millions of
/// entries, nearly all for files with one name.
fn leaf_node(name: Box<str>, kind: NodeKind, facts: &Facts, track: bool) -> Node {
    let mut node = Node::entry(name, kind, facts.size);
    if track {
        node.inode = facts.identity;
    }
    node.modified = facts.modified;
    node
}

/// Last write time in Unix seconds, from the metadata the walk already has.
fn modified_seconds(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
}

fn measure(meta: &Metadata, apparent_size: bool) -> u64 {
    if apparent_size {
        meta.len()
    } else {
        allocated_bytes(meta).unwrap_or(meta.len())
    }
}

/// Allocated bytes, `st_blocks * 512`: sparse files spend less than they
/// claim, and this is the number that matches `du`.
fn allocated_bytes(meta: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt as _;
    Some(meta.blocks().saturating_mul(512))
}

fn shares_inode(meta: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    meta.nlink() > 1
}

fn device_of(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    meta.dev()
}

/// `(device, inode)` for files, used to de-duplicate hardlinks and detect
/// symlink loops.
fn file_identity(meta: &Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    Some((meta.dev(), meta.ino()))
}

fn identity_of(meta: &Metadata) -> Option<(u64, u64)> {
    file_identity(meta)
}

fn kind_of(meta: &Metadata, file_type: fs::FileType) -> NodeKind {
    if meta.is_dir() || file_type.is_dir() {
        NodeKind::Directory
    } else if meta.is_file() || file_type.is_file() {
        NodeKind::File
    } else if file_type.is_symlink() {
        NodeKind::Symlink
    } else {
        NodeKind::Other
    }
}

/// The display name of a scanned root: its final component, or the path
/// itself for `/`.
pub fn file_name(path: &Path) -> Box<str> {
    path.file_name()
        .map_or_else(
            || path.to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
        .into()
}

/// A poisoned lock means another task panicked; the data is still a valid
/// tree prefix, and refusing to read it would turn one panic into two.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Apparent sizes, so the assertions are about the tree rather than
    /// about how the filesystem rounds a small file up to a block.
    fn options() -> ScanOptions {
        ScanOptions {
            apparent_size: true,
            ..ScanOptions::default()
        }
    }

    fn write(root: &Path, relative: &str, bytes: usize) -> PathBuf {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, vec![b'x'; bytes]).expect("write");
        path
    }

    fn scan_dir(root: &Path, options: &ScanOptions) -> Node {
        scan(root, options.clone()).expect("scan")
    }

    fn child<'a>(node: &'a Node, name: &str) -> &'a Node {
        node.children
            .iter()
            .find(|child| &*child.name == name)
            .unwrap_or_else(|| panic!("no child named {name} in {:?}", node.name))
    }

    #[test]
    fn totals_are_summed_bottom_up() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "a/one.bin", 1000);
        write(root, "a/nested/two.bin", 2000);
        write(root, "b/three.bin", 500);

        let tree = scan_dir(root, &options());
        assert_eq!(tree.bytes, 3500);
        assert_eq!(tree.files, 3);
        assert_eq!(tree.dirs, 4, "root, a, a/nested, b");
        assert_eq!(child(&tree, "a").bytes, 3000);
        assert_eq!(child(child(&tree, "a"), "nested").bytes, 2000);
        assert_eq!(child(&tree, "b").bytes, 500);
    }

    #[test]
    fn children_are_ranked_largest_first() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "small.bin", 10);
        write(root, "large.bin", 1000);
        write(root, "medium.bin", 100);

        let tree = scan_dir(root, &options());
        let names: Vec<&str> = tree.children.iter().map(|c| &*c.name).collect();
        assert_eq!(names, vec!["large.bin", "medium.bin", "small.bin"]);
    }

    #[test]
    fn a_directory_reports_its_own_bytes_separately() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "direct.bin", 700);
        write(root, "sub/deep.bin", 300);

        let tree = scan_dir(root, &options());
        assert_eq!(tree.own_bytes, 700);
        assert_eq!(tree.own_files, 1);
        assert_eq!(tree.bytes, 1000);
        assert_eq!(tree.files, 2);
    }

    #[test]
    fn hardlinks_are_charged_once_by_default() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        let original = write(root, "original.bin", 4096);
        fs::hard_link(&original, root.join("link.bin")).expect("hard link");

        let deduped = scan_dir(root, &options());
        assert_eq!(deduped.bytes, 4096);
        assert_eq!(deduped.files, 2, "both names still exist");

        let counted = scan_dir(
            root,
            &ScanOptions {
                dedup_hardlinks: false,
                ..options()
            },
        );
        assert_eq!(counted.bytes, 8192);
    }

    #[test]
    fn hidden_entries_are_included_by_default_and_excluded_on_request() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, ".cache/blob.bin", 900);
        write(root, "visible.bin", 100);

        let default = scan_dir(root, &options());
        assert_eq!(default.bytes, 1000);
        assert_eq!(child(&default, ".cache").bytes, 900);

        let without = scan_dir(
            root,
            &ScanOptions {
                include_hidden: false,
                ..options()
            },
        );
        assert_eq!(without.bytes, 100);
    }

    #[test]
    fn symlinks_are_not_followed_by_default() {
        let temp = TempDir::new().expect("tempdir");
        let outside = TempDir::new().expect("tempdir");
        write(outside.path(), "elsewhere.bin", 5000);
        let root = temp.path();
        write(root, "real.bin", 100);
        assert!(
            std::os::unix::fs::symlink(outside.path(), root.join("link")).is_ok(),
            "a link"
        );

        let tree = scan_dir(root, &options());
        // Only the real file's bytes; the link itself holds just its target
        // string, and its 5000-byte destination is outside this tree.
        assert!(tree.bytes < 200, "{}", tree.bytes);
        let link = child(&tree, "link");
        assert_eq!(link.kind, NodeKind::Symlink);
        assert!(link.bytes < 100, "a link holds only its target string");
    }

    #[test]
    fn followed_symlink_loops_do_not_hang_the_scan() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "sub/leaf.bin", 42);
        assert!(std::os::unix::fs::symlink(root, root.join("sub/loop")).is_ok());

        let tree = scan_dir(
            root,
            &ScanOptions {
                follow_links: true,
                ..options()
            },
        );
        assert_eq!(tree.bytes, 42);
    }

    /// Disk usage is what the volume allocated, not the length: whole
    /// blocks, so it covers the data and ends on a block boundary.
    #[test]
    fn disk_usage_is_whole_allocation_units() {
        let temp = TempDir::new().expect("tempdir");
        // Noise, so a compressing filesystem cannot store it in less.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let noise: Vec<u8> = (0..100_000)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state.to_le_bytes()[0]
            })
            .collect();
        fs::write(temp.path().join("data.bin"), noise).expect("write");
        let tree = scan_dir(temp.path(), &ScanOptions::default());
        assert!(tree.bytes >= 100_000, "{}", tree.bytes);
        assert_eq!(tree.bytes % 512, 0, "{}", tree.bytes);
        let apparent = scan_dir(temp.path(), &options());
        assert_eq!(apparent.bytes, 100_000);
    }

    #[test]
    fn max_depth_stops_descending_but_keeps_direct_files() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "here.bin", 10);
        write(root, "a/there.bin", 20);
        write(root, "a/b/far.bin", 30);

        let tree = scan_dir(
            root,
            &ScanOptions {
                max_depth: Some(1),
                ..options()
            },
        );
        assert_eq!(tree.own_bytes, 10);
        let a = child(&tree, "a");
        assert_eq!(
            a.bytes, 20,
            "direct files at the cut-off depth still count"
        );
        assert!(
            a.children.iter().all(|child| !child.is_dir()),
            "descending stopped at depth 1"
        );
        assert_eq!(a.files, 1);
    }

    #[test]
    fn file_count_metric_ranks_by_entries() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        for index in 0..5 {
            write(root, &format!("many/f{index}.bin"), 1);
        }
        write(root, "one/huge.bin", 100_000);

        let tree = scan_dir(root, &options());
        assert_eq!(child(&tree, "one").bytes, 100_000);
        assert_eq!(&*tree.children[0].name, "one");

        let by_files = scan_dir(
            root,
            &ScanOptions {
                metric: Metric::Files,
                ..options()
            },
        );
        assert_eq!(&*by_files.children[0].name, "many");
        assert_eq!(by_files.children[0].files, 5);
    }

    #[test]
    fn an_unreadable_directory_is_recorded_not_fatal() {
        use std::os::unix::fs::PermissionsExt as _;
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "readable.bin", 11);
        let locked = root.join("locked");
        fs::create_dir_all(&locked).expect("mkdir");
        write(&locked, "hidden.bin", 22);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))
            .expect("chmod");

        // Running as root would bypass the permission bits entirely.
        let readable = fs::read_dir(&locked).is_ok();
        let tree = scan_dir(root, &options());
        let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o700));

        assert_eq!(tree.bytes, 11);
        let node = child(&tree, "locked");
        if readable {
            assert!(node.read_error);
        }
    }

    #[test]
    fn a_file_root_is_rejected() {
        let temp = TempDir::new().expect("tempdir");
        let path = write(temp.path(), "file.bin", 1);
        let error = scan(&path, options()).expect_err("a file is not a tree");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn a_spawned_scan_reports_progress_and_a_result() {
        let temp = TempDir::new().expect("tempdir");
        let root = temp.path();
        write(root, "a/one.bin", 1024);
        write(root, "b/two.bin", 2048);

        let handle = ScanHandle::spawn(root.to_path_buf(), options());
        let mut outcome = None;
        for _ in 0..2000 {
            if let Some(result) = handle.poll() {
                outcome = Some(result);
                break;
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        let tree = outcome.expect("the scan finished").expect("a tree");
        assert_eq!(tree.bytes, 3072);

        let snapshot = handle.progress.snapshot();
        assert!(snapshot.finished);
        assert_eq!(snapshot.files, 2);
        assert_eq!(snapshot.errors, 0);
    }
}
