use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const MAX_PRODUCTION_LINES: u64 = 500;
const MAX_FILE_LINES: u64 = 500;
const MINIMUM_LINE_COVERAGE: u64 = 95;

fn main() {
    if let Err(error) = execute(env::args().skip(1).collect()) {
        eprintln!("verify failed: {error}");
        std::process::exit(1);
    }
}

fn execute(arguments: Vec<String>) -> Result<(), String> {
    if arguments.as_slice() != ["verify"] {
        return Err("usage: cargo xtask verify".into());
    }

    let root = repository_root()?;
    println!("NYA repository verification");

    run(&root, "cargo", &["fmt", "--all", "--", "--check"])?;
    run(&root, "cargo", &["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"])?;
    enforce_line_budget(&root)?;
    run(&root, "cargo", &["test", "--workspace", "--all-features", "--locked"])?;
    let minimum_coverage = MINIMUM_LINE_COVERAGE.to_string();
    run(&root, "cargo", &["llvm-cov", "--package", "not-you-again", "--all-features", "--locked", "--fail-under-lines", &minimum_coverage])?;

    println!("verify passed");
    Ok(())
}

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_path_buf).ok_or_else(|| "xtask must be located inside the repository".into())
}

fn run(root: &Path, program: &str, arguments: &[&str]) -> Result<(), String> {
    println!("  > {program} {}", arguments.join(" "));
    let status = Command::new(program).args(arguments).current_dir(root).status().map_err(|error| format!("could not start {program}: {error}"))?;

    if status.success() { Ok(()) } else { Err(format!("{program} exited with {status}")) }
}

fn enforce_line_budget(root: &Path) -> Result<(), String> {
    println!("  > tokei src --output json");
    let output = Command::new("tokei").args(["src", "--output", "json"]).current_dir(root).output().map_err(|error| format!("could not start tokei: {error}"))?;
    let (total, files) = production_lines(&output)?;
    if total > MAX_PRODUCTION_LINES {
        return Err(format!("production line budget exceeded: {total}/{MAX_PRODUCTION_LINES}"));
    }
    if let Some((path, lines)) = files.into_iter().find(|(_, lines)| *lines > MAX_FILE_LINES) {
        return Err(format!("production file budget exceeded: {path} has {lines}/{MAX_FILE_LINES} lines"));
    }
    println!("    production lines: {total}/{MAX_PRODUCTION_LINES} (max file {MAX_FILE_LINES})");
    Ok(())
}

fn production_lines(output: &Output) -> Result<(u64, Vec<(String, u64)>), String> {
    if !output.status.success() {
        return Err(format!("tokei exited with {}", output.status));
    }
    parse_production_lines(&output.stdout)
}

fn parse_production_lines(json: &[u8]) -> Result<(u64, Vec<(String, u64)>), String> {
    let report: serde_json::Value = serde_json::from_slice(json).map_err(|error| format!("invalid tokei JSON: {error}"))?;
    let rust = report.get("Rust").ok_or_else(|| "tokei JSON did not contain Rust".to_string())?;
    let total = rust.get("code").and_then(serde_json::Value::as_u64).ok_or_else(|| "tokei JSON did not contain Rust.code".to_string())?;
    let files = rust
        .get("reports")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| Some((item.get("name")?.as_str()?.to_owned(), item.pointer("/stats/code")?.as_u64()?)))
        .collect();
    Ok((total, files))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_code_lines_and_files() {
        let (total, files) = parse_production_lines(
            br#"{"Rust":{"code":487,"reports":[{"name":"src/lib.rs","stats":{"code":464}},{"name":"src/main.rs","stats":{"code":23}}]}}"#,
        )
        .unwrap();
        assert_eq!(total, 487);
        assert_eq!(files, vec![("src/lib.rs".into(), 464), ("src/main.rs".into(), 23)]);
    }

    #[test]
    fn rejects_missing_rust_report() {
        assert_eq!(parse_production_lines(br#"{"TOML":{"code":20}}"#).unwrap_err(), "tokei JSON did not contain Rust");
    }

    #[test]
    fn enforces_total_and_file_budgets() {
        assert!(MAX_PRODUCTION_LINES >= MAX_FILE_LINES);
        let ok = parse_production_lines(br#"{"Rust":{"code":500,"reports":[{"name":"src/lib.rs","stats":{"code":500}}]}}"#).unwrap();
        assert_eq!(ok.0, 500);
        assert!(ok.1.iter().all(|(_, lines)| *lines <= MAX_FILE_LINES));
        let oversized = parse_production_lines(br#"{"Rust":{"code":501,"reports":[{"name":"src/lib.rs","stats":{"code":501}}]}}"#).unwrap();
        assert!(oversized.0 > MAX_PRODUCTION_LINES || oversized.1.iter().any(|(_, lines)| *lines > MAX_FILE_LINES));
    }
}
