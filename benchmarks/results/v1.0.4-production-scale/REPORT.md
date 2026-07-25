# Not You Again Production Scale Benchmark

Run from `2026-07-25T02:45:00.894831+00:00` to `2026-07-25T02:53:56.648413+00:00` on `Windows-11-10.0.26200-SP0`.

## Deterministic scale and completeness

| Metric | Result |
| --- | ---: |
| Versioned scars | 10000 |
| Scars applicable to one file | 1000 |
| Recall targets found | 64 / 64 |
| Recall targets ranked first | 64 / 64 |
| Unrelated recall candidates | 0 |
| Recall p50 | 0.578 s |
| Recall p95 | 0.625 s |
| Recall maximum, including first cold read | 58.547 s |
| Large positive diff | 135124 bytes |
| Late target detected | yes |
| Applicable scars audited | 1000 |
| Corrected-control findings | 0 |
| Judge calls | 169 |
| Total prompt bytes | 14355256 |
| Maximum prompt bytes | 96656 |

The deterministic judge places the only recurrence in the final applicable scar and after the first 100,000 bytes of the changed file. It measures NYA orchestration completeness, not model intelligence.

## Cross-model semantic variance

| Model | Run | Detected | Unexpected | Negative findings | Calls | Prompt bytes | Reported tokens | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `gpt-5.6-sol` | 1 | 4 / 4 | 0 | 0 | 13 | 36886 | 57 | PASS |
| `gpt-5.6-sol` | 2 | 4 / 4 | 0 | 0 | 13 | 36803 | 56 | PASS |
| `gpt-5.3-codex-spark` | 1 | 4 / 4 | 0 | 0 | 12 | 34516 | 30 | PASS |
| `gpt-5.3-codex-spark` | 2 | 4 / 4 | 0 | 0 | 12 | 34440 | 30 | PASS |

### Variance summary

- `gpt-5.6-sol`: 2 / 2 passing runs, detection standard deviation 0.0, latency standard deviation 11.93 seconds.
- `gpt-5.3-codex-spark`: 2 / 2 passing runs, detection standard deviation 0.0, latency standard deviation 2.227 seconds.

Overall benchmark result: **PASS**.

Prompt bytes and judge calls measure context cost. Reported model tokens are included only when the underlying CLI exposes them. No monetary estimate is invented.

Development runs that exposed confirmation bias and overcorrection are
preserved in [ATTEMPTS.md](ATTEMPTS.md).
