mod common;

use common::{Repo, conditional, judge, matching_judge, output};
use nya::{RememberRequest, ReplayRequest, SpecRequest};
use serde_json::json;
use std::{fs, process::Command};

fn scar(repo: &Repo) -> nya::Scar {
    nya::remember(
        &repo.root,
        RememberRequest {
            title: Some("Retry workers lost idempotency".into()),
            lesson: Some("Every retry-capable worker must require an idempotency key.".into()),
            scope: vec!["src/workers/**".into()],
            tags: vec!["retry".into(), "idempotency".into()],
            source: Some("incident:payments-17".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

fn spec_request() -> SpecRequest {
    SpecRequest { files: vec!["specs/retry-worker.md".into()], task: "Design the retry worker".into(), paths: vec!["src/workers/payment.rs".into()], limit: Some(32) }
}

fn gap(id: &str) -> String {
    json!({
        "gaps": [{
            "scar_id": id,
            "reason": "The retry worker is in scope but the specification omits the proven idempotency requirement.",
            "requirement": "Require and persist an idempotency key for every retryable operation."
        }]
    })
    .to_string()
}

#[test]
fn spec_passes_without_relevant_scars_and_rejects_unsafe_inputs() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("specs/retry-worker.md", "# Retry worker\n\nUse exponential backoff.\n");

    let result = nya::spec(&repo.root, spec_request()).unwrap();
    assert!(result.passed);
    assert_eq!(result.files_reviewed, 1);
    assert_eq!(result.scars_reviewed, 0);

    let mut missing = spec_request();
    missing.files = vec!["../outside.md".into()];
    assert!(nya::spec(&repo.root, missing).unwrap_err().to_string().contains("was not found"));

    let mut zero = spec_request();
    zero.limit = Some(0);
    assert!(nya::spec(&repo.root, zero).unwrap_err().to_string().contains("greater than zero"));
}

#[test]
fn spec_uses_two_stage_confirmation_before_blocking() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo);
    repo.write("specs/retry-worker.md", "# Retry worker\n\nUse exponential backoff.\n");

    repo.configure(conditional(&gap(&scar.id), r#"{"gaps":[]}"#), 5);
    assert!(nya::spec(&repo.root, spec_request()).unwrap().passed);

    repo.configure(judge(&gap(&scar.id)), 5);
    let result = nya::spec(&repo.root, spec_request()).unwrap();
    assert!(!result.passed);
    assert_eq!(result.scars_reviewed, 1);
    assert_eq!(result.gaps[0].scar_id, scar.id);
}

#[test]
fn spec_fails_closed_on_invented_or_mutated_gaps() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo);
    repo.write("specs/retry-worker.md", "# Retry worker\n");

    repo.configure(judge(&gap("NYA-INVENTED")), 5);
    assert!(nya::spec(&repo.root, spec_request()).unwrap_err().to_string().contains("unknown scar"));

    let changed = json!({
        "gaps": [{
            "scar_id": scar.id,
            "reason": "Changed during confirmation.",
            "requirement": "A different requirement."
        }]
    })
    .to_string();
    repo.configure(conditional(&gap(&scar.id), &changed), 5);
    assert!(nya::spec(&repo.root, spec_request()).unwrap_err().to_string().contains("changed a specification gap"));
}

fn replay_repo() -> (Repo, nya::Scar, String) {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("src/workers/payment.rs", "retry_without_idempotency();\n");
    repo.commit_all("add retry worker");
    repo.write("src/workers/payment.rs", "retry_with_idempotency_key();\n");
    repo.commit_all("fix retry worker idempotency");
    let commit = output(&repo.root, &["rev-parse", "--short", "HEAD"]);
    let scar = scar(&repo);
    (repo, scar, commit)
}

fn replay_verdict(id: &str, commit: &str, passed: bool) -> String {
    json!({
        "scar_id": id,
        "commit": commit,
        "source": "incident:payments-17",
        "before_repeats": passed,
        "after_fixes": passed,
        "before_evidence": if passed { "-retry_without_idempotency();" } else { "" },
        "after_evidence": if passed { "+retry_with_idempotency_key();" } else { "" },
        "reason": if passed { "The removed call lacked idempotency and the replacement adds it." } else { "The historical pair does not prove both sides." }
    })
    .to_string()
}

#[test]
fn replay_validates_a_real_before_and_after_pair() {
    let (repo, scar, commit) = replay_repo();
    repo.configure(matching_judge("need not include the entire line", &replay_verdict(&scar.id, &commit, true)), 5);

    let result = nya::replay(&repo.root, ReplayRequest { scar: Some(scar.id.clone()), limit: Some(1) }).unwrap();
    assert!(result.passed);
    assert_eq!(result.eligible, 1);
    assert_eq!(result.replayed, 1);
    assert_eq!(result.cases[0].before_evidence, "-retry_without_idempotency();");
}

#[test]
fn replay_reports_failed_unavailable_and_unsupported_pairs() {
    let (repo, scar, commit) = replay_repo();
    repo.configure(judge(&replay_verdict(&scar.id, &commit, false)), 5);
    let result = nya::replay(&repo.root, ReplayRequest { scar: Some(scar.id.clone()), limit: Some(1) }).unwrap();
    assert!(!result.passed);

    let path = repo.root.join(format!(".nya/scars/{}.toml", scar.id));
    let mut stored: nya::Scar = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    stored.occurrences[0].commit = Some("deadbeef".into());
    fs::write(path, toml::to_string_pretty(&stored).unwrap()).unwrap();
    let unavailable = nya::replay(&repo.root, ReplayRequest { scar: Some(scar.id), limit: Some(1) }).unwrap();
    assert!(!unavailable.passed);
    assert_eq!(unavailable.replayed, 0);
    assert!(unavailable.cases[0].reason.contains("unavailable"));
}

#[test]
fn replay_rejects_missing_identity_evidence_and_empty_corpora() {
    let (repo, scar, commit) = replay_repo();
    repo.configure(judge(&replay_verdict("NYA-INVENTED", &commit, true)), 5);
    assert!(nya::replay(&repo.root, ReplayRequest { scar: Some(scar.id.clone()), limit: Some(1) }).unwrap_err().to_string().contains("changed replay identity"));

    repo.configure(judge(&replay_verdict(&scar.id, &commit, true).replace("-retry_without_idempotency();", "-absent();")), 5);
    assert!(nya::replay(&repo.root, ReplayRequest { scar: Some(scar.id), limit: Some(1) }).unwrap_err().to_string().contains("unsupported before evidence"));

    let empty = Repo::new(&[]);
    nya::init(&empty.root).unwrap();
    assert!(nya::replay(&empty.root, ReplayRequest::default()).unwrap_err().to_string().contains("no replayable"));
}

#[test]
fn cli_exposes_spec_and_replay_exit_codes_and_json() {
    let (repo, scar, commit) = replay_repo();
    repo.write("specs/retry-worker.md", "# Retry worker\n");
    repo.configure(judge(&gap(&scar.id)), 5);
    let spec = Command::new(env!("CARGO_BIN_EXE_nya"))
        .arg("--repository")
        .arg(&repo.root)
        .args(["--format", "json", "spec", "--file", "specs/retry-worker.md", "--path", "src/workers/payment.rs"])
        .output()
        .unwrap();
    assert_eq!(spec.status.code(), Some(1));
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&spec.stdout).unwrap()["passed"], false);

    repo.configure(judge(&replay_verdict(&scar.id, &commit, true)), 5);
    let replay = Command::new(env!("CARGO_BIN_EXE_nya")).arg("--repository").arg(&repo.root).args(["replay", "--scar", &scar.id, "--limit", "1"]).output().unwrap();
    assert!(replay.status.success());
    assert!(String::from_utf8(replay.stdout).unwrap().contains("PASS"));
}
