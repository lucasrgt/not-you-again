# Synthetic Recurrence Smoke Benchmark

This benchmark measures whether a repository scar changes the final result of
an otherwise identical coding task.

The repositories are synthetic. The failure patterns are not invented for the
score. Each case represents a recurring review concern documented by a primary
source:

| Case | Recurrence | Reference |
| --- | --- | --- |
| `design-token` | A component introduces literal colors instead of semantic tokens | [U.S. Web Design System design tokens](https://designsystem.digital.gov/design-tokens/) |
| `localized-string` | User-facing text bypasses the repository message catalog | [Android localization guidance](https://developer.android.com/guide/topics/resources/localization) |
| `shell-arguments` | Dynamic arguments are interpolated into a shell command | [Node.js child process documentation](https://nodejs.org/api/child_process.html) |
| `aware-datetime` | Offset-aware timestamps are compared with a naive current time | [Python datetime documentation](https://docs.python.org/3/library/datetime.html) |
| `api-compatibility` | A new presentation field changes the meaning of an existing API field | [Semantic Versioning specification](https://semver.org/) |

## Built-in replay versus agent benchmarks

`nya replay` is a local corpus audit. It checks whether the configured judge
can identify a stored failure on the removed side of its historical correction
patch and the corresponding fix on the added side. It does not execute an agent
or claim prevention.

The protocols below execute paired agents and use hidden evaluation. Use them
when measuring recurrence prevention. Use `nya replay --format json` when
validating scar and judge behavior or selecting auditable historical cases for
an external benchmark.

## Protocol

For every case, the runner creates two fresh Git repositories from the same
files and sends the same task prompt to a fresh agent process.

The Codex process is ephemeral and runs with the `workspace-write` sandbox. The
reference run uses the included Docker image with only the benchmark, a released
NYA binary, and an ephemeral copy of Codex authentication mounted into it. No
host MCP server, skill, project, or user configuration is available inside the
container.

| Arm | Repository state |
| --- | --- |
| Baseline | No Not You Again files or scar |
| NYA | `nya init`, one seeded scar, the managed agent instructions, and a local Codex judge |

After the task agent exits, the runner invokes `nya check` from the host process.
This models a pre-push host gate, MCP server, or CI job. A built-in network judge
cannot call its provider from inside a task agent's network-disabled shell, so
NYA detects that environment and fails fast instead of retrying.

The seeded occurrence is explicitly synthetic and represents an earlier team
review in the test repository. It is never presented as a real public pull
request.

The evaluator is held outside both repositories. The agent cannot see its
checks. It reports three distinct outcomes:

| Outcome | Meaning |
| --- | --- |
| `pass` | The requested behavior exists and the known failure is absent |
| `recurrence` | The requested behavior exists but repeats the known failure |
| `incomplete` | The task itself is missing or broken |

An error counts as avoided only when the baseline result is `recurrence` and
the paired NYA result is `pass`. A baseline that already passes does not add to
the avoided count. A remaining NYA recurrence is reported separately when the
host gate blocks it with exit code 1. An incomplete task does not count either
way.

This is a smoke benchmark, not a statistically powered model evaluation. A
single run can establish that the end-to-end loop works and expose concrete
paired examples. It cannot establish a general prevention rate.

## Run

Build the pinned runner image:

```bash
docker build -t nya-benchmark:0.1.2 benchmarks
```

Run it with the released Linux x64 binary, an empty output directory, and a
read-only authentication seed:

```bash
docker run --rm \
  --security-opt seccomp=unconfined \
  -v "$HOME/.codex/auth.json:/seed/auth.json:ro" \
  -v "$PWD/benchmarks/smoke.py:/benchmarks/smoke.py:ro" \
  -v "$PWD/nya:/usr/local/bin/nya:ro" \
  -v "$PWD/benchmark-output:/output" \
  nya-benchmark:0.1.2 \
  python3 /benchmarks/smoke.py \
  --nya /usr/local/bin/nya \
  --output /output \
  --model gpt-5.6-sol
```

The seccomp override permits the Codex bubblewrap sandbox to create its nested
Linux namespace. The container is not privileged and receives no Docker socket.
Only the result directory is writable on the host.

The output directory contains the machine-readable summary, a Markdown report,
the final diff for every arm, the agent's final message, and the command event
log. A recall or check is marked as observed only for a completed
`command_execution` event, never from prose. Timestamps and randomized order are
recorded so the run can be audited.

## Real GitHub review recurrence smoke

`github_review.py` consumes a scar whose provenance was verified from a real
line-level GitHub review comment. It then gives two fresh agents the same later
task in randomized order:

| Arm | Repository state |
| --- | --- |
| Baseline | No Not You Again files or scar |
| NYA | The exact versioned scar created from the public review correction |

The later task uses a different module and entity names while repeating the
same multi-tenant cache-isolation hazard. A hidden evaluator checks task
completion and cross-organization isolation. Prevention is recorded only when
the baseline repeats the failure and the NYA arm passes.

```bash
python benchmarks/github_review.py \
  --nya /path/to/released/nya \
  --scar /path/to/NYA-....toml \
  --output benchmark-output \
  --source-pr https://github.com/lucasrgt/nya-github-review-benchmark/pull/1 \
  --source-comment https://github.com/lucasrgt/nya-github-review-benchmark/pull/1#discussion_r3647349286
```

The public fixture PR discloses that its initial defect is intentional. The
review comment, correcting commit, scar file, agent event log, paired diffs,
and machine-readable result remain auditable independently.

## Persisted scar detection benchmark

`detection.py` measures the narrower product invariant directly. It creates
five fresh repositories, commits one known scar into each repository, injects
the corresponding recurrence, and invokes the real two-stage `nya check`
judge.

A case passes only when:

1. `nya recall` returns the persisted scar.
2. `nya check` exits with code 1.
3. The JSON verdict identifies the exact scar ID and changed path.
4. The finding includes verbatim evidence from the diff.

```bash
python benchmarks/detection.py \
  --nya /path/to/released/nya \
  --output benchmark-output \
  --model gpt-5.6-sol \
  --source-archive https://github.com/owner/repository/releases/download/v1.0.1/nya.zip \
  --source-archive-sha256 <verified-archive-sha256>
```

The output contains every seeded scar, injected diff, recall result, check
verdict, stderr log, a machine-readable summary, and a Markdown report. This
benchmark measures detection of already-known recurrences. It does not measure
whether an agent avoids producing them without a final gate.

## 1,024 scar stress benchmark

`stress.py` crosses 64 documented error families with 16 synthetic monorepo
surfaces to create 1,024 versioned scars. The families cover frontend
performance, design and accessibility, backend and API design, security and
privacy, infrastructure and reliability, data and machine learning, scientific
computing, concurrency, clients, testing, documentation, and operations.

The runner measures three separate properties:

1. Bounded deterministic recall with stratified positive and unrelated negative
   probes.
2. One multi-file check containing 16 known recurrences across eight domains.
3. One matching negative check containing the 16 corrected implementations.

The generated records are scale fixtures derived from the versioned
`stress_catalog.json`. They are not presented as 1,024 independent real
incidents. Every family retains a primary reference, while the injected diffs
and exact expected scar IDs remain auditable.

```bash
python benchmarks/stress.py \
  --nya /path/to/candidate/nya \
  --baseline-nya /path/to/nya-v1.0.2 \
  --output benchmark-output \
  --probes 128 \
  --model gpt-5.6-sol
```

A pass requires every positive probe, bounded result counts, empty unrelated
queries, all 16 exact recurrences with verbatim evidence, no unexpected
finding, and zero findings for the corrected controls.

## Production-scale and variance benchmark

`scale.py` creates 10,000 versioned scars, makes 1,000 of them applicable to
one changed file, and places the only deterministic recurrence in the final
applicable scar after the first 100,000 bytes of the diff.

The deterministic phase measures:

1. Positive and unrelated retrieval across the 10,000-scar corpus.
2. Exhaustive crossing of scar batches and overlapping diff windows.
3. Exact late-target detection and a matching corrected control.
4. Recall latency, judge calls, total prompt bytes, and maximum prompt size.

The semantic phase runs paired recurrence and corrected fixtures for
`useMemo`, design tokens, SQL parameterization, and scientific units. Every
configured model is executed repeatedly. A single missed recurrence,
unexpected identity, corrected-control finding, or failed gate fails the
complete benchmark.

```bash
python benchmarks/scale.py \
  --nya /path/to/candidate/nya \
  --output benchmark-output \
  --corpus-size 10000 \
  --applicable 1000 \
  --probes 64 \
  --model gpt-5.6-sol \
  --model gpt-5.3-codex-spark \
  --repetitions 2
```

`judge_proxy.py` records context cost without changing verdicts.
`scale_judge.py` is deterministic and tests orchestration rather than model
intelligence. Reported token counts are included only when the underlying CLI
exposes them; the benchmark never invents monetary cost.
