//! disktree-rhai — a disk space treemap explorer built on gpui-rhai.
//!
//! A faithful port of tobi/disktree whose entire UI is a Rhai skin. The Rust
//! side owns scanning (engine) and window plumbing; everything the user sees
//! is the default skin embedded from `ui/`, which users can export, modify,
//! and load back.

use disktree_rhai::capability::DisktreeExtension;
use disktree_rhai::embedded;
use std::path::{Path, PathBuf};

use gpui::{AppContext, IntoElement, ParentElement};
use gpui_rhai::{EmbeddedScriptView, FileScriptView, PreparedScriptView};

/// Script operation budget for this app's engine. The gpui-rhai default
/// (1M) fits UI callbacks but not the skin's cached one-shot layout pass:
/// a full-viewport depth-3 layout measured 2.7M ops / 78ms (see
/// examples/spike_squarify.rs and gpui-rhai PR #96). 10M bounds one-shot
/// phases with room to grow while leaving the frame path far under.
const SCRIPT_OPERATION_BUDGET: u64 = 10_000_000;

const USAGE: &str = "disktree-rhai — disk space as a squarified treemap, skinned in Rhai

USAGE:
    disktree-rhai [PATH] [OPTIONS]

ARGS:
    PATH                     Directory to scan (default: home)

OPTIONS:
    -a, --apparent-size      Measure apparent size instead of disk usage
    -H, --no-hidden          Skip dot files and directories
    -d, --depth N            Levels drawn at once, 1-6 (default 3)
        --metric files|bytes|size
                             Rank by file count or bytes (default size)
        --skin NAME|PATH     Load a skin: a directory (or name under
                             $XDG_CONFIG_HOME/disktree-rhai/skins/)
        --skin-dev PATH      Like --skin, with hot reload while editing
        --export-skin [NAME] Write the built-in skin to
                             $XDG_CONFIG_HOME/disktree-rhai/skins/NAME/ (default NAME: default)
        --theme dark|light   Theme variant for the active skin (default dark)
        --dev                Run the built-in ui/ from source with hot reload
    -h, --help               Print this help

The engine measures like du: allocated blocks (st_blocks x 512), hard links
counted once, single filesystem, symlinks not followed, hidden files included.";

struct Args {
    path: Option<PathBuf>,
    apparent_size: bool,
    no_hidden: bool,
    depth: Option<u32>,
    metric: Option<String>,
    skin: Option<String>,
    skin_dev: Option<String>,
    export_skin: Option<String>,
    theme: Option<String>,
    dev: bool,
    help: bool,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut args = Args {
        path: None,
        apparent_size: false,
        no_hidden: false,
        depth: None,
        metric: None,
        skin: None,
        skin_dev: None,
        export_skin: None,
        theme: None,
        dev: false,
        help: false,
    };
    let mut iter = argv.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => args.help = true,
            "-a" | "--apparent-size" => args.apparent_size = true,
            "-H" | "--no-hidden" => args.no_hidden = true,
            "--dev" => args.dev = true,
            "-d" | "--depth" => {
                let value = iter.next().ok_or("--depth needs a number (1-6)")?;
                args.depth = Some(value.parse::<u32>().map_err(|_| "invalid depth")?);
            }
            "--metric" => {
                let value = iter
                    .next()
                    .ok_or("--metric needs files|bytes|size")?
                    .to_owned();
                if !matches!(value.as_str(), "files" | "bytes" | "size") {
                    return Err(format!("unknown metric `{value}` (files|bytes|size)"));
                }
                args.metric = Some(value);
            }
            "--skin" => {
                args.skin = Some(iter.next().ok_or("--skin needs a name or path")?.to_owned());
            }
            "--skin-dev" => {
                args.skin_dev = Some(iter.next().ok_or("--skin-dev needs a path")?.to_owned());
            }
            "--export-skin" => {
                // Optional value: only swallow the next argv if it doesn't look
                // like a flag or a path to scan.
                args.export_skin = match iter.next() {
                    Some(next) if !next.starts_with('-') => Some(next.clone()),
                    _ => Some("default".to_owned()),
                };
            }
            "--theme" => {
                let value = iter
                    .next()
                    .ok_or("--theme needs dark|light|system")?
                    .to_owned();
                if !matches!(value.as_str(), "dark" | "light" | "system") {
                    return Err(format!("unknown theme `{value}` (dark|light|system)"));
                }
                args.theme = Some(value);
            }
            other => {
                if other.starts_with('-') {
                    return Err(format!("unknown option `{other}` — try --help"));
                }
                if args.path.is_some() {
                    return Err("only one path can be scanned".to_owned());
                }
                args.path = Some(PathBuf::from(other));
            }
        }
    }
    Ok(args)
}

fn config_root() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

/// Resolve a `--skin`/`--skin-dev` argument to a skin directory: a directory
/// path as given, a `main.rhai` file's parent, or a name under the config root.
fn resolve_skin(spec: &str) -> Result<PathBuf, String> {
    let direct = Path::new(spec);
    if direct.exists() {
        if direct.is_file() {
            return Ok(direct
                .parent()
                .ok_or_else(|| "skin file has no parent directory".to_owned())?
                .to_path_buf());
        }
        return Ok(direct.to_path_buf());
    }
    if spec.contains('/') || spec.contains('.') && direct.is_dir() {
        return Err(format!("skin path `{spec}` does not exist"));
    }
    let named = config_root().join("disktree-rhai").join("skins").join(spec);
    if named.join("main.rhai").exists() {
        Ok(named)
    } else {
        Err(format!(
            "no skin named `{spec}` under {}",
            config_root().join("disktree-rhai").join("skins").display()
        ))
    }
}

/// The colour mode to start in: an explicit `--theme` wins, then the
/// Omarchy theme (its `theme/colors.toml` states `mode = "dark|light"`),
/// then the dark default. Degrades silently when Omarchy is absent.
fn resolve_theme_mode(flag: &Option<String>) -> &'static str {
    if let Some(mode) = flag {
        return match mode.as_str() {
            "light" => "light",
            _ => "dark",
        };
    }
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return "dark";
    };
    for candidate in [".local/state/omarchy/current", ".config/omarchy/current"] {
        let colors = home.join(candidate).join("theme/colors.toml");
        let Ok(source) = std::fs::read_to_string(&colors) else {
            continue;
        };
        for line in source.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("mode") {
                let value = value.trim_start_matches(['=', ' ']).trim();
                if value == "\"light\"" || value == "light" {
                    return "light";
                }
                if value == "\"dark\"" || value == "dark" {
                    return "dark";
                }
            }
        }
    }
    "dark"
}

fn export_skin(name: &str) -> Result<(), String> {
    let target = config_root().join("disktree-rhai").join("skins").join(name);
    if target.exists() {
        return Err(format!(
            "skin `{name}` already exists at {} — pick another name or edit it in place",
            target.display()
        ));
    }
    std::fs::create_dir_all(&target)
        .map_err(|error| format!("create {}: {error}", target.display()))?;
    for (file, contents) in embedded::SKIN_FILES {
        let path = target.join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
        }
        std::fs::write(&path, contents)
            .map_err(|error| format!("write {}: {error}", path.display()))?;
    }
    println!("Exported the built-in skin to {}", target.display());
    println!("Edit it, then run: disktree-rhai --skin {}", name);
    Ok(())
}

/// The runtime's expression-depth defaults (64 global / 32 per function) are
/// below what this skin's deeply nested view functions need; 128 per function
/// covers the deepest chrome paths with headroom.
const EXPRESSION_DEPTH_LIMITS: (usize, usize) = (64, 128);

fn prepare_view(args: &Args) -> Result<PreparedScriptView, gpui_rhai::ScriptViewError> {
    // The scan root: the path the user gave, or home, canonicalized like
    // tobi does (dunce-free: plain canonicalize keeps symlinks resolved).
    let scan_root = args
        .path
        .clone()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| gpui_rhai::ScriptViewError::Io {
            path: PathBuf::from("<home>"),
            source: std::io::Error::other("no path given and no home directory"),
        })?;
    let metric = args.metric.clone().unwrap_or_else(|| "size".to_owned());
    let theme_mode = resolve_theme_mode(&args.theme).to_owned();
    let extension = || {
        DisktreeExtension::new().with_args(
            scan_root.clone(),
            args.apparent_size,
            args.no_hidden,
            args.depth.unwrap_or(3),
            &metric,
            &theme_mode,
        )
    };
    // History chords: alt-left / alt-right dispatch the disktree.back /
    // disktree.forward actions, which the skin registers in init.
    let history_bindings = || -> Result<Vec<gpui_rhai::KeyBindingSpec>, String> {
        let back = gpui_rhai::KeyBindingSpec::new(
            "alt-left",
            gpui_rhai::ActionId::parse("disktree.back").map_err(|error| error.to_string())?,
            None,
        )
        .map_err(|error| error.to_string())?;
        let forward = gpui_rhai::KeyBindingSpec::new(
            "alt-right",
            gpui_rhai::ActionId::parse("disktree.forward").map_err(|error| error.to_string())?,
            None,
        )
        .map_err(|error| error.to_string())?;
        Ok(vec![back, forward])
    };
    let history_bindings = match history_bindings() {
        Ok(bindings) => bindings,
        Err(error) => {
            return Err(gpui_rhai::ScriptViewError::Io {
                path: PathBuf::from("<key bindings>"),
                source: std::io::Error::other(error),
            });
        }
    };

    if args.dev {
        let entry = Path::new("ui").join("main.rhai");
        return FileScriptView::new(entry)
            .development(true)
            .viewport_breakpoints(
                gpui_rhai::ViewportBreakpoints::new(832.0, 1280.0).expect("valid breakpoints"),
            )
            .extension(extension())
            .operation_limit(SCRIPT_OPERATION_BUDGET)
            .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
            .key_binding(history_bindings[0].clone())
            .key_binding(history_bindings[1].clone())
            .prepare();
    }
    if let Some(spec) = args.skin_dev.as_ref().or(args.skin.as_ref()) {
        let skin_dir = resolve_skin(spec).map_err(|error| gpui_rhai::ScriptViewError::Io {
            path: spec.clone().into(),
            source: std::io::Error::other(error),
        })?;
        let hot = args.skin_dev.is_some();
        // A file skin brings its own tokens.rhai and assets/ beside main.rhai;
        // the host picks both up automatically.
        return FileScriptView::new(skin_dir.join("main.rhai"))
            .development(hot)
            .viewport_breakpoints(
                gpui_rhai::ViewportBreakpoints::new(832.0, 1280.0).expect("valid breakpoints"),
            )
            .extension(extension())
            .operation_limit(SCRIPT_OPERATION_BUDGET)
            .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
            .key_binding(history_bindings[0].clone())
            .key_binding(history_bindings[1].clone())
            .prepare();
    }
    let mut theme_sources = vec![(
        "themes/flexoki_light.rhai".to_owned(),
        embedded::FLEXOKI_LIGHT.to_owned(),
    )];
    theme_sources.extend(
        embedded::bundled_palette_sources()
            .into_iter()
            .map(|(name, source)| (name, source.to_owned())),
    );
    EmbeddedScriptView::new(embedded::entry(), embedded::sources(), embedded::THEME)
        .viewport_breakpoints(
            gpui_rhai::ViewportBreakpoints::new(832.0, 1280.0).expect("valid breakpoints"),
        )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .locale_sources([("en".to_owned(), embedded::EN_LOCALE.to_owned())])
        .theme_sources(theme_sources)
        .asset_sources(embedded::assets())
        .manifest(embedded::manifest())
        .extension(extension())
        .operation_limit(SCRIPT_OPERATION_BUDGET)
        .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
        .key_binding(history_bindings[0].clone())
        .key_binding(history_bindings[1].clone())
        .prepare()
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("disktree-rhai: {error} — try --help");
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{USAGE}");
        return;
    }
    if let Some(name) = args.export_skin.as_ref() {
        if let Err(error) = export_skin(name) {
            eprintln!("disktree-rhai: {error}");
            std::process::exit(1);
        }
        return;
    }

    let result = prepare_view(&args).and_then(run_window);
    if let Err(error) = result {
        eprintln!("disktree-rhai: {error}");
        std::process::exit(1);
    }
}

/// The window root: the mounted script view inside the host container, with
/// script errors mirrored to stderr so a failed skin is diagnosable outside
/// the window.
struct Shell {
    host: gpui_rhai::ScriptViewHost,
    view: Option<gpui_rhai::ScriptViewHandle>,
    reported: std::cell::RefCell<Option<String>>,
}

impl gpui::Render for Shell {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        if let Some(view) = &self.view
            && let Ok(Some(error)) = view.last_error(cx)
        {
            let mut reported = self.reported.borrow_mut();
            let text = error.clone();
            if reported.as_deref() != Some(text.as_str()) {
                eprintln!("disktree-rhai: script error: {error}");
                *reported = Some(text);
            }
        }
        self.host.container(self.view.as_ref().map_or_else(
            || gpui::div().child("no skin mounted").into_any_element(),
            |view| {
                view.element()
                    .unwrap_or_else(|error| gpui::div().child(error.to_string()).into_any_element())
            },
        ))
    }
}

fn run_window(prepared: gpui_rhai::PreparedScriptView) -> Result<(), gpui_rhai::ScriptViewError> {
    gpui_platform::application().run(move |cx: &mut gpui::App| {
        gpui_rhai::install(cx);
        // This host owns the window and mounts the view itself. 0.2 folds the
        // old new_with_policy(ApplicationOwned) into `new`: a host-created
        // window is application-owned, so the skin's `q` still closes it.
        let host = match gpui_rhai::ScriptViewHost::new("main", cx) {
            Ok(host) => host,
            Err(error) => {
                eprintln!("disktree-rhai: {error}");
                cx.quit();
                return;
            }
        };
        // History lives on modifier chords (alt-left / alt-right), which node
        // key handlers cannot express by design; the host owns the chord and
        // the skin owns the action.
        if !prepared.key_bindings().is_empty()
            && let Err(error) = host.bind_keys(prepared.key_bindings().iter().cloned(), cx)
        {
            eprintln!("disktree-rhai: key bindings rejected: {error}");
        }
        let shell = cx.new(|_| Shell {
            host: host.clone(),
            view: None,
            reported: std::cell::RefCell::new(None),
        });
        let options = gpui::WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                origin: gpui::Point::new(gpui::px(120.0), gpui::px(90.0)),
                size: gpui::size(gpui::px(1440.0), gpui::px(900.0)),
            })),
            // The skin draws its own title bar (the official TitleBar component,
            // with window dragging). A transparent-appearing titlebar keeps the
            // full-size content view AND the traffic lights overlaid —
            // `titlebar: None` drops the closable/minimizable masks and with
            // them the buttons entirely.
            titlebar: Some(gpui::TitlebarOptions {
                title: None,
                appears_transparent: true,
                ..gpui::TitlebarOptions::default()
            }),
            // The bar's drag areas move the window through the runtime's
            // start_window_move; AppKit must not also drag or delay titlebar
            // clicks disambiguating double-presses.
            app_owns_titlebar_drag: true,
            window_min_size: Some(gpui::size(gpui::px(900.0), gpui::px(600.0))),
            ..gpui::WindowOptions::default()
        };
        let opened = cx.open_window(options, |window, cx| {
            let root = shell.clone();
            let weak = root.downgrade();
            let host = host.clone();
            window.defer(cx, move |window, cx| {
                let mounted = prepared.mount(
                    gpui_rhai::ScriptViewConfig::new("main")
                        .paint_background(true)
                        // The skin draws its own TitleBar; its background
                        // drags the window (and double-press zooms it).
                        .window_drag_areas(true),
                    host,
                    window,
                    cx,
                );
                match mounted {
                    Ok(view) => {
                        let _ = view.focus(window, cx);
                        let _ = weak.update(cx, |shell, cx| {
                            shell.view = Some(view);
                            cx.notify();
                        });
                    }
                    Err(error) => eprintln!("disktree-rhai: mount failed: {error}"),
                }
            });
            root
        });
        if opened.is_err() {
            cx.quit();
            return;
        }
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
    Ok(())
}
