//! The engine: a faithful port of tobi/disktree's `disktree-core` for Linux.
//!
//! Split from the UI exactly like the original: scanning, tree aggregation,
//! classification, volume rules, and insights have no knowledge of gpui-rhai.
//! The capability layer (`crate::capability`) is the only bridge to skins.

pub mod classify;
pub mod filter;
pub mod insights;
pub mod node;
pub mod scan;
pub mod space;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use node::Node;

/// A finished scan, shared between the capability layer and the UI side.
pub struct ScannedTree {
    /// The scan root as the user spelled it.
    pub root_path: PathBuf,
    pub node: Arc<Node>,
    /// Space on the root's volume, for the Disk sidebar.
    pub space: Option<space::SpaceInfo>,
    /// What the root's filesystem is mounted from, e.g. `/dev/nvme0n1p2`.
    pub device: Option<String>,
    /// Errors from the scan, for the trail line ("N unreadable").
    pub errors: u64,
    pub messages: Vec<String>,
}

/// The one tree the app currently shows; replaced whole on every scan.
pub type SharedTree = Arc<Mutex<Option<Arc<ScannedTree>>>>;
