#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", required=True, type=Path)
    parser.add_argument("--label", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        raise SystemExit("judge command is required after --")

    prompt = sys.stdin.buffer.read()
    started = time.monotonic()
    result = subprocess.run(command, input=prompt, capture_output=True, check=False)
    elapsed = round(time.monotonic() - started, 4)
    stderr = result.stderr.decode("utf-8", errors="replace")
    tokens = re.findall(r"tokens used\s*[\r\n:]+\s*([\d,]+)", stderr, re.IGNORECASE)
    record = {
        "label": args.label,
        "stage": "confirmation" if b"<PROPOSED>" in prompt else "audit",
        "prompt_bytes": len(prompt),
        "output_bytes": len(result.stdout),
        "seconds": elapsed,
        "reported_tokens": int(tokens[-1].replace(",", "")) if tokens else None,
        "exit": result.returncode,
    }
    args.log.parent.mkdir(parents=True, exist_ok=True)
    with args.log.open("a", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(record) + "\n")
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(result.stderr)
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
