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
    },
    /// Fail when a managed file was modified or deleted since the last sync.
    Check,
}

/// Print what changed; true when nothing is in conflict.
fn print_changes(report: &Report) -> bool {
    for (path, outcome) in &report.entries {
        if *outcome != Outcome::Unchanged {
            println!("{outcome}: {path}");
        }
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
        Command::Update { rev } => {
            update(&cli.root, rev.as_deref(), &SyncOptions::default()).map(|done| {
                println!(
                    "rev: {} -> {}",
                    done.previous.as_deref().unwrap_or("(none)"),
                    done.rev
                );
                print_changes(&done.sync)
            })
        }
        Command::Check => check(&cli.root).map(|problems| {
            for problem in &problems {
                println!("{problem}");
            }
            problems.is_empty()
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
