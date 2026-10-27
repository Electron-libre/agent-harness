//! The launcher of `harness run` (spec `specs/tool-binaries.md`): fake `curl` and `cargo` on no network.
#![cfg(unix)]

use std::ffi::OsString;
use std::io::Write;
use std::process::Stdio;

use super::*;
use crate::run::{binary_of, command, target, Programs};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE: &str = "https://github.com/owner/repo.git";

/// Write an executable `sh` script. A child process writes it, so that no write descriptor of this process, which
/// another test thread's fork could inherit, keeps it busy when it is executed (`ETXTBSY`).
fn script(path: &Path, body: &str) -> OsString {
    let draft = path.with_extension("draft");
    fs::write(&draft, format!("#!/bin/sh\n{body}")).unwrap();
    let copied = Command::new("cp").arg(&draft).arg(path).status().unwrap();
    assert!(copied.success());
    set_executable(path, true).unwrap();
    path.as_os_str().to_owned()
}

/// A fake tools binary (`harness tool <name> <code> ...`): prints its arguments and stdin, and exits with `<code>`.
const TOOL: &str = "echo \"args: $*\"; cat; echo \"to stderr\" >&2; exit \"$3\"\n";

/// A `curl` that serves the files of `served` by the last part of the URL, logging each call to `log`.
fn fake_curl(dir: &Path, served: &Path, log: &Path) -> OsString {
    script(
        &dir.join("curl"),
        &format!(
            r#"out=""; url=""
while [ $# -gt 0 ]; do case "$1" in --output) out="$2"; shift 2;; -*) shift;; *) url="$1"; shift;; esac; done
echo "$url" >> {log}
file={served}/"${{url##*/}}"
[ -f "$file" ] || exit 22
if [ -n "$out" ]; then cat "$file" > "$out"; else cat "$file"; fi
"#,
            log = log.display(),
            served = served.display()
        ),
    )
}

/// A `cargo` whose `install` writes the fake tool under `--root`, logging each call to `log`.
fn fake_cargo(dir: &Path, log: &Path) -> OsString {
    script(
        &dir.join("cargo"),
        &format!(
            r#"echo "$*" >> {log}
root=""; while [ $# -gt 0 ]; do case "$1" in --root) root="$2"; shift 2;; *) shift;; esac; done
mkdir -p "$root/bin"; printf '#!/bin/sh\n{TOOL}' > "$root/bin/harness"
"#,
            log = log.display()
        ),
    )
}

struct Fixture {
    dir: TempDir,
    cache: PathBuf,
    served: PathBuf,
    log: PathBuf,
    programs: Programs,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempdir().unwrap();
        let cache = dir.path().join("cache");
        let served = dir.path().join("served");
        fs::create_dir_all(&served).unwrap();
        let log = dir.path().join("log");
        fs::write(&log, "").unwrap();
        let programs = Programs {
            curl: fake_curl(dir.path(), &served, &log),
            cargo: dir.path().join("no-cargo").into_os_string(),
        };
        Self {
            dir,
            cache,
            served,
            log,
            programs,
        }
    }

    fn with_cargo(mut self) -> Self {
        self.programs.cargo = fake_cargo(self.dir.path(), &self.log);
        self
    }

    /// Publish `binary` as the asset of the platform, with the digest `digest` (its own when `None`).
    fn publish(&self, binary: &str, digest: Option<&str>) {
        let asset = format!("harness-{}", target());
        fs::write(self.served.join(&asset), binary).unwrap();
        let digest = digest.map_or_else(|| hash(binary.as_bytes()), str::to_string);
        fs::write(
            self.served.join(format!("{asset}.sha256")),
            format!("{digest}  {asset}\n"),
        )
        .unwrap();
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap()
    }

    fn cached(&self) -> PathBuf {
        self.cache
            .join("harness/bin")
            .join(COMMIT)
            .join(target())
            .join("harness")
    }

    /// Nothing but the binary, if any, in the directory of the cached binary.
    fn leftovers(&self) -> Vec<String> {
        let dir = self.cached().parent().unwrap().to_path_buf();
        fs::read_dir(dir).map_or_else(
            |_| Vec::new(),
            |d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n != "harness")
                    .collect()
            },
        )
    }

    /// A project pinned to `COMMIT` of `SOURCE`.
    fn project(&self) -> PathBuf {
        let root = self.dir.path().join("project");
        fs::create_dir_all(root.join(".claude/sub")).unwrap();
        fs::write(
            root.join(CONFIG),
            format!("[harness]\nsource = \"{SOURCE}\"\nrev = \"{COMMIT}\"\n"),
        )
        .unwrap();
        root
    }
}

fn run_with_stdin(mut cmd: Command, stdin: &str) -> (i32, String, String) {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

// spec: BIN-1, BIN-2
#[test]
fn run_executes_the_cached_binary_of_the_pinned_commit_with_no_fetch() {
    let f = Fixture::new();
    let cached = f.cached();
    fs::create_dir_all(cached.parent().unwrap()).unwrap();
    script(&cached, TOOL);
    let root = f.project();

    let cmd = command(
        &root.join(".claude/sub"),
        None,
        Some(&f.cache),
        &f.programs,
        &["my-tool".into(), "3".into(), "--flag".into()],
    )
    .unwrap();
    let (code, stdout, stderr) = run_with_stdin(cmd, "the input\n");

    assert_eq!(code, 3);
    assert_eq!(stdout, "args: tool my-tool 3 --flag\nthe input\n");
    assert_eq!(stderr, "to stderr\n");
    assert_eq!(f.log(), "", "no download nor build");
}

// spec: BIN-3, BIN-4
#[test]
fn a_missing_binary_is_downloaded_with_its_digest_and_installed_whole() {
    let f = Fixture::new();
    f.publish(&format!("#!/bin/sh\n{TOOL}"), None);

    let binary = binary_of(&f.cache, SOURCE, COMMIT, &f.programs).unwrap();

    assert_eq!(binary, f.cached());
    assert!(is_executable(&binary));
    let asset = format!(
        "https://github.com/owner/repo/releases/download/harness-{COMMIT}/harness-{}",
        target()
    );
    assert_eq!(f.log(), format!("{asset}\n{asset}.sha256\n"));
    assert!(f.leftovers().is_empty(), "{:?}", f.leftovers());
    // The next call finds it.
    binary_of(&f.cache, SOURCE, COMMIT, &f.programs).unwrap();
    assert_eq!(f.log().lines().count(), 2);
}

// spec: BIN-10
#[test]
fn an_asset_that_does_not_match_its_digest_is_not_installed() {
    let f = Fixture::new().with_cargo();
    f.publish("tampered", Some(&hash(b"genuine")));

    let err = binary_of(&f.cache, SOURCE, COMMIT, &f.programs).unwrap_err();

    assert!(err.to_string().contains(COMMIT), "{err}");
    assert!(!f.cached().exists());
    assert!(f.leftovers().is_empty(), "{:?}", f.leftovers());
    assert!(!f.log().contains("install"), "a mismatch is not built over");
}

// spec: BIN-5
#[test]
fn with_no_asset_the_binary_is_built_with_cargo_install() {
    let f = Fixture::new().with_cargo();

    let binary = binary_of(&f.cache, SOURCE, COMMIT, &f.programs).unwrap();

    assert!(is_executable(&binary));
    let install = f
        .log()
        .lines()
        .find(|l| l.starts_with("install"))
        .unwrap()
        .to_string();
    assert!(
        install.contains(&format!("--locked --git {SOURCE} --rev {COMMIT} --root")),
        "{install}"
    );
    assert!(f.leftovers().is_empty(), "{:?}", f.leftovers());
}

// spec: BIN-5
#[test]
fn a_source_not_on_github_is_built_without_download() {
    let f = Fixture::new().with_cargo();

    binary_of(&f.cache, "https://gitlab.com/o/r.git", COMMIT, &f.programs).unwrap();

    assert!(f.log().starts_with("install"), "{}", f.log());
}

// spec: BIN-11
#[test]
fn with_no_asset_nor_cargo_the_fetch_fails_naming_the_commit_the_platform_and_update() {
    let f = Fixture::new();

    let err = binary_of(&f.cache, SOURCE, COMMIT, &f.programs)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains(COMMIT) && err.contains(&target()) && err.contains("harness update"),
        "{err}"
    );
    assert!(!f.cached().exists());
    assert!(f.leftovers().is_empty(), "{:?}", f.leftovers());
}

/// A project on a local source whose fake `cargo build` (returned) reports the binary `built`, or fails when `built`
/// is `None`.
fn local_source(f: &Fixture, built: Option<&Path>) -> OsString {
    let source = f.dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    let body = match built {
        Some(built) => format!(
            "echo \"$*\" >> {log}\necho '{{\"reason\":\"compiler-artifact\",\"target\":{{\"name\":\"harness\"}},\"executable\":\"{exe}\"}}'\n",
            log = f.log.display(),
            exe = built.display()
        ),
        None => "echo 'error[E0425]: cannot find value' >&2; exit 101\n".to_string(),
    };
    let root = f.dir.path().join("project");
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(
        root.join(CONFIG),
        format!("[harness]\nsource = \"{}\"\n", source.display()),
    )
    .unwrap();
    script(&f.dir.path().join("cargo-build"), &body)
}

fn local_command(f: &Fixture, cargo: &OsString) -> Result<Command> {
    let programs = Programs {
        cargo: cargo.clone(),
        ..f.programs.clone()
    };
    command(
        &f.dir.path().join("project"),
        None,
        Some(&f.cache),
        &programs,
        &["t".into(), "0".into()],
    )
}

// spec: BIN-6
#[test]
fn a_local_source_runs_the_crate_it_builds() {
    let f = Fixture::new();
    let built = f.dir.path().join("target-harness");
    script(&built, TOOL);
    let cargo = local_source(&f, Some(&built));

    let cmd = local_command(&f, &cargo).unwrap();

    assert_eq!(Path::new(cmd.get_program()), built);
    let build = f.log();
    assert!(
        build.starts_with("build ") && build.contains("source/Cargo.toml"),
        "{build}"
    );
}

// spec: BIN-12
#[test]
fn a_local_source_that_does_not_build_runs_its_last_good_build() {
    let f = Fixture::new();
    let built = f.dir.path().join("target-harness");
    script(&built, TOOL);
    let cargo = local_source(&f, Some(&built));
    local_command(&f, &cargo).unwrap();

    local_source(&f, None);
    let cmd = local_command(&f, &cargo).unwrap();

    let program = PathBuf::from(cmd.get_program());
    assert_ne!(program, built);
    assert!(program.starts_with(&f.cache), "{}", program.display());
    let (code, stdout, _) = run_with_stdin(cmd, "");
    assert_eq!((code, stdout.as_str()), (0, "args: tool t 0\n"));
}

// spec: BIN-12
#[test]
fn a_local_source_that_never_built_fails_with_the_build_errors() {
    let f = Fixture::new();
    let cargo = local_source(&f, None);

    let err = local_command(&f, &cargo).unwrap_err().to_string();

    assert!(
        err.contains("error[E0425]") && err.contains("no earlier build"),
        "{err}"
    );
}

// spec: BIN-13
#[test]
fn a_failed_prefetch_is_a_warning() {
    let f = Fixture::new();

    let warning = prefetch(&f.cache, SOURCE, COMMIT, &f.programs).unwrap();

    assert!(
        warning.starts_with("warning: ") && warning.contains(COMMIT),
        "{warning}"
    );
    f.publish(&format!("#!/bin/sh\n{TOOL}"), None);
    assert_eq!(prefetch(&f.cache, SOURCE, COMMIT, &f.programs), None);
}

// spec: BIN-1
#[test]
fn a_working_directory_outside_any_project_falls_back_to_the_project_dir() {
    let f = Fixture::new();
    let cached = f.cached();
    fs::create_dir_all(cached.parent().unwrap()).unwrap();
    script(&cached, TOOL);
    let root = f.project();
    let elsewhere = tempdir().unwrap();

    let cmd = command(
        elsewhere.path(),
        Some(&root),
        Some(&f.cache),
        &f.programs,
        &["t".into(), "0".into()],
    )
    .unwrap();

    assert_eq!(Path::new(cmd.get_program()), cached);
    assert!(command(elsewhere.path(), None, Some(&f.cache), &f.programs, &[]).is_err());
}

/// Run `git args` in `dir`, as a test user.
fn git_ok(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
}

// spec: BIN-2
#[test]
fn a_rev_that_is_no_hash_resolves_in_the_cached_clone_without_fetching() {
    let f = Fixture::new();
    // a source with one commit on `main`, cloned into the cache as `sync` leaves it
    let origin = tempdir().unwrap();
    git_ok(origin.path(), &["init", "-q", "-b", "main"]);
    git_ok(origin.path(), &["commit", "-q", "--allow-empty", "-m", "x"]);
    let url = format!("file://{}", origin.path().display());
    let clone = fetch(&f.cache, &url).unwrap();
    let commit = git_output(&clone, &["rev-parse", "origin/main"]).unwrap();
    let cached = f
        .cache
        .join("harness/bin")
        .join(&commit)
        .join(target())
        .join("harness");
    fs::create_dir_all(cached.parent().unwrap()).unwrap();
    script(&cached, TOOL);
    let root = f.dir.path().join("project");
    fs::create_dir_all(root.join(".claude")).unwrap();
    let config = format!("[harness]\nsource = \"{url}\"\nrev = \"main\"\n");
    fs::write(root.join(CONFIG), config).unwrap();
    // the source is gone: a fetch would fail
    fs::remove_dir_all(origin.path()).unwrap();

    let cmd = command(&root, None, Some(&f.cache), &f.programs, &[]).unwrap();

    assert_eq!(Path::new(cmd.get_program()), cached);
}

// spec: BIN-5
#[test]
fn a_scp_style_source_is_built_from_its_ssh_url() {
    let f = Fixture::new().with_cargo();

    binary_of(&f.cache, "git@gitlab.com:o/r.git", COMMIT, &f.programs).unwrap();

    assert!(
        f.log().contains("--git ssh://git@gitlab.com/o/r.git --rev"),
        "{}",
        f.log()
    );
}
