mod common;

use common::{Repo, empty_verdict, finding, judge};
use serde_json::{Value, json};
use std::{
    io::{Cursor, Write},
    process::{Command, Stdio},
};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nya"))
}

fn command(repo: &Repo, args: &[&str]) -> std::process::Output {
    bin().arg("--repository").arg(&repo.root).args(args).output().unwrap()
}

#[test]
fn cli_covers_human_json_and_exit_code_contracts() {
    let repo = Repo::new(&["AGENTS.md"]);
    let init = command(&repo, &["init"]);
    assert!(init.status.success());
    assert!(String::from_utf8(init.stdout).unwrap().contains("AGENTS.md"));

    let remembered = command(&repo, &["--format", "json", "remember", "--title", "Literal copy", "--lesson", "Use the catalog", "--scope", "src/**", "--reported-by", "github:reviewer"]);
    assert!(remembered.status.success());
    let scar: Value = serde_json::from_slice(&remembered.stdout).unwrap();
    assert!(scar["id"].as_str().unwrap().starts_with("NYA-"));

    let recalled = command(&repo, &["recall", "--task", "catalog copy", "--path", "src/app.rs"]);
    assert!(recalled.status.success());
    assert!(String::from_utf8(recalled.stdout).unwrap().contains("Literal copy"));

    repo.commit_all("scar");
    let clean = command(&repo, &["check"]);
    assert_eq!(clean.status.code(), Some(0));

    repo.write("src/app.rs", "const COPY: &str = \"literal\";\n");
    repo.configure(judge(&empty_verdict()), 5);
    assert_eq!(command(&repo, &["--format", "json", "check"]).status.code(), Some(0));

    let id = scar["id"].as_str().unwrap();
    repo.configure(judge(&finding(id, "src/app.rs")), 5);
    let failed = command(&repo, &["check"]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(String::from_utf8(failed.stdout).unwrap().contains(id));

    repo.configure(vec![], 5);
    let error = command(&repo, &["check"]);
    assert_eq!(error.status.code(), Some(2));
    assert!(String::from_utf8(error.stderr).unwrap().contains("has no command"));
}

#[test]
fn setup_separates_user_and_repository_local_judges() {
    let repo = Repo::new(&[]);
    assert!(command(&repo, &["init"]).status.success());
    let home = tempfile::tempdir().unwrap();
    let global = home.path().join("config.toml");

    for judge in ["codex", "claude", "hermes"] {
        let output = bin().env("NYA_CONFIG", &global).arg("--repository").arg(&repo.root).args(["setup", "--judge", judge]).output().unwrap();
        assert!(output.status.success());
        let body = std::fs::read_to_string(&global).unwrap();
        assert!(body.contains(&format!("judge = \"{judge}\"")));
        assert!(body.contains("command = []"));
    }

    let local = command(&repo, &["setup", "--local", "--judge", "claude"]);
    assert!(local.status.success());
    assert!(std::fs::read_to_string(repo.root.join(".nya/config.local.toml")).unwrap().contains("judge = \"claude\""));

    let unknown = bin().env("NYA_CONFIG", &global).arg("setup").args(["--judge", "company"]).output().unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8(unknown.stderr).unwrap().contains("requires a command"));

    let mut custom = bin();
    custom.env("NYA_CONFIG", &global).args(["setup", "--judge", "company", "--"]).args(judge(&empty_verdict()));
    assert!(custom.output().unwrap().status.success());
    assert!(std::fs::read_to_string(global).unwrap().contains("judge = \"company\""));
}

#[test]
fn check_prefers_repository_local_config_and_falls_back_to_user_config() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar =
        nya::remember(&repo.root, nya::RememberRequest { title: Some("No literal copy".into()), lesson: Some("Use the message catalog.".into()), scope: vec!["src/**".into()], ..Default::default() })
            .unwrap();
    repo.commit_all("scar");
    repo.write("src/app.rs", "literal\n");
    repo.configure(judge(&empty_verdict()), 5);

    let home = tempfile::tempdir().unwrap();
    let global = home.path().join("config.toml");
    std::fs::copy(repo.root.join(".nya/config.local.toml"), &global).unwrap();
    std::fs::remove_file(repo.root.join(".nya/config.local.toml")).unwrap();
    let fallback = bin().env("NYA_CONFIG", &global).arg("--repository").arg(&repo.root).arg("check").output().unwrap();
    assert_eq!(fallback.status.code(), Some(0));

    repo.configure(judge(&finding(&scar.id, "src/app.rs")), 5);
    let overridden = bin().env("NYA_CONFIG", &global).arg("--repository").arg(&repo.root).arg("check").output().unwrap();
    assert_eq!(overridden.status.code(), Some(1));
}

#[test]
fn cli_reports_empty_recall_and_invalid_repository() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let empty = command(&repo, &["recall", "--task", "nothing"]);
    assert!(String::from_utf8(empty.stdout).unwrap().contains("No relevant scars"));

    let outside = tempfile::tempdir().unwrap();
    let output = bin().arg("--repository").arg(outside.path()).arg("init").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr).unwrap().contains("Git repository"));
}

#[test]
fn in_process_cli_dispatch_covers_each_agent_action() {
    let repo = Repo::new(&["AGENTS.md"]);
    let root = repo.root.to_string_lossy().to_string();
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "init"]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "setup", "--local", "--judge", "codex"]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "--format", "json", "remember", "--title", "Direct scar", "--lesson", "Use the direct path.", "--scope", "src/**",]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "recall", "--task", "direct path"]).unwrap(), 0);
    repo.commit_all("direct scar");
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "check"]).unwrap(), 0);

    repo.write("src/direct.rs", "literal\n");
    let scars = nya::recall(&repo.root, nya::RecallRequest { task: String::new(), paths: vec!["src/direct.rs".into()], limit: None }).unwrap();
    repo.configure(judge(&finding(&scars[0].id, "src/direct.rs")), 5);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "check"]).unwrap(), 1);
    assert!(nya::run_cli(["nya", "--repository", &root, "unknown"]).is_err());
}

#[test]
fn mcp_exposes_only_the_three_domain_tools_and_calls_the_core() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let mut child = bin().arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let root = repo.root.to_string_lossy();
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nya_remember","arguments":{"repository":root,"title":"MCP scar","lesson":"Use the safe path.","scope":["src/**"],"reported_by":"github:reviewer"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nya_recall","arguments":{"repository":root,"task":"safe path","paths":["src/app.rs"]}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"nya_check","arguments":{"repository":root}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"other","arguments":{"repository":root}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"unknown"}),
    ];
    let wire = requests.iter().map(|request| format!("{request}\n")).collect::<String>();
    let mut direct_output = Vec::new();
    nya::serve_mcp_io(Cursor::new(wire), &mut direct_output).unwrap();
    assert_eq!(String::from_utf8(direct_output).unwrap().lines().count(), 8);
    {
        let stdin = child.stdin.as_mut().unwrap();
        for request in &requests {
            writeln!(stdin, "{}", request).unwrap();
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses = String::from_utf8(output.stdout).unwrap().lines().map(|line| serde_json::from_str::<Value>(line).unwrap()).collect::<Vec<_>>();
    assert_eq!(responses.len(), 8);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(responses[1]["result"], json!({}));
    let tools = responses[2]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
    assert_eq!(tools.iter().map(|tool| tool["name"].as_str().unwrap()).collect::<Vec<_>>(), ["nya_remember", "nya_recall", "nya_check"]);
    assert!(responses[3]["result"]["structuredContent"]["id"].as_str().unwrap().starts_with("NYA-"));
    assert_eq!(responses[4]["result"]["structuredContent"].as_array().unwrap().len(), 1);
    assert_eq!(responses[5]["result"]["structuredContent"]["passed"], true);
    assert!(responses[6]["error"]["message"].as_str().unwrap().contains("unknown tool"));
    assert!(responses[7]["error"]["message"].as_str().unwrap().contains("method not found"));
}

#[test]
fn mcp_errors_on_missing_repository_and_malformed_input() {
    assert!(nya::serve_mcp_io(Cursor::new("not json\n"), Vec::new()).is_err());
    let mut child = bin().arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{}", json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nya_recall","arguments":{}}})).unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["error"]["message"].as_str().unwrap().contains("repository is required"));

    let mut malformed = bin().arg("mcp").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    malformed.stdin.as_mut().unwrap().write_all(b"not json\n").unwrap();
    drop(malformed.stdin.take());
    assert_eq!(malformed.wait().unwrap().code(), Some(2));
}
