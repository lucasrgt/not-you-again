mod common;

use common::{Repo, fake_program, run};
use nya::{CheckRequest, RememberRequest};
use std::{fs, path::Path};

fn changed() -> Repo {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    nya::remember(
        &repo.root,
        RememberRequest { title: Some("Preserve the checked contract".into()), lesson: Some("Keep the checked contract valid.".into()), scope: vec!["src/**".into()], ..Default::default() },
    )
    .unwrap();
    repo.commit_all("record contract");
    repo.write("src/new.rs", "const VALUE: &str = \"literal\";\n");
    run(&repo.root, &["add", "src/new.rs"]);
    repo
}

fn recorder(repo: &Repo, name: &str, body: &str) -> Vec<String> {
    let log = repo.root.join("calls.log");
    let program = fake_program(
        &repo.root,
        name,
        &format!("use std::io::Write; let mut log = std::fs::OpenOptions::new().create(true).append(true).open({:?}).unwrap(); writeln!(log, \"called\").unwrap(); {body}", log.to_string_lossy()),
    );
    vec![program.to_string_lossy().into_owned()]
}

fn calls(repo: &Repo) -> usize {
    fs::read_to_string(repo.root.join("calls.log")).unwrap().lines().count()
}

const CLEAN: &str = "print!(\"{{\\\"findings\\\":[]}}\");";

#[test]
fn identical_clean_audits_reuse_receipts_across_task_labels_and_commit_boundaries() {
    let repo = changed();
    repo.configure(recorder(&repo, "judge", CLEAN), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    assert!(nya::check(&repo.root, CheckRequest { task: Some("pre-commit".into()), ..Default::default() }).unwrap().passed);
    repo.commit_all("add checked code");
    assert!(nya::check(&repo.root, CheckRequest { base: Some("HEAD^".into()), task: Some("pre-push".into()) }).unwrap().passed);
    assert_eq!(calls(&repo), 1);
    assert!(!common::output(&repo.root, &["ls-files"]).contains("nya-audit-cache"));
}

#[test]
fn changed_code_scar_or_executable_invalidates_a_clean_receipt() {
    let repo = changed();
    repo.configure(recorder(&repo, "judge", CLEAN), 5);
    nya::check(&repo.root, CheckRequest::default()).unwrap();
    repo.write("src/new.rs", "const VALUE: &str = \"literal changed\";\n");
    nya::check(&repo.root, CheckRequest::default()).unwrap();
    let scar_path = fs::read_dir(repo.root.join(".nya/scars")).unwrap().next().unwrap().unwrap().path();
    let mut scar: toml::Value = toml::from_str(&fs::read_to_string(&scar_path).unwrap()).unwrap();
    scar["lesson"] = toml::Value::String("Keep the updated checked contract valid.".into());
    fs::write(&scar_path, toml::to_string(&scar).unwrap()).unwrap();
    nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert_eq!(calls(&repo), 3);
    recorder(&repo, "judge", "std::process::exit(9);");
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("judge exited"));
    assert_eq!(calls(&repo), 4);
    recorder(&repo, "judge", "print!(\"invalid JSON\");");
    for _ in 0..2 {
        assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("malformed verdict"));
    }
    assert_eq!(calls(&repo), 6);
}

#[test]
fn corrupt_or_unwritable_receipts_recompute_without_changing_the_verdict() {
    let repo = changed();
    repo.configure(recorder(&repo, "judge", CLEAN), 5);
    nya::check(&repo.root, CheckRequest::default()).unwrap();
    let cache = repo.git_dir().join("nya-audit-cache-v1");
    let receipt = fs::read_dir(&cache).unwrap().next().unwrap().unwrap().path();
    fs::write(&receipt, "incomplete receipt").unwrap();
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    assert_eq!(calls(&repo), 2);
    fs::remove_dir_all(&cache).unwrap();
    fs::write(&cache, "not a directory").unwrap();
    for _ in 0..2 {
        assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    }
    assert_eq!(calls(&repo), 4);
}

#[test]
fn changing_a_script_argument_invalidates_its_receipt() {
    let repo = changed();
    let verdict = repo.root.join("judge-verdict.json");
    fs::write(&verdict, "{\"findings\":[]}").unwrap();
    let mut command = recorder(&repo, "script-judge", "let output = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap(); print!(\"{}\", output);");
    command.push(verdict.to_string_lossy().into_owned());
    repo.configure(command, 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    fs::write(&verdict, "not JSON").unwrap();
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("malformed verdict"));
    assert_eq!(calls(&repo), 2);
}

fn parallel_repo() -> Repo {
    let repo = Repo::new(&[]);
    nya::init(&repo.root).unwrap();
    for index in 0..7 {
        nya::remember(
            &repo.root,
            RememberRequest { title: Some(format!("Contract {index}")), lesson: Some(format!("Keep contract {index} valid.")), scope: vec![format!("src/{index}.rs")], ..Default::default() },
        )
        .unwrap();
        repo.write(&format!("src/{index}.rs"), "const VALUE: &str = \"literal\";\n");
    }
    repo
}

fn timeline_judge(repo: &Repo, directory: &Path, fail: bool) -> Vec<String> {
    let body = format!(
        "use std::io::Read; let mut prompt = String::new(); std::io::stdin().read_to_string(&mut prompt).unwrap(); let dir = std::path::Path::new({directory:?}); let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(); let start = now(); std::thread::sleep(std::time::Duration::from_millis(400)); let end = now(); std::fs::write(dir.join(format!(\"{{}}.interval\", std::process::id())), format!(\"{{start}} {{end}}\")).unwrap(); if {fail} && prompt.contains(\"src/6.rs\") {{ std::process::exit(7); }} print!(\"{{{{\\\"findings\\\":[]}}}}\");",
        directory = directory.to_string_lossy()
    );
    vec![fake_program(&repo.root, "timeline-judge", &body).to_string_lossy().into_owned()]
}

#[test]
fn independent_batches_are_bounded_parallel_and_complete_before_success() {
    let repo = parallel_repo();
    let events = tempfile::tempdir().unwrap();
    repo.configure(timeline_judge(&repo, events.path(), false), 5);
    let result = nya::check(&repo.root, CheckRequest::default()).unwrap();
    assert!(result.passed);
    assert_eq!(result.scars_checked, 7);
    let mut timeline = Vec::new();
    for file in fs::read_dir(events.path()).unwrap() {
        let interval = fs::read_to_string(file.unwrap().path()).unwrap();
        let times = interval.split_whitespace().map(|value| value.parse::<u128>().unwrap()).collect::<Vec<_>>();
        timeline.push((times[0], 1));
        timeline.push((times[1], -1));
    }
    assert_eq!(timeline.len(), 14);
    timeline.sort();
    let (mut active, mut peak) = (0, 0);
    for (_, change) in timeline {
        active += change;
        peak = peak.max(active);
    }
    assert!((2..=4).contains(&peak), "peak concurrent judges: {peak}");
    assert_eq!(active, 0);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap().passed);
    assert_eq!(fs::read_dir(events.path()).unwrap().count(), 7);
}

#[test]
fn one_failed_parallel_batch_prevents_a_pass() {
    let repo = parallel_repo();
    let events = tempfile::tempdir().unwrap();
    repo.configure(timeline_judge(&repo, events.path(), true), 5);
    assert!(nya::check(&repo.root, CheckRequest::default()).unwrap_err().to_string().contains("judge exited"));
}
