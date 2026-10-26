//! The retention warning of the binary (SYN-28): once after `check`, `sync` and `update`, never after `init`.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

/// The stderr of `harness <args>` run in `root` with the user settings in `config`, outside CI.
fn stderr(root: &Path, config: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_harness"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("CLAUDE_CONFIG_DIR", config)
        .env_remove("CI")
        .output()
        .unwrap();
    String::from_utf8(out.stderr).unwrap()
}

fn retention_warnings(stderr: &str) -> usize {
    stderr.matches("the transcripts are deleted after").count()
}

// spec: SYN-28
#[test]
fn the_binary_warns_of_the_retention_once_after_check_and_sync_never_after_init() {
    let source = tempdir().unwrap();
    fs::create_dir_all(source.path().join("files")).unwrap();
    fs::write(source.path().join("files/a.txt"), "a").unwrap();
    let config = tempdir().unwrap();
    let root = tempdir().unwrap();

    let init = stderr(
        root.path(),
        config.path(),
        &["init", source.path().to_str().unwrap()],
    );
    assert_eq!(retention_warnings(&init), 0, "{init}");
    for args in [&["sync"][..], &["sync", "--dry-run"], &["check"]] {
        let out = stderr(root.path(), config.path(), args);
        assert_eq!(retention_warnings(&out), 1, "{args:?}: {out}");
    }
    // a local source has no revision: `update` fails, and says nothing of the retention
    let update = stderr(root.path(), config.path(), &["update"]);
    assert!(
        update.contains("error:") && retention_warnings(&update) == 0,
        "{update}"
    );

    fs::write(
        config.path().join("settings.json"),
        r#"{"cleanupPeriodDays": 365}"#,
    )
    .unwrap();
    assert_eq!(
        retention_warnings(&stderr(root.path(), config.path(), &["check"])),
        0
    );
}
