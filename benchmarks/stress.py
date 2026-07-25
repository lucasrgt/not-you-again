#!/usr/bin/env python3
import argparse
import hashlib
import json
import math
import os
import platform
import random
import shutil
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from smoke import run


CONTEXTS = [
    "checkout",
    "admin",
    "account",
    "search",
    "billing",
    "tenancy",
    "gateway",
    "scheduler",
    "platform",
    "analytics",
    "science",
    "mobile",
    "desktop",
    "docs",
    "experiments",
    "accessibility",
]


@dataclass(frozen=True)
class Target:
    context: str
    key: str
    path: str
    recurrence: str
    correction: str


TARGETS = [
    Target(
        "checkout",
        "react-expensive-derived-usememo",
        "apps/checkout/frontend-performance/InvoiceTotals.tsx",
        """export function InvoiceTotals({ invoices }) {
  const visibleInvoices = invoices
    .filter((invoice) => invoice.status !== "void")
    .sort((left, right) => right.total - left.total);
  return visibleInvoices.map((invoice) => invoice.total).join(", ");
}
""",
        """import { useMemo } from "react";

export function InvoiceTotals({ invoices }) {
  const visibleInvoices = useMemo(
    () =>
      invoices
        .filter((invoice) => invoice.status !== "void")
        .sort((left, right) => right.total - left.total),
    [invoices],
  );
  return visibleInvoices.map((invoice) => invoice.total).join(", ");
}
""",
    ),
    Target(
        "admin",
        "design-literal-color",
        "apps/admin/design-accessibility/DeleteButton.tsx",
        """export const deleteButtonStyle = {
  background: "#b91c1c",
  color: "#ffffff",
};
""",
        """export const deleteButtonStyle = {
  background: "var(--color-danger)",
  color: "var(--color-on-danger)",
};
""",
    ),
    Target(
        "account",
        "ui-localized-string",
        "apps/account/design-accessibility/SessionBanner.tsx",
        """export function SessionBanner() {
  return <p>Your session expired. Sign in again.</p>;
}
""",
        """import { message } from "./messages";

export function SessionBanner() {
  return <p>{message("session.expired")}</p>;
}
""",
    ),
    Target(
        "search",
        "backend-shell-interpolation",
        "apps/search/backend-api/SearchService.ts",
        """import { exec } from "node:child_process";

export function countMatches(pattern: string, directory: string) {
  return exec(`rg "${pattern}" "${directory}"`);
}
""",
        """import { execFile } from "node:child_process";

export function countMatches(pattern: string, directory: string) {
  return execFile("rg", [pattern, directory]);
}
""",
    ),
    Target(
        "billing",
        "backend-sql-interpolation",
        "apps/billing/backend-api/invoice_query.py",
        """def load_invoice(connection, organization_id: str, invoice_id: str):
    return connection.execute(
        f"SELECT id, total FROM invoices WHERE organization_id = ? AND id = '{invoice_id}'",
        (organization_id,),
    ).fetchone()
""",
        """def load_invoice(connection, organization_id: str, invoice_id: str):
    return connection.execute(
        "SELECT id, total FROM invoices WHERE organization_id = ? AND id = ?",
        (organization_id, invoice_id),
    ).fetchone()
""",
    ),
    Target(
        "tenancy",
        "security-tenant-authorization",
        "apps/tenancy/security-privacy/UserCache.ts",
        """export function getUser(repository: UserRepository, userId: string) {
  return repository.findOne({ id: userId });
}
""",
        """export function getUser(
  repository: UserRepository,
  organizationId: string,
  userId: string,
) {
  return repository.findOne({ organizationId, id: userId });
}
""",
    ),
    Target(
        "gateway",
        "security-ssrf",
        "apps/gateway/security-privacy/proxy.py",
        """import requests


def preview(target_url: str):
    return requests.get(target_url, timeout=10).text
""",
        """import requests

PREVIEW_DESTINATIONS = {
    "documentation": "https://docs.example.com/preview",
}


def preview(destination: str):
    target_url = PREVIEW_DESTINATIONS[destination]
    return requests.get(target_url, timeout=10).text
""",
    ),
    Target(
        "scheduler",
        "reliability-retry-no-backoff",
        "apps/scheduler/infrastructure-reliability/retry.py",
        """def deliver(send):
    for _ in range(8):
        try:
            return send()
        except TemporaryError:
            continue
    raise TemporaryError("delivery failed")
""",
        """import random
import time


def deliver(send):
    for attempt in range(4):
        try:
            return send()
        except TemporaryError:
            if attempt == 3:
                raise
            time.sleep((2**attempt) * 0.1 + random.uniform(0, 0.05))
""",
    ),
    Target(
        "platform",
        "infra-missing-resource-bounds",
        "apps/platform/infrastructure-reliability/deployment.yaml",
        """apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
  namespace: production
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: api
          image: example/api@sha256:abc123
""",
        """apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
  namespace: production
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: api
          image: example/api@sha256:abc123
          resources:
            requests:
              cpu: 250m
              memory: 256Mi
            limits:
              cpu: "1"
              memory: 512Mi
""",
    ),
    Target(
        "analytics",
        "ml-training-test-leakage",
        "apps/analytics/data-ml-science/train.py",
        """from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler


def prepare(features, labels):
    scaled = StandardScaler().fit_transform(features)
    return train_test_split(scaled, labels, test_size=0.2, random_state=42)
""",
        """from sklearn.model_selection import train_test_split
from sklearn.preprocessing import StandardScaler


def prepare(features, labels):
    x_train, x_test, y_train, y_test = train_test_split(
        features, labels, test_size=0.2, random_state=42
    )
    scaler = StandardScaler().fit(x_train)
    return scaler.transform(x_train), scaler.transform(x_test), y_train, y_test
""",
    ),
    Target(
        "science",
        "science-unit-mismatch",
        "apps/science/data-ml-science/velocity.py",
        """def velocity_meters_per_second(distance_m: float, duration_ms: float) -> float:
    return distance_m / duration_ms
""",
        """def velocity_meters_per_second(distance_m: float, duration_ms: float) -> float:
    duration_seconds = duration_ms / 1000.0
    return distance_m / duration_seconds
""",
    ),
    Target(
        "mobile",
        "mobile-main-thread-io",
        "apps/mobile/concurrency-clients/ProfileView.kt",
        """fun renderProfile(): String {
    val connection = URL("https://api.example.com/profile").openConnection()
    connection.connectTimeout = 5000
    connection.readTimeout = 5000
    return connection.getInputStream().bufferedReader().use { it.readText() }
}
""",
        """suspend fun loadProfile(io: CoroutineDispatcher): String =
    withContext(io) {
        val connection = URL("https://api.example.com/profile").openConnection()
        connection.connectTimeout = 5000
        connection.readTimeout = 5000
        connection.getInputStream().bufferedReader().use { it.readText() }
    }
""",
    ),
    Target(
        "desktop",
        "filesystem-nonatomic-write",
        "apps/desktop/concurrency-clients/settings.py",
        """import json
from pathlib import Path

SETTINGS_ROOT = (Path.home() / ".stress-settings").resolve()


def settings_path(name: str) -> Path:
    path = (SETTINGS_ROOT / name).resolve()
    if SETTINGS_ROOT not in path.parents:
        raise ValueError("invalid settings path")
    return path


def save_settings(name, settings):
    path = settings_path(name)
    path.write_text(json.dumps(settings), encoding="utf-8")
""",
        """import json
import os
import tempfile
from pathlib import Path

SETTINGS_ROOT = (Path.home() / ".stress-settings").resolve()


def settings_path(name: str) -> Path:
    path = (SETTINGS_ROOT / name).resolve()
    if SETTINGS_ROOT not in path.parents:
        raise ValueError("invalid settings path")
    return path


def save_settings(name, settings):
    path = settings_path(name)
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        json.dump(settings, handle)
        handle.flush()
        os.fsync(handle.fileno())
        temporary = handle.name
    os.replace(temporary, path)
""",
    ),
    Target(
        "docs",
        "runbook-destructive-no-backup",
        "apps/docs/quality-operations/restore.md",
        """# Restore the service

Run the following command on the production host:

```bash
rm -rf /srv/app/data
tar -xf backup.tar -C /srv/app/data
```
""",
        """# Restore the service

Verify the target host, create and validate a filesystem snapshot, and preview
the archive contents. After explicit incident-commander approval, move the
current data directory to the documented recovery location and extract the
validated archive. Keep the snapshot until post-restore verification passes.
""",
    ),
    Target(
        "experiments",
        "experiment-missing-guardrail",
        "apps/experiments/quality-operations/analyze.py",
        """def choose_winner(control, treatment):
    return "treatment" if treatment.conversion > control.conversion else "control"
""",
        """def choose_winner(control, treatment):
    if not treatment.data_quality_ok or treatment.harm_rate > 0.01:
        return "stop"
    if treatment.sample_size < 10_000:
        return "continue"
    return "treatment" if treatment.lift_lower_bound > 0.0 else "control"
""",
    ),
    Target(
        "accessibility",
        "a11y-input-label",
        "apps/accessibility/design-accessibility/Signup.tsx",
        """import { message } from "./messages";

export function Signup() {
  return <input name="email" placeholder={message("signup.email")} />;
}
""",
        """import { message } from "./messages";

export function Signup() {
  return (
    <label>
      {message("signup.email")}
      <input name="email" type="email" />
    </label>
  );
}
""",
    ),
]


def scar_id(catalog, key, context):
    family = next(index for index, item in enumerate(catalog) if item["key"] == key)
    surface = CONTEXTS.index(context)
    return f"NYA-STRESS-{family:02}-{surface:02}"


def quote(value):
    return json.dumps(value, ensure_ascii=False)


def scar_text(item, context, family_index, context_index):
    identifier = f"NYA-STRESS-{family_index:02}-{context_index:02}"
    tags = item["tags"] + [item["domain"], "stress"]
    return "\n".join(
        [
            "schema = 1",
            f"id = {quote(identifier)}",
            f"title = {quote(f'{item['title']} in the {context} surface')}",
            f"lesson = {quote(item['lesson'])}",
            f"scope = [{quote(f'apps/{context}/{item['domain']}/**')}]",
            f"tags = [{', '.join(quote(tag) for tag in tags)}]",
            'created_at = "2026-01-01T00:00:00Z"',
            "",
            "[[occurrences]]",
            'occurred_at = "2026-01-01T00:00:00Z"',
            f"source = {quote(f'benchmark:stress/{item['key']}/{context}')}",
            'reported_by = "benchmark:reviewer"',
            'corrected_by = "benchmark:developer"',
            'recorded_by = "benchmark:runner"',
            "",
        ]
    )


def initialize(root, nya, catalog):
    root.mkdir(parents=True)
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "NYA Stress Benchmark"], root)
    run(["git", "config", "user.email", "stress@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)
    (root / "AGENTS.md").write_text(
        "# Stress fixture\n\nFollow repository instructions and make no unrelated changes.\n",
        encoding="utf-8",
        newline="\n",
    )
    run([str(nya), "--repository", str(root), "init"], root)
    for target in TARGETS:
        path = root / target.path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("", encoding="utf-8", newline="\n")
    for family_index, item in enumerate(catalog):
        for context_index, context in enumerate(CONTEXTS):
            path = root / ".nya/scars" / f"NYA-STRESS-{family_index:02}-{context_index:02}.toml"
            path.write_text(
                scar_text(item, context, family_index, context_index),
                encoding="utf-8",
                newline="\n",
            )
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "seed 1024 stress scars"], root)


def recall(nya, root, task, paths, limit=12):
    command = [
        str(nya),
        "--repository",
        str(root),
        "--format",
        "json",
        "recall",
        "--task",
        task,
        "--limit",
        str(limit),
    ]
    for path in paths:
        command.extend(["--path", path])
    started = time.monotonic()
    result = run(command, root, timeout=300)
    return json.loads(result.stdout), round(time.monotonic() - started, 4)


def configure_judge(root, nya, codex, model):
    command = [
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
    ]
    if model:
        command.extend(["--model", model])
    command.extend(
        [
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
    )
    run(command, root)


def write_targets(root, attribute):
    for target in TARGETS:
        path = root / target.path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(getattr(target, attribute), encoding="utf-8", newline="\n")


def execute_check(root, output, label, nya, task):
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
        timeout=1800,
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
    return result.returncode, elapsed, verdict, diff


def percentile(values, fraction):
    if not values:
        return None
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def render_report(summary):
    retrieval = summary["retrieval"]
    positive = summary.get("positive_check")
    negative = summary.get("negative_check")
    lines = [
        "# Not You Again 1,024 Scar Stress Benchmark",
        "",
        f"Run from `{summary['started_at']}` to `{summary['completed_at']}` on "
        f"`{summary['platform']}`.",
        "",
        "The corpus crosses 64 documented error families with 16 synthetic monorepo "
        "surfaces. The 1,024 records are scale fixtures, not 1,024 claimed real incidents.",
        "",
        "| Domain | Families | Generated scars |",
        "| --- | ---: | ---: |",
    ]
    for domain, count in summary["domains"].items():
        lines.append(f"| `{domain}` | {count} | {count * len(CONTEXTS)} |")
    lines += [
        "",
        "## Retrieval",
        "",
        "| Metric | Result |",
        "| --- | ---: |",
        f"| Corpus size | {summary['corpus_size']} |",
        f"| Positive probes | {retrieval['probes']} |",
        f"| Exact target recalled | {retrieval['targets_recalled']} |",
        f"| Target ranked first | {retrieval['targets_ranked_first']} |",
        f"| Results respected limit | {retrieval['bounded']} |",
        f"| Negative probes returning no scars | {retrieval['negative_empty']} / {retrieval['negative_probes']} |",
        f"| Recall latency p50 | {retrieval['latency_seconds']['p50']} s |",
        f"| Recall latency p95 | {retrieval['latency_seconds']['p95']} s |",
        f"| Recall latency p99 | {retrieval['latency_seconds']['p99']} s |",
        f"| Maximum recall candidates | {retrieval['max_candidates']} |",
    ]
    if summary.get("baseline"):
        lines += [
            "",
            f"The comparison binary `{summary['baseline']['version']}` returned "
            f"**{summary['baseline']['multi_path_candidates']} candidates despite "
            f"`--limit 12`** for the 16-path stress query. The candidate binary "
            f"returned **{summary['candidate_multi_path_candidates']}**.",
        ]
    if positive and negative:
        lines += [
            "",
            "## End-to-end judge",
            "",
            "| Check | Result |",
            "| --- | ---: |",
            f"| Injected recurrences | {positive['targets']} |",
            f"| Exact recurrences detected | {positive['detected']} |",
            f"| Unexpected findings | {positive['unexpected_findings']} |",
            f"| Unique scars audited in bounded batches | {positive['scars_checked']} |",
            f"| Positive gate exit | {positive['exit']} |",
            f"| Positive check latency | {positive['seconds']} s |",
            f"| Corrected controls | {negative['controls']} |",
            f"| Negative findings | {negative['findings']} |",
            f"| Negative gate exit | {negative['exit']} |",
            f"| Negative check latency | {negative['seconds']} s |",
            "",
            "| Injected case | Domain | Detected |",
            "| --- | --- | --- |",
        ]
        for result in positive["results"]:
            lines.append(
                f"| `{result['key']}` | `{result['domain']}` | "
                f"{'yes' if result['detected'] else 'no'} |"
            )
    lines += [
        "",
        f"Overall benchmark result: **{'PASS' if summary['passed'] else 'FAIL'}**.",
        "",
        "A pass requires bounded retrieval, every exact positive probe, empty unrelated "
        "queries, every injected recurrence identified by exact scar ID and path with "
        "verbatim diff evidence, no unexpected finding, and zero findings for corrected "
        "controls.",
        "",
        "This benchmark measures retrieval and known-recurrence detection at one corpus "
        "size. It does not estimate a universal prevention rate. The catalog, generator, "
        "diffs, structured verdicts, timings, binary hashes, and summary remain auditable.",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nya", required=True, type=Path)
    parser.add_argument("--baseline-nya", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--catalog", type=Path, default=Path(__file__).with_name("stress_catalog.json"))
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model")
    parser.add_argument("--probes", type=int, default=128)
    parser.add_argument("--seed", type=int, default=20260724)
    parser.add_argument("--skip-judge", action="store_true")
    args = parser.parse_args()
    nya = args.nya.resolve()
    baseline_nya = args.baseline_nya.resolve() if args.baseline_nya else None
    output = args.output.resolve()
    catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
    if len(catalog) != 64 or len(CONTEXTS) != 16:
        raise SystemExit("stress corpus must contain 64 families and 16 contexts")
    if not nya.is_file() or (baseline_nya and not baseline_nya.is_file()):
        raise SystemExit("NYA binary not found")
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    work = Path(tempfile.mkdtemp(prefix="nya-stress-benchmark-"))
    positive_root = work / "positive"
    initialize(positive_root, nya, catalog)

    target_paths = [target.path for target in TARGETS]
    stress_task = "Review known recurrences across " + " ".join(
        target.key for target in TARGETS
    )
    baseline = None
    if baseline_nya:
        recalled, seconds = recall(
            baseline_nya, positive_root, stress_task, target_paths, 12
        )
        baseline = {
            "version": run([str(baseline_nya), "--version"], Path.cwd()).stdout.strip(),
            "sha256": hashlib.sha256(baseline_nya.read_bytes()).hexdigest(),
            "multi_path_candidates": len(recalled),
            "seconds": seconds,
        }

    all_pairs = [
        (family_index, context_index)
        for family_index in range(len(catalog))
        for context_index in range(len(CONTEXTS))
    ]
    required_pairs = {
        (
            next(index for index, item in enumerate(catalog) if item["key"] == target.key),
            CONTEXTS.index(target.context),
        )
        for target in TARGETS
    }
    remaining = [pair for pair in all_pairs if pair not in required_pairs]
    random.Random(args.seed).shuffle(remaining)
    selected = list(required_pairs) + remaining[: max(0, args.probes - len(required_pairs))]
    probe_results = []
    timings = []
    probe_log = output / "recall-probes.jsonl"
    with probe_log.open("w", encoding="utf-8", newline="\n") as handle:
        for offset, (family_index, context_index) in enumerate(selected, start=1):
            item = catalog[family_index]
            context = CONTEXTS[context_index]
            identifier = f"NYA-STRESS-{family_index:02}-{context_index:02}"
            recalled, seconds = recall(
                nya,
                positive_root,
                f"{context} {item['key']} {item['title']} {item['lesson']}",
                [f"apps/{context}/{item['domain']}/probe-{item['key']}.txt"],
                12,
            )
            ids = [scar["id"] for scar in recalled]
            result = {
                "probe": offset,
                "scar_id": identifier,
                "key": item["key"],
                "context": context,
                "candidate_count": len(ids),
                "rank": ids.index(identifier) + 1 if identifier in ids else None,
                "seconds": seconds,
            }
            probe_results.append(result)
            timings.append(seconds)
            handle.write(json.dumps(result) + "\n")
            print(f"[recall {offset}/{len(selected)}] {identifier}", flush=True)

    negative_results = []
    for index, domain in enumerate(sorted({item["domain"] for item in catalog})):
        recalled, seconds = recall(
            nya,
            positive_root,
            f"qzxv{index} plugh{index} zort{index}",
            [f"qzxvroot{index}/plughfile{index}.zzz"],
            12,
        )
        negative_results.append(
            {"domain": domain, "candidate_count": len(recalled), "seconds": seconds}
        )

    candidate_multi, candidate_multi_seconds = recall(
        nya, positive_root, stress_task, target_paths, 12
    )
    retrieval = {
        "probes": len(probe_results),
        "targets_recalled": sum(result["rank"] is not None for result in probe_results),
        "targets_ranked_first": sum(result["rank"] == 1 for result in probe_results),
        "bounded": sum(result["candidate_count"] <= 12 for result in probe_results),
        "max_candidates": max(result["candidate_count"] for result in probe_results),
        "negative_probes": len(negative_results),
        "negative_empty": sum(result["candidate_count"] == 0 for result in negative_results),
        "latency_seconds": {
            "p50": percentile(timings, 0.50),
            "p95": percentile(timings, 0.95),
            "p99": percentile(timings, 0.99),
            "max": max(timings),
        },
        "results": probe_results,
        "negative_results": negative_results,
    }
    retrieval_passed = (
        retrieval["targets_recalled"] == retrieval["probes"]
        and retrieval["bounded"] == retrieval["probes"]
        and retrieval["negative_empty"] == retrieval["negative_probes"]
        and len(candidate_multi) <= 12
    )

    positive_check = None
    negative_check = None
    if not args.skip_judge:
        configure_judge(positive_root, nya, args.codex, args.model)
        write_targets(positive_root, "recurrence")
        exit_code, seconds, verdict, diff = execute_check(
            positive_root, output, "positive", nya, stress_task
        )
        findings = verdict.get("findings", [])
        expected = {
            (scar_id(catalog, target.key, target.context), target.path): target
            for target in TARGETS
        }
        matched = set()
        unexpected = []
        for finding in findings:
            identity = (finding.get("scar_id"), finding.get("path"))
            evidence = finding.get("evidence")
            if identity in expected and isinstance(evidence, str) and evidence in diff:
                matched.add(identity)
            else:
                unexpected.append(finding)
        positive_results = []
        for target in TARGETS:
            identifier = scar_id(catalog, target.key, target.context)
            item = next(item for item in catalog if item["key"] == target.key)
            positive_results.append(
                {
                    "key": target.key,
                    "domain": item["domain"],
                    "scar_id": identifier,
                    "path": target.path,
                    "detected": (identifier, target.path) in matched,
                }
            )
        positive_check = {
            "targets": len(TARGETS),
            "detected": len(matched),
            "unexpected_findings": len(unexpected),
            "scars_checked": verdict.get("scars_checked"),
            "findings": len(findings),
            "exit": exit_code,
            "seconds": seconds,
            "results": positive_results,
        }

        negative_root = work / "negative"
        initialize(negative_root, nya, catalog)
        configure_judge(negative_root, nya, args.codex, args.model)
        write_targets(negative_root, "correction")
        negative_exit, negative_seconds, negative_verdict, _ = execute_check(
            negative_root, output, "negative", nya, stress_task
        )
        negative_check = {
            "controls": len(TARGETS),
            "findings": len(negative_verdict.get("findings", [])),
            "scars_checked": negative_verdict.get("scars_checked"),
            "exit": negative_exit,
            "seconds": negative_seconds,
        }

    expected_audited = sum(
        sum(item["domain"] == next(value["domain"] for value in catalog if value["key"] == target.key) for item in catalog)
        for target in TARGETS
    )
    judge_passed = args.skip_judge or (
        positive_check["exit"] == 1
        and positive_check["detected"] == len(TARGETS)
        and positive_check["unexpected_findings"] == 0
        and positive_check["scars_checked"] == expected_audited
        and negative_check["exit"] == 0
        and negative_check["findings"] == 0
    )
    summary = {
        "schema": 1,
        "benchmark": "scar-corpus-stress",
        "started_at": started_at,
        "completed_at": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "seed": args.seed,
        "corpus_size": len(catalog) * len(CONTEXTS),
        "families": len(catalog),
        "contexts": len(CONTEXTS),
        "expected_scars_audited": expected_audited,
        "domains": {
            domain: sum(item["domain"] == domain for item in catalog)
            for domain in sorted({item["domain"] for item in catalog})
        },
        "candidate": {
            "version": run([str(nya), "--version"], Path.cwd()).stdout.strip(),
            "sha256": hashlib.sha256(nya.read_bytes()).hexdigest(),
        },
        "baseline": baseline,
        "candidate_multi_path_candidates": len(candidate_multi),
        "candidate_multi_path_seconds": candidate_multi_seconds,
        "judge": (
            None
            if args.skip_judge
            else f"{run([args.codex, '--version'], Path.cwd()).stdout.strip()} "
            f"({args.model or 'default model'})"
        ),
        "retrieval": retrieval,
        "positive_check": positive_check,
        "negative_check": negative_check,
        "passed": retrieval_passed and judge_passed,
        "worktree": str(work),
    }
    (output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    report = render_report(summary)
    (output / "REPORT.md").write_text(report, encoding="utf-8", newline="\n")
    print(report)
    print(f"worktree={work}", flush=True)
    if not summary["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
