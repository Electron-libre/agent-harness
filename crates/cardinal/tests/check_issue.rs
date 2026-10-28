//! Behavior test for `harness tool check-issue`: accepts a good issue, flags each template violation.

const GOOD: &str = "# Fix the thing\n\n# Goal\n\nMake it work.\n\n# Approach\n\n1. Do it.\n\n# Resources\n\n* `a.rs`\n";

#[test]
fn check_issue() {
    let script = std::path::PathBuf::from("/tools/check_issue.rs");
    let tmp = tempfile::tempdir().unwrap().keep();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues/done")).unwrap();
    std::fs::write(tmp.join("issues/done/old.md"), "x").unwrap();

    let run = |file: &str, body: &str| {
        std::fs::write(tmp.join(file), body).unwrap();
        let out = tool_command(&script)
            .arg(file)
            .current_dir(&tmp)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (ok, out) = run("issues/good.md", GOOD);
    check("good issue passes", ok && out.is_empty());
    let (ok, out) = run("issues/Bad-Name.md", GOOD);
    check(
        "non snake_case name is flagged",
        !ok && out.contains("snake_case"),
    );
    let (ok, out) = run("issues/old.md", GOOD);
    check(
        "name already in done/ is flagged",
        !ok && out.contains("already exists"),
    );
    let (ok, out) = run(
        "issues/a.md",
        &GOOD.replace("# Approach\n\n1. Do it.\n\n", ""),
    );
    check(
        "missing section is flagged",
        !ok && out.contains("`# Approach`"),
    );
    let (ok, out) = run("issues/b.md", &format!("{GOOD}\n# Tracking\n"));
    check(
        "tracking section is flagged",
        !ok && out.contains("Tracking"),
    );
    let (ok, out) = run("issues/c.md", &GOOD.replace("Make it work.", ""));
    check("empty goal is flagged", !ok && out.contains("empty"));
    let (ok, out) = run(
        "issues/d.md",
        &GOOD.replace("Make it work.", "See /home/me/x."),
    );
    check(
        "private path is flagged",
        !ok && out.contains("private path"),
    );
    let (ok, out) = run("issues/e.md", &GOOD.replace("# Fix the thing\n\n", ""));
    check("missing title is flagged", !ok && out.contains("title"));

    let (ok, out) = run(
        "issues/f.md",
        &GOOD.replace(
            "# Resources",
            "# Specs\n\n* `specs/rerun.md`: RER-2\n\n# Resources",
        ),
    );
    check("filled specs section passes", ok && out.is_empty());
    let (ok, out) = run(
        "issues/g.md",
        &GOOD.replace("# Resources", "# Specs\n\n# Resources"),
    );
    check(
        "empty specs section is flagged",
        !ok && out.contains("`# Specs` is empty"),
    );

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// `harness tool <name>` for the script path the checks were written against (`next_issue.rs` is `next-issue`).
fn tool_command(script: impl AsRef<std::path::Path>) -> std::process::Command {
    let stem = script
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('_', "-");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.arg("tool").arg(stem);
    cmd
}
