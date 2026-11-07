use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use gpui_rhai_cli::{BundledRegistry, Project, ProjectError};

#[derive(Debug, Parser)]
#[command(name = "gpui-rhai", version, about)]
struct Cli {
    /// Cargo project root.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Print intended filesystem changes without writing.
    #[arg(long, global = true)]
    dry_run: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize GPUI Rhai files in a Cargo project.
    Init {
        /// Install a bundled application profile, such as `productivity`.
        #[arg(long)]
        profile: Option<String>,
        /// The GPUI fork to build on: `pre` (gpui-pre, the default) or `fast`
        /// (gpui-fast, for a binary that also links a gpui-fast UI).
        #[arg(long, default_value = "pre", value_parser = ["pre", "fast"])]
        backend: String,
    },
    /// Add source components and their dependencies, or Lucide icons.
    Add {
        /// Registry component identifiers or short names, or `lucide/<name>`
        /// for a Lucide icon in `ui/assets/lucide/`.
        components: Vec<String>,
    },
    /// Validate scripts, schemas, themes, and compatibility.
    Check,
    /// Run the Cargo application.
    Dev {
        /// Run the debug build inside a macOS application bundle, so system
        /// notifications and the Dock badge work.
        #[arg(long)]
        bundle: bool,
    },
    /// Build a macOS application bundle around the application's binaries,
    /// configured by `[package.metadata.gpui-rhai.bundle]` in Cargo.toml.
    Bundle(BundleArgs),
    /// Render the view offscreen and save PNGs, through the application's
    /// `examples/shot.rs` (macOS).
    ///
    /// The steps run in order: `--size WxH` (1280x800), `--theme
    /// Family/Variant` or `--theme Variant`, `--locale ID`, `--click NAME`,
    /// `--key CHORD` (such as `cmd-k`), `--wait MS`, `--out PATH` and `--steps
    /// FILE` (JSON lines: automation commands plus `click`, `keys`, `wait`,
    /// `capture`, `theme` and `locale`). Example: `gpui-rhai shot --theme
    /// Nord/Dark --click Settings --out settings.png`.
    Shot {
        /// The steps, passed on to `examples/shot.rs`.
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "STEPS"
        )]
        args: Vec<String>,
    },
    /// Compare installed source with its registry baseline.
    Diff,
    /// Merge source updates from the bundled registry.
    Update,
    /// Removed in 0.4.3; build.rs embeds ui/ through gpui-rhai-build.
    #[command(hide = true)]
    Embed,
    /// Generate schema metadata and basic editor snippets.
    Metadata,
    /// Write the agent skills `gpui-rhai` and `gpui-rhai-design` into a directory.
    ///
    /// Run it again after upgrading the CLI to refresh them: it replaces the
    /// files it wrote and leaves the rest of the directory alone.
    Skills {
        /// Skills directory, relative to the project root: `.claude/skills` for
        /// Claude Code, `.agents/skills` for Codex, Cursor and other agents.
        directory: PathBuf,
    },
    /// Open the first-party gpui-rhai theme editor and component specimen.
    ThemeStudio {
        /// Existing gpui-rhai theme to open. Omit to create a new draft.
        path: Option<PathBuf>,
    },
    /// Open the Gallery, the acceptance application of the design system.
    Gallery {
        /// Print the Gallery pages and bundled stories without creating a window.
        #[arg(long)]
        list: bool,
        /// Open one Gallery page ID.
        #[arg(long, default_value = gpui_rhai_cli::acceptance::DEFAULT_PAGE)]
        page: String,
        /// Select comfortable or compact density.
        #[arg(long, default_value = "comfortable")]
        density: String,
        /// Open one development story in a standalone window instead of the Gallery.
        #[arg(long)]
        story: Option<String>,
        /// Select a deterministic case within the story.
        #[arg(long, default_value = "basic")]
        case: String,
        /// Select a bundled theme slug.
        #[arg(long, default_value = "default-dark")]
        theme: String,
        /// Select en, zh-CN, or ar.
        #[arg(long, default_value = "en")]
        locale: String,
        /// Select normal, reduced, or none motion.
        #[arg(long, default_value = "normal")]
        motion: String,
    },
}

/// `gpui-rhai bundle` options; they take precedence over
/// `[package.metadata.gpui-rhai.bundle]`.
#[derive(Debug, clap::Args)]
struct BundleArgs {
    /// Display name; the package name by default.
    #[arg(long)]
    name: Option<String>,
    /// Reverse-DNS bundle identifier, such as `com.example.app`.
    #[arg(long)]
    identifier: Option<String>,
    /// Square PNG for the application icon, relative to the project root.
    #[arg(long)]
    icon: Option<PathBuf>,
    /// Code signing identity; ad-hoc (`-`) when absent.
    #[arg(long)]
    sign: Option<String>,
    /// Bundle a debug build instead of a release build.
    #[arg(long)]
    debug: bool,
}

fn run(cli: Cli) -> Result<(), ProjectError> {
    let registry = BundledRegistry::load()?;
    let root = cli.root.clone();
    let project = Project::new(cli.root);
    match cli.command {
        Command::Init { profile, backend } => {
            let backend = if backend == "fast" {
                gpui_rhai_cli::GpuiBackend::Fast
            } else {
                gpui_rhai_cli::GpuiBackend::Pre
            };
            let plan = project.plan_init_for(profile.as_deref(), backend)?;
            println!("{}", plan.summary());
            if !cli.dry_run {
                plan.apply()?;
            }
        }
        Command::Add { components } => {
            let plan = project.plan_add(&registry, &components)?;
            println!("{}", plan.summary());
            if !cli.dry_run {
                plan.apply()?;
            }
        }
        Command::Check => {
            let report = project.check()?;
            println!("{}", report.summary());
        }
        Command::Dev { bundle: false } => project.dev()?,
        Command::Dev { bundle: true } => project.dev_bundle()?,
        Command::Shot { args } => project.shot(&args)?,
        Command::Bundle(args) => {
            let options = gpui_rhai_cli::bundle::BundleOptions {
                name: args.name,
                identifier: args.identifier,
                icon: args.icon,
                sign: args.sign,
            };
            println!("{}", project.bundle(&options, !args.debug, cli.dry_run)?);
        }
        Command::Diff => println!("{}", project.diff()?.join("\n")),
        Command::Update => {
            let plan = project.plan_update(&registry)?;
            println!("{}", plan.summary());
            let conflicts = plan.conflicts().to_vec();
            let file_conflicts = plan.file_conflicts().to_vec();
            if !cli.dry_run {
                plan.apply()?;
            }
            if !conflicts.is_empty() {
                return Err(ProjectError::UpdateConflictsWritten(conflicts));
            }
            if !file_conflicts.is_empty() {
                return Err(ProjectError::UpdateFileConflictsWritten(file_conflicts));
            }
        }
        Command::Embed => return Err(ProjectError::EmbedRemoved),
        Command::Metadata => {
            let plan = project.plan_editor_metadata()?;
            println!("{}", plan.summary());
            if !cli.dry_run {
                plan.apply()?;
            }
        }
        Command::Skills { directory } => {
            let plan = project.plan_skills(&directory)?;
            println!("{}", plan.summary());
            if !cli.dry_run {
                plan.apply()?;
            }
        }
        Command::ThemeStudio { path } => {
            if cli.dry_run {
                println!("would open Theme Studio");
            } else {
                gpui_rhai_cli::theme_studio::run(root, path).map_err(ProjectError::ThemeStudio)?;
            }
        }
        Command::Gallery {
            list,
            page,
            density,
            story,
            case,
            theme,
            locale,
            motion,
        } => {
            if list {
                println!("{}", gallery_list_text());
            } else if cli.dry_run {
                println!("would open Gallery");
            } else {
                let motion = gpui_rhai_cli::gallery::parse_motion_preference(&motion)
                    .map_err(ProjectError::Gallery)?;
                launch_gallery(story, case, page, density, theme, locale, motion)?;
            }
        }
    }
    Ok(())
}

/// Gallery pages first, then the development stories.
fn gallery_list_text() -> String {
    let pages = gpui_rhai_cli::acceptance::page_ids()
        .into_iter()
        .map(|page| format!("page\t{page}"))
        .collect::<Vec<_>>()
        .join("\n");
    let stories = gpui_rhai_cli::gallery::list_text()
        .lines()
        .map(|line| format!("story\t{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{pages}\n{stories}")
}

fn launch_gallery(
    story: Option<String>,
    case: String,
    page: String,
    density: String,
    theme: String,
    locale: String,
    motion: gpui_rhai::MotionPreference,
) -> Result<(), ProjectError> {
    match story {
        Some(story) => gpui_rhai_cli::gallery::run(&gpui_rhai_cli::gallery::GalleryLaunch {
            story,
            case,
            theme,
            locale,
            motion,
        }),
        None => gpui_rhai_cli::acceptance::run(&gpui_rhai_cli::acceptance::AcceptanceLaunch {
            page,
            density,
            theme,
            locale,
            motion,
        }),
    }
    .map_err(ProjectError::Gallery)
}

fn main() -> ExitCode {
    let result = if std::env::var("GPUI_RHAI_GALLERY").is_ok_and(|value| value == "1") {
        let env = |name: &str, fallback: &str| {
            std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
        };
        gpui_rhai_cli::gallery::parse_motion_preference(&env("GPUI_RHAI_GALLERY_MOTION", "normal"))
            .map_err(ProjectError::Gallery)
            .and_then(|motion| {
                launch_gallery(
                    std::env::var("GPUI_RHAI_GALLERY_STORY").ok(),
                    env("GPUI_RHAI_GALLERY_CASE", "basic"),
                    env(
                        "GPUI_RHAI_GALLERY_PAGE",
                        gpui_rhai_cli::acceptance::DEFAULT_PAGE,
                    ),
                    env("GPUI_RHAI_GALLERY_DENSITY", "comfortable"),
                    env("GPUI_RHAI_GALLERY_THEME", "default-dark"),
                    env("GPUI_RHAI_GALLERY_LOCALE", "en"),
                    motion,
                )
            })
    } else if std::env::var("GPUI_RHAI_THEME_STUDIO").is_ok_and(|value| value == "1") {
        std::env::current_dir()
            .map_err(|error| ProjectError::ThemeStudio(error.to_string()))
            .and_then(|root| {
                gpui_rhai_cli::theme_studio::run(root, None).map_err(ProjectError::ThemeStudio)
            })
    } else {
        run(Cli::parse())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gpui-rhai: {error}");
            ExitCode::FAILURE
        }
    }
}
