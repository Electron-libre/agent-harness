//! Helpers of the tests that run the `harness` binary on temp dirs, offline. The cache, the user settings, a failing
//! `cargo` and a `harness` that knows every subcommand live in a temp home: the prefetch of the tools binary (BIN-13),
//! the retention warning (SYN-28), the hooks `check` judges (BIN-15) and git never reach outside it. The fakes are
//! shell scripts: these tests run on Unix.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

pub const CONFIG: &str = ".claude/harness.toml";
pub const LOCK: &str = ".claude/harness.lock";

pub fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn get(dir: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(dir.join(rel)).ok()
}

/// What one run of the binary gave.
#[derive(Debug)]
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The lines of stdout, in order.
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// The first word of the outcome printed for `path` (`added`, `conflict`…), if any.
    pub fn outcome(&self, path: &str) -> Option<&str> {
        let suffix = format!(": {path}");
        self.stdout
            .lines()
            .find_map(|line| line.strip_suffix(&suffix))
            .map(|outcome| outcome.split(' ').next().unwrap())
    }
}

/// The home the binary runs in: its cache, its user settings (a year of retention) and its `PATH`.
pub struct Home {
    dir: TempDir,
}

impl Home {
    pub fn new() -> Self {
        let dir = tempdir().unwrap();
        put(
            dir.path(),
            "config/settings.json",
            r#"{"cleanupPeriodDays": 365}"#,
        );
        // the prefetch of a git source would build the tools binary with `cargo install`: it fails at once here;
        // the `harness` on the path knows every subcommand, so `check` refuses no hook whatever is installed (BIN-15)
        for (program, code) in [("cargo", 1), ("harness", 0)] {
            let rel = format!("bin/{program}");
            put(dir.path(), &rel, &format!("#!/bin/sh\nexit {code}\n"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::Permissions::from_mode(0o755);
                fs::set_permissions(dir.path().join(rel), mode).unwrap();
            }
        }
        Self { dir }
    }

    /// `harness <args> --root <root>`.
    pub fn harness(&self, root: &Path, args: &[&str]) -> Run {
        let mut path = vec![self.dir.path().join("bin")];
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_harness"));
        cmd.args(args)
            .arg("--root")
            .arg(root)
            .env("PATH", std::env::join_paths(path).unwrap())
            .env("XDG_CACHE_HOME", self.dir.path().join("cache"))
            .env("CLAUDE_CONFIG_DIR", self.dir.path().join("config"))
            .env_remove("CI")
            .env_remove("CLAUDE_PROJECT_DIR");
        let out = self.isolated(&mut cmd).output().unwrap();
        Run {
            code: out.status.code(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    }

    /// `git <args>` in `dir` with a test identity, which must succeed; its trimmed stdout.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new("git");
        cmd.args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir);
        let out = self.isolated(&mut cmd).output().unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// `cmd` with this home and no git configuration nor repository of the caller: a test run from a git hook or
    /// with a global `core.hooksPath` or `tag.gpgSign` neither touches the real repository nor fails.
    pub fn isolated<'a>(&self, cmd: &'a mut Command) -> &'a mut Command {
        cmd.env("HOME", self.dir.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"));
        for var in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_OBJECT_DIRECTORY",
            "GIT_COMMON_DIR",
            "GIT_CONFIG",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            cmd.env_remove(var);
        }
        cmd
    }

    /// `harness <args>`, which must exit 0.
    pub fn ok(&self, root: &Path, args: &[&str]) -> Run {
        let run = self.harness(root, args);
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        run
    }
}

/// What a path of a tree holds.
#[derive(Debug, PartialEq, Eq)]
pub enum Node {
    Dir,
    File(Vec<u8>, bool),
    Link(PathBuf),
}

/// Every directory, file (bytes and executable bit) and symlink under `dir`: two equal snapshots are an identical tree.
pub fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Node> {
    let mut nodes = BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(next) = todo.pop() {
        for entry in fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(dir).unwrap().to_path_buf();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            let node = if kind.is_symlink() {
                Node::Link(fs::read_link(&path).unwrap())
            } else if kind.is_dir() {
                todo.push(path);
                Node::Dir
            } else {
                Node::File(fs::read(&path).unwrap(), executable(&path))
            };
            nodes.insert(rel, node);
        }
    }
    nodes
}

#[cfg(unix)]
pub fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub fn executable(_path: &Path) -> bool {
    false
}

/// A source with `files/a.txt`, `files/sub/b.txt` and `seed/s.txt`, and a project initialised on it.
pub fn project(home: &Home) -> (TempDir, TempDir) {
    let source = tempdir().unwrap();
    put(source.path(), "files/a.txt", "a1");
    put(source.path(), "files/sub/b.txt", "b1");
    put(source.path(), "seed/s.txt", "seed1");
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    (source, root)
}
