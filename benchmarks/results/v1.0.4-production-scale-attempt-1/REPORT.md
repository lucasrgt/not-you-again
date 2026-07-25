# Not You Again Production Scale Benchmark

Run from `2026-07-25T02:13:41.251102+00:00` to `2026-07-25T02:22:20.547756+00:00` on `Windows-11-10.0.26200-SP0`.

## Deterministic scale and completeness

| Metric | Result |
| --- | ---: |
| Versioned scars | 10000 |
| Scars applicable to one file | 1000 |
| Recall targets found | 64 / 64 |
| Recall targets ranked first | 64 / 64 |
| Unrelated recall candidates | 0 |
| Recall p50 | 0.562 s |
| Recall p95 | 0.594 s |
| Large positive diff | 135124 bytes |
| Late target detected | yes |
| Applicable scars audited | 1000 |
| Corrected-control findings | 0 |
| Judge calls | 169 |
| Total prompt bytes | 14329407 |
| Maximum prompt bytes | 96503 |

The deterministic judge places the only recurrence in the final applicable scar and after the first 100,000 bytes of the changed file. It measures NYA orchestration completeness, not model intelligence.

## Cross-model semantic variance

| Model | Run | Detected | Unexpected | Negative findings | Calls | Prompt bytes | Reported tokens | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `gpt-5.6-sol` | 1 | 4 / 4 | 0 | 0 | 13 | 33966 | 102 | PASS |
| `gpt-5.6-sol` | 2 | 4 / 4 | 0 | 0 | 13 | 33997 | 89 | PASS |
| `gpt-5.3-codex-spark` | 1 | 4 / 4 | 0 | 1 | 13 | 34470 | 89 | FAIL |
| `gpt-5.3-codex-spark` | 2 | 4 / 4 | 0 | 0 | 11 | 29293 | 40 | FAIL |

### Variance summary

- `gpt-5.6-sol`: 2 / 2 passing runs, detection standard deviation 0.0, latency standard deviation 9.969 seconds.
- `gpt-5.3-codex-spark`: 0 / 2 passing runs, detection standard deviation 0.0, latency standard deviation 4.836 seconds.

Overall benchmark result: **FAIL**.

Prompt bytes and judge calls measure context cost. Reported model tokens are included only when the underlying CLI exposes them. No monetary estimate is invented.
