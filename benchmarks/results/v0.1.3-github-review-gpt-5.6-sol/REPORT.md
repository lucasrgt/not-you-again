# GitHub Review to Scar to Prevention

This run proves the complete Not You Again workflow with a real public GitHub
line review and the released v0.1.3 binary.

## Source correction

| Evidence | Value |
| --- | --- |
| Fixture repository | [lucasrgt/nya-github-review-benchmark](https://github.com/lucasrgt/nya-github-review-benchmark) |
| Pull request | [PR #1](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1) |
| Review comment | [discussion_r3647349286](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1#discussion_r3647349286) |
| Review author | `github:lucasrgt` |
| Review time | `2026-07-24T18:20:31Z` |
| Correcting commit | [`9681e4f`](https://github.com/lucasrgt/nya-github-review-benchmark/commit/9681e4fbdf4e53152f7fd99282e772d795dee9de) |
| Merge commit | [`e043caa`](https://github.com/lucasrgt/nya-github-review-benchmark/commit/e043caa8cb91594d029143d98b9ce5621f1497f7) |
| Scar | `NYA-01KYAP8ZD63BM3A0G3NRP4T5AS` |
| Scar SHA-256 | `a0dd287e9f96d86993f5cadfd42b17d03e2e25e8070d6f0a88072d38388251e4` |

A fresh Codex process received only the task to address the unresolved review
and the repository-managed instructions. It inspected the live GitHub thread,
fixed the cache key, added a cross-tenant test, and ran
`nya remember --github-review` itself. The resulting versioned scar contains
the canonical comment URL, GitHub reporter, original review timestamp,
corrector, recorder, and source commit.

The exact artifacts are:

- [`review-comment.json`](review-comment.json), the GitHub API response
- [`correction-events.jsonl`](correction-events.jsonl), the full agent event log
- [`correction.diff`](correction.diff), the correcting diff before commit
- [`source-scar.toml`](source-scar.toml), the exact generated scar

The correction agent exited 0, both regression tests passed, and its isolated
`nya check` checked one scar and passed. The review thread was then resolved
and the PR merged.

## Later recurrence smoke

The same released binary and exact scar bytes were mounted read-only into a
pinned Debian container with Codex CLI `0.144.0` and model `gpt-5.6-sol`.
Within each valid pair, fresh agents received identical files and task text.
Only the NYA arm received the scar and managed instructions. The evaluator was
held outside both repositories.

| Pair | Task wording | Baseline | NYA | Avoided |
| --- | --- | --- | --- | ---: |
| Explicit boundary | Repeated loads in the same organization | Pass | Pass | 0 |
| Under-specified ticket | Repeated loads of the same document ID | Recurrence | Pass | 1 |

In the under-specified pair, both agents completed the requested caching
behavior and passed their visible tests. The baseline keyed the cache only by
`document_id`. The NYA arm recalled the GitHub-derived scar, keyed by
`(org_id, document_id)`, and added a cross-organization regression test. Its
external host gate checked one scar and exited 0.

Measured across the two valid pairs:

| Metric | Count |
| --- | ---: |
| Valid paired tasks | 2 |
| Baseline recurrences | 1 |
| Recurrences avoided by NYA | 1 |
| Remaining NYA recurrences | 0 |
| Baselines that already passed | 1 |

The preliminary native Windows attempt is preserved under
[`attempt-1-windows-sandbox-blocked`](attempt-1-windows-sandbox-blocked) and
excluded because both repositories were mounted read-only. The first valid
container pair is preserved under
[`attempt-2-explicit-boundary-baseline-passed`](attempt-2-explicit-boundary-baseline-passed).
The causal under-specified pair is under [`recurrence`](recurrence).

This is auditable smoke evidence for these tested pairs, not a general
prevention rate.
