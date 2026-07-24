#!/usr/bin/env python3
import argparse
import hashlib
import importlib.util
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path


TASK = (
    "Change load_document to accept a dictionary cache as its first argument. "
    "Repeated loads of the same document ID must call the loader only once. "
    "Update the tests and keep the public function name."
)

FILES = {
    "documents.py": '''from collections.abc import Callable

Document = dict[str, str]
Loader = Callable[[str, str], Document]


def load_document(org_id: str, document_id: str, loader: Loader) -> Document:
    return loader(org_id, document_id)
''',
    "test_documents.py": '''import unittest

from documents import load_document


class DocumentTests(unittest.TestCase):
    def test_loads_the_requested_document(self) -> None:
        document = load_document(
            "org-a",
            "doc-1",
            lambda org, document_id: {"id": document_id, "org": org},
        )
        self.assertEqual(document, {"id": "doc-1", "org": "org-a"})


if __name__ == "__main__":
    unittest.main()
''',
    "AGENTS.md": """# Repository instructions

Make the smallest complete change. Inspect existing conventions, run relevant
tests, and follow any repository skill installed by the benchmark.
""",
}


def run(command, cwd, env=None, timeout=120, check=True):
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


def write_repo(root: Path):
    root.mkdir(parents=True)
    for relative, body in FILES.items():
        (root / relative).write_text(body, encoding="utf-8", newline="\n")
    run(["git", "init", "-q"], root)
    run(["git", "config", "user.name", "NYA Benchmark"], root)
    run(["git", "config", "user.email", "benchmark@example.test"], root)
    run(["git", "config", "core.autocrlf", "false"], root)


def initialize(root: Path, arm: str, nya: Path, scar: Path):
    write_repo(root)
    if arm == "nya":
        run([str(nya), "--repository", str(root), "init"], root)
        shutil.copy2(scar, root / ".nya/scars" / scar.name)
        run([str(nya), "--repository", str(root), "setup", "--local", "--judge", "codex"], root)
    run(["git", "add", "."], root)
    run(["git", "commit", "-qm", "benchmark baseline"], root)


def load_module(path: Path):
    name = f"nya_github_benchmark_{time.time_ns()}"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def evaluate(root: Path):
    tests = run([sys.executable, "-m", "unittest", "-v"], root, check=False)
    try:
        module = load_module(root / "documents.py")
        calls = []

        def loader(org, document_id):
            calls.append((org, document_id))
            return {"id": document_id, "org": org}

        cache = {}
        first = module.load_document(cache, "org-a", "doc-1", loader)
        repeated = module.load_document(cache, "org-a", "doc-1", loader)
        isolated = module.load_document(cache, "org-b", "doc-1", loader)
        task_ok = tests.returncode == 0 and first is repeated and calls.count(("org-a", "doc-1")) == 1
        scar_ok = isolated == {"id": "doc-1", "org": "org-b"} and calls.count(("org-b", "doc-1")) == 1
        detail = "overlapping document IDs remain isolated by organization"
    except Exception as error:
        task_ok = False
        scar_ok = False
        detail = f"evaluation failed: {type(error).__name__}: {error}"
    outcome = "pass" if task_ok and scar_ok else "recurrence" if task_ok else "incomplete"
    return {
        "outcome": outcome,
        "task_ok": task_ok,
        "scar_ok": scar_ok,
        "detail": detail,
        "tests_exit": tests.returncode,
    }


def command_observed(events: str, action: str):
    pattern = re.compile(rf"\bnya(?:\.exe)?\s+{re.escape(action)}\b", re.I)
    for line in events.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        item = event.get("item", {})
        if event.get("type") == "item.completed" and item.get("type") == "command_execution" and pattern.search(item.get("command", "")):
            return True
    return False


def execute(arm, root, output, nya, scar, codex, model):
    initialize(root, arm, nya, scar)
    prompt = (
        "Implement the task in this repository. Make the smallest complete change, "
        "inspect existing conventions, run relevant checks, and stop when the work "
        f"is ready to commit. Do not ask questions.\n\nTask: {TASK}"
    )
    last = output / f"{arm}-last.md"
    env = os.environ.copy()
    env["PATH"] = str(nya.parent) + os.pathsep + env.get("PATH", "")
    command = [
        codex,
        "exec",
        "--model",
        model,
        "--ephemeral",
        "--ignore-user-config",
        "--sandbox",
        "workspace-write",
        "--json",
        "--output-last-message",
        str(last),
        "-C",
        str(root),
        prompt,
    ]
    started = time.monotonic()
    agent = run(command, root, env=env, timeout=420, check=False)
    events = agent.stdout + agent.stderr
    (output / f"{arm}-events.jsonl").write_text(events, encoding="utf-8", newline="\n")
    diff = run(["git", "diff", "--binary", "HEAD"], root).stdout
    (output / f"{arm}.diff").write_text(diff, encoding="utf-8", newline="\n")
    result = evaluate(root)
    result.update(
        {
            "agent_exit": agent.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "recall_observed": command_observed(events, "recall"),
            "check_observed": command_observed(events, "check"),
        }
    )
    if arm == "nya":
        gate = run([str(nya), "--repository", str(root), "check", "--format", "json"], root, env=env, timeout=300, check=False)
        (output / "nya-gate.log").write_text(gate.stdout + gate.stderr, encoding="utf-8", newline="\n")
        result["gate_exit"] = gate.returncode
        try:
            result["gate"] = json.loads(gate.stdout)
        except json.JSONDecodeError:
            result["gate"] = None
    return result


def report(summary):
    rows = []
    for arm in ["baseline", "nya"]:
        result = summary["results"][arm]
        rows.append(
            f"| {arm} | {result['outcome']} | {result['task_ok']} | "
            f"{result['scar_ok']} | {result.get('recall_observed', False)} | "
            f"{result.get('gate_exit', 'n/a')} |"
        )
    prevented = summary["prevented"]
    return f"""# Real GitHub Review Recurrence Smoke

The source scar was created from a real line-level review on
[fixture PR #1]({summary['source_pr']}) using
[this exact comment]({summary['source_comment']}).

| Arm | Outcome | Task complete | Isolation preserved | Recall observed | Host gate |
| --- | --- | --- | --- | --- | --- |
{chr(10).join(rows)}

Prevention evidence: **{str(prevented).lower()}**.

This one paired smoke proves or disproves the tested causal example only. It is
not a general prevention rate.
"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--nya", required=True, type=Path)
    parser.add_argument("--scar", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--source-pr", required=True)
    parser.add_argument("--source-comment", required=True)
    parser.add_argument("--codex", default=shutil.which("codex") or "codex")
    parser.add_argument("--model", default="gpt-5.6-sol")
    args = parser.parse_args()
    args.nya = args.nya.resolve()
    args.scar = args.scar.resolve()
    args.output = args.output.resolve()
    if args.output.exists() and any(args.output.iterdir()):
        raise SystemExit("output directory must be empty")
    args.output.mkdir(parents=True, exist_ok=True)
    order = ["baseline", "nya"]
    random.Random(20260724).shuffle(order)
    summary = {
        "schema": 1,
        "started_at": datetime.now(timezone.utc).isoformat(),
        "source_pr": args.source_pr,
        "source_comment": args.source_comment,
        "scar_sha256": hashlib.sha256(args.scar.read_bytes()).hexdigest(),
        "nya_version": run([str(args.nya), "--version"], Path.cwd()).stdout.strip(),
        "codex_version": run([args.codex, "--version"], Path.cwd()).stdout.strip(),
        "model": args.model,
        "order": order,
        "task": TASK,
        "results": {},
    }
    with tempfile.TemporaryDirectory(prefix="nya-github-review-") as temporary:
        for arm in order:
            summary["results"][arm] = execute(arm, Path(temporary) / arm, args.output, args.nya, args.scar, args.codex, args.model)
    baseline = summary["results"]["baseline"]["outcome"]
    nya = summary["results"]["nya"]["outcome"]
    summary["prevented"] = baseline == "recurrence" and nya == "pass"
    summary["finished_at"] = datetime.now(timezone.utc).isoformat()
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n")
    (args.output / "REPORT.md").write_text(report(summary), encoding="utf-8", newline="\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
