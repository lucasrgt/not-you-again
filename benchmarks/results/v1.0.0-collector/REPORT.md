# Not You Again v1.0 Collector Smoke

## Purpose

This smoke tests whether `nya collect` can recover durable scars from real
repository history without acting as a generic reviewer or inventing
provenance.

It uses two existing repositories:

1. A public pull request with a real line-level review, later correction, and
   merge history.
2. The mature AeroFortress Harness repository with an existing catalog of 45
   scars.

The machine-readable results are in
[`summary.json`](summary.json).

## Environment

| Component | Value |
| --- | --- |
| Date | 2026-07-24 |
| Collector | Not You Again v1.0 release candidate |
| Evaluator profile | Built-in Codex profile |
| Codex CLI | 0.144.0 |
| GitHub CLI | 2.95.0 |
| Git | 2.54.0.windows.1 |

No result was entered by hand into a consumer repository. The public fixture
was cloned from GitHub, its previously recorded benchmark scar was removed from
the temporary clone, and the collector rebuilt the lesson from repository and
GitHub evidence.

## Run 1: real GitHub review recovery

Repository:
[`lucasrgt/nya-github-review-benchmark`](https://github.com/lucasrgt/nya-github-review-benchmark)

Source review:
[`discussion_r3647349286`](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1#discussion_r3647349286)

The review states that a profile cache key omitted `tenant_id`, allowing the
same user ID in two tenants to collide. Commit
[`9681e4f`](https://github.com/lucasrgt/nya-github-review-benchmark/commit/9681e4fbdf4e)
later corrected the same path.

Command:

```bash
nya collect --all
```

Result:

| Metric | Value |
| --- | ---: |
| Sources scanned | 7 |
| Correction candidates | 1 |
| New scars | 1 |
| Occurrences appended | 0 |
| Insufficient evidence | 0 |
| Ambiguous | 0 |

The generated scar contained:

```text
title:        Tenant omitted from profile cache key
scope:        profiles.py
source:       the exact review permalink
reported_by:  github:lucasrgt
corrected_by: git:lucas.tinoco@hotmail.com
recorded_by:  nya:collector
commit:       9681e4fbdf4e
```

An immediate incremental `nya collect` returned zero correction candidates,
proving source and checkpoint idempotency for this fixture.

## Run 2: mature repository classification

Repository:
[`lucasrgt/aerofortress-harness`](https://github.com/lucasrgt/aerofortress-harness)

The command ran as a non-writing audit:

```bash
nya collect --since HEAD~8 --dry-run
```

Result:

| Metric | Value |
| --- | ---: |
| Sources scanned | 9 |
| Correction candidates | 4 |
| New scars | 2 |
| Existing scars matched | 2 |
| Insufficient evidence | 0 |
| Ambiguous | 0 |

The two existing matches were not title-only duplicates:

1. Commit
   [`1b5a940`](https://github.com/lucasrgt/aerofortress-harness/commit/1b5a940908ae127f5f73fde12f5d377b3bd8c01c)
   matched `Call count alone does not prove tool compilation`.
2. Commit
   [`25c124c`](https://github.com/lucasrgt/aerofortress-harness/commit/25c124cb095ae282f7d60dcc64d3770460a396eb)
   matched `Streaming must not reparse accumulated Markdown for every token`.

The two new lessons came from:

1. Commit
   [`b74e8fe`](https://github.com/lucasrgt/aerofortress-harness/commit/b74e8febb25726562868a28931f1e2e96246f7af),
   which stopped the updater from mutating dirty registered checkouts.
2. Commit
   [`1a4b0a4`](https://github.com/lucasrgt/aerofortress-harness/commit/1a4b0a4b95188636957553ec72fa6e922cfc900c),
   which normalized malformed question collections at both replay and live
   rendering boundaries.

Because this run used `--dry-run`, no scar or checkpoint was written.

## Guardrails discovered during the smoke

The first public-fixture attempt produced a descriptive scope that did not
match a repository path. The collector rejected that shape after path-bound
scope validation was added.

The first mature-repository attempt selected an overly long evidence paragraph
and paraphrased part of it. The collector rejected the entire batch. The final
contract limits evidence to a short exact substring of at most 240 characters
and repeats the verbatim requirement in both the prompt and output schema.

These are observed failures from the smoke, not synthetic test cases. Both
became permanent validation tests before release.

## Interpretation

The smoke demonstrates:

1. A corrected GitHub review can become a scar without manual transcription.
2. Reporter, corrector, recorder, source, time, and correction commit survive
   collection.
3. Existing semantic lessons receive occurrences instead of duplicate files.
4. Dry-run output is auditable before persistence.
5. Unsupported shapes fail closed.

It does not establish a universal extraction rate. The public review sample is
one pull request, and the mature-repository sample covers eight recent
revisions. Squashed or rebased reviews whose reviewed commit is absent from
local ancestry are intentionally skipped in v1.0 rather than inferred.
