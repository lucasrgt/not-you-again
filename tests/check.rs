mod common;

use common::{Repo, conditional, empty_verdict, failing_judge, finding, judge, recording_judge, slow_judge};
use nya::{CheckRequest, RememberRequest};
use std::fs;

fn scar(repo: &Repo) -> nya::Scar {
    nya::remember(
        &repo.root,
        RememberRequest {
            title: Some("Magic strings bypass the catalog".into()),
            lesson: Some("Use the typed message catalog instead of literal copy.".into()),
            scope: vec!["src/**".into()],
            reported_by: Some("github:reviewer".into()),
            ..Default::default()
        },
    )
    .unwrap()
}

fn changed_repo() -> (Repo, nya::Scar) {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo);
    repo.commit_all("add scar");
    repo.write("src/new.rs", "const MESSAGE: &str = \"literal\";\n");
    (repo, scar)
}

#[test]
fn check_short_circuits_clean_and_irrelevant_diffs() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);

    nya::remember(&repo.root, RememberRequest { title: Some("Database only".into()), lesson: Some("qvv zyx".into()), scope: vec!["db/**".into()], ..Default::default() }).unwrap();
    repo.commit_all("add irrelevant scar");
    repo.write("src/new.rs", "alpha\n");
    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert!(result.passed);
    assert_eq!(result.scars_checked, 0);
}

#[test]
fn check_passes_after_isolated_batch_judge() {
    let (repo, _) = changed_repo();
    repo.configure(judge(&empty_verdict()), 5);
    let result = nya::check(&repo.root, CheckRequest { task: Some("Add a message".into()), ..Default::default() }).unwrap();
    assert!(result.passed);
    assert_eq!(result.scars_checked, 1);
}

#[test]
fn check_requires_focused_confirmation_before_blocking() {
    let (repo, scar) = changed_repo();
    let proposed = finding(&scar.id, "src/new.rs");
    repo.configure(conditional(&proposed, &empty_verdict()), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);

    repo.configure(judge(&proposed), 5);
    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert!(!result.passed);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].scar_id, scar.id);
}

#[test]
fn check_honors_explicit_base_for_committed_work() {
    let (repo, _) = changed_repo();
    repo.configure(judge(&empty_verdict()), 5);
    repo.commit_all("implementation");
    let result = nya::check(&repo.root, CheckRequest { base: Some("HEAD^".into()), ..Default::default() }).unwrap();
    assert!(result.passed);
    assert_eq!(result.scars_checked, 1);
}

#[test]
fn check_bounds_initial_and_confirmation_requests() {
    let (repo, scar) = changed_repo();
    repo.write("src/large.rs", &format!("literal\n{}", "x".repeat(130_000)));
    let log = repo.root.join("judge-sizes.txt");
    repo.configure(recording_judge(&log, &finding(&scar.id, "src/new.rs")), 5);
    assert!(!nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    let sizes = fs::read_to_string(log).unwrap().lines().map(|line| line.parse::<usize>().unwrap()).collect::<Vec<_>>();
    assert_eq!(sizes.len(), 2);
    assert!(sizes.iter().all(|size| *size < 110_000), "{sizes:?}");
}

#[test]
fn judge_configuration_and_process_fail_closed() {
    let (repo, _) = changed_repo();
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("judge is not configured"));

    repo.configure(vec![], 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("has no command"));

    repo.configure(vec!["missing-nya-judge-command".into()], 1);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("failed to start"));

    repo.configure(failing_judge(), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("runner"));

    repo.configure(judge("not-json"), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("malformed verdict"));

    repo.configure(slow_judge(), 1);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("timed out"));
}

#[test]
fn judge_cannot_invent_scars_paths_or_incomplete_findings() {
    let (repo, scar) = changed_repo();
    repo.configure(judge(&finding("NYA-INVENTED", "src/new.rs")), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("unknown scar"));

    repo.configure(judge(&finding(&scar.id, "src/unchanged.rs")), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("unchanged path"));

    let incomplete = serde_json::json!({
        "findings": [{"scar_id": scar.id, "path": "src/new.rs", "line": 0, "evidence": "", "reason": ""}]
    })
    .to_string();
    repo.configure(judge(&incomplete), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("incomplete finding"));

    let unsupported = serde_json::json!({
        "findings": [{"scar_id": scar.id, "path": "src/new.rs", "line": 1, "evidence": "absent evidence", "reason": "Unsupported"}]
    })
    .to_string();
    repo.configure(judge(&unsupported), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("does not occur"));
}

#[test]
fn unsupported_or_malformed_config_fails_closed() {
    let (repo, _) = changed_repo();
    fs::write(repo.root.join(".nya/config.toml"), "schema = 2\n[check]\ntimeout_seconds = 1\n").unwrap();
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("unsupported config schema"));
    fs::write(repo.root.join(".nya/config.toml"), "not toml =").unwrap();
    assert!(nya::check(&repo.root, CheckRequest::default()).is_err());
}
