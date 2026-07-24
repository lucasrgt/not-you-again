mod common;

use common::{Repo, output};
use nya::{RecallRequest, RememberRequest};
use std::fs;

fn remember(title: &str, lesson: &str, scope: &[&str]) -> RememberRequest {
    RememberRequest {
        title: Some(title.into()),
        lesson: Some(lesson.into()),
        scope: scope.iter().map(|s| (*s).into()).collect(),
        tags: vec!["frontend".into()],
        source: Some("https://example.test/review/1".into()),
        reported_by: Some("github:reviewer".into()),
        corrected_by: Some("github:developer".into()),
        recorded_by: Some("agent:codex".into()),
        ..Default::default()
    }
}

#[test]
fn init_is_idempotent_and_manages_existing_agent_files() {
    let repo = Repo::new(&["AGENTS.md", "CLAUDE.md"]);
    let installed = nya::init(&repo.root).unwrap();
    assert_eq!(installed, ["AGENTS.md", "CLAUDE.md"]);
    assert!(repo.root.join(".nya/scars").is_dir());
    assert!(fs::read_to_string(repo.root.join(".nya/SKILL.md")).unwrap().contains("name: not-you-again"));
    let config = fs::read_to_string(repo.root.join(".nya/config.toml")).unwrap();
    assert!(config.contains("timeout_seconds = 120"));
    assert!(!config.contains("judge"));
    assert_eq!(fs::read_to_string(repo.root.join(".nya/.gitignore")).unwrap(), "config.local.toml\n");
    fs::write(repo.root.join(".nya/config.local.toml"), "judge = \"codex\"\n").unwrap();
    assert_eq!(output(&repo.root, &["check-ignore", ".nya/config.local.toml"]), ".nya/config.local.toml");

    let installed_again = nya::init(&repo.root).unwrap();
    assert_eq!(installed_again, installed);
    let agents = fs::read_to_string(repo.root.join("AGENTS.md")).unwrap();
    assert!(agents.contains("Original instructions."));
    assert_eq!(agents.matches("nya:instructions:start").count(), 1);

    let plain = Repo::new(&[]);
    assert!(nya::init(&plain.root).unwrap().is_empty());
}

#[test]
fn remember_creates_and_appends_with_complete_provenance() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let first = nya::remember(&repo.root, remember("Literal colors bypass tokens", "Use semantic design tokens.", &["src/**/*.tsx"])).unwrap();
    assert!(first.id.starts_with("NYA-"));
    assert_eq!(first.occurrences.len(), 1);
    assert_eq!(first.occurrences[0].reported_by.as_deref(), Some("github:reviewer"));
    assert!(first.occurrences[0].commit.is_some());

    let second =
        nya::remember(&repo.root, RememberRequest { title: Some("  literal   COLORS bypass TOKENS ".into()), source: Some("https://example.test/review/2".into()), ..Default::default() }).unwrap();
    assert_eq!(second.id, first.id);
    assert_eq!(second.occurrences.len(), 2);
    assert_eq!(second.occurrences[1].corrected_by.as_deref(), Some("git:test@example.com"));

    let third = nya::remember(&repo.root, RememberRequest { scar: Some(first.id.clone()), ..Default::default() }).unwrap();
    assert_eq!(third.occurrences.len(), 3);
    let stored = fs::read_to_string(repo.root.join(format!(".nya/scars/{}.toml", first.id))).unwrap();
    assert!(stored.contains("[[occurrences]]"));
}

#[test]
fn remember_rejects_unverified_or_invalid_records() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    assert!(nya::remember(&repo.root, RememberRequest::default()).unwrap_err().to_string().contains("--title"));
    assert!(nya::remember(&repo.root, RememberRequest { title: Some("Title".into()), ..Default::default() }).unwrap_err().to_string().contains("--lesson"));
    assert!(nya::remember(&repo.root, RememberRequest { scar: Some("NYA-MISSING".into()), ..Default::default() }).unwrap_err().to_string().contains("was not found"));
    assert!(
        nya::remember(&repo.root, RememberRequest { reported_by: Some("reviewer".into()), ..remember("Bad actor", "Names must be namespaced.", &[]) }).unwrap_err().to_string().contains("namespaced")
    );
    assert!(nya::remember(&repo.root, remember("Bad scope", "Scope must compile.", &["["])).unwrap_err().to_string().contains("invalid scope"));
}

#[test]
fn recall_combines_exact_scope_fts_and_occurrence_rank() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    let scoped = nya::remember(&repo.root, remember("Design tokens", "Never use literal colors.", &["src/ui/**/*.tsx"])).unwrap();
    let memo = nya::remember(&repo.root, remember("Memoize totals", "Use memoization for expensive totals.", &[])).unwrap();
    nya::remember(&repo.root, RememberRequest { scar: Some(memo.id.clone()), ..Default::default() }).unwrap();
    nya::remember(&repo.root, remember("Database transaction", "Commit inventory atomically.", &["db/**"])).unwrap();

    let exact = nya::recall(&repo.root, RecallRequest { task: "Unrelated copy change".into(), paths: vec!["src/ui/cart/Total.tsx".into()], limit: Some(0) }).unwrap();
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].id, scoped.id);

    let semantic = nya::recall(&repo.root, RecallRequest { task: "Optimize expensive totals with memoization".into(), paths: vec![], limit: Some(2) }).unwrap();
    assert_eq!(semantic[0].id, memo.id);

    let all = nya::recall(&repo.root, RecallRequest::default()).unwrap();
    assert_eq!(all[0].id, memo.id);
    assert!(repo.git_dir().join("nya/index-v1.sqlite3").is_file());
}

#[test]
fn corrupt_index_is_disposable_and_rebuilt() {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    nya::remember(&repo.root, remember("Magic string", "Use the central constant.", &[])).unwrap();
    nya::recall(&repo.root, RecallRequest::default()).unwrap();
    let index = repo.git_dir().join("nya/index-v1.sqlite3");
    fs::write(&index, b"not sqlite").unwrap();
    let recalled = nya::recall(&repo.root, RecallRequest { task: "Replace magic string".into(), ..Default::default() }).unwrap();
    assert_eq!(recalled.len(), 1);
    assert!(fs::metadata(index).unwrap().len() > 10);
}

#[test]
fn repository_and_scar_errors_are_explicit() {
    let outside = tempfile::tempdir().unwrap();
    assert!(nya::repository(outside.path()).unwrap_err().to_string().contains("Git repository"));

    let repo = Repo::new(&[]);
    assert!(nya::recall(&repo.root, RecallRequest::default()).unwrap_err().to_string().contains("run nya init"));
    nya::init(&repo.root).unwrap();
    fs::write(repo.root.join(".nya/scars/broken.toml"), "schema = nope").unwrap();
    assert!(nya::recall(&repo.root, RecallRequest::default()).unwrap_err().to_string().contains("invalid scar"));
}
