//! List the sources of the instruction audit: per project of the transcripts directory, the sessions, the
//! messages typed by the user, the memory files and the instruction files (`CLAUDE.md`, `AGENTS.md`) of its
//! working directory. With `--messages`, print the typed messages as `project<TAB>session<TAB>text`
//! (newlines escaped); subagent transcripts are skipped. Prints private content: never commit the output.
//! Usage: `harness run audit-sources [projects_dir] [--messages]` (default `~/.claude/projects`).

use super::transcripts;

use std::path::{Path, PathBuf};
use transcripts::{for_each_line, projects, projects_root, sessions, sorted_entries, typed_text};

/// One line per message: tab, carriage return and backslash are escaped along with the newline.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

const INSTRUCTION_FILES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    let print_messages = args.iter().any(|a| a == "--messages");
    if let Some(flag) = args
        .iter()
        .find(|a| a.starts_with('-') && *a != "--messages")
    {
        eprintln!(
            "unknown option {flag}\nusage: harness run audit-sources [projects_dir] [--messages]"
        );
        std::process::exit(2);
    }
    let root = match projects_root(args.iter().find(|a| !a.starts_with('-'))) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    for project in projects(&root) {
        let name = project.file_name().unwrap().to_string_lossy().into_owned();
        let (mut session_count, mut messages, mut cwds) = (0, 0, Vec::<String>::new());
        for session in sessions(&project) {
            session_count += 1;
            let stem = session.file_stem().unwrap().to_string_lossy().into_owned();
            let read = for_each_line(&session, |line| {
                if let Some(c) = line["cwd"]
                    .as_str()
                    .filter(|c| !cwds.iter().any(|k| k == c))
                {
                    cwds.push(c.to_string());
                }
                if let Some(t) = typed_text(line) {
                    messages += 1;
                    if print_messages {
                        println!("{name}\t{stem}\t{}", escape(&t));
                    }
                }
            });
            let skipped = match read {
                Ok(skipped) => skipped,
                Err(e) => {
                    eprintln!("cannot read {}: {e}", session.display());
                    continue;
                }
            };
            if skipped > 0 {
                eprintln!("{}: {skipped} unreadable lines skipped", session.display());
            }
        }
        if print_messages {
            continue;
        }
        println!("{name}: {session_count} sessions, {messages} messages");
        let is_note = |p: &PathBuf| {
            p.extension().is_some_and(|e| e == "md")
                && p.file_name().is_some_and(|n| n != "MEMORY.md")
        };
        let memory = sorted_entries(&project.join("memory"))
            .iter()
            .filter(|p| is_note(p))
            .count();
        if memory > 0 {
            println!("  memory: {memory} files");
        }
        for cwd in &cwds {
            for f in INSTRUCTION_FILES
                .iter()
                .filter(|f| Path::new(cwd).join(f).is_file())
            {
                println!("  instructions: {cwd}/{f}");
            }
        }
    }
    0
}
