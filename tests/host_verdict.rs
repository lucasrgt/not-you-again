mod common;

use common::{Repo, failing_judge};
use nya::{CheckRequest, RememberRequest, Scar};
use serde_json::{Value, json};
use std::{io::Cursor, process::Command};

fn scar(repo: &Repo, title: &str, scope: &str) -> Scar {
    nya::remember(
        &repo.root,
        RememberRequest { title: Some(title.into()), lesson: Some("Use the typed message catalog instead of literal copy.".into()), scope: vec![scope.into()], ..Default::default() },
    )
    .unwrap()
}

fn changed_repo() -> (Repo, Scar) {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo, "Magic strings bypass the catalog", "src/**");
    repo.commit_all("add scar");
    repo.write("src/new.rs", "const MESSAGE: &str = \"literal\";\n");
    (repo, scar)
}

fn brief(repo: &Repo) -> Value {
    let result = nya::check(&repo.root, CheckRequest { brief: true, ..Default::default() }).unwrap();
    assert!(!result.passed);
    assert!(result.findings.is_empty());
    result.pending.unwrap()
}

fn answer(repo: &Repo, verdict: Value) -> anyhow::Result<nya::CheckResult> {
    nya::check(&repo.root, CheckRequest { verdict: Some(verdict.to_string()), ..Default::default() })
}

fn repeated(scar: &Scar) -> Value {
    json!({"scar_id": scar.id, "path": "src/new.rs", "line": 1, "evidence": "\"literal\"", "reason": "The message is a literal instead of a catalog entry."})
}

fn bin(repo: &Repo, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nya")).arg("--repository").arg(&repo.root).args(args).output().unwrap()
}

#[test]
fn brief_hands_the_applicable_scars_to_the_host_without_a_judge() {
    let (repo, scar) = changed_repo();
    self::scar(&repo, "Database only", "db/**");

    let pending = brief(&repo);

    assert_eq!(pending["audit"].as_str().unwrap().len(), 64);
    assert_eq!(pending["answer"]["audit"], pending["audit"]);
    assert!(pending["paths"].as_array().unwrap().contains(&json!("src/new.rs")));
    assert_eq!(pending["scars"].as_array().unwrap().len(), 1);
    assert_eq!(pending["scars"][0], json!({"id": scar.id, "title": scar.title, "lesson": scar.lesson, "scope": scar.scope}));
}

#[test]
fn brief_passes_at_once_when_no_scar_applies() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    scar(&repo, "Database only", "db/**");
    repo.commit_all("add scar");
    repo.write("src/new.rs", "alpha\n");

    let result = nya::check(&repo.root, CheckRequest { brief: true, ..Default::default() }).unwrap();

    assert!(result.passed);
    assert!(result.pending.is_none());
    assert_eq!(bin(&repo, &["check", "--brief"]).status.code(), Some(0));
}

#[test]
fn host_verdict_decides_the_check_and_never_starts_the_configured_judge() {
    let (repo, scar) = changed_repo();
    repo.configure(failing_judge(), 5);
    let audit = brief(&repo)["audit"].clone();

    let clean = answer(&repo, json!({"audit": audit, "clear": [scar.id], "findings": []})).unwrap();
    assert!(clean.passed);
    assert_eq!(clean.scars_checked, 1);
    assert!(clean.pending.is_none());

    let blocked = answer(&repo, json!({"audit": audit, "clear": [], "findings": [repeated(&scar)]})).unwrap();
    assert!(!blocked.passed);
    assert_eq!(blocked.findings.len(), 1);
    assert_eq!(blocked.findings[0].scar_id, scar.id);
    assert!(blocked.pending.is_none());
}

#[test]
fn host_verdict_must_answer_every_applicable_scar_exactly_once() {
    let (repo, first) = changed_repo();
    let second = scar(&repo, "Second lesson", "src/*.rs");
    let audit = brief(&repo)["audit"].clone();

    let omitted = answer(&repo, json!({"audit": audit, "clear": [first.id], "findings": []})).unwrap_err();
    assert!(omitted.to_string().contains(&second.id));
    let both = answer(&repo, json!({"audit": audit, "clear": [first.id, second.id], "findings": [repeated(&first)]})).unwrap_err();
    assert!(both.to_string().contains(&first.id));
    let unknown = answer(&repo, json!({"audit": audit, "clear": [first.id, second.id, "NYA-UNKNOWN"], "findings": []})).unwrap_err();
    assert!(unknown.to_string().contains("unknown scar NYA-UNKNOWN"));

    let mixed = answer(&repo, json!({"audit": audit, "clear": [second.id], "findings": [repeated(&first)]})).unwrap();
    assert!(!mixed.passed);
    assert_eq!(mixed.scars_checked, 2);
}

#[test]
fn host_verdict_is_bound_to_the_briefed_diff_and_scars() {
    let (repo, scar) = changed_repo();
    let audit = brief(&repo)["audit"].clone();

    let missing = answer(&repo, json!({"clear": [scar.id], "findings": []})).unwrap_err();
    assert!(missing.to_string().contains("stale"));

    repo.write("src/new.rs", "const MESSAGE: &str = \"another literal\";\n");
    let stale = answer(&repo, json!({"audit": audit, "clear": [scar.id], "findings": []})).unwrap_err();
    assert!(stale.to_string().contains("rerun `nya check --brief`"));

    let fresh = brief(&repo)["audit"].clone();
    assert_ne!(fresh, audit);
    assert!(answer(&repo, json!({"audit": fresh, "clear": [scar.id], "findings": []})).unwrap().passed);
}

#[test]
fn host_findings_keep_the_evidence_contract_of_the_judge() {
    let (repo, scar) = changed_repo();
    let audit = brief(&repo)["audit"].clone();
    let finding = |change: Value| {
        let mut finding = repeated(&scar);
        finding.as_object_mut().unwrap().extend(change.as_object().unwrap().clone());
        answer(&repo, json!({"audit": audit, "clear": [], "findings": [finding]})).unwrap_err().to_string()
    };

    assert!(finding(json!({"evidence": "not in the diff"})).contains("evidence does not occur"));
    assert!(finding(json!({"path": "baseline.txt"})).contains("unchanged path"));
    assert!(finding(json!({"scar_id": "NYA-UNKNOWN"})).contains("unknown scar"));
    assert!(finding(json!({"reason": " "})).contains("incomplete finding"));
    assert!(nya::check(&repo.root, CheckRequest { verdict: Some("{not json".into()), ..Default::default() }).unwrap_err().to_string().contains("malformed host verdict"));
    assert!(nya::check(&repo.root, CheckRequest { verdict: Some("missing-verdict.json".into()), ..Default::default() }).unwrap_err().to_string().contains("was not found"));
}

#[test]
fn cli_reports_a_pending_brief_with_exit_code_two_and_accepts_a_verdict_file() {
    let (repo, scar) = changed_repo();

    let human = bin(&repo, &["check", "--brief"]);
    assert_eq!(human.status.code(), Some(2));
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("Host verdict required for 1 scar.") && text.contains(&scar.id));

    let machine = bin(&repo, &["--format", "json", "check", "--brief"]);
    assert_eq!(machine.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&machine.stdout).unwrap();
    assert_eq!(value["passed"], false);
    let audit = value["pending"]["audit"].clone();

    let outside = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(outside.path(), json!({"audit": audit, "clear": [scar.id], "findings": []}).to_string()).unwrap();
    let clean = bin(&repo, &["--format", "json", "check", "--verdict", &outside.path().to_string_lossy()]);
    assert_eq!(clean.status.code(), Some(0));
    assert!(serde_json::from_slice::<Value>(&clean.stdout).unwrap().get("pending").is_none());

    let inline = json!({"audit": audit, "clear": [], "findings": [repeated(&scar)]}).to_string();
    let blocked = bin(&repo, &["check", "--verdict", &inline]);
    assert_eq!(blocked.status.code(), Some(1));
    assert!(String::from_utf8(blocked.stdout).unwrap().contains(&scar.id));

    let stale = bin(&repo, &["check", "--verdict", r#"{"audit":"old","clear":[],"findings":[]}"#]);
    assert_eq!(stale.status.code(), Some(2));
    assert!(String::from_utf8(stale.stderr).unwrap().contains("stale"));
    assert_ne!(bin(&repo, &["check", "--brief", "--verdict", "{}"]).status.code(), Some(0));
}

#[test]
fn mcp_check_serves_the_brief_and_validates_the_verdict() {
    let (repo, scar) = changed_repo();
    let root = repo.root.to_string_lossy();
    let call = |arguments: Value| {
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"nya_check","arguments":arguments}});
        let mut output = Vec::new();
        nya::serve_mcp_io(Cursor::new(format!("{request}\n")), &mut output).unwrap();
        serde_json::from_slice::<Value>(&output).unwrap()["result"]["structuredContent"].clone()
    };

    let pending = call(json!({"repository": root, "brief": true}));
    assert_eq!(pending["pending"]["scars"][0]["id"], scar.id.as_str());
    let verdict = json!({"audit": pending["pending"]["audit"], "clear": [scar.id], "findings": []}).to_string();
    assert_eq!(call(json!({"repository": root, "verdict": verdict}))["passed"], true);
}
