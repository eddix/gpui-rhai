//! Scan a directory with the engine and print totals, for comparison with
//! `du -s -B1 <dir>` — the acceptance check for scan semantics.

use std::path::PathBuf;

use disktree_rhai::engine::scan::scan;

fn main() {
    let root = std::env::args()
        .filter(|a| !a.starts_with('-'))
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let started = std::time::Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let options = if args.iter().any(|a| a == "--apparent") {
        disktree_rhai::engine::scan::ScanOptions {
            apparent_size: true,
            ..Default::default()
        }
    } else {
        Default::default()
    };
    let tree = scan(&root, options).expect("scan");
    println!("engine_bytes: {}", tree.bytes);
    println!("files: {} dirs: {}", tree.files, tree.dirs);
    println!("in {:.1?}", started.elapsed());
}
