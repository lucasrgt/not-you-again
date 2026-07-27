#![allow(dead_code)]

use serde::Serialize;
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};
use tempfile::TempDir;

pub struct Repo {
    _temp: TempDir,
    pub root: PathBuf,
}

pub struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

pub static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
struct Config {
    schema: u8,
    check: Check,
}

#[derive(Serialize)]
struct Check {
    timeout_seconds: u64,
}

#[derive(Serialize)]
struct UserConfig {
    schema: u8,
    judge: String,
    command: Vec<String>,
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
        self.configure_as("test", command, timeout_seconds);
    }

    pub fn configure_as(&self, judge: &str, command: Vec<String>, timeout_seconds: u64) {
        let timeout_seconds = if cfg!(windows) && timeout_seconds == 5 { 45 } else { timeout_seconds };
        let policy = toml::to_string(&Config { schema: 1, check: Check { timeout_seconds } }).unwrap();
        let user = toml::to_string(&UserConfig { schema: 1, judge: judge.into(), command }).unwrap();
        fs::write(self.root.join(".nya/config.toml"), policy).unwrap();
        fs::write(self.root.join(".nya/config.local.toml"), user).unwrap();
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

pub fn fake_gh(root: &Path, response: &str) -> PathBuf {
    let literal = format!("{response:?}");
    fake_program(root, "fake-gh", &format!("print!(\"{{}}\", {literal});"))
}

pub fn fake_program(root: &Path, name: &str, body: &str) -> PathBuf {
    let source = root.join(format!("{name}.rs"));
    let binary = root.join(if cfg!(windows) { format!("{name}.exe") } else { name.to_owned() });
    fs::write(&source, format!("fn main() {{ {body} }}")).unwrap();
    let status = Command::new("rustc").arg(&source).arg("-o").arg(&binary).status().unwrap();
    assert!(status.success(), "failed to compile fake gh");
    binary
}

impl EnvGuard {
    pub fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
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
            format!("$text=[Console]::In.ReadToEnd(); if ($text -match '<PROPOS') {{ Write-Output '{}' }} else {{ Write-Output '{}' }}", confirmation, first),
        ]
    } else {
        vec!["sh".into(), "-c".into(), format!("text=$(cat); if printf '%s' \"$text\" | grep -q '<PROPOS'; then printf '%s' '{}'; else printf '%s' '{}'; fi", confirmation, first)]
    }
}

pub fn matching_judge(needle: &str, verdict: &str) -> Vec<String> {
    let needle = needle.replace('\'', "''");
    if cfg!(windows) {
        vec![
            "powershell".into(),
            "-NoProfile".into(),
            "-Command".into(),
            format!("$text=[Console]::In.ReadToEnd(); if ($text.Contains('{needle}')) {{ Write-Output '{verdict}' }} else {{ Write-Output '{{\"findings\":[]}}' }}"),
        ]
    } else {
        vec!["sh".into(), "-c".into(), format!("text=$(cat); if printf '%s' \"$text\" | grep -Fq '{needle}'; then printf '%s' '{verdict}'; else printf '%s' '{{\"findings\":[]}}'; fi")]
    }
}

pub fn recording_matching_judge(path: &Path, needle: &str, verdict: &str) -> Vec<String> {
    let path = path.to_string_lossy().replace('\'', "''");
    let needle = needle.replace('\'', "''");
    if cfg!(windows) {
        vec![
            "powershell".into(),
            "-NoProfile".into(),
            "-Command".into(),
            format!(
                "$text=[Console]::In.ReadToEnd(); Add-Content -LiteralPath '{path}' -Value ([Text.Encoding]::UTF8.GetByteCount($text)); if ($text.Contains('{needle}')) {{ Write-Output '{verdict}' }} else {{ Write-Output '{{\"findings\":[]}}' }}"
            ),
        ]
    } else {
        vec![
            "sh".into(),
            "-c".into(),
            format!(
                "text=$(cat); printf '%s\\n' \"$(printf '%s' \"$text\" | wc -c)\" >> '{path}'; if printf '%s' \"$text\" | grep -Fq '{needle}'; then printf '%s' '{verdict}'; else printf '%s' '{{\"findings\":[]}}'; fi"
            ),
        ]
    }
}

pub fn failing_judge() -> Vec<String> {
    if cfg!(windows) {
        vec!["powershell".into(), "-NoProfile".into(), "-Command".into(), "[Console]::In.ReadToEnd() | Out-Null; [Console]::Error.Write('runner failed'); exit 7".into()]
    } else {
        vec!["sh".into(), "-c".into(), "cat >/dev/null; printf runner-failed >&2; exit 7".into()]
    }
}

pub fn isolated_home_judge() -> Vec<String> {
    if cfg!(windows) {
        vec![
            "powershell".into(),
            "-NoProfile".into(),
            "-Command".into(),
            "if (-not (Test-Path -LiteralPath $env:CODEX_HOME -PathType Container)) { exit 9 }; [Console]::In.ReadToEnd() | Out-Null; Write-Output '{\"findings\":[]}'".into(),
        ]
    } else {
        vec!["sh".into(), "-c".into(), "test -d \"$CODEX_HOME\" || exit 9; cat >/dev/null; printf '%s' '{\"findings\":[]}'".into()]
    }
}

pub fn slow_judge() -> Vec<String> {
    if cfg!(windows) {
        vec!["powershell".into(), "-NoProfile".into(), "-Command".into(), "Start-Sleep -Seconds 2; Write-Output '{\"findings\":[]}'".into()]
    } else {
        vec!["sh".into(), "-c".into(), "sleep 2; printf '%s' '{\"findings\":[]}'".into()]
    }
}

pub fn recording_judge(path: &Path, verdict: &str) -> Vec<String> {
    let path = path.to_string_lossy();
    if cfg!(windows) {
        vec![
            "powershell".into(),
            "-NoProfile".into(),
            "-Command".into(),
            format!("$text=[Console]::In.ReadToEnd(); Add-Content -LiteralPath '{}' -Value ([Text.Encoding]::UTF8.GetByteCount($text)); Write-Output '{}'", path.replace('\'', "''"), verdict),
        ]
    } else {
        vec!["sh".into(), "-c".into(), format!("text=$(cat); printf '%s\\n' \"$(printf '%s' \"$text\" | wc -c)\" >> '{}'; printf '%s' '{}'", path.replace('\'', "'\\''"), verdict)]
    }
}
