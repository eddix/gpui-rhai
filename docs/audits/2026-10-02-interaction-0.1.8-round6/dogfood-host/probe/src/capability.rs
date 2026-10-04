//! The capability bridge between skins and the engine.
//!
//! Three capabilities, registered by [`DisktreeExtension`]:
//!
//! - `disktree.scan` (subscription): start a scan; emits throttled
//!   `progress` events, then one `done`, `cancelled`, or `failed` event.
//!   Stopping the owning effect cancels the walk.
//! - `disktree.tree` (sync): read the current tree — `children` returns a
//!   ranked, capped window of one directory (the layout's only data path),
//!   `path` resolves crumbs to an absolute path.
//! - `disktree.open` (task): show a path in the desktop file manager.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use gpui_rhai::{
    AsyncCapabilityHandler, CapabilityDescriptor, CapabilityHandler, CapabilityId,
    CapabilityMethod, ComponentStateSchema, ScriptViewExtension, StateField,
    StoreId, SubscriptionCapabilityHandler, SubscriptionWork, UiRuntimeState,
    UiValue, ValueSchema, ObjectField,
};
use semver::Version;

use crate::engine::insights::{self, Finding};
use crate::engine::node::{Metric, Node, NodeKind};
use crate::engine::scan::{ScanHandle, ScanOptions};
use crate::engine::space;
use crate::engine::SharedTree;

/// How often the scan subscription reports progress.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

fn map(pairs: Vec<(&str, UiValue)>) -> UiValue {
    UiValue::Map(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn number(value: u64) -> UiValue {
    // UiValue carries i64; sizes stay far inside it.
    UiValue::Integer(value as i64)
}

fn kind_key(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Directory => "dir",
        NodeKind::File => "file",
        NodeKind::Symlink => "symlink",
        NodeKind::Other => "other",
    }
}

/// One entry of `disktree.tree.children`, everything the layout and the
/// selection card need about a node. `index` is the child's position in the
/// scanned tree, so tile crumbs keep addressing the tree rather than this
/// ranked window.
fn entry_value(node: &Node, index: usize) -> UiValue {
    map(vec![
        ("index", UiValue::Integer(index as i64)),
        ("name", UiValue::String(node.name.to_string())),
        ("kind", UiValue::String(kind_key(node.kind).to_owned())),
        ("bytes", number(node.bytes)),
        ("own_bytes", number(node.own_bytes)),
        ("files", number(node.files)),
        ("dirs", number(node.dirs)),
        ("modified", UiValue::Integer(node.modified)),
        (
            "category",
            UiValue::String(node.category.key().to_owned()),
        ),
        (
            "reclaim",
            match node.reclaim {
                Some(reason) => UiValue::String(reason.key().to_owned()),
                None => UiValue::Null,
            },
        ),
        ("read_error", UiValue::Bool(node.read_error)),
    ])
}

fn space_value(info: &space::SpaceInfo) -> UiValue {
    map(vec![
        ("total", number(info.total)),
        ("free", number(info.free)),
        ("available", number(info.available)),
    ])
}

fn insight_value(candidate: &insights::Candidate) -> UiValue {
    let (kind, detail) = match &candidate.finding {
        Finding::Reclaimable(reason) => ("reclaimable", reason.label().to_owned()),
        Finding::Worktrees { count, oldest_days } => {
            ("worktrees", format!("{count} · oldest {oldest_days} d"))
        }
        Finding::StaleExperiments { count } => {
            ("stale_experiments", format!("{count} untouched"))
        }
    };
    map(vec![
        (
            "crumbs",
            UiValue::Array(
                candidate
                    .crumbs
                    .iter()
                    .map(|index| UiValue::Integer(*index as i64))
                    .collect(),
            ),
        ),
        ("bytes", number(candidate.bytes)),
        ("kind", UiValue::String(kind.to_owned())),
        ("detail", UiValue::String(detail)),
    ])
}

/// The `disktree.scan` subscription: owns a walk, forwards progress.
struct ScanCapability {
    tree: SharedTree,
}

impl SubscriptionCapabilityHandler for ScanCapability {
    fn subscribe(&mut self, method: &str, input: UiValue) -> Result<SubscriptionWork, String> {
        if method != "start" {
            return Err(format!("unknown disktree.scan method `{method}`"));
        }
        let UiValue::Map(fields) = input else {
            return Err("scan.start expects a map".to_owned());
        };
        let path = match fields.get("path") {
            Some(UiValue::String(path)) => PathBuf::from(path),
            _ => return Err("scan.start expects `path`".to_owned()),
        };
        let flag = |name: &str| {
            matches!(fields.get(name), Some(UiValue::Bool(true)))
        };
        let metric = match fields.get("metric") {
            Some(UiValue::String(label)) => Metric::parse(label)
                .ok_or_else(|| format!("unknown metric `{label}`"))?,
            _ => Metric::Bytes,
        };
        let options = ScanOptions {
            apparent_size: flag("apparent_size"),
            include_hidden: !flag("no_hidden"),
            metric,
            ..ScanOptions::default()
        };

        let receiver = forward_scan(path, options, self.tree.clone());
        Ok(SubscriptionWork::from_receiver(receiver))
    }
}

/// Spawn a scan and forward its lifetime as UiValue events on a channel:
/// throttled `progress`, then one `done`/`cancelled`/`failed` event. A
/// dropped receiver (the subscription stopping) cancels the walk.
fn forward_scan(
    root_for_device: PathBuf,
    options: ScanOptions,
    tree: SharedTree,
) -> mpsc::Receiver<UiValue> {
    let handle = ScanHandle::spawn(root_for_device.clone(), options);
    let (sender, receiver) = mpsc::channel::<UiValue>();

    std::thread::Builder::new()
        .name("disktree-rhai-scan-forwarder".into())
        .spawn(move || {
            run_forwarder(handle, root_for_device, tree, sender);
        })
        .expect("scan forwarder thread");
    receiver
}

fn run_forwarder(
    handle: ScanHandle,
    root_for_device: PathBuf,
    tree: SharedTree,
    sender: mpsc::Sender<UiValue>,
) {
    let started = Instant::now();
    let mut last_progress = Instant::now() - PROGRESS_INTERVAL - Duration::from_millis(1);
    loop {
        let snapshot = handle.progress.snapshot();
        if last_progress.elapsed() >= PROGRESS_INTERVAL {
            let event = map(vec![
                ("type", UiValue::String("progress".into())),
                ("files", number(snapshot.files)),
                ("dirs", number(snapshot.dirs)),
                ("bytes", number(snapshot.bytes)),
                ("errors", number(snapshot.errors)),
            ]);
            if sender.send(event).is_err() {
                handle.cancel();
                return;
            }
            last_progress = Instant::now();
        }
        if let Some(outcome) = handle.poll() {
            let event = match outcome {
                Ok(node) if !snapshot.cancelled && !handle.progress.is_cancelled() => {
                    let space = space::space_info(&root_for_device).ok();
                    let space_event = space.map(|info| space_value(&info));
                    let device = device_of(&root_for_device);
                    let device_event = device.clone();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|since| since.as_secs() as i64)
                        .unwrap_or(0);
                    let findings = insights::worth_a_look(&node, now, 6);
                    let scanned = crate::engine::ScannedTree {
                        root_path: root_for_device.clone(),
                        node: std::sync::Arc::new(node),
                        space,
                        device,
                        errors: snapshot.errors,
                        messages: snapshot.messages.clone(),
                    };
                    let done = map(vec![
                        ("type", UiValue::String("done".into())),
                        ("root", UiValue::String(root_for_device.display().to_string())),
                        ("files", number(snapshot.files)),
                        ("dirs", number(snapshot.dirs)),
                        ("bytes", number(snapshot.bytes)),
                        ("errors", number(snapshot.errors)),
                        (
                            "messages",
                            UiValue::Array(
                                snapshot
                                    .messages
                                    .iter()
                                    .take(5)
                                    .map(|message| UiValue::String(message.clone()))
                                    .collect(),
                            ),
                        ),
                        ("space", space_event.unwrap_or(UiValue::Null)),
                        (
                            "device",
                            match &device_event {
                                Some(device) => UiValue::String(device.clone()),
                                None => UiValue::Null,
                            },
                        ),
                        (
                            "insights",
                            UiValue::Array(
                                findings.iter().map(insight_value).collect(),
                            ),
                        ),
                        (
                            "elapsed_ms",
                            UiValue::Integer(started.elapsed().as_millis() as i64),
                        ),
                        // Wall clock at scan time: the Age color mode and
                        // "last write" figures age entries from it.
                        (
                            "now",
                            UiValue::Integer(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|since| since.as_secs() as i64)
                                    .unwrap_or(0),
                            ),
                        ),
                    ]);
                    *tree.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        Some(std::sync::Arc::new(scanned));
                    done
                }
                Ok(_) => map(vec![
                    ("type", UiValue::String("cancelled".into())),
                    ("files", number(snapshot.files)),
                    ("dirs", number(snapshot.dirs)),
                    ("bytes", number(snapshot.bytes)),
                    ("errors", number(snapshot.errors)),
                ]),
                Err(error) => map(vec![
                    ("type", UiValue::String("failed".into())),
                    ("error", UiValue::String(error.to_string())),
                ]),
            };
            let _ = sender.send(event);
            return;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// The `disktree.tree` sync capability: the layout's only data path.
struct TreeCapability {
    tree: SharedTree,
    filter_cache: FilterCache,
}

impl CapabilityHandler for TreeCapability {
    fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String> {
        match method {
            "children" => self.children(input),
            "path" => self.path(input),
            "resolve" => self.resolve(input),
            "matches" => self.matches_summary(input),
            other => Err(format!("unknown disktree.tree method `{other}`")),
        }
    }
}

/// The matches cached for one layout pass: a needle plus the subtree it was
/// searched in. Deeper layout levels re-search their (much smaller) kept
/// subtrees, so one entry is enough.
#[derive(Default)]
struct FilterCache {
    needle: String,
    base: Vec<usize>,
    tree_ptr: usize,
    matches: Option<crate::engine::filter::Matches>,
}

impl FilterCache {
    /// Compute or reuse the matches for one (needle, base) pair on the
    /// current tree. A rescan invalidates by identity.
    fn matches_for(
        &mut self,
        scanned: &std::sync::Arc<crate::engine::ScannedTree>,
        needle: &str,
        base: &[usize],
        node: &Node,
    ) -> Option<crate::engine::filter::Matches> {
        if self.needle == needle
            && self.base == base
            && self.tree_ptr == std::sync::Arc::as_ptr(scanned) as usize
        {
            return self.matches.clone();
        }
        let matches = crate::engine::filter::filter(node, base, needle);
        self.needle = needle.to_owned();
        self.base = base.to_vec();
        self.tree_ptr = std::sync::Arc::as_ptr(scanned) as usize;
        self.matches = matches.clone();
        matches
    }
}

/// Crumbs out of a capability input map.
fn crumbs_of(fields: &std::collections::BTreeMap<String, UiValue>) -> Result<Vec<usize>, String> {
    match fields.get("crumbs") {
        Some(UiValue::Array(items)) => items
            .iter()
            .map(|item| match item {
                UiValue::Integer(index) => Ok(*index as usize),
                _ => Err("crumbs entries must be integers".to_owned()),
            })
            .collect(),
        _ => Err("expected `crumbs`".to_owned()),
    }
}

impl TreeCapability {
    fn current(&self) -> Result<std::sync::Arc<crate::engine::ScannedTree>, String> {
        self.tree
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .ok_or_else(|| "no scan has finished yet".to_owned())
    }

    /// The find field's live summary: how much the needle matches under
    /// `crumbs`, before Enter applies it to the layout.
    fn matches_summary(&mut self, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Map(fields) = input else {
            return Err("tree.matches expects a map".to_owned());
        };
        let crumbs = crumbs_of(&fields)?;
        let needle = match fields.get("needle") {
            Some(UiValue::String(needle)) => needle.clone(),
            _ => return Err("tree.matches expects `needle`".to_owned()),
        };
        let scanned = self.current()?;
        if needle.trim().is_empty() {
            return Ok(UiValue::Null);
        }
        let Some(node) = scanned.node.resolve(&crumbs) else {
            return Ok(UiValue::Null);
        };
        let matches = self
            .filter_cache
            .matches_for(&scanned, &needle, &crumbs, node);
        Ok(match matches {
            Some(found) => map(vec![
                ("count", number(found.count as u64)),
                ("bytes", number(found.bytes)),
                ("files", number(found.files)),
                ("needle", UiValue::String(found.needle.clone())),
            ]),
            None => UiValue::Null,
        })
    }

    fn children(&mut self, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Map(fields) = input else {
            return Err("tree.children expects a map".to_owned());
        };
        let crumbs = match fields.get("crumbs") {
            Some(UiValue::Array(items)) => items
                .iter()
                .map(|item| match item {
                    UiValue::Integer(index) => Ok(*index as usize),
                    _ => Err("crumbs entries must be integers".to_owned()),
                })
                .collect::<Result<Vec<usize>, String>>()?,
            _ => return Err("tree.children expects `crumbs`".to_owned()),
        };
        let metric = match fields.get("metric") {
            Some(UiValue::String(label)) => Metric::parse(label)
                .ok_or_else(|| format!("unknown metric `{label}`"))?,
            _ => Metric::Bytes,
        };
        let max = match fields.get("max") {
            Some(UiValue::Integer(value)) if *value > 0 => *value as usize,
            _ => 96,
        };
        let scanned = self.current()?;
        let node = scanned
            .node
            .resolve(&crumbs)
            .ok_or_else(|| "crumbs do not resolve in the current tree".to_owned())?;

        // With a needle, this directory's children are the filtered view:
        // matches keep their own value, directories holding matches shrink
        // to what matched, everything else drops. A directory whose own name
        // matches shows its whole subtree, unfiltered (tobi's Keep::Whole).
        let needle = match fields.get("needle") {
            Some(UiValue::String(needle)) if !needle.trim().is_empty() => {
                Some(needle.clone())
            }
            _ => None,
        };
        if let Some(needle) = needle {
            return self.children_filtered(&scanned, node, crumbs, needle, metric, max);
        }

        // Rank by the requested metric ourselves: the tree is stored sorted
        // by the scan metric, and a metric switch must not produce a bad
        // layout window.
        let mut ranked: Vec<(usize, &Node)> = node
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.value(metric) > 0)
            .collect();
        ranked.sort_by(|(_, left), (_, right)| {
            right
                .value(metric)
                .cmp(&left.value(metric))
                .then_with(|| left.name.cmp(&right.name))
        });
        let kept = ranked.len().min(max);
        let tail_count = ranked.len() - kept;
        let mut tail_value: u64 = 0;
        for (_, child) in &ranked[kept..] {
            tail_value = tail_value.saturating_add(child.value(metric));
        }

        Ok(map(vec![
            (
                "entries",
                UiValue::Array(
                    ranked[..kept]
                        .iter()
                        .map(|(index, node)| entry_value(node, *index))
                        .collect(),
                ),
            ),
            ("tail_count", number(tail_count as u64)),
            ("tail_value", number(tail_value)),
            ("read_error", UiValue::Bool(node.read_error)),
        ]))
    }

    /// The filtered children of `node` at `crumbs`: entries carry the value
    /// they are laid out by (their matched totals for partial keeps).
    fn children_filtered(
        &mut self,
        scanned: &std::sync::Arc<crate::engine::ScannedTree>,
        node: &Node,
        crumbs: Vec<usize>,
        needle: String,
        metric: Metric,
        max: usize,
    ) -> Result<UiValue, String> {
        use crate::engine::filter::{contains_ignoring_case, Keep};

        // A directory that itself matches shows everything beneath it.
        if contains_ignoring_case(&node.name, &needle) {
            let mut ranked: Vec<(usize, &Node)> = node
                .children
                .iter()
                .enumerate()
                .filter(|(_, child)| child.value(metric) > 0)
                .collect();
            ranked.sort_by(|(_, left), (_, right)| {
                right
                    .value(metric)
                    .cmp(&left.value(metric))
                    .then_with(|| left.name.cmp(&right.name))
            });
            let kept = ranked.len().min(max);
            return Ok(map(vec![
                (
                    "entries",
                    UiValue::Array(
                        ranked[..kept]
                            .iter()
                            .map(|(index, node)| entry_value(node, *index))
                            .collect(),
                    ),
                ),
                (
                    "tail_count",
                    number((ranked.len() - kept) as u64),
                ),
                (
                    "tail_value",
                    number(
                        ranked[kept..]
                            .iter()
                            .map(|(_, child)| child.value(metric))
                            .fold(0u64, u64::saturating_add),
                    ),
                ),
                ("read_error", UiValue::Bool(node.read_error)),
            ]));
        }

        let matches = self
            .filter_cache
            .matches_for(scanned, &needle, &crumbs, node);
        let Some(matches) = matches else {
            return Ok(map(vec![
                ("entries", UiValue::Array(Vec::new())),
                ("tail_count", number(0)),
                ("tail_value", number(0)),
                ("read_error", UiValue::Bool(node.read_error)),
            ]));
        };

        let mut ranked: Vec<(usize, &Node, u64, u64, u64)> = Vec::new();
        for (index, child) in node.children.iter().enumerate() {
            let mut crumbs_child = crumbs.clone();
            crumbs_child.push(index);
            let Some(keep) = matches.keep(&crumbs_child) else {
                continue;
            };
            let (bytes, files) = match keep {
                Keep::Whole => (child.bytes, child.files),
                Keep::Partial { bytes, files } => (bytes, files),
            };
            let value = match metric {
                Metric::Bytes => bytes,
                Metric::Files => files,
            };
            if value > 0 {
                ranked.push((index, child, bytes, files, value));
            }
        }
        ranked.sort_by(|left, right| {
            right.4.cmp(&left.4).then_with(|| left.1.name.cmp(&right.1.name))
        });
        let kept = ranked.len().min(max);
        let tail_count = ranked.len() - kept;
        let tail_value = ranked[kept..].iter().map(|entry| entry.4).fold(0u64, u64::saturating_add);

        Ok(map(vec![
            (
                "entries",
                UiValue::Array(
                    ranked[..kept]
                        .iter()
                        .map(|(index, child, bytes, files, _)| {
                            let mut entry = entry_value(child, *index);
                            if let UiValue::Map(fields) = &mut entry {
                                // The sizes the filtered layout and its labels
                                // show: the matched totals, not the whole
                                // subtree.
                                fields.insert("bytes".to_owned(), number(*bytes));
                                fields.insert("files".to_owned(), number(*files));
                            }
                            entry
                        })
                        .collect(),
                ),
            ),
            ("tail_count", number(tail_count as u64)),
            ("tail_value", number(tail_value)),
            ("read_error", UiValue::Bool(node.read_error)),
        ]))
    }

    fn path(&self, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Map(fields) = input else {
            return Err("tree.path expects a map".to_owned());
        };
        let crumbs = match fields.get("crumbs") {
            Some(UiValue::Array(items)) => items
                .iter()
                .map(|item| match item {
                    UiValue::Integer(index) => Ok(*index as usize),
                    _ => Err("crumbs entries must be integers".to_owned()),
                })
                .collect::<Result<Vec<usize>, String>>()?,
            _ => return Err("tree.path expects `crumbs`".to_owned()),
        };
        let scanned = self.current()?;
        let path = crate::engine::node::path_of(&scanned.root_path, &scanned.node, &crumbs);
        Ok(UiValue::String(path.display().to_string()))
    }

    /// Path to crumbs: history and metric switches re-rank children, so
    /// positions move; names do not. `()` when the path left the tree (a
    /// rescan without it, or a different root).
    fn resolve(&self, input: UiValue) -> Result<UiValue, String> {
        let UiValue::String(wanted) = input else {
            return Err("tree.resolve expects a path string".to_owned());
        };
        let scanned = self.current()?;
        let wanted_path = Path::new(&wanted);
        let mut crumbs: Vec<usize> = Vec::new();
        let mut node: &Node = &scanned.node;
        let components = wanted_path
            .strip_prefix(&scanned.root_path)
            .map(|rest| rest.components().collect::<Vec<_>>())
            .map_err(|_| "path is outside the scanned root".to_owned())?;
        for component in components {
            let name = match component.as_os_str().to_str() {
                Some(name) => name,
                // A non-UTF-8 name can never round-trip through rhai strings;
                // treat it as gone rather than failing the whole resolve.
                None => return Ok(UiValue::Null),
            };
            let Some(index) = node.children.iter().position(|child| child.name.as_ref() == name)
            else {
                return Ok(UiValue::Null);
            };
            node = &node.children[index];
            crumbs.push(index);
        }
        Ok(UiValue::Array(
            crumbs.into_iter().map(|index| UiValue::Integer(index as i64)).collect(),
        ))
    }
}

/// The `disktree.open` task capability: show a path in the file manager.
struct OpenCapability;

impl AsyncCapabilityHandler for OpenCapability {
    fn start(&mut self, method: &str, input: UiValue) -> Result<gpui_rhai::TaskWork, String> {
        if method != "reveal" {
            return Err(format!("unknown disktree.open method `{method}`"));
        }
        let UiValue::String(path) = input else {
            return Err("open.reveal expects a path string".to_owned());
        };
        Ok(gpui_rhai::TaskWork::new(move || {
            // xdg-open on a directory shows it in the file manager; on a
            // file, opens it in its default app, like tobi's `o`.
            let target = Path::new(&path);
            let shown = if target.is_dir() {
                target.to_path_buf()
            } else {
                target
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| target.to_path_buf())
            };
            std::process::Command::new("xdg-open")
                .arg(&shown)
                .spawn()
                .map(|_| UiValue::Null)
                .map_err(|error| format!("xdg-open {}: {error}", shown.display()))
        }))
    }
}

/// The `disktree.args` sync capability: what the user asked for on the
/// command line, so a skin can start from the same place the binary did.
struct ArgsCapability {
    args: UiValue,
}

impl CapabilityHandler for ArgsCapability {
    fn call(&mut self, method: &str, _input: UiValue) -> Result<UiValue, String> {
        match method {
            "get" => Ok(self.args.clone()),
            // The user's home, for `~/` path display.
            "home" => Ok(std::env::var_os("HOME")
                .map(|home| UiValue::String(home.to_string_lossy().into_owned()))
                .unwrap_or(UiValue::String(String::new()))),
            other => Err(format!("unknown disktree.args method `{other}`")),
        }
    }
}

/// Registers every disktree capability. Attach with `.extension(...)` on
/// the view builder before `prepare()`.
pub struct DisktreeExtension {
    tree: SharedTree,
    args: UiValue,
}

impl DisktreeExtension {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: std::sync::Arc::new(std::sync::Mutex::new(None)),
            args: UiValue::Map(BTreeMap::new()),
        }
    }

    /// The resolved CLI request: scan root and the display/measurement
    /// options a skin should start from.
    #[must_use]
    pub fn with_args(
        mut self,
        path: PathBuf,
        apparent_size: bool,
        no_hidden: bool,
        depth: u32,
        metric: &str,
        theme_mode: &str,
    ) -> Self {
        self.args = map(vec![
            ("path", UiValue::String(path.display().to_string())),
            ("apparent_size", UiValue::Bool(apparent_size)),
            ("no_hidden", UiValue::Bool(no_hidden)),
            ("depth", UiValue::Integer(depth as i64)),
            ("metric", UiValue::String(metric.to_owned())),
            ("theme_mode", UiValue::String(theme_mode.to_owned())),
        ]);
        self
    }
}

impl Default for DisktreeExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptViewExtension for DisktreeExtension {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        // The engine-owned data store skins read and write: the scan state
        // the subscription maintains, and the CLI args read at init. Store
        // writes are schema-checked, so the contract is declared here, not
        // invented by skins.
        let anything = ValueSchema::Map {
            values: Box::new(ValueSchema::UiValue),
        };
        runtime
            .stores
            .declare(
                StoreId::app("disktree"),
                ComponentStateSchema::new(BTreeMap::from([
                    (
                        "args".to_owned(),
                        StateField::new(anything.clone(), UiValue::Map(BTreeMap::new())),
                    ),
                    (
                        "scan".to_owned(),
                        StateField::new(anything.clone(), UiValue::Map(BTreeMap::new())),
                    ),
                    // The skin's cached layout (tiles) and cursor state
                    // (hover/selection). Engine-neutral: skins may store
                    // their own view state here.
                    (
                        "layout".to_owned(),
                        StateField::new(anything.clone(), UiValue::Map(BTreeMap::new())),
                    ),
                    (
                        "cursor".to_owned(),
                        StateField::new(anything, UiValue::Map(BTreeMap::new())),
                    ),
                ]))
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;

        let scan_id = CapabilityId::parse("disktree.scan").map_err(|e| e.to_string())?;
        runtime
            .capabilities
            .register_subscription(
                CapabilityDescriptor {
                    id: scan_id,
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "start".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Object {
                                fields: BTreeMap::from([
                                    (
                                        "path".to_owned(),
                                        ObjectField::required(ValueSchema::string()),
                                    ),
                                    (
                                        "apparent_size".to_owned(),
                                        ObjectField::optional(ValueSchema::Bool),
                                    ),
                                    (
                                        "no_hidden".to_owned(),
                                        ObjectField::optional(ValueSchema::Bool),
                                    ),
                                    (
                                        "metric".to_owned(),
                                        ObjectField::optional(ValueSchema::string()),
                                    ),
                                ]),
                                allow_unknown: false,
                            },
                            output: ValueSchema::Map {
                                values: Box::new(ValueSchema::UiValue),
                            },
                        },
                    )]),
                },
                ScanCapability {
                    tree: self.tree.clone(),
                },
            )
            .map_err(|e| e.to_string())?;

        let tree_id = CapabilityId::parse("disktree.tree").map_err(|e| e.to_string())?;
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: tree_id,
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([
                        (
                            "children".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Object {
                                    fields: BTreeMap::from([
                                        (
                                            "crumbs".to_owned(),
                                            ObjectField::required(ValueSchema::Array {
                                                items: Box::new(ValueSchema::integer()),
                                                max_items: Some(512),
                                            }),
                                        ),
                                        (
                                            "metric".to_owned(),
                                            ObjectField::optional(ValueSchema::string()),
                                        ),
                                        (
                                            "max".to_owned(),
                                            ObjectField::optional(ValueSchema::integer()),
                                        ),
                                        (
                                            "needle".to_owned(),
                                            ObjectField::optional(ValueSchema::string()),
                                        ),
                                    ]),
                                    allow_unknown: false,
                                },
                                output: ValueSchema::Map {
                                    values: Box::new(ValueSchema::UiValue),
                                },
                            },
                        ),
                        (
                            "matches".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Object {
                                    fields: BTreeMap::from([
                                        (
                                            "crumbs".to_owned(),
                                            ObjectField::required(ValueSchema::Array {
                                                items: Box::new(ValueSchema::integer()),
                                                max_items: Some(512),
                                            }),
                                        ),
                                        (
                                            "needle".to_owned(),
                                            ObjectField::required(ValueSchema::string()),
                                        ),
                                    ]),
                                    allow_unknown: false,
                                },
                                output: ValueSchema::UiValue,
                            },
                        ),
                        (
                            "path".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Object {
                                    fields: BTreeMap::from([(
                                        "crumbs".to_owned(),
                                        ObjectField::required(ValueSchema::Array {
                                            items: Box::new(ValueSchema::integer()),
                                            max_items: Some(512),
                                        }),
                                    )]),
                                    allow_unknown: false,
                                },
                                output: ValueSchema::string(),
                            },
                        ),
                        (
                            "resolve".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                // Array of indices, or () when the path left
                                // the tree.
                                output: ValueSchema::UiValue,
                            },
                        ),
                    ]),
                },
                TreeCapability {
                    tree: self.tree.clone(),
                    filter_cache: FilterCache::default(),
                },
            )
            .map_err(|e| e.to_string())?;

        let open_id = CapabilityId::parse("disktree.open").map_err(|e| e.to_string())?;
        runtime
            .capabilities
            .register_async(
                CapabilityDescriptor {
                    id: open_id,
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "reveal".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::string(),
                            output: ValueSchema::Null,
                        },
                    )]),
                },
                OpenCapability,
            )
            .map_err(|e| e.to_string())?;

        let args_id = CapabilityId::parse("disktree.args").map_err(|e| e.to_string())?;
        runtime
            .capabilities
            .register(
                CapabilityDescriptor {
                    id: args_id,
                    version: Version::new(1, 0, 0),
                methods: BTreeMap::from([
                    (
                        "get".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Null,
                            output: ValueSchema::Map {
                                values: Box::new(ValueSchema::UiValue),
                            },
                        },
                    ),
                    (
                        "home".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Null,
                            output: ValueSchema::string(),
                        },
                    ),
                ]),
                },
                ArgsCapability {
                    args: self.args.clone(),
                },
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// What the root's filesystem is mounted from (e.g. `/dev/nvme0n1p2`):
/// the longest mount-point prefix in `/proc/self/mounts`.
fn device_of(path: &Path) -> Option<String> {
    let table = std::fs::read_to_string("/proc/self/mounts").ok()?;
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    space::parse_mounts(&table)
        .into_iter()
        .filter(|mount| canonical.starts_with(&mount.point))
        .max_by_key(|mount| mount.point.as_os_str().len())
        .map(|mount| mount.source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn map_get<'a>(value: &'a UiValue, key: &str) -> &'a UiValue {
        match value {
            UiValue::Map(fields) => fields
                .get(key)
                .unwrap_or_else(|| panic!("no field {key}")),
            other => panic!("not a map: {other:?}"),
        }
    }

    fn fixture() -> tempfile::TempDir {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = temp.path();
        fs::create_dir_all(root.join(".cache/kache")).expect("mkdir");
        fs::create_dir_all(root.join("src")).expect("mkdir");
        fs::write(root.join(".cache/blob.bin"), vec![0u8; 8192]).expect("write");
        fs::write(root.join("src/main.rs"), vec![0u8; 4096]).expect("write");
        temp
    }

    /// The subscription emits progress and then done, and done carries the
    /// totals, space info, and insights the panel renders.
    #[test]
    fn scan_subscription_reports_progress_then_done() {
        let temp = fixture();
        let tree: SharedTree = Default::default();
        let receiver = forward_scan(
            temp.path().to_path_buf(),
            ScanOptions {
                apparent_size: true,
                ..Default::default()
            },
            tree.clone(),
        );
        let mut done = None;
        for event in receiver {
            match map_get(&event, "type") {
                UiValue::String(kind) if kind == "progress" => {
                    assert!(matches!(map_get(&event, "files"), UiValue::Integer(_)));
                }
                UiValue::String(kind) if kind == "done" => {
                    done = Some(event);
                    break;
                }
                other => panic!("unexpected event type: {other:?}"),
            }
        }
        let done = done.expect("a done event");
        assert_eq!(map_get(&done, "files"), &UiValue::Integer(2));
        assert_eq!(map_get(&done, "bytes"), &UiValue::Integer(12_288));
        assert!(matches!(map_get(&done, "space"), UiValue::Map(_)));
        assert!(matches!(map_get(&done, "insights"), UiValue::Array(_)));

        // The tree slot holds the finished scan for the sync capability.
        let scanned = tree.lock().unwrap().clone().expect("tree stored");
        assert_eq!(scanned.node.bytes, 12_288);
        assert!(scanned.node.child_named(".cache").is_some());
    }

    /// A failed scan (a file as root) reports failure instead of done.
    #[test]
    fn scan_subscription_reports_failure() {
        let temp = fixture();
        let file = temp.path().join("src/main.rs");
        let receiver = forward_scan(file, Default::default(), Default::default());
        let last = receiver.into_iter().last().expect("an event");
        assert_eq!(map_get(&last, "type"), &UiValue::String("failed".into()));
    }

    fn crumbs(values: &[i64]) -> UiValue {
        UiValue::Array(values.iter().map(|v| UiValue::Integer(*v)).collect())
    }

    /// tree.children ranks by the requested metric, caps the window, and
    /// merges the tail; tree.path resolves crumbs to an absolute path.
    #[test]
    fn tree_children_windows_and_paths() {
        let temp = fixture();
        let node = crate::engine::scan::scan(
            temp.path(),
            ScanOptions {
                apparent_size: true,
                ..Default::default()
            },
        )
        .expect("scan");
        let root = temp.path().canonicalize().expect("canonical");
        let tree: SharedTree = std::sync::Arc::new(std::sync::Mutex::new(Some(
            std::sync::Arc::new(crate::engine::ScannedTree {
                root_path: root.clone(),
                node: std::sync::Arc::new(node),
                space: None,
                device: None,
                errors: 0,
                messages: Vec::new(),
            }),
        )));
        let mut capability = TreeCapability { tree, filter_cache: FilterCache::default() };

        let reply = capability
            .call(
                "children",
                map(vec![
                    ("crumbs", crumbs(&[])),
                    ("metric", UiValue::String("size".into())),
                    ("max", UiValue::Integer(1)),
                ]),
            )
            .expect("children");
        let entries = match map_get(&reply, "entries") {
            UiValue::Array(entries) => entries.clone(),
            other => panic!("entries: {other:?}"),
        };
        assert_eq!(entries.len(), 1, "the max window");
        // .cache (8 KiB) outranks src (4 KiB).
        assert_eq!(map_get(&entries[0], "name"), &UiValue::String(".cache".into()));
        assert_eq!(map_get(&entries[0], "kind"), &UiValue::String("dir".into()));
        assert_eq!(map_get(&entries[0], "category"), &UiValue::String("cache".into()));
        // The tail merges what the window cut off.
        assert_eq!(map_get(&reply, "tail_count"), &UiValue::Integer(1));
        assert_eq!(map_get(&reply, "tail_value"), &UiValue::Integer(4096));

        // Deeper crumbs rank within .cache and carry the reclaim reason.
        let reply = capability
            .call(
                "children",
                map(vec![
                    ("crumbs", crumbs(&[0])),
                    ("metric", UiValue::String("size".into())),
                ]),
            )
            .expect("children");
        let entries = match map_get(&reply, "entries") {
            UiValue::Array(entries) => entries.clone(),
            other => panic!("entries: {other:?}"),
        };
        assert_eq!(map_get(&entries[0], "name"), &UiValue::String("blob.bin".into()));
        assert_eq!(
            map_get(&entries[0], "reclaim"),
            &UiValue::String("regenerable".into())
        );

        let path = capability
            .call("path", map(vec![("crumbs", crumbs(&[0, 0]))]))
            .expect("path");
        assert_eq!(
            path,
            UiValue::String(root.join(".cache/blob.bin").display().to_string())
        );
    }
}

// Audit-only fixture. Original forwarder logic above is unchanged.
pub fn audit_cancel_publication() -> bool {
    fn one(disconnect:bool)->std::path::PathBuf {
        let shared:SharedTree=Default::default();
        let (handle,result)=ScanHandle::audit_pending_handle();
        let (sender,receiver)=std::sync::mpsc::channel();
        let observed=shared.clone();
        let worker=std::thread::spawn(move||run_forwarder(handle,
            std::path::PathBuf::from("/audit/nonexistent-old-generation"),observed,sender));
        let first=receiver.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let UiValue::Map(first)=first else{panic!()};
        assert_eq!(first["type"],UiValue::String("progress".to_owned()));
        // Newer scan B has won before old scan A completes.
        *shared.lock().unwrap()=Some(std::sync::Arc::new(crate::engine::ScannedTree{
            root_path:std::path::PathBuf::from("/audit/new-generation"),node:std::sync::Arc::new(Node::directory("new")),
            space:None,device:None,errors:0,messages:vec![]}));
        let receiver=if disconnect{drop(receiver);None}else{Some(receiver)};
        result.send(Ok(Node::directory("old"))).unwrap();
        worker.join().unwrap();
        drop(receiver);
        let path=shared.lock().unwrap().as_ref().unwrap().root_path.clone();path
    }
    let connected=one(false);
    assert_eq!(connected,std::path::PathBuf::from("/audit/nonexistent-old-generation"));
    println!("control: connected consumer allows publication: {}",connected.display());
    let disconnected=one(true);
    println!("cancelled old producer: expected /audit/new-generation, actual {}",disconnected.display());
    disconnected==std::path::PathBuf::from("/audit/new-generation")
}

pub fn audit_path_roundtrip() -> bool {
    use std::os::unix::ffi::OsStringExt;
    let raw=std::ffi::OsString::from_vec(vec![b'a',0xff]);
    let original=std::path::PathBuf::from("/audit/root").join(&raw);
    let mut root=Node::directory("root");
    root.children.push(Node::entry(raw.to_string_lossy().into_owned(),NodeKind::File,1));
    let resolved=crate::engine::node::path_of(std::path::Path::new("/audit/root"),&root,&[0]);
    let invalid=crate::engine::node::path_of(std::path::Path::new("/audit/root"),&root,&[99]);
    println!("non-UTF8 path: original={original:?}, reconstructed={resolved:?}, equal={}",original==resolved);
    println!("invalid crumb [99] silently resolves to {invalid:?}");
    original==resolved
}
