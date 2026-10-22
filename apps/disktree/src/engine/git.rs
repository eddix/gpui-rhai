//! What git says about a checkout the selection lands on.
//!
//! A faithful port of tobi/disktree's git.rs: "everything a clone can lose"
//! — uncommitted changes, stashes, commits nobody pushed — asked with
//! fsmonitor, hooks, pagers, signature verifiers and the network all off,
//! and not asked at all when the checkout names its own filter drivers.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// The git state of one checkout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitState {
    /// Paths `git status --porcelain` lists: changed, staged or untracked.
    /// `None` when the checkout names its own filter drivers.
    pub changed: Option<usize>,
    pub stashes: usize,
    /// Commits on HEAD nobody pushed. `None` without an upstream.
    pub unpushed: Option<usize>,
}

impl GitState {
    /// One short line: `clean, no stash`, `3 changed, 1 stash, 2 unpushed`.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(match self.changed {
            Some(0) => "clean".to_owned(),
            Some(count) => format!("{count} changed"),
            None => "changes unknown".to_owned(),
        });
        parts.push(match self.stashes {
            0 => "no stash".to_owned(),
            1 => "1 stash".to_owned(),
            count => format!("{count} stashes"),
        });
        match self.unpushed {
            Some(0) => {}
            Some(count) => parts.push(format!("{count} unpushed")),
            None => parts.push("no upstream".to_owned()),
        }
        parts.join(", ")
    }
}

/// Whether `path` is the top of a checkout: it has a `.git` directory, or
/// the `.git` file a worktree gets.
#[must_use]
pub fn is_checkout(path: &Path) -> bool {
    std::fs::symlink_metadata(path.join(".git")).is_ok()
}

/// Ask git. `None` when `path` is not a checkout or git is not installed.
#[must_use]
pub fn state(path: &Path) -> Option<GitState> {
    if !is_checkout(path) || !git_installed() {
        return None;
    }
    let changed = if names_its_own_filters(path) {
        None
    } else {
        // Submodules are other checkouts with configs of their own.
        let status = run(
            path,
            &["status", "--porcelain=v1", "-z", "--ignore-submodules=all"],
        )?;
        Some(status.split('\0').filter(|line| !line.is_empty()).count())
    };
    // Counted from the reflog, not listed: `stash list` is `git log`, which
    // verifies signatures with the checkout's `gpg.program` when its config
    // sets `log.showSignature`.
    let stashes = run(
        path,
        &["rev-list", "--walk-reflogs", "--count", "refs/stash"],
    )
    .and_then(|count| count.trim().parse().ok())
    .unwrap_or(0);
    let unpushed = run(path, &["rev-list", "--count", "@{upstream}..HEAD"])
        .and_then(|count| count.trim().parse().ok());
    Some(GitState {
        changed,
        stashes,
        unpushed,
    })
}

/// Whether running `git` runs git. On macOS `/usr/bin/git` is there even
/// without the developer tools, as a stub that opens an "install the command
/// line developer tools" dialog, which selecting a checkout must not do.
/// `xcode-select -p` answers without prompting. Asked once.
fn git_installed() -> bool {
    static INSTALLED: OnceLock<bool> = OnceLock::new();
    *INSTALLED.get_or_init(|| {
        if !cfg!(target_os = "macos") {
            return true;
        }
        Command::new("xcode-select")
            .arg("-p")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
            })
    })
}

/// Whether the checkout's own config defines filter drivers.
///
/// `git status` runs a file's `clean` filter to compare it with the index,
/// and a downloaded repository can name any program as one. No `-c` can
/// turn off drivers whose names are not known in advance, so a checkout
/// that defines any is not asked for its status at all. Filters from the
/// user's own config, such as git-lfs, are theirs and still run. Reading
/// config runs nothing; if it cannot be read, the answer is the cautious
/// one.
fn names_its_own_filters(path: &Path) -> bool {
    let Some(output) = git(
        path,
        &[
            "config",
            "--show-scope",
            "--includes",
            "--get-regexp",
            r"^filter\.",
        ],
    ) else {
        return true;
    };
    match output.status.code() {
        // No filter anywhere.
        Some(1) => false,
        Some(0) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| matches!(line.split('\t').next(), Some("local" | "worktree"))),
        _ => true,
    }
}

fn run(path: &Path, args: &[&str]) -> Option<String> {
    let output = git(path, args)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git(path: &Path, args: &[&str]) -> Option<std::process::Output> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(path)
        // A checkout's own config can name programs for git to run: an
        // fsmonitor on every `status`, hooks, a pager, a signature verifier,
        // and whatever a partial clone would run to fetch a missing object.
        // Selecting a directory in a disk viewer must not execute anything
        // it contains, and a command-line `-c` outranks the repository's
        // config. (From tobi/disktree#10.)
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.pager=cat",
            "-c",
            "log.showSignature=false",
            "-c",
            "gpg.program=false",
            "-c",
            "gpg.ssh.program=false",
            "-c",
            "gpg.x509.program=false",
            "-c",
            "core.sshCommand=false",
            "-c",
            "protocol.allow=never",
        ])
        .args(args)
        // Never prompt, never page, never take a lock for the index refresh.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .output()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checkout_reports_state_and_a_plain_dir_does_not() {
        // The app lives below the top of the gpui-rhai checkout.
        let top = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|dir| is_checkout(dir))
            .expect("the app lives in a checkout");
        let state = state(top).expect("git answers for a checkout");
        assert!(state.changed.is_some());
        assert!(
            state.summary().contains("changed") || state.summary().contains("clean"),
            "summary: {}",
            state.summary()
        );
        assert!(!is_checkout(Path::new("/tmp")));
    }
}
