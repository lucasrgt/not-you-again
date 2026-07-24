#!/usr/bin/env python3
import argparse
import importlib.util
import json
import os
import platform
import random
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path


@dataclass(frozen=True)
class Case:
    name: str
    task: str
    title: str
    lesson: str
    scope: str
    reference: str
    files: dict[str, str]


CASES = [
    Case(
        "design-token",
        "Add a destructive tone to buttonStyle for delete actions. It should use a red background and a readable light foreground while preserving the existing API.",
        "Literal component colors bypass semantic design tokens",
        "Component colors must reference role-based CSS custom properties. Add a semantic token when a role is missing instead of placing a literal color in a component.",
        "src/**/*.tsx",
        "https://designsystem.digital.gov/design-tokens/",
        {
            "src/Button.tsx": '''type Tone = "default";

const toneStyles: Record<Tone, { background: string; color: string }> = {
  default: {
    background: "var(--color-accent)",
    color: "var(--color-on-accent)",
  },
};

export function buttonStyle(tone: Tone) {
  return toneStyles[tone];
}
''',
            "src/tokens.css": ''':root {
  --color-accent: #3155a4;
  --color-on-accent: #ffffff;
}
''',
        },
    ),
    Case(
        "localized-string",
        'Add a sessionExpiredMessage export for the timeout banner. It must display exactly "Your session expired. Sign in again."',
        "User-facing strings bypass the message catalog",
        "Store every user-facing sentence in src/messages.ts and reference its key from UI modules. Do not place the English sentence directly in a component module.",
        "src/**/*.ts",
        "https://developer.android.com/guide/topics/resources/localization",
        {
            "src/messages.ts": '''export const messages = {
  "profile.saved": "Profile saved.",
} as const;

export type MessageKey = keyof typeof messages;

export function message(key: MessageKey): string {
  return messages[key];
}
''',
            "src/session.ts": '''import { message } from "./messages";

export const profileSavedMessage = (): string => message("profile.saved");
''',
        },
    ),
    Case(
        "shell-arguments",
        "Add an async countMatches(pattern, directory) export in src/search.ts. Invoke ripgrep and return the number of matching lines. Patterns and directories may contain spaces.",
        "Dynamic process arguments were interpolated into a shell command",
        "Invoke external programs without a shell. Pass every dynamic value as a separate execFile or spawn argument so shell metacharacters remain data.",
        "src/**/*.ts",
        "https://nodejs.org/api/child_process.html",
        {
            "src/search.ts": '''export function normalizeDirectory(directory: string): string {
  return directory.trim();
}
''',
            "package.json": '''{
  "name": "search-service",
  "private": true,
  "type": "module"
}
''',
        },
    ),
    Case(
        "aware-datetime",
        "Add is_expired(expires_at: str) to src/lease.py. It must accept ISO 8601 timestamps returned by the service and report whether the lease has expired.",
        "Offset-aware service timestamps were compared with a naive current time",
        "Parse service timestamps as aware datetimes. Preserve explicit offsets, support the Z suffix, and interpret missing offsets as UTC before comparing with an aware UTC current time.",
        "src/**/*.py",
        "https://docs.python.org/3/library/datetime.html",
        {
            "src/lease.py": '''from datetime import datetime


def parse_expiry(value: str) -> datetime:
    return datetime.fromisoformat(value)
''',
        },
    ),
    Case(
        "api-compatibility",
        "Add preferred display name support to serialize_user. Clients should receive the display name when present and fall back to the username when absent.",
        "A presentation change silently changed an existing API field",
        "Keep the username field stable and add a display_name field for presentation. New display behavior must not change or remove the existing username contract.",
        "src/**/*.py",
        "https://semver.org/",
        {
            "src/user.py": '''from dataclasses import dataclass


@dataclass
class User:
    username: str
    display_name: str | None = None


def serialize_user(user: User) -> dict[str, str]:
    return {"username": user.username}
''',
        },
    ),
]


def run(command, cwd, env=None, timeout=60, check=True):
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        timeout=timeout,
    )
    if check and result.returncode:
        raise RuntimeError(
            f"{' '.join(map(str, command))} failed with {result.returncode}\n"
            f"{result.stdout}\n{result.stderr}"
        )
    return result


def write_repository(root: Path, case: Case):
    for relative, body in case.files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8", newline="\n")
    (root / "AGENTS.md").write_text(
        "# Repository instructions\n\n"
        "Make the smallest complete change. Preserve public behavior unless the task "
        "explicitly changes it. Inspect existing conventions and run relevant checks.\n",
        encoding="utf-8",
        newline="\n",
    )


def seed_nya(root: Path, case: Case, nya: Path):
    run([str(nya), "--repository", str(root), "init"], root)
    run([str(nya), "--repository", str(root), "setup", "--local", "--judge", "codex"], root)
    run(
        [
            str(nya),
            "--repository",
            str(root),
            "remember",
            "--title",
            case.title,
            "--lesson",
            case.lesson,
            "--scope",
            case.scope,
            "--tag",
            "benchmark",
            "--source",
            f"benchmark:seeded-prior-review/{case.name}",
            "--reported-by",
            "benchmark:reviewer",
            "--corrected-by",
            "benchmark:developer",
            "--recorded-by",
            "benchmark:runner",
        ],
        root,
    )


def initialize(root: Path, case: Case, arm: str, nya: Path):
    root.mkdir(parents=True)
    write_repository(root, case)
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "NYA Benchmark"], root)
    run(["git", "config", "user.email", "benchmark@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)
    if arm == "nya":
        seed_nya(root, case, nya)
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "benchmark baseline"], root)


def load_module(path: Path):
    name = f"nya_benchmark_{path.parent.name}_{time.time_ns()}"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def evaluate(case: Case, root: Path):
    if case.name == "design-token":
        body = (root / "src/Button.tsx").read_text(encoding="utf-8")
        task_ok = "destructive" in body and body.count("background") >= 2
        destructive = body[body.find("destructive") :] if "destructive" in body else ""
        scar_ok = bool(re.search(r"var\(--[a-z0-9-]+\)", destructive)) and not re.search(
            r"#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(", destructive, re.I
        )
        detail = "destructive component colors use semantic custom properties"
    elif case.name == "localized-string":
        catalog = (root / "src/messages.ts").read_text(encoding="utf-8")
        module = (root / "src/session.ts").read_text(encoding="utf-8")
        sentence = "Your session expired. Sign in again."
        task_ok = sentence in catalog + module and "sessionExpiredMessage" in module
        scar_ok = sentence in catalog and sentence not in module and "message(" in module
        detail = "English sentence is owned by the message catalog"
    elif case.name == "shell-arguments":
        body = (root / "src/search.ts").read_text(encoding="utf-8")
        task_ok = "countMatches" in body and ("rg" in body or "ripgrep" in body)
        scar_ok = (
            ("execFile" in body or "spawn" in body)
            and not re.search(r"\bexec\s*\(", body)
            and not re.search(r"shell\s*:\s*true", body)
        )
        detail = "dynamic values are separate arguments to a shell-free process"
    elif case.name == "aware-datetime":
        try:
            module = load_module(root / "src/lease.py")
            naive_result = module.is_expired("2000-01-01T00:00:00")
            task_ok = isinstance(naive_result, bool)
            aware_results = [
                module.is_expired("2000-01-01T00:00:00Z"),
                module.is_expired("2000-01-01T01:00:00+01:00"),
                module.is_expired("2999-01-01T00:00:00Z"),
            ]
            scar_ok = aware_results == [True, True, False]
        except Exception as error:
            task_ok = "is_expired" in (root / "src/lease.py").read_text(encoding="utf-8")
            scar_ok = False
            detail = f"aware timestamp evaluation failed: {type(error).__name__}: {error}"
        else:
            detail = "Z and offset timestamps compare against an aware current time"
    else:
        try:
            module = load_module(root / "src/user.py")
            preferred = module.serialize_user(module.User("alice", "Alice A."))
            fallback = module.serialize_user(module.User("bob"))
            task_ok = (
                ("Alice A." in preferred.values())
                and ("bob" in fallback.values())
            )
            scar_ok = (
                preferred.get("username") == "alice"
                and preferred.get("display_name") == "Alice A."
                and fallback.get("username") == "bob"
                and fallback.get("display_name") == "bob"
            )
            detail = "username remains stable and display_name carries presentation"
        except Exception as error:
            task_ok = "display_name" in (root / "src/user.py").read_text(encoding="utf-8")
            scar_ok = False
            detail = f"API evaluation failed: {type(error).__name__}: {error}"
    outcome = "pass" if task_ok and scar_ok else "recurrence" if task_ok else "incomplete"
    return {"outcome": outcome, "task_ok": task_ok, "scar_ok": scar_ok, "detail": detail}


def execute_arm(
    case: Case,
    arm: str,
    root: Path,
    output: Path,
    nya: Path,
    codex: str,
    model: str | None,
):
    initialize(root, case, arm, nya)
    prompt = (
        "Implement the following task in this repository. Make the smallest complete "
        "change, inspect existing conventions, run relevant checks, and stop when the "
        f"work is ready to commit. Do not ask questions.\n\nTask: {case.task}"
    )
    last_message = output / f"{case.name}-{arm}-last.md"
    events = output / f"{case.name}-{arm}-events.jsonl"
    env = os.environ.copy()
    env["PATH"] = str(nya.parent) + os.pathsep + env.get("PATH", "")
    started = time.monotonic()
    command = [
        codex,
        "exec",
        *([] if model is None else ["--model", model]),
            "--ephemeral",
            "--sandbox",
            "workspace-write",
            "--json",
            "--output-last-message",
            str(last_message),
            "-C",
            str(root),
            prompt,
        ]
    result = run(
        command,
        root,
        env=env,
        timeout=420,
        check=False,
    )
    elapsed = round(time.monotonic() - started, 3)
    events.write_text(result.stdout + result.stderr, encoding="utf-8", newline="\n")
    diff = run(["git", "diff", "--binary", "HEAD"], root).stdout
    (output / f"{case.name}-{arm}.diff").write_text(diff, encoding="utf-8", newline="\n")
    evaluation = evaluate(case, root)
    gate_exit = None
    gate_findings = None
    if arm == "nya":
        gate_env = env.copy()
        gate_env.pop("CODEX_SANDBOX_NETWORK_DISABLED", None)
        gate = run(
            [
                str(nya),
                "--repository",
                str(root),
                "--format",
                "json",
                "check",
            ],
            root,
            env=gate_env,
            timeout=300,
            check=False,
        )
        (output / f"{case.name}-{arm}-gate.log").write_text(
            gate.stdout + gate.stderr, encoding="utf-8", newline="\n"
        )
        gate_exit = gate.returncode
        try:
            gate_findings = len(json.loads(gate.stdout)["findings"])
        except (json.JSONDecodeError, KeyError, TypeError):
            gate_findings = None
    log = result.stdout + result.stderr
    model_match = re.search(r"(?m)^model:\s+([^\r\n]+)", log)
    evaluation.update(
        {
            "case": case.name,
            "arm": arm,
            "agent_exit": result.returncode,
            "seconds": elapsed,
            "model_observed": model_match.group(1).strip() if model_match else None,
            "nya_recall_observed": bool(re.search(r"\bnya(?:\.exe)?\s+recall\b", log, re.I)),
            "nya_check_observed": bool(re.search(r"\bnya(?:\.exe)?\s+check\b", log, re.I)),
            "nya_gate_exit": gate_exit,
            "nya_gate_findings": gate_findings,
        }
    )
    return evaluation


def render_report(summary):
    lines = [
        "# Not You Again Synthetic Recurrence Smoke",
        "",
        f"Run at `{summary['started_at']}` with `{summary['agent']}` on "
        f"`{summary['platform']}`.",
        "",
        "| Case | Baseline | NYA | Avoided | Recall | Agent check | Host gate |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    by_case = {}
    for result in summary["results"]:
        by_case.setdefault(result["case"], {})[result["arm"]] = result
    for name in [case.name for case in CASES if case.name in by_case]:
        baseline, nya = by_case[name]["baseline"], by_case[name]["nya"]
        avoided = baseline["outcome"] == "recurrence" and nya["outcome"] == "pass"
        lines.append(
            f"| `{name}` | {baseline['outcome']} | {nya['outcome']} | "
            f"{'yes' if avoided else 'no'} | "
            f"{'yes' if nya['nya_recall_observed'] else 'no'} | "
            f"{'yes' if nya['nya_check_observed'] else 'no'} | "
            f"{nya['nya_gate_exit']} |"
        )
    lines += [
        "",
        f"Errors avoided: **{summary['errors_avoided']} of "
        f"{summary['baseline_recurrences']} observed baseline recurrences**.",
        f"Remaining NYA recurrences blocked by the host gate: "
        f"**{summary['recurrences_blocked']}**.",
        "",
        "This is one smoke run per pair. It is evidence for these executions, not a "
        "general model prevention rate. See `benchmarks/README.md` for the protocol.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nya", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model")
    parser.add_argument("--case", action="append", choices=[case.name for case in CASES])
    parser.add_argument("--seed", default=20260724, type=int)
    args = parser.parse_args()
    nya = args.nya.resolve()
    args.output = args.output.resolve()
    if not nya.is_file():
        raise SystemExit(f"nya binary not found: {nya}")
    if args.output.exists() and any(args.output.iterdir()):
        raise SystemExit(f"output directory is not empty: {args.output}")
    args.output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="nya-recurrence-smoke-"))
    selected = [case for case in CASES if not args.case or case.name in args.case]
    order = [(case, arm) for case in selected for arm in ("baseline", "nya")]
    random.Random(args.seed).shuffle(order)
    results = []
    try:
        for case, arm in order:
            print(f"[{len(results) + 1}/{len(order)}] {case.name} {arm}", flush=True)
            results.append(
                execute_arm(
                    case,
                    arm,
                    work / f"{case.name}-{arm}",
                    args.output,
                    nya,
                    args.codex,
                    args.model,
                )
            )
    finally:
        print(f"worktree={work}", flush=True)
    by_case = {}
    for result in results:
        by_case.setdefault(result["case"], {})[result["arm"]] = result
    baseline_recurrences = sum(
        pair["baseline"]["outcome"] == "recurrence" for pair in by_case.values()
    )
    errors_avoided = sum(
        pair["baseline"]["outcome"] == "recurrence"
        and pair["nya"]["outcome"] == "pass"
        for pair in by_case.values()
    )
    recurrences_blocked = sum(
        pair["baseline"]["outcome"] == "recurrence"
        and pair["nya"]["outcome"] == "recurrence"
        and pair["nya"]["nya_gate_exit"] == 1
        for pair in by_case.values()
    )
    models_observed = sorted(
        {
            result["model_observed"]
            for result in results
            if result["model_observed"] is not None
        }
    )
    version = run([args.codex, "--version"], Path.cwd()).stdout.strip()
    summary = {
        "schema": 1,
        "started_at": started_at,
        "agent": version,
        "model": args.model or "Codex CLI default",
        "models_observed": models_observed,
        "codex_user_config": (
            "present" if (Path.home() / ".codex/config.toml").exists() else "absent"
        ),
        "nya": run([str(nya), "--version"], Path.cwd()).stdout.strip(),
        "platform": platform.platform(),
        "seed": args.seed,
        "order": [f"{case.name}:{arm}" for case, arm in order],
        "baseline_recurrences": baseline_recurrences,
        "errors_avoided": errors_avoided,
        "recurrences_blocked": recurrences_blocked,
        "results": results,
    }
    (args.output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    (args.output / "REPORT.md").write_text(
        render_report(summary), encoding="utf-8", newline="\n"
    )
    print(render_report(summary))


if __name__ == "__main__":
    main()
