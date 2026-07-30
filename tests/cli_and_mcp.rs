mod common;

use common::{EnvGuard, Repo, empty_verdict, fake_gh, fake_program, finding, judge};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Cursor, Write},
    process::{Command, Stdio},
};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nya"))
}

fn command(repo: &Repo, args: &[&str]) -> std::process::Output {
    bin().arg("--repository").arg(&repo.root).args(args).output().unwrap()
}

fn interactive(repo: &Repo, args: &[&str]) -> std::process::Output {
    bin().arg("--repository").arg(&repo.root).env("NYA_FORCE_TTY", "1").env("NYA_ASCII", "1").env("NO_COLOR", "1").args(args).output().unwrap()
}

#[test]
fn csm_storage_is_opt_in_and_does_not_rewrite_root_instructions() {
    let repo = Repo::new(&["AGENTS.md"]);
    let instructions = fs::read_to_string(repo.root.join("AGENTS.md")).unwrap();
    let initialized = bin().arg("--repository").arg(&repo.root).env("CSM_STORAGE_ROOT", ".csm").arg("init").output().unwrap();

    assert!(initialized.status.success());
    assert!(repo.root.join(".csm/nya/config.toml").is_file());
    assert!(!repo.root.join(".nya").exists());
    assert_eq!(fs::read_to_string(repo.root.join("AGENTS.md")).unwrap(), instructions);
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
fn interactive_cli_is_branded_while_json_and_pipes_stay_clean() {
    let repo = Repo::new(&["AGENTS.md"]);
    let initialized = interactive(&repo, &["init"]);
    let initialized = String::from_utf8(initialized.stdout).unwrap();
    assert!(initialized.contains("NOT YOU AGAIN"));
    assert!(initialized.contains("[ok] .nya created"));
    assert!(!initialized.contains('\u{1b}'));

    let configured = interactive(&repo, &["setup", "--judge", "codex"]);
    assert!(String::from_utf8(configured.stdout).unwrap().contains("[ok] Personal judge selected"));

    let remembered = interactive(&repo, &["remember", "--title", "Literal copy", "--lesson", "Use the message catalog.", "--scope", "src/**"]);
    let remembered = String::from_utf8(remembered.stdout).unwrap();
    assert!(remembered.contains("Scar remembered") && remembered.contains("[ok] Literal copy"));

    let recalled = bin()
        .arg("--repository")
        .arg(&repo.root)
        .env("NYA_FORCE_TTY", "1")
        .env_remove("NYA_ASCII")
        .env("TERM", "xterm-256color")
        .args(["recall", "--task", "literal copy", "--path", "src/app.rs"])
        .output()
        .unwrap();
    assert!(String::from_utf8(recalled.stdout).unwrap().contains("✓ Literal copy"));

    repo.commit_all("scar");
    let clean = interactive(&repo, &["check"]);
    let clean = String::from_utf8(clean.stdout).unwrap();
    assert!(clean.contains("Recurrence check"));
    assert!(clean.contains("[ok] Repository inspected"));
    assert!(clean.contains("[ok] No known scars repeated"));

    let json = bin().arg("--repository").arg(&repo.root).env("NYA_FORCE_TTY", "1").args(["--format", "json", "check"]).output().unwrap();
    assert!(json.status.success());
    let value: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["passed"], true);
    assert!(!String::from_utf8(json.stdout).unwrap().contains("NOT YOU AGAIN"));

    repo.configure(judge(&empty_verdict()), 5);
    let collected = interactive(&repo, &["collect", "--offline", "--dry-run"]);
    let collected = String::from_utf8(collected.stdout).unwrap();
    assert!(collected.contains("Historical scar collection"));
    assert!(collected.contains("[ok] Sources scanned"));

    repo.write("src/app.rs", "const COPY: &str = \"literal\";\n");
    let scar = nya::recall(&repo.root, nya::RecallRequest::default()).unwrap().remove(0);
    repo.configure(judge(&finding(&scar.id, "src/app.rs")), 5);
    let failed = interactive(&repo, &["check"]);
    assert_eq!(failed.status.code(), Some(1));
    let failed = String::from_utf8(failed.stdout).unwrap();
    assert!(failed.contains("[x] Known scar repeated"));
    assert!(failed.contains("[x] SCAR 1/1"));
    assert!(failed.contains("Fix every recurrence"));

    repo.configure(vec![], 5);
    let error = interactive(&repo, &["check"]);
    assert_eq!(error.status.code(), Some(2));
    let error = String::from_utf8(error.stderr).unwrap();
    assert!(error.contains("[x]") && !error.contains('\u{1b}'));
}

#[test]
fn github_review_permalink_becomes_verified_scar_provenance() {
    let _lock = common::ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let repo = Repo::new(&[]);
    assert!(command(&repo, &["init"]).status.success());
    let permalink = "https://github.com/acme/store/pull/142#discussion_r123";
    let review = json!({
        "html_url": permalink,
        "created_at": "2026-07-24T12:34:56Z",
        "user": {"login": "alice"}
    })
    .to_string();
    let gh = fake_gh(&repo.root, &review);
    let _gh = EnvGuard::set("NYA_GH", &gh);
    let remembered = bin()
        .arg("--repository")
        .arg(&repo.root)
        .args([
            "--format",
            "json",
            "remember",
            "--title",
            "Tenant-safe cache keys",
            "--lesson",
            "Include tenant identity in every cache key.",
            "--scope",
            "src/**",
            "--github-review",
            permalink,
            "--corrected-by",
            "github:bob",
            "--recorded-by",
            "agent:codex",
        ])
        .output()
        .unwrap();
    assert!(remembered.status.success(), "{}", String::from_utf8_lossy(&remembered.stderr));
    let scar: Value = serde_json::from_slice(&remembered.stdout).unwrap();
    let occurrence = &scar["occurrences"][0];
    assert_eq!(occurrence["source"], permalink);
    assert_eq!(occurrence["occurred_at"], "2026-07-24T12:34:56Z");
    assert_eq!(occurrence["reported_by"], "github:alice");
    assert_eq!(occurrence["corrected_by"], "github:bob");
    assert_eq!(occurrence["recorded_by"], "agent:codex");
    let stored = std::fs::read_to_string(repo.root.join(format!(".nya/scars/{}.toml", scar["id"].as_str().unwrap()))).unwrap();
    assert!(stored.contains(permalink) && stored.contains("github:alice") && stored.contains("agent:codex"));

    let appended = nya::remember(
        &repo.root,
        nya::RememberRequest {
            scar: scar["id"].as_str().map(str::to_owned),
            github_review: Some(permalink.into()),
            corrected_by: Some("github:bob".into()),
            recorded_by: Some("agent:codex".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(appended.occurrences.len(), 2);

    let conflict =
        bin().arg("--repository").arg(&repo.root).args(["remember", "--title", "Conflict", "--lesson", "Reject ambiguity.", "--github-review", permalink, "--source", "manual"]).output().unwrap();
    assert_eq!(conflict.status.code(), Some(2));
    assert!(String::from_utf8(conflict.stderr).unwrap().contains("supplies source and reporter"));

    let invalid = bin()
        .arg("--repository")
        .arg(&repo.root)
        .args(["remember", "--title", "Invalid", "--lesson", "Reject invalid links.", "--github-review", "https://github.com/acme/store/pull/142"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));

    assert!(
        nya::remember(
            &repo.root,
            nya::RememberRequest {
                title: Some("HTTP link".into()),
                lesson: Some("Require verified links.".into()),
                github_review: Some("http://github.com/acme/store/pull/142#discussion_r123".into()),
                ..Default::default()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("https")
    );

    let failed = fake_program(&repo.root, "failed-gh", "eprintln!(\"denied\"); std::process::exit(7);");
    let _failed = EnvGuard::set("NYA_GH", &failed);
    assert!(
        nya::remember(&repo.root, nya::RememberRequest { title: Some("API failure".into()), lesson: Some("Fail closed.".into()), github_review: Some(permalink.into()), ..Default::default() })
            .unwrap_err()
            .to_string()
            .contains("gh api failed")
    );
    drop(_failed);

    let malformed = fake_program(&repo.root, "malformed-gh", "print!(\"not-json\");");
    let malformed_guard = EnvGuard::set("NYA_GH", &malformed);
    assert!(
        nya::remember(&repo.root, nya::RememberRequest { title: Some("Malformed response".into()), lesson: Some("Fail closed.".into()), github_review: Some(permalink.into()), ..Default::default() })
            .unwrap_err()
            .to_string()
            .contains("invalid review comment")
    );
    drop(malformed_guard);

    let mismatch = json!({
        "html_url": "https://github.com/acme/store/pull/142#discussion_r999",
        "created_at": "2026-07-24T12:34:56Z",
        "user": {"login": "alice"}
    })
    .to_string();
    let mismatch = fake_gh(&repo.root, &mismatch);
    let mismatch_guard = EnvGuard::set("NYA_GH", &mismatch);
    assert!(
        nya::remember(&repo.root, nya::RememberRequest { title: Some("Mismatched response".into()), lesson: Some("Fail closed.".into()), github_review: Some(permalink.into()), ..Default::default() })
            .unwrap_err()
            .to_string()
            .contains("different review comment")
    );
    drop(mismatch_guard);

    let missing = repo.root.join(if cfg!(windows) { "missing-gh.exe" } else { "missing-gh" });
    let _missing = EnvGuard::set("NYA_GH", &missing);
    assert!(
        nya::remember(&repo.root, nya::RememberRequest { title: Some("Missing CLI".into()), lesson: Some("Fail closed.".into()), github_review: Some(permalink.into()), ..Default::default() })
            .unwrap_err()
            .to_string()
            .contains("install and authenticate")
    );
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
fn builtin_codex_judge_requests_external_execution_in_a_network_sandbox() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    nya::remember(&repo.root, nya::RememberRequest { title: Some("No literal copy".into()), lesson: Some("Use the catalog.".into()), scope: vec!["src/**".into()], ..Default::default() }).unwrap();
    repo.commit_all("scar");
    repo.write("src/app.rs", "literal\n");
    repo.configure_as("codex", vec![], 5);
    let output = bin().env("CODEX_SANDBOX_NETWORK_DISABLED", "1").arg("--repository").arg(&repo.root).arg("check").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr).unwrap().contains("host, MCP server, or CI"));
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

    let remember_help = bin().args(["remember", "--help"]).output().unwrap();
    let help = String::from_utf8(remember_help.stdout).unwrap();
    assert!(help.contains("Existing scar ID") && help.contains("Required affected glob") && help.contains("#discussion_r"));
    let collect_help = String::from_utf8(bin().args(["collect", "--help"]).output().unwrap().stdout).unwrap();
    assert!(collect_help.contains("--all") && collect_help.contains("--since") && collect_help.contains("--dry-run") && collect_help.contains("--offline"));
    let check_help = String::from_utf8(bin().args(["check", "--help"]).output().unwrap().stdout).unwrap();
    assert!(check_help.contains("committed review") && check_help.contains("Task or review context"));
    let spec_help = String::from_utf8(bin().args(["spec", "--help"]).output().unwrap().stdout).unwrap();
    assert!(spec_help.contains("specification file") && spec_help.contains("Expected implementation path"));
    let replay_help = String::from_utf8(bin().args(["replay", "--help"]).output().unwrap().stdout).unwrap();
    assert!(replay_help.contains("Replay only this scar ID") && replay_help.contains("historical correction pairs"));
}

#[test]
fn in_process_cli_dispatch_covers_each_agent_action() {
    let _lock = common::ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _tty = EnvGuard::set("NYA_FORCE_TTY", "1");
    let _ascii = EnvGuard::set("NYA_ASCII", "1");
    let _color = EnvGuard::set("NO_COLOR", "1");
    let repo = Repo::new(&["AGENTS.md"]);
    let root = repo.root.to_string_lossy().to_string();
    let _config = EnvGuard::set("NYA_CONFIG", repo.root.join("user-config.toml"));
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "init"]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "setup", "--judge", "codex"]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "setup", "--local", "--judge", "codex"]).unwrap(), 0);
    repo.write("src/direct.rs", "literal\n");
    repo.commit_all("add unsafe direct path");
    repo.write("src/direct.rs", "safe\n");
    repo.commit_all("fix direct path");
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "remember", "--title", "Direct scar", "--lesson", "Use the direct path.", "--scope", "src/**",]).unwrap(), 0);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "recall", "--task", "direct path"]).unwrap(), 0);
    let scars = nya::recall(&repo.root, nya::RecallRequest { task: String::new(), paths: vec!["src/direct.rs".into()], limit: None }).unwrap();
    let scar = &scars[0];
    let commit = common::output(&repo.root, &["rev-parse", "--short", "HEAD"]);
    repo.write("spec.md", "The direct path must remain safe.\n");
    repo.configure(judge(r#"{"gaps":[]}"#), 5);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "spec", "--file", "spec.md", "--path", "src/direct.rs"]).unwrap(), 0);
    let replay = json!({"scar_id":scar.id,"commit":commit,"source":null,"before_repeats":true,"after_fixes":true,"before_evidence":"-literal","after_evidence":"+safe","reason":"The correction replaces the unsafe direct path."}).to_string();
    repo.configure(judge(&replay), 5);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "replay", "--scar", &scar.id]).unwrap(), 0);
    repo.commit_all("direct scar");
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "check"]).unwrap(), 0);
    repo.configure(judge(r#"{"candidates":[]}"#), 5);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "collect", "--offline"]).unwrap(), 0);

    repo.write("src/direct.rs", "literal\n");
    repo.configure(judge(&finding(&scars[0].id, "src/direct.rs")), 5);
    assert_eq!(nya::run_cli(["nya", "--repository", &root, "check"]).unwrap(), 1);
    repo.configure_as("test", vec![], 5);
    let error = nya::run_cli(["nya", "--repository", &root, "check"]).unwrap_err();
    nya::print_error(&error);
    assert!(nya::run_cli(["nya", "--repository", &root, "unknown"]).is_err());
}

#[test]
fn mcp_exposes_the_domain_tools_and_calls_the_core() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("spec.md", "# Unrelated release notes\n");
    repo.configure(judge("not-json"), 5);
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
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nya_collect","arguments":{"repository":root,"offline":true}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"nya_spec","arguments":{"repository":root,"files":["spec.md"],"task":"unrelated release notes"}}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"nya_replay","arguments":{"repository":root,"limit":1}}}),
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"other","arguments":{"repository":root}}}),
        json!({"jsonrpc":"2.0","id":11,"method":"unknown"}),
    ];
    let wire = requests.iter().map(|request| format!("{request}\n")).collect::<String>();
    let mut direct_output = Vec::new();
    nya::serve_mcp_io(Cursor::new(wire), &mut direct_output).unwrap();
    assert_eq!(String::from_utf8(direct_output).unwrap().lines().count(), 11);
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
    assert_eq!(responses.len(), 11);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(responses[1]["result"], json!({}));
    let tools = responses[2]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 6);
    assert_eq!(tools.iter().map(|tool| tool["name"].as_str().unwrap()).collect::<Vec<_>>(), ["nya_remember", "nya_recall", "nya_check", "nya_collect", "nya_spec", "nya_replay"]);
    assert!(responses[3]["result"]["structuredContent"]["id"].as_str().unwrap().starts_with("NYA-"));
    assert_eq!(responses[4]["result"]["structuredContent"].as_array().unwrap().len(), 1);
    assert_eq!(responses[5]["result"]["structuredContent"]["passed"], true);
    assert_eq!(responses[6]["result"]["structuredContent"]["correction_candidates"], 0);
    assert_eq!(responses[7]["result"]["structuredContent"]["passed"], true);
    assert!(responses[8]["error"]["message"].as_str().unwrap().contains("malformed verdict"));
    assert!(responses[9]["error"]["message"].as_str().unwrap().contains("unknown tool"));
    assert!(responses[10]["error"]["message"].as_str().unwrap().contains("method not found"));
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
