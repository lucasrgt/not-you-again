#!/usr/bin/env python3
import argparse
import json
import sys
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--scar", required=True)
    parser.add_argument("--path", required=True)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--line", required=True, type=int)
    args = parser.parse_args()
    prompt = sys.stdin.read()
    started = time.monotonic()
    matched = args.scar in prompt and args.evidence in prompt
    findings = []
    if matched:
        findings.append(
            {
                "scar_id": args.scar,
                "path": args.path,
                "line": args.line,
                "evidence": args.evidence,
                "reason": "The late changed line repeats the supplied scale scar.",
            }
        )
    record = {
        "stage": "confirmation" if "<PROPOSED>" in prompt else "audit",
        "prompt_bytes": len(prompt.encode()),
        "seconds": round(time.monotonic() - started, 6),
        "matched": matched,
    }
    args.log.parent.mkdir(parents=True, exist_ok=True)
    with args.log.open("a", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(record) + "\n")
    print(json.dumps({"findings": findings}))


if __name__ == "__main__":
    main()
