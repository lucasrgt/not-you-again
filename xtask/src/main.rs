use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const MAX_PRODUCTION_LINES: u64 = 500;
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
    let lines = production_lines(&output)?;
    check_line_budget(lines)?;
    println!("    production lines: {lines}/{MAX_PRODUCTION_LINES}");
    Ok(())
}

fn production_lines(output: &Output) -> Result<u64, String> {
    if !output.status.success() {
        return Err(format!("tokei exited with {}", output.status));
    }

    parse_production_lines(&output.stdout)
}

fn parse_production_lines(json: &[u8]) -> Result<u64, String> {
    let report: serde_json::Value = serde_json::from_slice(json).map_err(|error| format!("invalid tokei JSON: {error}"))?;
    report.pointer("/Rust/code").and_then(serde_json::Value::as_u64).ok_or_else(|| "tokei JSON did not contain Rust.code".into())
}

fn check_line_budget(lines: u64) -> Result<(), String> {
    if lines <= MAX_PRODUCTION_LINES { Ok(()) } else { Err(format!("production line budget exceeded: {lines}/{MAX_PRODUCTION_LINES}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_code_lines() {
        assert_eq!(parse_production_lines(br#"{"Rust":{"code":487}}"#).unwrap(), 487);
    }

    #[test]
    fn rejects_missing_rust_report() {
        assert_eq!(parse_production_lines(br#"{"TOML":{"code":20}}"#).unwrap_err(), "tokei JSON did not contain Rust.code");
    }

    #[test]
    fn enforces_line_budget_boundary() {
        assert!(check_line_budget(MAX_PRODUCTION_LINES).is_ok());
        assert_eq!(check_line_budget(MAX_PRODUCTION_LINES + 1).unwrap_err(), "production line budget exceeded: 501/500");
    }
}
