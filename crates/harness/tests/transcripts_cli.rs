//! The default projects directory of the tools that read the transcripts (EVA-15), when the configuration directory
//! cannot be located or holds no `projects/`; `session-metrics` proves the default itself, the others share its
//! `projects_root`.

use std::process::{Command, Output};

/// `harness tool <name> <args>` with neither `CLAUDE_CONFIG_DIR` nor `HOME` set to a value.
fn unlocated(name: &str, args: &[&std::path::Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_harness"))
        .arg("tool")
        .arg(name)
        .args(args)
        .env("CLAUDE_CONFIG_DIR", "")
        .env_remove("HOME")
        .output()
        .unwrap()
}

// spec: EVA-15
#[test]
fn the_transcript_tools_fail_naming_both_variables() {
    for name in [
        "session-metrics",
        "rule-breaches",
        "fixed-context",
        "audit-sources",
    ] {
        let out = unlocated(name, &[]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{name}");
        assert!(
            stderr.contains(
                "the configuration directory cannot be located (set HOME or CLAUDE_CONFIG_DIR)"
            ),
            "{name}: {stderr}"
        );
    }
}

// spec: EVA-15
#[test]
fn delivered_quality_runs_without_the_transcripts() {
    let repo = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(repo.path())
        .status()
        .unwrap()
        .success());
    let out = unlocated("delivered-quality", &[repo.path()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// spec: EVA-15
#[test]
fn the_transcript_tools_warn_when_the_default_projects_directory_is_missing() {
    let config = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(repo.path())
        .status()
        .unwrap()
        .success());
    let missing = config.path().join("projects");
    for (name, args) in [
        ("session-metrics", vec![]),
        ("rule-breaches", vec![]),
        ("fixed-context", vec![]),
        ("audit-sources", vec![]),
        ("delivered-quality", vec![repo.path()]),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_harness"))
            .arg("tool")
            .arg(name)
            .args(args)
            .env("CLAUDE_CONFIG_DIR", config.path())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{name}: {stderr}");
        assert!(
            stderr.contains(&format!(
                "the projects directory {} is missing",
                missing.display()
            )),
            "{name}: {stderr}"
        );
    }
}
