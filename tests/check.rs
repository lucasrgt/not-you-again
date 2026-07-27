mod common;

use common::{EnvGuard, Repo, conditional, empty_verdict, failing_judge, fake_program, finding, isolated_home_judge, judge, matching_judge, recording_judge, recording_matching_judge, slow_judge};
use nya::{CheckRequest, Occurrence, RememberRequest, Scar};
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
    let (repo, scar) = changed_repo();
    repo.configure(judge(&finding(&scar.id, "src/new.rs")), 5);
    repo.commit_all("implementation");
    let default = nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert!(default.passed);
    assert_eq!(default.scars_checked, 0);
    let result = nya::check(&repo.root, CheckRequest { base: Some("HEAD^".into()), ..Default::default() }).unwrap();
    assert!(!result.passed);
    assert_eq!(result.scars_checked, 1);
    assert_eq!(result.findings[0].scar_id, scar.id);
}

#[test]
fn check_preserves_unicode_changed_paths_for_the_judge_contract() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo);
    repo.commit_all("add scar");
    let path = "src/conclusão.rs";
    repo.write(path, "const MESSAGE: &str = \"literal\";\n");
    let verdict = finding(&scar.id, path);
    let judge = fake_program(&repo.root, "unicode-judge", &format!("print!(\"{{}}\", {verdict:?});"));
    repo.configure(vec![judge.to_string_lossy().into_owned()], 5);

    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();

    assert!(!result.passed);
    assert_eq!(result.findings[0].path, path);
}

#[test]
fn check_accepts_both_changed_sides_of_a_rename() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scar = scar(&repo);
    repo.write("src/old.rs", "const MESSAGE: &str = \"literal\";\n");
    repo.commit_all("add original path");
    common::run(&repo.root, &["mv", "src/old.rs", "src/new.rs"]);
    repo.write("src/new.rs", "const MESSAGE: &str = \"literal changed\";\n");
    repo.configure(judge(&finding(&scar.id, "src/old.rs")), 5);

    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();

    assert!(!result.passed);
    assert_eq!(result.findings[0].path, "src/old.rs");
}

#[test]
fn check_bounds_and_finds_target_in_thousand_scar_corpus() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let target_id = "NYA-STRESS-1023";
    for index in 0..1024 {
        let target = index == 1023;
        let scar = Scar {
            schema: 1,
            id: format!("NYA-STRESS-{index:04}"),
            title: if target { "Measured expensive invoice totals are recomputed during React renders".into() } else { format!("Unrelated repository lesson {index:04}") },
            lesson: if target { "Cache the measured expensive invoice total calculation with useMemo.".into() } else { format!("Preserve unrelated invariant {index:04}.") },
            scope: vec!["src/**".into()],
            tags: vec!["stress".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
            occurrences: vec![Occurrence {
                occurred_at: "2026-01-01T00:00:00Z".into(),
                source: Some(format!("benchmark:stress/{index:04}")),
                reported_by: Some("benchmark:reviewer".into()),
                corrected_by: Some("benchmark:developer".into()),
                recorded_by: Some("benchmark:runner".into()),
                recorded_for: None,
                commit: None,
            }],
        };
        fs::write(repo.root.join(format!(".nya/scars/{}.toml", scar.id)), toml::to_string_pretty(&scar).unwrap()).unwrap();
    }
    repo.commit_all("seed stress corpus");
    repo.write("src/Dashboard.tsx", "const literal = calculateExpensiveInvoiceTotals(invoices);\n");
    repo.configure(matching_judge(target_id, &finding(target_id, "src/Dashboard.tsx")), 5);

    let result = nya::check(&repo.root, CheckRequest { task: Some("Memoize measured expensive invoice totals with useMemo".into()), ..Default::default() }).unwrap();

    assert!(!result.passed);
    assert_eq!(result.scars_checked, 1024);
    assert_eq!(result.findings[0].scar_id, target_id);
}

#[test]
fn codex_judge_receives_an_isolated_writable_home() {
    let (repo, _) = changed_repo();
    repo.configure_as("codex", isolated_home_judge(), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
}

#[test]
fn check_bounds_initial_and_confirmation_requests() {
    let (repo, scar) = changed_repo();
    let evidence = "const LATE_RECURRENCE: &str = \"present\";";
    repo.write("src/new.rs", &format!("{}\n{evidence}\n", "x".repeat(130_000)));
    let log = repo.root.join("judge-sizes.txt");
    let verdict = serde_json::json!({"findings":[{"scar_id":scar.id,"path":"src/new.rs","line":2,"evidence":evidence,"reason":"The late changed code repeats the supplied scar."}]}).to_string();
    repo.configure(recording_matching_judge(&log, evidence, &verdict), 5);
    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert!(!result.passed);
    assert_eq!(result.findings[0].evidence, evidence);
    let sizes = fs::read_to_string(log).unwrap().lines().map(|line| line.trim().parse::<usize>().unwrap()).collect::<Vec<_>>();
    assert_eq!(sizes.len(), 3);
    assert!(sizes.iter().all(|size| *size < 110_000), "{sizes:?}");
}

#[test]
fn check_batches_hundreds_of_paths_into_bounded_judge_calls() {
    let (repo, _) = changed_repo();
    for index in 0..256 {
        repo.write(&format!("src/components/View{index:03}.tsx"), "export const View = () => <section>literal</section>;\n");
    }
    let log = repo.root.join("judge-sizes.txt");
    repo.configure(recording_judge(&log, &empty_verdict()), 5);

    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);

    let sizes = fs::read_to_string(log).unwrap().lines().map(|line| line.trim().parse::<usize>().unwrap()).collect::<Vec<_>>();
    assert_eq!(sizes.len(), 5);
    assert!(sizes.iter().all(|size| *size < 110_000), "{sizes:?}");
}

#[test]
fn judge_configuration_and_process_fail_closed() {
    let config_home = tempfile::tempdir().unwrap();
    let _config = EnvGuard::set("NYA_CONFIG", config_home.path().join("missing.toml"));
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
