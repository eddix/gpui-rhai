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
    Init,
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
}

fn run(cli: Cli) -> Result<(), ProjectError> {
    let registry = BundledRegistry::load()?;
    let root = cli.root.clone();
    let project = Project::new(cli.root);
    match cli.command {
        Command::Init => {
            let plan = project.plan_init()?;
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
    }
    Ok(())
}

fn main() -> ExitCode {
    let result = if std::env::var("GPUI_RHAI_THEME_STUDIO").is_ok_and(|value| value == "1") {
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
