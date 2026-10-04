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
    },
    /// Add source components and their dependencies.
    Add {
        /// Registry component identifiers or short names.
        components: Vec<String>,
    },
    /// Validate scripts, schemas, themes, and compatibility.
    Check,
    /// Run the Cargo application.
    Dev,
    /// Compare installed source with its registry baseline.
    Diff,
    /// Merge source updates from the bundled registry.
    Update,
    /// Generate the production embedded-source Rust module.
    Embed,
    /// Generate schema metadata and basic editor snippets.
    Metadata,
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

fn run(cli: Cli) -> Result<(), ProjectError> {
    let registry = BundledRegistry::load()?;
    let root = cli.root.clone();
    let project = Project::new(cli.root);
    match cli.command {
        Command::Init { profile } => {
            let plan = project.plan_init_with_profile(profile.as_deref())?;
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
        Command::Dev => project.dev()?,
        Command::Diff => println!("{}", project.diff()?.join("\n")),
        Command::Update => {
            let plan = project.plan_update(&registry)?;
            println!("{}", plan.summary());
            let conflicts = plan.conflicts().to_vec();
            if !cli.dry_run {
                plan.apply()?;
            }
            if !conflicts.is_empty() {
                return Err(ProjectError::UpdateConflictsWritten(conflicts));
            }
        }
        Command::Embed => {
            let plan = project.plan_embed()?;
            println!("{}", plan.summary());
            if !cli.dry_run {
                plan.apply()?;
            }
        }
        Command::Metadata => {
            let plan = project.plan_editor_metadata()?;
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
