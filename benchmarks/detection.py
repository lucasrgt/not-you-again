#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import platform
import shutil
import tempfile
import time
import tomllib
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from smoke import CASES, initialize, run


@dataclass(frozen=True)
class Recurrence:
    case: str
    path: str
    body: str


RECURRENCES = [
    Recurrence(
        "design-token",
        "src/Button.tsx",
        '''type Tone = "default" | "destructive";

const toneStyles: Record<Tone, { background: string; color: string }> = {
  default: {
    background: "var(--color-accent)",
    color: "var(--color-on-accent)",
  },
  destructive: {
    background: "#b91c1c",
    color: "#ffffff",
  },
};

export function buttonStyle(tone: Tone) {
  return toneStyles[tone];
}
''',
    ),
    Recurrence(
        "localized-string",
        "src/session.ts",
        '''import { message } from "./messages";

export const profileSavedMessage = (): string => message("profile.saved");
export const sessionExpiredMessage = (): string =>
  "Your session expired. Sign in again.";
''',
    ),
    Recurrence(
        "shell-arguments",
        "src/search.ts",
        '''import { exec } from "node:child_process";
import { promisify } from "node:util";

const execute = promisify(exec);

export function normalizeDirectory(directory: string): string {
  return directory.trim();
}

export async function countMatches(pattern: string, directory: string) {
  const { stdout } = await execute(`rg "${pattern}" "${directory}"`);
  return stdout.trim().split("\\n").filter(Boolean).length;
}
''',
    ),
    Recurrence(
        "aware-datetime",
        "src/lease.py",
        '''from datetime import datetime


def parse_expiry(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def is_expired(expires_at: str) -> bool:
    return datetime.now() >= parse_expiry(expires_at)
''',
    ),
    Recurrence(
        "api-compatibility",
        "src/user.py",
        '''from dataclasses import dataclass


@dataclass
class User:
    username: str
    display_name: str | None = None


def serialize_user(user: User) -> dict[str, str]:
    return {"username": user.display_name or user.username}
''',
    ),
]


def configure_judge(root: Path, nya: Path, codex: str, model: str | None):
    if model is None:
        return
    run(
        [
            str(nya),
            "--repository",
            str(root),
            "setup",
            "--local",
            "--judge",
            "codex",
            "--",
            codex,
            "exec",
            "--model",
            model,
            "--ephemeral",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--ignore-user-config",
            "--ignore-rules",
            "--output-schema",
            "{schema}",
            "-",
        ],
        root,
    )


def execute_case(
    recurrence: Recurrence,
    root: Path,
    output: Path,
    nya: Path,
    codex: str,
    model: str | None,
):
    case = next(case for case in CASES if case.name == recurrence.case)
    initialize(root, case, "nya", nya)
    configure_judge(root, nya, codex, model)
    scar_path = next((root / ".nya/scars").glob("*.toml"))
    scar = tomllib.loads(scar_path.read_text(encoding="utf-8"))
    shutil.copy2(scar_path, output / f"{case.name}-scar.toml")

    changed = root / recurrence.path
    changed.parent.mkdir(parents=True, exist_ok=True)
    changed.write_text(recurrence.body, encoding="utf-8", newline="\n")
    diff = run(["git", "diff", "--binary", "HEAD"], root).stdout
    (output / f"{case.name}.diff").write_text(diff, encoding="utf-8", newline="\n")

    recall = run(
        [
            str(nya),
            "--repository",
            str(root),
            "--format",
            "json",
            "recall",
            "--task",
            case.task,
            "--path",
            recurrence.path,
        ],
        root,
    )
    (output / f"{case.name}-recall.json").write_text(
        recall.stdout, encoding="utf-8", newline="\n"
    )
    recalled = json.loads(recall.stdout)

    env = os.environ.copy()
    env.pop("CODEX_SANDBOX_NETWORK_DISABLED", None)
    started = time.monotonic()
    gate = run(
        [
            str(nya),
            "--repository",
            str(root),
            "--format",
            "json",
            "check",
            "--task",
            case.task,
        ],
        root,
        env=env,
        timeout=300,
        check=False,
    )
    elapsed = round(time.monotonic() - started, 3)
    (output / f"{case.name}-check.json").write_text(
        gate.stdout, encoding="utf-8", newline="\n"
    )
    (output / f"{case.name}-check.stderr.log").write_text(
        gate.stderr, encoding="utf-8", newline="\n"
    )
    try:
        verdict = json.loads(gate.stdout)
        findings = verdict["findings"]
        scars_checked = verdict["scars_checked"]
    except (json.JSONDecodeError, KeyError, TypeError):
        findings = []
        scars_checked = None
    matching = [
        finding
        for finding in findings
        if finding.get("scar_id") == scar["id"]
        and finding.get("path") == recurrence.path
        and finding.get("evidence") in diff
    ]
    recalled_ids = [item["id"] for item in recalled]
    detected = gate.returncode == 1 and bool(matching)
    return {
        "case": case.name,
        "scar_id": scar["id"],
        "path": recurrence.path,
        "scar_recalled": scar["id"] in recalled_ids,
        "scars_checked": scars_checked,
        "check_exit": gate.returncode,
        "finding_count": len(findings),
        "matching_findings": len(matching),
        "detected": detected,
        "seconds": elapsed,
    }


def render_report(summary):
    lines = [
        "# Not You Again Persisted Scar Detection Benchmark",
        "",
        f"Run from `{summary['started_at']}` to `{summary['completed_at']}` with "
        f"`{summary['judge']}` on `{summary['platform']}`.",
        "",
        f"NYA binary: `{summary['nya']}` with SHA256 `{summary['nya_sha256']}`.",
        *(
            [
                "",
                f"Source archive: `{summary['source_archive']}` with SHA256 "
                f"`{summary['source_archive_sha256']}`.",
            ]
            if summary["source_archive"]
            else []
        ),
        "",
        "Each fresh repository contains one committed scar. The runner then injects "
        "a concrete recurrence into the matching scope and invokes the real "
        "`nya check` two-stage judge.",
        "",
        "| Case | Recalled | Scars checked | Exit | Matching finding | Detected |",
        "| --- | --- | ---: | ---: | ---: | --- |",
    ]
    for result in summary["results"]:
        lines.append(
            f"| `{result['case']}` | {'yes' if result['scar_recalled'] else 'no'} | "
            f"{result['scars_checked']} | {result['check_exit']} | "
            f"{result['matching_findings']} | "
            f"{'yes' if result['detected'] else 'no'} |"
        )
    lines += [
        "",
        f"Persisted recurrences detected and blocked: "
        f"**{summary['detected']} of {summary['total']}**.",
        "",
        "A case passes only when the persisted scar is recalled, `nya check` exits "
        "with code 1, and the structured verdict contains that exact scar ID, "
        "changed path, and verbatim diff evidence.",
        "",
        "This benchmark measures known-recurrence detection, not agent behavior or "
        "a general prevention rate. The seeded scars and injected recurrences are "
        "synthetic and are retained as auditable fixtures.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nya", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model")
    parser.add_argument("--source-archive")
    parser.add_argument("--source-archive-sha256")
    args = parser.parse_args()
    nya = args.nya.resolve()
    output = args.output.resolve()
    if not nya.is_file():
        raise SystemExit(f"nya binary not found: {nya}")
    if bool(args.source_archive) != bool(args.source_archive_sha256):
        raise SystemExit("source archive URL and SHA256 must be supplied together")
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="nya-detection-benchmark-"))
    results = []
    for index, recurrence in enumerate(RECURRENCES, start=1):
        print(f"[{index}/{len(RECURRENCES)}] {recurrence.case}", flush=True)
        results.append(
            execute_case(
                recurrence,
                work / recurrence.case,
                output,
                nya,
                args.codex,
                args.model,
            )
        )
    summary = {
        "schema": 1,
        "benchmark": "persisted-scar-detection",
        "started_at": started_at,
        "completed_at": datetime.now(timezone.utc).isoformat(),
        "nya": run([str(nya), "--version"], Path.cwd()).stdout.strip(),
        "nya_sha256": hashlib.sha256(nya.read_bytes()).hexdigest(),
        "source_archive": args.source_archive,
        "source_archive_sha256": args.source_archive_sha256,
        "judge": (
            f"{run([args.codex, '--version'], Path.cwd()).stdout.strip()} "
            f"({args.model or 'default model'})"
        ),
        "platform": platform.platform(),
        "total": len(results),
        "detected": sum(result["detected"] for result in results),
        "results": results,
    }
    (output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    report = render_report(summary)
    (output / "REPORT.md").write_text(report, encoding="utf-8", newline="\n")
    print(report)
    print(f"worktree={work}", flush=True)
    if summary["detected"] != summary["total"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
