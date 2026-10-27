//! The exit code of the binary on an error of `init`, `sync`, `update` and `check` (SYN-37): 3, never the 2 of a
//! refusal.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

/// The exit code and stderr of `harness <args>` run in `root`.
fn harness(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_harness"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("CLAUDE_CONFIG_DIR", root)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

// spec: SYN-37
#[test]
fn an_error_of_a_subcommand_exits_3_with_the_error_on_stderr() {
    let root = tempdir().unwrap();
    let source = tempdir().unwrap();
    for args in [&["sync"][..], &["update"][..], &["check"][..]] {
        let (code, stderr) = harness(root.path(), args);
        assert_eq!(code, Some(3), "{args:?}: {stderr}");
        assert!(stderr.contains("error:"), "{args:?}: {stderr}");
    }
    fs::create_dir_all(root.path().join(".claude")).unwrap();
    fs::write(root.path().join(".claude/harness.toml"), "[harness]\n").unwrap();
    let (code, stderr) = harness(root.path(), &["init", source.path().to_str().unwrap()]);
    assert_eq!(code, Some(3), "init: {stderr}");
    assert!(stderr.contains("error:"), "init: {stderr}");
}
