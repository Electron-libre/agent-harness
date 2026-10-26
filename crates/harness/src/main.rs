//! `harness`: copy the shared agent harness into a project and tell when a managed file drifted.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use harness::{check, init, sync, update, Outcome, Report, SyncOptions};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Project root (default: the current directory).
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create `.claude/harness.toml` pointing at a source (a directory, or a git URL with `--rev`).
    Init {
        source: String,
        /// Branch, tag or commit of a git source.
        #[arg(long)]
        rev: Option<String>,
    },
    /// Copy the managed files of the source into the project and seed the missing project-owned ones.
    Sync {
        /// Overwrite managed files modified locally.
        #[arg(long)]
        force: bool,
        /// Print what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Move the project to a revision of its git source (default: the head of its default branch), then sync.
    Update {
        /// Branch, tag or commit; `rev` stores the resolved commit.
        #[arg(long)]
        rev: Option<String>,
        /// Overwrite managed files modified locally.
        #[arg(long)]
        force: bool,
        /// Print the target revision and what would change, change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Fail when a managed file was modified or deleted since the last sync, or the git review gate is missing or edited (not looked at when `CI` is set); warn when the settings lack a deny rule of the review waiver, run a missing hook or leave the attribution on.
    Check,
}

/// Print what changed, and the warnings on the settings to stderr; true when nothing is in conflict.
fn print_changes(report: &Report) -> bool {
    for (path, outcome) in &report.entries {
        if *outcome != Outcome::Unchanged {
            println!("{outcome}: {path}");
        }
    }
    for warning in &report.warnings {
        eprintln!("{warning}");
    }
    report.conflicts() == 0
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Init { source, rev } => init(&cli.root, &source, rev.as_deref()).map(|()| true),
        Command::Sync { force, dry_run } => {
            sync(&cli.root, &SyncOptions { force, dry_run }).map(|report| print_changes(&report))
        }
        Command::Update {
            rev,
            force,
            dry_run,
        } => update(&cli.root, rev.as_deref(), &SyncOptions { force, dry_run }).map(|done| {
            println!(
                "rev: {} -> {}",
                done.previous.as_deref().unwrap_or("(none)"),
                done.rev
            );
            let clean = print_changes(&done.sync);
            if clean && dry_run {
                println!("dry run: nothing written");
            } else if !clean {
                println!("conflicts: nothing written, `rev` unchanged (fix them or use --force)");
            }
            clean
        }),
        Command::Check => check(&cli.root).map(|checked| {
            for problem in &checked.problems {
                println!("{problem}");
            }
            for warning in &checked.warnings {
                eprintln!("{warning}");
            }
            checked.problems.is_empty()
        }),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
