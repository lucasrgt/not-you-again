# Not You Again 1,024 Scar Stress Benchmark

Run from `2026-07-25T01:09:31.464015+00:00` to `2026-07-25T01:16:26.614226+00:00` on `Windows-11-10.0.26200-SP0`.

The corpus crosses 64 documented error families with 16 synthetic monorepo surfaces. The 1,024 records are scale fixtures, not 1,024 claimed real incidents.

| Domain | Families | Generated scars |
| --- | ---: | ---: |
| `backend-api` | 8 | 128 |
| `concurrency-clients` | 8 | 128 |
| `data-ml-science` | 8 | 128 |
| `design-accessibility` | 8 | 128 |
| `frontend-performance` | 8 | 128 |
| `infrastructure-reliability` | 8 | 128 |
| `quality-operations` | 8 | 128 |
| `security-privacy` | 8 | 128 |

## Retrieval

| Metric | Result |
| --- | ---: |
| Corpus size | 1024 |
| Positive probes | 128 |
| Exact target recalled | 128 |
| Target ranked first | 128 |
| Results respected limit | 128 |
| Negative probes returning no scars | 8 / 8 |
| Recall latency p50 | 0.109 s |
| Recall latency p95 | 0.125 s |
| Recall latency p99 | 0.14 s |
| Maximum recall candidates | 12 |

The comparison binary `nya 1.0.2` returned **128 candidates despite `--limit 12`** for the 16-path stress query. The candidate binary returned **12**.

## End-to-end judge

| Check | Result |
| --- | ---: |
| Injected recurrences | 16 |
| Exact recurrences detected | 16 |
| Unexpected findings | 0 |
| Unique scars audited in bounded batches | 128 |
| Positive gate exit | 1 |
| Positive check latency | 271.266 s |
| Corrected controls | 16 |
| Negative findings | 0 |
| Negative gate exit | 0 |
| Negative check latency | 120.594 s |

| Injected case | Domain | Detected |
| --- | --- | --- |
| `react-expensive-derived-usememo` | `frontend-performance` | yes |
| `design-literal-color` | `design-accessibility` | yes |
| `ui-localized-string` | `design-accessibility` | yes |
| `backend-shell-interpolation` | `backend-api` | yes |
| `backend-sql-interpolation` | `backend-api` | yes |
| `security-tenant-authorization` | `security-privacy` | yes |
| `security-ssrf` | `security-privacy` | yes |
| `reliability-retry-no-backoff` | `infrastructure-reliability` | yes |
| `infra-missing-resource-bounds` | `infrastructure-reliability` | yes |
| `ml-training-test-leakage` | `data-ml-science` | yes |
| `science-unit-mismatch` | `data-ml-science` | yes |
| `mobile-main-thread-io` | `concurrency-clients` | yes |
| `filesystem-nonatomic-write` | `concurrency-clients` | yes |
| `runbook-destructive-no-backup` | `quality-operations` | yes |
| `experiment-missing-guardrail` | `quality-operations` | yes |
| `a11y-input-label` | `design-accessibility` | yes |

Overall benchmark result: **PASS**.

A pass requires bounded retrieval, every exact positive probe, empty unrelated queries, every injected recurrence identified by exact scar ID and path with verbatim diff evidence, no unexpected finding, and zero findings for corrected controls.

This benchmark measures retrieval and known-recurrence detection at one corpus size. It does not estimate a universal prevention rate. The catalog, generator, diffs, structured verdicts, timings, binary hashes, and summary remain auditable.

Three unsuccessful development runs that exposed retrieval, batching, scope,
and fixture problems are preserved in [ATTEMPTS.md](ATTEMPTS.md).
