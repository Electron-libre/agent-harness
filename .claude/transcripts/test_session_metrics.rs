#!/usr/bin/env rust-script
//! Behavior test for `session_metrics.rs` on fixture transcripts: columns, deduplicated assistant messages,
//! sub-agents, typed messages, hook refusals, skills, project, tolerance to unknown lines, no message text.
//! Usage: `just test-harness`.
//! ```cargo
//! [dependencies]
//! sha2 = "0.10"
//! tempfile = "3"
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git").args(args).current_dir(dir).output().is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// A transcript line of `type` in `cwd` at `ts`, with extra fields (`,"k":v` form).
fn line(kind: &str, cwd: &Path, ts: &str, extra: &str) -> String {
    format!("{{\"type\":\"{kind}\",\"cwd\":{:?},\"timestamp\":\"{ts}\",\"sessionId\":\"x\"{extra}}}\n", cwd.display().to_string())
}

fn typed(cwd: &Path, ts: &str, text: &str) -> String {
    line("user", cwd, ts, &format!(",\"version\":\"2.1.1\",\"gitBranch\":\"main\",\"message\":{{\"role\":\"user\",\"content\":{text:?}}}"))
}

fn assistant(cwd: &Path, ts: &str, id: &str, model: &str, blocks: &str, usage: &str) -> String {
    line("assistant", cwd, ts, &format!(",\"version\":\"2.1.2\",\"gitBranch\":\"my_issue\",\"message\":{{\"id\":\"{id}\",\"model\":\"{model}\",\"content\":[{blocks}]{usage}}}"))
}

fn usage(input: u64, output: u64, thinking: Option<u64>, read: u64, creation: u64) -> String {
    let details = thinking.map(|t| format!(",\"output_tokens_details\":{{\"thinking_tokens\":{t}}}")).unwrap_or_default();
    format!(",\"usage\":{{\"input_tokens\":{input},\"output_tokens\":{output},\"cache_read_input_tokens\":{read},\"cache_creation_input_tokens\":{creation}{details}}}")
}

fn refusal(cwd: &Path, ts: &str, script: &str) -> String {
    error_result(cwd, ts, &format!("PreToolUse:Bash hook error: [rust-script \\\"$CLAUDE_PROJECT_DIR\\\"/{script} check]: SECRET refusal text"))
}

fn error_result(cwd: &Path, ts: &str, text: &str) -> String {
    line("user", cwd, ts, &format!(",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"t\",\"is_error\":true,\"content\":\"{text}\"}}]}}"))
}

const SKILL: &str = "{\"type\":\"tool_use\",\"id\":\"a\",\"name\":\"Skill\",\"input\":{\"skill\":\"commit-message\",\"args\":\"SECRET args\"}}";
const BASH: &str = "{\"type\":\"tool_use\",\"id\":\"b\",\"name\":\"Bash\",\"input\":{\"command\":\"echo SECRET\"}}";
const REVIEW: &str = "{\"type\":\"tool_use\",\"id\":\"c\",\"name\":\"Skill\",\"input\":{\"skill\":\"code-review\"}}";
const TEXT: &str = "{\"type\":\"text\",\"text\":\"SECRET answer\"}";

/// Rows of the output keyed by session, each a map column -> value; `None` when the header is wrong.
fn rows(out: &str) -> Option<HashMap<String, HashMap<String, String>>> {
    let mut lines = out.lines();
    let header: Vec<&str> = lines.next()?.split('\t').collect();
    if header.join("\t") != COLUMNS {
        return None;
    }
    Some(
        lines
            .map(|l| {
                let row: HashMap<String, String> = header.iter().map(|h| h.to_string()).zip(l.split('\t').map(str::to_string)).collect();
                (row.get("session").cloned().unwrap_or_default(), row)
            })
            .collect(),
    )
}

/// Builds the fixture projects directory; returns (projects dir, main repository).
fn fixture(tmp: &Path) -> (PathBuf, PathBuf) {
    let repo = tmp.join("myrepo");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    sh(&repo, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "init"]);
    let worktree = tmp.join("myrepo-my_issue");
    sh(&repo, &["worktree", "add", "-q", "-b", "my_issue", worktree.to_str().unwrap()]);
    let projects = tmp.join("projects");
    let proj = projects.join("-tmp-myrepo");
    std::fs::create_dir_all(proj.join("s1/subagents")).unwrap();
    let s1 = [
        typed(&worktree, "2026-10-26T10:00:00.000Z", "please fix SECRET"),
        typed(&worktree, "2026-10-26T10:00:05.000Z", "OK!"),
        typed(&worktree, "2026-10-26T10:00:06.000Z", "1 a, 2 c"),
        typed(&worktree, "2026-10-26T10:00:07.000Z", "c'est bon."),
        typed(&worktree, "2026-10-26T10:00:08.000Z", "4k"),
        typed(&worktree, "2026-10-26T10:00:09.000Z", "2a"),
        assistant(&worktree, "2026-10-26T10:00:10.000Z", "m1", "model-a", SKILL, &usage(10, 20, Some(5), 100, 7)),
        assistant(&worktree, "2026-10-26T10:00:11.000Z", "m1", "model-a", BASH, &usage(10, 20, Some(5), 100, 7)),
        assistant(&worktree, "2026-10-26T10:00:12.000Z", "m1", "model-a", TEXT, &usage(10, 20, Some(5), 100, 7)),
        assistant(&worktree, "2026-10-26T10:00:20.000Z", "m2", "model-b", BASH, &usage(1, 2, None, 3, 4)),
        refusal(&worktree, "2026-10-26T10:00:21.000Z", ".claude/hooks/require_code_review.rs"),
        refusal(&worktree, "2026-10-26T10:00:22.000Z", ".claude/hooks/require_code_review.rs"),
        refusal(&worktree, "2026-10-26T10:00:23.000Z", "nothing"),
        refusal(&worktree, "2026-10-26T10:00:23.500Z", "~/.rsync/notes.rst .claude/hooks/block_ai_attribution.rs"),
        error_result(&worktree, "2026-10-26T10:00:23.700Z", "PreToolUse:Bash failed: the hook error: was elsewhere"),
        line("brand-new-type", &worktree, "2026-10-26T10:00:24.000Z", ""),
        "{not json\n".to_string(),
        line("assistant", &worktree, "2026-10-26T10:00:25.000Z", ",\"message\":{\"id\":\"m9\",\"model\":\"model-a\",\"content\":[]}"),
        line("attachment", &worktree, "2026-10-26T11:00:00.000Z", ""),
        "{\"type\":\"ai-title\",\"sessionId\":\"x\"}\n".to_string(),
    ]
    .concat();
    std::fs::write(proj.join("s1.jsonl"), s1).unwrap();
    let sub = [
        line("user", &worktree, "2026-10-26T10:00:30.000Z", ",\"isSidechain\":true,\"message\":{\"role\":\"user\",\"content\":\"subagent SECRET prompt\"}"),
        assistant(&worktree, "2026-10-26T10:00:31.000Z", "m3", "model-c", REVIEW, &usage(1000, 0, None, 0, 0)),
        refusal(&worktree, "2026-10-26T10:00:32.000Z", ".claude/hooks/clippy_gate.rs"),
        "garbage\n".to_string(),
    ]
    .concat();
    std::fs::write(proj.join("s1/subagents/agent-x.jsonl"), sub).unwrap();
    let gone = Path::new("/nonexistent/elsewhere");
    let s2 = [typed(gone, "2026-10-26T09:00:00.000Z", "yes")].concat();
    std::fs::create_dir_all(projects.join("-nonexistent-elsewhere")).unwrap();
    std::fs::write(projects.join("-nonexistent-elsewhere/s2.jsonl"), s2).unwrap();
    let lib = tmp.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    sh(&lib, &["init", "-q", "-b", "main"]);
    sh(&lib, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "init"]);
    sh(&repo, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", lib.to_str().unwrap(), "vendor/lib"]);
    let s3 = typed(&repo.join("vendor/lib"), "2026-10-26T09:00:00.000Z", "yes");
    let removed = tmp.join("myrepo-gone_issue");
    let s4 = line("user", &removed, "2026-10-26T09:00:00.000Z", ",\"gitBranch\":\"gone_issue\",\"message\":{\"role\":\"user\",\"content\":\"ok\"}");
    std::fs::write(proj.join("s3.jsonl"), s3).unwrap();
    std::fs::write(proj.join("s4.jsonl"), s4).unwrap();
    (projects, repo)
}

// spec: EVA-1, EVA-2, EVA-3, EVA-4, EVA-5, EVA-6, EVA-7, EVA-9, EVA-10
fn row_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let (projects, _) = fixture(tmp);
    let out = Command::new("rust-script").arg(dir.join("session_metrics.rs")).arg(&projects).output().expect("rust-script must be installed");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let rows = rows(&stdout).unwrap_or_default();
    let empty = HashMap::new();
    let s1 = rows.get("s1").unwrap_or(&empty);
    let s2 = rows.get("s2").unwrap_or(&empty);
    let s3 = rows.get("s3").unwrap_or(&empty);
    let s4 = rows.get("s4").unwrap_or(&empty);
    let col = |r: &HashMap<String, String>, c: &str| r.get(c).cloned().unwrap_or_else(|| "<none>".into());
    vec![
        ("the script succeeds", out.status.success()),
        ("header names every column, one row per session, none for the sub-agent", rows.len() == 4),
        ("a message repeated on 3 lines is one turn; the sub-agent adds one", col(s1, "turns") == "3"),
        ("tool calls count every tool_use block, sub-agent included", col(s1, "tool_calls") == "4"),
        ("tokens are added once per message, sub-agent included", col(s1, "input_tokens") == "1011" && col(s1, "output_tokens") == "22"),
        ("thinking tokens come from output_tokens_details", col(s1, "thinking_tokens") == "5"),
        ("cache tokens are added once per message", col(s1, "cache_read_tokens") == "103" && col(s1, "cache_creation_tokens") == "11"),
        ("models in order of first appearance, sub-agent included", col(s1, "models") == "model-a,model-b,model-c"),
        ("versions in order of first appearance", col(s1, "versions") == "2.1.1,2.1.2"),
        ("issues are the branches other than main", col(s1, "issues") == "my_issue"),
        ("start is the earliest timestamp, duration to the latest", col(s1, "start") == "2026-10-26T10:00:00.000Z" && col(s1, "duration_s") == "3600"),
        ("plain validations and stop answers are told apart from the rest", col(s1, "validations") == "3" && col(s1, "other_messages") == "3"),
        ("a bare yes is a validation", col(s2, "validations") == "1" && col(s2, "other_messages") == "0"),
        ("hook refusals are counted by hook, sorted, sub-agent included", col(s1, "hook_refusals") == "block_ai_attribution=1;clippy_gate=1;require_code_review=2;unknown=1"),
        ("skills are counted by name, sorted", col(s1, "skills") == "code-review=1;commit-message=1"),
        ("a linked worktree counts with its main repository", col(s1, "project") == "myrepo"),
        ("a cwd outside git falls back to its last component", col(s2, "project") == "elsewhere"),
        ("a submodule is its own project", col(s3, "project") == "lib"),
        ("a removed issue worktree counts with its repository", col(s4, "project") == "myrepo"),
        ("unreadable lines are counted, sub-agent included", col(s1, "unreadable_lines") == "2"),
        ("unknown line types are counted", col(s1, "unknown_lines") == "1"),
        ("assistant lines without usage are missing fields, metadata lines without timestamp are not", col(s1, "missing_fields") == "1"),
        ("no text of a message, a tool input or a tool result is printed", !stdout.contains("SECRET")),
    ]
}

// spec: EVA-12, EVA-13
fn shared_reader_checks(dir: &Path) -> Vec<(&'static str, bool)> {
    use sha2::Digest;
    let audit = std::fs::read_to_string(dir.join("../skills/audit-instructions/audit_sources.rs")).unwrap_or_default();
    let metrics = std::fs::read_to_string(dir.join("session_metrics.rs")).unwrap_or_default();
    let module = std::fs::read(dir.join("transcripts.rs")).unwrap_or_default();
    let stamp = format!("// transcripts.rs sha256: {:x}", sha2::Sha256::digest(&module));
    for (name, text) in [("audit_sources.rs", &audit), ("session_metrics.rs", &metrics)] {
        if !text.contains(&stamp) {
            println!("{name}: write the line `{stamp}` so rust-script rebuilds it with the current transcripts.rs");
        }
    }
    let includes = |s: &str, path: &str| s.contains(&format!("#[path = \"{path}\"]"));
    vec![
        ("audit_sources.rs includes the shared module", includes(&audit, "../../transcripts/transcripts.rs")),
        ("session_metrics.rs includes the shared module", includes(&metrics, "transcripts.rs")),
        ("both scripts carry the SHA-256 of the current transcripts.rs", audit.contains(&stamp) && metrics.contains(&stamp)),
        ("neither script defines its own typed-message filter", !audit.contains("fn typed_text") && !metrics.contains("fn typed_text")),
    ]
}

/// Commits `files` (path, content) in `repo` at `date` (committer and author); returns the commit hash.
fn commit_at(repo: &Path, date: &str, files: &[(&str, &str)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    sh(repo, &["add", "-A", "--force"]);
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", date])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "commit failed in {}: {}{}", repo.display(), String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let head = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(repo).output().unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn new_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    sh(dir, &["init", "-q", "-b", "main"]);
    dir.to_path_buf()
}

// spec: EVA-8
fn revision_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tmp.join("revision");
    let pinned = new_repo(&tmp.join("pinned"));
    let toml = |rev: &str| format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"  # v1\n");
    commit_at(&pinned, "2026-01-01T10:00:00Z", &[(".claude/harness.toml", &toml("aaa"))]);
    commit_at(&pinned, "2026-01-01T12:00:00Z", &[(".claude/harness.toml", &toml("bbb"))]);
    let worktree = tmp.join("pinned-branch");
    sh(&pinned, &["worktree", "add", "-q", "-b", "branch", worktree.to_str().unwrap()]);
    commit_at(&worktree, "2026-01-01T12:30:00Z", &[(".claude/harness.toml", &toml("ccc"))]);
    sh(&pinned, &["checkout", "-q", "-b", "bump"]);
    commit_at(&pinned, "2026-01-01T12:15:00Z", &[(".claude/harness.toml", &toml("ddd"))]);
    sh(&pinned, &["checkout", "-q", "main"]);
    let merge = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "merge", "-q", "--no-ff", "-m", "merge", "bump"])
        .env("GIT_AUTHOR_DATE", "2026-01-01T14:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T14:00:00Z")
        .current_dir(&pinned)
        .output()
        .unwrap();
    assert!(merge.status.success(), "merge failed");
    let own = new_repo(&tmp.join("own"));
    commit_at(&own, "2026-01-01T10:00:00Z", &[(".claude/harness.toml", "[harness]\nsource = \".\"\n"), ("files/a", "1")]);
    let files_commit = commit_at(&own, "2026-01-01T11:00:00Z", &[("files/a", "2")]);
    commit_at(&own, "2026-01-01T11:10:00Z", &[("other", "x")]);
    let unpinned = new_repo(&tmp.join("unpinned"));
    commit_at(&unpinned, "2026-01-01T10:00:00Z", &[(".claude/harness.toml", "[harness]\nsource = \"https://example.org/harness\"\n")]);
    let projects = tmp.join("projects");
    let sessions: [(&str, &Path, &str); 8] = [
        ("before", &pinned, "2026-01-01T09:00:00.000Z"),
        ("first", &pinned, "2026-01-01T11:00:00.000Z"),
        ("second", &pinned, "2026-01-01T13:00:00.000Z"),
        ("merged", &pinned, "2026-01-01T15:00:00.000Z"),
        ("on_branch", &worktree, "2026-01-01T13:00:00.000Z"),
        ("own", &own, "2026-01-01T11:30:00.000Z"),
        ("unpinned", &unpinned, "2026-01-01T11:00:00.000Z"),
        ("gone", Path::new("/nonexistent/gone"), "2026-01-01T11:00:00.000Z"),
    ];
    for (id, cwd, ts) in sessions {
        let proj = projects.join(format!("-{id}"));
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{id}.jsonl")), typed(cwd, ts, "ok")).unwrap();
    }
    let out = Command::new("rust-script").arg(dir.join("session_metrics.rs")).arg(&projects).output().expect("rust-script must be installed");
    let rows = rows(&String::from_utf8_lossy(&out.stdout)).unwrap_or_default();
    let rev = |id: &str| rows.get(id).and_then(|r| r.get("revision")).cloned().unwrap_or_else(|| "<none>".into());
    vec![
        ("no revision before the first pin", rev("before").is_empty()),
        ("the pin in force at the start of the session", rev("first") == "aaa"),
        ("a later pin applies to a later session, not one merged later", rev("second") == "bbb"),
        ("a pin merged into the default branch applies after the merge", rev("merged") == "ddd"),
        ("a linked worktree reads the pin of the default branch", rev("on_branch") == "bbb"),
        ("the harness repository itself gives its last commit touching files/", rev("own") == format!("self:{files_commit}")),
        ("a harness.toml with no rev is unpinned", rev("unpinned") == "unpinned"),
        ("an unreadable project gives an empty revision", rev("gone").is_empty()),
    ]
}

// spec: EVA-11, EVA-14
fn archive_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let tmp = tmp.join("archive");
    let projects = tmp.join("projects/-p");
    std::fs::create_dir_all(&projects).unwrap();
    let cwd = Path::new("/nonexistent/p");
    std::fs::write(projects.join("b.jsonl"), [typed(cwd, "2026-01-02T00:00:00.000Z", "ok"), typed(cwd, "2026-01-02T00:00:01.000Z", "ok")].concat()).unwrap();
    std::fs::write(projects.join("c.jsonl"), typed(cwd, "2026-01-01T12:00:00.000Z", "ok")).unwrap();
    let archive = tmp.join("state/deep/sessions.tsv");
    let run = || Command::new("rust-script").arg(dir.join("session_metrics.rs")).arg(tmp.join("projects")).arg("--archive").arg(&archive).output().expect("rust-script must be installed");
    let first = run();
    let created = std::fs::read_to_string(&archive).unwrap_or_default();
    let old_row = |id: &str, start: &str| format!("{id}\tp\t\t\t\t\t{start}\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t9\t0\t0\t0\t0\n");
    std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
    let resolved = "c\tkeep\t\trrr\t\t\t2026-01-01T12:00:00.000Z\t0\t0\t0\t0\t0\t0\t0\t0\t\t\t9\t0\t0\t0\t0\n";
    std::fs::write(&archive, format!("{COLUMNS}\n{}{}{resolved}", old_row("b", "2026-01-02T00:00:00.000Z"), old_row("a", "2026-01-01T00:00:00.000Z"))).unwrap();
    let merged = run();
    let text = std::fs::read_to_string(&archive).unwrap_or_default();
    let ids: Vec<&str> = text.lines().skip(1).map(|l| l.split('\t').next().unwrap_or_default()).collect();
    let merged_rows = rows(&text).unwrap_or_default();
    let cell = |id: &str, c: &str| merged_rows.get(id).and_then(|r| r.get(c)).cloned().unwrap_or_default();
    let b_validations = cell("b", "validations");
    let foreign = "session\tsomething_else\na\tx\n";
    std::fs::write(&archive, foreign).unwrap();
    let refused = run();
    vec![
        ("the archive and its missing parent directories are created", first.status.success() && created.lines().count() == 3),
        ("the rows are still printed with --archive", String::from_utf8_lossy(&merged.stdout).lines().count() == 3),
        ("one header, sessions kept, replaced or added, sorted by start", merged.status.success() && text.starts_with(COLUMNS) && ids == ["a", "c", "b"]),
        ("an archived session is replaced by its new row", b_validations == "2"),
        ("a resolved project and revision survive a new row that lost them", cell("c", "project") == "keep" && cell("c", "revision") == "rrr" && cell("c", "validations") == "1"),
        ("an archive with another header is refused and left unchanged", !refused.status.success() && std::fs::read_to_string(&archive).unwrap_or_default() == foreign),
    ]
}

fn main() {
    let dir = std::fs::canonicalize(Path::new(file!()).parent().unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let tmp = temp.path().to_path_buf();
    let checks: Vec<_> = row_checks(&dir, &tmp).into_iter().chain(revision_checks(&dir, &tmp)).chain(archive_checks(&dir, &tmp)).chain(shared_reader_checks(&dir)).collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
    if failed {
        std::process::exit(1);
    }
}
