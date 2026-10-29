//! `.github/install_cardinal.sh`, the CI installer of the managed content, run with `sh` and a fake `cargo` and `curl`
//! first on the path.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::{output, put, script, Run, CONFIG};
use tempfile::tempdir;

/// Run the installer in `root` with fakes in `bin`: `cargo` logs its arguments to `<bin>/log`, `curl` finds nothing.
fn install(root: &Path, bin: &Path) -> Run {
    script(
        &bin.join("cargo"),
        &format!("echo \"$*\" >> '{}/log'\n", bin.display()),
    );
    script(&bin.join("curl"), "exit 22\n");
    let installer =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../files/.github/install_cardinal.sh");
    let mut cmd = Command::new("sh");
    let path = std::env::join_paths(std::iter::once(bin.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    cmd.arg(installer)
        .current_dir(root)
        .env("PATH", path)
        .env("HOME", bin)
        .env_remove("XDG_CACHE_HOME");
    output(cmd, "")
}

// spec: SYN-50
#[test]
fn a_path_git_source_is_installed_from_its_file_url_from_the_project_root() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("a bé%");
    let root = base.join("project");
    fs::create_dir_all(&root).unwrap();
    // the root is read with its symlinks resolved, a path source as written
    let physical = fs::canonicalize(dir.path()).unwrap();
    let encoded = "a%20b%C3%A9%25";
    for (source, url) in [
        (
            "../x.git".to_string(),
            format!("file://{}/{encoded}/x.git", physical.display()),
        ),
        (
            format!("{}/./x.git", base.display()),
            format!("file://{}/{encoded}/x.git", dir.path().display()),
        ),
        (
            "git@mirror/x.git".to_string(),
            format!(
                "file://{}/{encoded}/project/git%40mirror/x.git",
                physical.display()
            ),
        ),
        ("host:x.git".to_string(), "ssh://host/x.git".to_string()),
    ] {
        let bin = tempdir().unwrap();
        put(
            &root,
            CONFIG,
            &format!("[cardinal]\nsource = \"{source}\"\nrev = \"main\"\n"),
        );

        let run = install(&root, bin.path());

        assert_eq!(run.code, Some(0), "{source}: {run:?}");
        let log = fs::read_to_string(bin.path().join("log")).unwrap_or_default();
        assert_eq!(
            log,
            format!("install --quiet --force --git {url} --rev main --locked\n"),
            "{source}: {run:?}"
        );
    }
}

#[test]
fn a_work_tree_without_the_configuration_fails_naming_it() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stderr.contains(CONFIG), "{run:?}");
    assert!(!bin.path().join("log").exists(), "{run:?}");
}

#[test]
fn a_configuration_without_a_source_fails_naming_the_key_and_the_file() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();
    put(root.path(), CONFIG, "[cardinal]\nrev = \"main\"\n");

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stderr.contains("source"), "{run:?}");
    assert!(run.stderr.contains(CONFIG), "{run:?}");
    assert!(!bin.path().join("log").exists(), "{run:?}");
}

#[test]
fn a_literal_string_source_is_installed() {
    let root = tempdir().unwrap();
    let bin = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        "[cardinal]\nsource = 'host:x.git'\nrev = 'main'\n",
    );

    let run = install(root.path(), bin.path());

    assert_eq!(run.code, Some(0), "{run:?}");
    let log = fs::read_to_string(bin.path().join("log")).unwrap_or_default();
    assert_eq!(
        log, "install --quiet --force --git ssh://host/x.git --rev main --locked\n",
        "{run:?}"
    );
}
