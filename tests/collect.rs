mod common;

use common::{ENV_LOCK, EnvGuard, Repo, conditional, fake_gh, judge, output, run};
use nya::{CollectRequest, RememberRequest};
use serde_json::json;
use std::fs;

fn proposal(source: &str, classification: &str, scar: &str, title: &str, evidence: &str) -> String {
    json!({
        "candidates": [{
            "source_id": source,
            "classification": classification,
            "scar_id": scar,
            "title": title,
            "lesson": "Use repository design tokens instead of literal presentation values.",
            "scope": ["src/**"],
            "tags": ["design-tokens"],
            "evidence": evidence,
            "reason": "The source shows a literal value being corrected to a repository token."
        }]
    })
    .to_string()
}

#[test]
fn git_collection_writes_new_scars_appends_recurrences_and_checkpoints() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("src/card.css", ".card { color: var(--text); }\n");
    repo.commit_all("fix: replace literal color with token");
    let first_commit = output(&repo.root, &["rev-parse", "HEAD"]);
    let first_source = format!("git:{first_commit}");
    let first = proposal(&first_source, "new", "", "Literal colors bypass design tokens", "fix: replace literal color with token");
    repo.configure(judge(&first), 5);

    let result = nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap();
    assert_eq!((result.new_scars, result.occurrences_appended, result.correction_candidates), (1, 0, 1));
    assert_eq!(result.records[0].classification, "new");
    assert_eq!(result.records[0].source, first_source);
    assert!(result.records[0].scar_id.is_some());
    let stored = nya::recall(&repo.root, nya::RecallRequest { task: "design token color".into(), ..Default::default() }).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].occurrences[0].source.as_deref(), Some(first_source.as_str()));
    assert_eq!(stored[0].occurrences[0].corrected_by.as_deref(), Some("git:test@example.com"));
    assert_eq!(stored[0].occurrences[0].recorded_by.as_deref(), Some("nya:collector"));

    let idle = nya::collect(&repo.root, CollectRequest { offline: true, ..Default::default() }).unwrap();
    assert_eq!(idle.correction_candidates, 0);

    repo.write("src/banner.css", ".banner { color: var(--muted); }\n");
    repo.commit_all("fix: remove another literal color");
    let second_commit = output(&repo.root, &["rev-parse", "HEAD"]);
    let second_source = format!("git:{second_commit}");
    let recurrence = proposal(&second_source, "recurrence", &stored[0].id, &stored[0].title, "fix: remove another literal color");
    repo.configure(judge(&recurrence), 5);
    let dry = nya::collect(&repo.root, CollectRequest { dry_run: true, offline: true, ..Default::default() }).unwrap();
    assert_eq!((dry.new_scars, dry.occurrences_appended), (0, 1));
    assert_eq!(dry.records[0].scar_id.as_deref(), Some(stored[0].id.as_str()));
    assert_eq!(nya::recall(&repo.root, nya::RecallRequest::default()).unwrap()[0].occurrences.len(), 1);

    let applied = nya::collect(&repo.root, CollectRequest { offline: true, ..Default::default() }).unwrap();
    assert_eq!(applied.occurrences_appended, 1);
    assert_eq!(nya::recall(&repo.root, nya::RecallRequest::default()).unwrap()[0].occurrences.len(), 2);
}

#[test]
fn resolved_github_review_carries_reporter_and_corrector_provenance() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    run(&repo.root, &["remote", "add", "origin", "https://github.com/acme/store.git"]);
    let reviewed = output(&repo.root, &["rev-parse", "HEAD"]);
    repo.write("src/theme.css", ".card { color: var(--text); }\n");
    repo.commit_all("fix: use the shared text token");
    let correction = output(&repo.root, &["rev-parse", "HEAD"]);
    let permalink = "https://github.com/acme/store/pull/142#discussion_r123";
    let response = json!({
        "html_url": permalink,
        "created_at": "2026-07-24T12:34:56Z",
        "user": {"login": "alice"},
        "body": "Use the shared design token here.",
        "diff_hunk": "@@ -1 +1 @@\n-.card { color: #111; }",
        "path": "src/theme.css",
        "commit_id": reviewed,
        "in_reply_to_id": null
    })
    .to_string();
    let gh = fake_gh(&repo.root, &response);
    let _gh = EnvGuard::set("NYA_GH", &gh);
    let verdict = proposal(permalink, "new", "", "Review corrections must become durable scars", "Use the shared design token here.");
    repo.configure(judge(&verdict), 5);

    let result = nya::collect(&repo.root, CollectRequest { all: true, ..Default::default() }).unwrap();
    assert_eq!(result.github, "scanned");
    assert_eq!(result.new_scars, 1);
    assert_eq!(result.records[0].source, permalink);
    let scar = nya::recall(&repo.root, nya::RecallRequest { task: "shared design token".into(), ..Default::default() }).unwrap().remove(0);
    let occurrence = &scar.occurrences[0];
    assert_eq!(occurrence.source.as_deref(), Some(permalink));
    assert_eq!(occurrence.reported_by.as_deref(), Some("github:alice"));
    assert_eq!(occurrence.corrected_by.as_deref(), Some("git:test@example.com"));
    assert_eq!(occurrence.commit.as_deref(), Some(&correction[..12]));
}

#[test]
fn collector_rejects_invented_evidence_and_changed_confirmations() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("src/card.css", ".card { color: var(--text); }\n");
    repo.commit_all("fix: replace literal color with token");
    let source = format!("git:{}", output(&repo.root, &["rev-parse", "HEAD"]));

    let invented = proposal(&source, "new", "", "Literal colors bypass tokens", "text absent from the source");
    repo.configure(judge(&invented), 5);
    assert!(nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap_err().to_string().contains("not verbatim"));

    let mut bad_scope: serde_json::Value = serde_json::from_str(&proposal(&source, "new", "", "Literal colors bypass tokens", "fix: replace literal color with token")).unwrap();
    bad_scope["candidates"][0]["scope"] = json!(["descriptive scope"]);
    repo.configure(judge(&bad_scope.to_string()), 5);
    assert!(nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap_err().to_string().contains("scope does not match"));

    bad_scope["candidates"][0]["scope"] = json!([]);
    repo.configure(judge(&bad_scope.to_string()), 5);
    assert!(nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap_err().to_string().contains("incomplete lesson or scope"));

    let proposed = proposal(&source, "new", "", "Literal colors bypass tokens", "fix: replace literal color with token");
    let changed = proposal(&source, "new", "", "A changed title", "fix: replace literal color with token");
    repo.configure(conditional(&proposed, &changed), 5);
    assert!(nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap_err().to_string().contains("changed a proposal"));
    assert!(fs::read_dir(repo.root.join(".nya/scars")).unwrap().next().is_none());
}

#[test]
fn manual_scar_sources_are_not_recollected() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("src/card.css", ".card { color: var(--text); }\n");
    repo.commit_all("fix: replace literal color with token");
    let commit = output(&repo.root, &["rev-parse", "HEAD"]);
    nya::remember(
        &repo.root,
        RememberRequest { title: Some("Already recorded".into()), lesson: Some("Use tokens.".into()), scope: vec!["src/**".into()], source: Some(format!("git:{commit}")), ..Default::default() },
    )
    .unwrap();
    let result = nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap();
    assert_eq!(result.correction_candidates, 0);
}

#[test]
fn collector_counts_skipped_ambiguous_and_unclassified_sources() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let mut sources = Vec::new();
    for (path, message) in [("a.rs", "fix: vague cleanup"), ("b.rs", "fix: uncertain behavior"), ("c.rs", "fix: undocumented bug")] {
        repo.write(path, "changed\n");
        repo.commit_all(message);
        sources.push((format!("git:{}", output(&repo.root, &["rev-parse", "HEAD"])), message));
    }
    let verdict = json!({
        "candidates": [
            {
                "source_id": sources[0].0, "classification": "skip", "scar_id": "", "title": "", "lesson": "",
                "scope": [], "tags": [], "evidence": "", "reason": ""
            },
            {
                "source_id": sources[1].0, "classification": "ambiguous", "scar_id": "", "title": "", "lesson": "",
                "scope": [], "tags": [], "evidence": "", "reason": ""
            }
        ]
    })
    .to_string();
    repo.configure(judge(&verdict), 5);
    let result = nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap();
    assert_eq!(result.correction_candidates, 3);
    assert_eq!((result.insufficient_evidence, result.ambiguous), (2, 1));
    assert!(fs::read_dir(repo.root.join(".nya/scars")).unwrap().next().is_none());
}

#[test]
fn github_collection_fails_closed_unless_offline_is_explicit() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    run(&repo.root, &["remote", "add", "origin", "https://github.com/acme/store.git"]);
    let missing = repo.root.join(if cfg!(windows) { "missing-gh.exe" } else { "missing-gh" });
    let _gh = EnvGuard::set("NYA_GH", missing);
    assert!(nya::collect(&repo.root, CollectRequest { all: true, ..Default::default() }).unwrap_err().to_string().contains("install and authenticate"));
    let result = nya::collect(&repo.root, CollectRequest { all: true, offline: true, ..Default::default() }).unwrap();
    assert_eq!(result.github, "skipped by --offline");
}

#[test]
fn since_bounds_history_and_conflicting_ranges_are_rejected() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    repo.write("src/old.rs", "old fix\n");
    repo.commit_all("fix: old defect");
    repo.write("marker.rs", "release\n");
    repo.commit_all("release marker");
    let marker = output(&repo.root, &["rev-parse", "HEAD"]);
    repo.write("src/new.rs", "new fix\n");
    repo.commit_all("fix: new defect");
    let source = format!("git:{}", output(&repo.root, &["rev-parse", "HEAD"]));
    let verdict = proposal(&source, "new", "", "Only recent corrections are collected", "fix: new defect");
    repo.configure(judge(&verdict), 5);
    let result = nya::collect(&repo.root, CollectRequest { since: Some(marker), offline: true, ..Default::default() }).unwrap();
    assert_eq!((result.sources_scanned, result.correction_candidates, result.new_scars), (1, 1, 1));
    let conflict = nya::collect(&repo.root, CollectRequest { all: true, since: Some("HEAD^".into()), offline: true, ..Default::default() }).unwrap_err();
    assert!(conflict.to_string().contains("cannot be combined"));
}
