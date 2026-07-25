#!/usr/bin/env python3
import argparse
import hashlib
import json
import math
import os
import platform
import random
import shutil
import statistics
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

from smoke import run
from stress import TARGETS


TARGET_PATH = "src/scale/LargeViewModel.tsx"
TARGET_EVIDENCE = 'export const lateBackground = "#b91c1c";'
SEMANTIC_KEYS = {
    "react-expensive-derived-usememo",
    "design-literal-color",
    "backend-sql-interpolation",
    "science-unit-mismatch",
}


def quote(value):
    return json.dumps(value, ensure_ascii=False)


def scar_text(identifier, title, lesson, scope, tags, source):
    return "\n".join(
        [
            "schema = 1",
            f"id = {quote(identifier)}",
            f"title = {quote(title)}",
            f"lesson = {quote(lesson)}",
            f"scope = [{quote(scope)}]",
            f"tags = [{', '.join(quote(tag) for tag in tags)}]",
            'created_at = "2026-01-01T00:00:00Z"',
            "",
            "[[occurrences]]",
            'occurred_at = "2026-01-01T00:00:00Z"',
            f"source = {quote(source)}",
            'reported_by = "benchmark:reviewer"',
            'corrected_by = "benchmark:developer"',
            'recorded_by = "benchmark:runner"',
            "",
        ]
    )


def init_repo(root, nya):
    root.mkdir(parents=True)
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "NYA Scale Benchmark"], root)
    run(["git", "config", "user.email", "scale@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)
    (root / "AGENTS.md").write_text("# NYA scale fixture\n", encoding="utf-8")
    run([str(nya), "--repository", str(root), "init"], root)


def seed_scale(root, nya, corpus_size, applicable):
    init_repo(root, nya)
    target = root / TARGET_PATH
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("", encoding="utf-8")
    scars = root / ".nya/scars"
    for index in range(corpus_size):
        identifier = f"NYA-SCALE-{index:05}"
        is_target = index == applicable - 1
        scope = "src/scale/**" if index < applicable else f"domains/{index:05}/**"
        title = (
            "Literal color bypasses the design token in a late large diff"
            if is_target
            else f"Scale invariant {index:05}"
        )
        lesson = (
            "Use a semantic design token instead of a literal component color."
            if is_target
            else f"Preserve the independently indexed scale invariant {index:05}."
        )
        body = scar_text(
            identifier,
            title,
            lesson,
            scope,
            ["scale", f"key-{index:05}"],
            f"benchmark:scale/{index:05}",
        )
        (scars / f"{identifier}.toml").write_text(
            body, encoding="utf-8", newline="\n"
        )
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", f"seed {corpus_size} scale scars"], root)


def recall(nya, root, task, path, limit=12):
    started = time.monotonic()
    result = run(
        [
            str(nya),
            "--repository",
            str(root),
            "--format",
            "json",
            "recall",
            "--task",
            task,
            "--path",
            path,
            "--limit",
            str(limit),
        ],
        root,
        timeout=600,
    )
    return json.loads(result.stdout), round(time.monotonic() - started, 4)


def large_source(recurrence, filler_lines):
    filler = "".join(
        f"export const filler{index:05} = {index};\n" for index in range(filler_lines)
    )
    ending = (
        TARGET_EVIDENCE
        if recurrence
        else 'export const lateBackground = "var(--color-danger)";'
    )
    return f"{filler}{ending}\n"


def configure(root, nya, judge, command):
    run(
        [
            str(nya),
            "--repository",
            str(root),
            "setup",
            "--local",
            "--judge",
            judge,
            "--",
            *map(str, command),
        ],
        root,
    )


def check(nya, root, output, label, task):
    env = os.environ.copy()
    env.pop("CODEX_SANDBOX_NETWORK_DISABLED", None)
    started = time.monotonic()
    result = run(
        [
            str(nya),
            "--repository",
            str(root),
            "--format",
            "json",
            "check",
            "--task",
            task,
        ],
        root,
        env=env,
        timeout=7200,
        check=False,
    )
    elapsed = round(time.monotonic() - started, 3)
    (output / f"{label}-check.json").write_text(
        result.stdout, encoding="utf-8", newline="\n"
    )
    (output / f"{label}-check.stderr.log").write_text(
        result.stderr, encoding="utf-8", newline="\n"
    )
    diff = run(["git", "diff", "--binary", "HEAD"], root).stdout
    (output / f"{label}.diff").write_text(diff, encoding="utf-8", newline="\n")
    try:
        verdict = json.loads(result.stdout)
    except json.JSONDecodeError:
        verdict = {"passed": False, "scars_checked": None, "findings": []}
    return result.returncode, elapsed, verdict, len(diff.encode())


def metrics(path):
    values = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    prompts = [value["prompt_bytes"] for value in values]
    elapsed = [value["seconds"] for value in values]
    reported = [
        value["reported_tokens"]
        for value in values
        if value.get("reported_tokens") is not None
    ]
    return {
        "calls": len(values),
        "audit_calls": sum(value["stage"] == "audit" for value in values),
        "confirmation_calls": sum(
            value["stage"] == "confirmation" for value in values
        ),
        "prompt_bytes": sum(prompts),
        "max_prompt_bytes": max(prompts, default=0),
        "judge_seconds": round(sum(elapsed), 3),
        "reported_tokens": sum(reported) if reported else None,
    }


def run_scale(args, root, output):
    seed_scale(root, args.nya, args.corpus_size, args.applicable)
    target_id = f"NYA-SCALE-{args.applicable - 1:05}"
    selected = {args.applicable - 1}
    remaining = [i for i in range(args.corpus_size) if i not in selected]
    random.Random(args.seed).shuffle(remaining)
    selected.update(remaining[: max(0, args.probes - 1)])
    probe_results = []
    for offset, index in enumerate(sorted(selected), start=1):
        path = (
            "src/scale/probe.tsx"
            if index < args.applicable
            else f"domains/{index:05}/probe.txt"
        )
        recalled, seconds = recall(
            args.nya,
            root,
            f"Find scale key {index:05} and invariant {index:05}",
            path,
        )
        ids = [scar["id"] for scar in recalled]
        identifier = f"NYA-SCALE-{index:05}"
        probe_results.append(
            {
                "scar_id": identifier,
                "rank": ids.index(identifier) + 1 if identifier in ids else None,
                "candidates": len(ids),
                "seconds": seconds,
            }
        )
        print(f"[scale recall {offset}/{len(selected)}] {identifier}", flush=True)
    negative, negative_seconds = recall(
        args.nya, root, "qzxvscale plughscale", "unrelated/none.zzz"
    )

    log = output / "scale-judge.jsonl"
    configure(
        root,
        args.nya,
        "scale",
        [
            sys.executable,
            Path(__file__).with_name("scale_judge.py").resolve(),
            "--log",
            log,
            "--scar",
            target_id,
            "--path",
            TARGET_PATH,
            "--evidence",
            TARGET_EVIDENCE,
            "--line",
            args.filler_lines + 1,
        ],
    )
    (root / TARGET_PATH).write_text(
        large_source(True, args.filler_lines), encoding="utf-8", newline="\n"
    )
    positive_exit, positive_seconds, positive, positive_diff_bytes = check(
        args.nya, root, output, "scale-positive", "Audit the late large diff"
    )
    run(["git", "checkout", "-q", "--", TARGET_PATH], root)
    (root / TARGET_PATH).write_text(
        large_source(False, args.filler_lines), encoding="utf-8", newline="\n"
    )
    negative_exit, check_negative_seconds, negative_check, negative_diff_bytes = check(
        args.nya, root, output, "scale-negative", "Audit the corrected large diff"
    )
    judge_metrics = metrics(log)
    findings = positive.get("findings", [])
    exact = any(
        finding.get("scar_id") == target_id
        and finding.get("path") == TARGET_PATH
        and finding.get("evidence") == TARGET_EVIDENCE
        for finding in findings
    )
    timings = [result["seconds"] for result in probe_results]
    result = {
        "corpus_size": args.corpus_size,
        "applicable_scars": args.applicable,
        "probes": len(probe_results),
        "recalled": sum(result["rank"] is not None for result in probe_results),
        "ranked_first": sum(result["rank"] == 1 for result in probe_results),
        "bounded": sum(result["candidates"] <= 12 for result in probe_results),
        "negative_candidates": len(negative),
        "negative_recall_seconds": negative_seconds,
        "recall_latency_seconds": {
            "p50": statistics.median(timings),
            "p95": sorted(timings)[max(0, math.ceil(len(timings) * 0.95) - 1)],
            "max": max(timings),
        },
        "positive": {
            "exit": positive_exit,
            "seconds": positive_seconds,
            "diff_bytes": positive_diff_bytes,
            "scars_checked": positive.get("scars_checked"),
            "findings": len(findings),
            "exact_target": exact,
        },
        "negative": {
            "exit": negative_exit,
            "seconds": check_negative_seconds,
            "diff_bytes": negative_diff_bytes,
            "scars_checked": negative_check.get("scars_checked"),
            "findings": len(negative_check.get("findings", [])),
        },
        "judge_cost": judge_metrics,
    }
    result["passed"] = (
        result["recalled"] == result["probes"]
        and result["ranked_first"] == result["probes"]
        and result["bounded"] == result["probes"]
        and result["negative_candidates"] == 0
        and result["positive"]["exit"] == 1
        and result["positive"]["scars_checked"] == args.applicable
        and result["positive"]["exact_target"]
        and result["negative"]["exit"] == 0
        and result["negative"]["scars_checked"] == args.applicable
        and result["negative"]["findings"] == 0
        and result["positive"]["diff_bytes"] > 100_000
        and result["judge_cost"]["max_prompt_bytes"] < 110_000
    )
    return result


def catalog_by_key(path):
    catalog = json.loads(path.read_text(encoding="utf-8"))
    return {item["key"]: item for item in catalog}


def seed_semantic(root, nya, catalog, targets):
    init_repo(root, nya)
    for index, target in enumerate(targets):
        path = root / target.path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("", encoding="utf-8")
        item = catalog[target.key]
        identifier = f"NYA-VARIANCE-{index:02}"
        body = scar_text(
            identifier,
            item["title"],
            item["lesson"],
            target.path,
            item["tags"] + [item["domain"], "variance"],
            f"benchmark:variance/{target.key}",
        )
        (root / ".nya/scars" / f"{identifier}.toml").write_text(
            body, encoding="utf-8", newline="\n"
        )
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "seed semantic variance scars"], root)


def run_semantic_once(args, model, repetition, work, output, catalog, targets):
    label = f"{model}-run-{repetition}".replace("/", "-")
    run_output = output / "semantic" / label
    run_output.mkdir(parents=True)
    proxy_log = run_output / "judge-metrics.jsonl"
    command = [
        sys.executable,
        Path(__file__).with_name("judge_proxy.py").resolve(),
        "--log",
        proxy_log,
        "--label",
        label,
        "--",
        args.codex,
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
    ]
    positive_root = work / f"semantic-{label}-positive"
    seed_semantic(positive_root, args.nya, catalog, targets)
    configure(positive_root, args.nya, "codex", command)
    for target in targets:
        (positive_root / target.path).write_text(
            target.recurrence, encoding="utf-8", newline="\n"
        )
    positive_exit, positive_seconds, positive, _ = check(
        args.nya, positive_root, run_output, "positive", "Audit semantic recurrences"
    )
    expected = {
        (f"NYA-VARIANCE-{index:02}", target.path)
        for index, target in enumerate(targets)
    }
    actual = {
        (finding.get("scar_id"), finding.get("path"))
        for finding in positive.get("findings", [])
    }

    negative_root = work / f"semantic-{label}-negative"
    seed_semantic(negative_root, args.nya, catalog, targets)
    configure(negative_root, args.nya, "codex", command)
    for target in targets:
        (negative_root / target.path).write_text(
            target.correction, encoding="utf-8", newline="\n"
        )
    negative_exit, negative_seconds, negative, _ = check(
        args.nya, negative_root, run_output, "negative", "Audit corrected controls"
    )
    result = {
        "model": model,
        "repetition": repetition,
        "targets": len(expected),
        "detected": len(expected & actual),
        "unexpected": len(actual - expected),
        "missed": sorted(f"{scar}:{path}" for scar, path in expected - actual),
        "positive_exit": positive_exit,
        "positive_seconds": positive_seconds,
        "negative_findings": len(negative.get("findings", [])),
        "negative_exit": negative_exit,
        "negative_seconds": negative_seconds,
        "cost": metrics(proxy_log),
    }
    result["passed"] = (
        result["detected"] == result["targets"]
        and result["unexpected"] == 0
        and result["positive_exit"] == 1
        and result["negative_findings"] == 0
        and result["negative_exit"] == 0
    )
    return result


def render(summary):
    scale = summary["scale"]
    lines = [
        "# Not You Again Production Scale Benchmark",
        "",
        f"Run from `{summary['started_at']}` to `{summary['completed_at']}` on "
        f"`{summary['platform']}`.",
        "",
        "## Deterministic scale and completeness",
        "",
        "| Metric | Result |",
        "| --- | ---: |",
        f"| Versioned scars | {scale['corpus_size']} |",
        f"| Scars applicable to one file | {scale['applicable_scars']} |",
        f"| Recall targets found | {scale['recalled']} / {scale['probes']} |",
        f"| Recall targets ranked first | {scale['ranked_first']} / {scale['probes']} |",
        f"| Unrelated recall candidates | {scale['negative_candidates']} |",
        f"| Recall p50 | {scale['recall_latency_seconds']['p50']} s |",
        f"| Recall p95 | {scale['recall_latency_seconds']['p95']} s |",
        f"| Recall maximum, including first cold read | {scale['recall_latency_seconds']['max']} s |",
        f"| Large positive diff | {scale['positive']['diff_bytes']} bytes |",
        f"| Late target detected | {'yes' if scale['positive']['exact_target'] else 'no'} |",
        f"| Applicable scars audited | {scale['positive']['scars_checked']} |",
        f"| Corrected-control findings | {scale['negative']['findings']} |",
        f"| Judge calls | {scale['judge_cost']['calls']} |",
        f"| Total prompt bytes | {scale['judge_cost']['prompt_bytes']} |",
        f"| Maximum prompt bytes | {scale['judge_cost']['max_prompt_bytes']} |",
        "",
        "The deterministic judge places the only recurrence in the final applicable "
        "scar and after the first 100,000 bytes of the changed file. It measures NYA "
        "orchestration completeness, not model intelligence.",
    ]
    if summary["semantic_runs"]:
        lines += [
            "",
            "## Cross-model semantic variance",
            "",
            "| Model | Run | Detected | Unexpected | Negative findings | Calls | Prompt bytes | Reported tokens | Result |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
        ]
        for value in summary["semantic_runs"]:
            lines.append(
                f"| `{value['model']}` | {value['repetition']} | "
                f"{value['detected']} / {value['targets']} | {value['unexpected']} | "
                f"{value['negative_findings']} | {value['cost']['calls']} | "
                f"{value['cost']['prompt_bytes']} | "
                f"{value['cost']['reported_tokens'] or 'not reported'} | "
                f"{'PASS' if value['passed'] else 'FAIL'} |"
            )
        lines += ["", "### Variance summary", ""]
        for model, value in summary["variance"].items():
            lines.append(
                f"- `{model}`: {value['passed_runs']} / {value['runs']} passing runs, "
                f"detection standard deviation {value['detection_stdev']}, "
                f"latency standard deviation {value['latency_stdev_seconds']} seconds."
            )
    lines += [
        "",
        f"Overall benchmark result: **{'PASS' if summary['passed'] else 'FAIL'}**.",
        "",
        "Prompt bytes and judge calls measure context cost. Reported model tokens are "
        "included only when the underlying CLI exposes them. No monetary estimate is "
        "invented.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nya", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--catalog", type=Path, default=Path(__file__).with_name("stress_catalog.json"))
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model", action="append", default=[])
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--corpus-size", type=int, default=10_000)
    parser.add_argument("--applicable", type=int, default=1_000)
    parser.add_argument("--probes", type=int, default=64)
    parser.add_argument("--filler-lines", type=int, default=4_000)
    parser.add_argument("--seed", type=int, default=20260725)
    args = parser.parse_args()
    args.nya = args.nya.resolve()
    args.output = args.output.resolve()
    if not args.nya.is_file():
        raise SystemExit("NYA binary not found")
    if not 1 <= args.applicable <= args.corpus_size:
        raise SystemExit("applicable must be between 1 and corpus-size")
    if args.output.exists() and any(args.output.iterdir()):
        raise SystemExit(f"output directory is not empty: {args.output}")
    args.output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="nya-production-scale-"))
    scale = run_scale(args, work / "scale", args.output)

    catalog = catalog_by_key(args.catalog)
    targets = [target for target in TARGETS if target.key in SEMANTIC_KEYS]
    semantic_runs = []
    for model in args.model:
        for repetition in range(1, args.repetitions + 1):
            print(f"[semantic] {model} run {repetition}", flush=True)
            semantic_runs.append(
                run_semantic_once(
                    args,
                    model,
                    repetition,
                    work,
                    args.output,
                    catalog,
                    targets,
                )
            )
    variance = {}
    for model in args.model:
        values = [value for value in semantic_runs if value["model"] == model]
        detections = [value["detected"] for value in values]
        latencies = [
            value["positive_seconds"] + value["negative_seconds"] for value in values
        ]
        variance[model] = {
            "runs": len(values),
            "passed_runs": sum(value["passed"] for value in values),
            "detection_stdev": round(statistics.pstdev(detections), 4),
            "latency_stdev_seconds": round(statistics.pstdev(latencies), 3),
        }
    summary = {
        "schema": 1,
        "benchmark": "production-scale",
        "started_at": started_at,
        "completed_at": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "candidate": {
            "version": run([str(args.nya), "--version"], Path.cwd()).stdout.strip(),
            "sha256": hashlib.sha256(args.nya.read_bytes()).hexdigest(),
        },
        "scale": scale,
        "semantic_runs": semantic_runs,
        "variance": variance,
        "passed": scale["passed"] and all(value["passed"] for value in semantic_runs),
        "worktree": str(work),
    }
    (args.output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    report = render(summary)
    (args.output / "REPORT.md").write_text(report, encoding="utf-8", newline="\n")
    print(report)
    print(f"worktree={work}", flush=True)
    if not summary["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
