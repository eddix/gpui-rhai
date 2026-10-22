//! The harness is macOS-only: see `macos.rs`. Elsewhere it exits with an
//! error rather than a silent pass, so the target still builds in Linux CI.

#[cfg(target_os = "macos")]
mod macos;

fn main() {
    #[cfg(target_os = "macos")]
    macos::main();
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("key_harness drives the macOS offscreen renderer; it needs macOS");
        std::process::exit(1);
    }
}
