#![allow(dead_code)]

use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

pub struct Repo {
    _temp: TempDir,
    pub root: PathBuf,
}

#[derive(Serialize)]
struct Config {
    schema: u8,
    judge: Judge,
}

#[derive(Serialize)]
struct Judge {
    command: Vec<String>,
    timeout_seconds: u64,
}

impl Repo {
    pub fn new(agent_files: &[&str]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        run(&root, &["init", "-q"]);
        run(&root, &["config", "user.name", "Test User"]);
        run(&root, &["config", "user.email", "test@example.com"]);
        run(&root, &["config", "core.autocrlf", "false"]);
        for name in agent_files {
            fs::write(root.join(name), format!("# {name}\n\nOriginal instructions.\n")).unwrap();
        }
        fs::write(root.join("baseline.txt"), "baseline\n").unwrap();
        run(&root, &["add", "."]);
        run(&root, &["commit", "-qm", "initial"]);
        Self { _temp: temp, root }
    }

    pub fn write(&self, path: &str, body: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    pub fn configure(&self, command: Vec<String>, timeout_seconds: u64) {
        let body = toml::to_string(&Config { schema: 1, judge: Judge { command, timeout_seconds } }).unwrap();
        fs::write(self.root.join(".nya/config.toml"), body).unwrap();
    }

    pub fn commit_all(&self, message: &str) {
        run(&self.root, &["add", "."]);
        run(&self.root, &["commit", "-qm", message]);
    }

    pub fn git_dir(&self) -> PathBuf {
        PathBuf::from(output(&self.root, &["rev-parse", "--absolute-git-dir"]))
    }
}

pub fn run(root: &Path, args: &[&str]) {
    let status = Command::new("git").arg("-C").arg(root).args(args).status().unwrap();
    assert!(status.success(), "git {args:?} failed");
}

pub fn output(root: &Path, args: &[&str]) -> String {
    let value = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
    assert!(value.status.success());
    String::from_utf8(value.stdout).unwrap().trim().to_owned()
}

pub fn empty_verdict() -> String {
    r#"{"findings":[]}"#.to_owned()
}

pub fn finding(scar_id: &str, path: &str) -> String {
    serde_json::json!({
        "findings": [{
            "scar_id": scar_id,
            "path": path,
            "line": 1,
            "evidence": "literal",
            "reason": "The changed code repeats the supplied scar."
        }]
    })
    .to_string()
}

pub fn judge(always: &str) -> Vec<String> {
    conditional(always, always)
}

pub fn conditional(first: &str, confirmation: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![
            "powershell".into(),
            "-NoProfile".into(),
            "-Command".into(),
            format!("$text=[Console]::In.ReadToEnd(); if ($text -match 'Confirm only') {{ Write-Output '{}' }} else {{ Write-Output '{}' }}", confirmation, first),
        ]
    } else {
        vec!["sh".into(), "-c".into(), format!("text=$(cat); if printf '%s' \"$text\" | grep -q 'Confirm only'; then printf '%s' '{}'; else printf '%s' '{}'; fi", confirmation, first)]
    }
}

pub fn failing_judge() -> Vec<String> {
    if cfg!(windows) {
        vec!["powershell".into(), "-NoProfile".into(), "-Command".into(), "[Console]::In.ReadToEnd() | Out-Null; [Console]::Error.Write('runner failed'); exit 7".into()]
    } else {
        vec!["sh".into(), "-c".into(), "cat >/dev/null; printf runner-failed >&2; exit 7".into()]
    }
}

pub fn slow_judge() -> Vec<String> {
    if cfg!(windows) {
        vec!["powershell".into(), "-NoProfile".into(), "-Command".into(), "Start-Sleep -Seconds 2; Write-Output '{\"findings\":[]}'".into()]
    } else {
        vec!["sh".into(), "-c".into(), "sleep 2; printf '%s' '{\"findings\":[]}'".into()]
    }
}
