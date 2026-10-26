//! The retention of the transcripts (SYN-28).

use super::*;

const ADVICE: &str = "(cleanupPeriodDays); set it to at least 365 in";

/// A project and a user settings file with `user` in it, `None` for no file.
fn project_and_user(user: Option<&str>) -> (TempDir, TempDir, PathBuf) {
    let root = tempdir().unwrap();
    let home = tempdir().unwrap();
    let path = home.path().join("settings.json");
    if let Some(text) = user {
        fs::write(&path, text).unwrap();
    }
    (root, home, path)
}

fn user_warning(days: &str, path: &Path) -> String {
    format!(
        "warning: the transcripts are deleted after {days} days {ADVICE} {}",
        path.display()
    )
}

// spec: SYN-28
#[test]
fn warns_when_the_user_settings_keep_the_transcripts_less_than_a_year() {
    for (text, days) in [
        (None, "30"),
        (Some("{}"), "30"),
        (Some(r#"{"cleanupPeriodDays": "long"}"#), "30"),
        (Some(r#"{"cleanupPeriodDays": 90}"#), "90"),
        (Some("{,}"), "30"),
    ] {
        let (root, _home, user) = project_and_user(text);
        assert_eq!(
            retention_with(root.path(), false, &user),
            vec![user_warning(days, &user)],
            "{text:?}"
        );
    }
    for text in [
        r#"{"cleanupPeriodDays": 365}"#,
        r#"{"cleanupPeriodDays": 3650}"#,
    ] {
        let (root, _home, user) = project_and_user(Some(text));
        assert!(
            retention_with(root.path(), false, &user).is_empty(),
            "{text}"
        );
    }
}

// spec: SYN-28
#[test]
fn warns_of_each_project_settings_file_that_lowers_the_retention() {
    let (root, _home, user) = project_and_user(Some(r#"{"cleanupPeriodDays": 365}"#));
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 30}"#,
    );
    put(
        root.path(),
        ".claude/settings.local.json",
        r#"{"cleanupPeriodDays": 7}"#,
    );
    assert_eq!(
        retention_with(root.path(), false, &user),
        vec![
            "warning: .claude/settings.json sets cleanupPeriodDays to 30 days, below 365"
                .to_string(),
            "warning: .claude/settings.local.json sets cleanupPeriodDays to 7 days, below 365"
                .to_string(),
        ]
    );
    // a project file that raises it, leaves it unset or is not valid JSON says nothing of the retention
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 400}"#,
    );
    put(root.path(), ".claude/settings.local.json", "{,}");
    assert!(retention_with(root.path(), false, &user).is_empty());
}

// spec: SYN-28
#[test]
fn says_nothing_of_the_retention_under_ci() {
    let (root, _home, user) = project_and_user(None);
    put(
        root.path(),
        ".claude/settings.json",
        r#"{"cleanupPeriodDays": 30}"#,
    );
    assert!(retention_with(root.path(), true, &user).is_empty());
}

// spec: SYN-28
#[test]
fn the_user_settings_are_in_the_config_dir_or_else_in_the_home() {
    assert_eq!(
        user_settings(Some("/cfg".into()), Some("/home/u".into())),
        PathBuf::from("/cfg/settings.json")
    );
    assert_eq!(
        user_settings(None, Some("/home/u".into())),
        PathBuf::from("/home/u/.claude/settings.json")
    );
    assert_eq!(
        user_settings(Some("".into()), None),
        PathBuf::from("~/.claude/settings.json")
    );
}
