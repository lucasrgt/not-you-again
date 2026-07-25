# Not You Again Production Scale Benchmark

Run from `2026-07-25T02:36:37.500696+00:00` to `2026-07-25T02:39:37.266267+00:00` on `Windows-11-10.0.26200-SP0`.

## Deterministic scale and completeness

| Metric | Result |
| --- | ---: |
| Versioned scars | 48 |
| Scars applicable to one file | 48 |
| Recall targets found | 2 / 2 |
| Recall targets ranked first | 2 / 2 |
| Unrelated recall candidates | 0 |
| Recall p50 | 0.18 s |
| Recall p95 | 0.313 s |
| Large positive diff | 135124 bytes |
| Late target detected | yes |
| Applicable scars audited | 48 |
| Corrected-control findings | 0 |
| Judge calls | 9 |
| Total prompt bytes | 740978 |
| Maximum prompt bytes | 96703 |

The deterministic judge places the only recurrence in the final applicable scar and after the first 100,000 bytes of the changed file. It measures NYA orchestration completeness, not model intelligence.

## Cross-model semantic variance

| Model | Run | Detected | Unexpected | Negative findings | Calls | Prompt bytes | Reported tokens | Result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `gpt-5.3-codex-spark` | 1 | 3 / 4 | 0 | 0 | 12 | 34702 | 29 | FAIL |
| `gpt-5.3-codex-spark` | 2 | 4 / 4 | 0 | 0 | 12 | 34562 | 30 | PASS |

### Variance summary

- `gpt-5.3-codex-spark`: 1 / 2 passing runs, detection standard deviation 0.5, latency standard deviation 2.109 seconds.

Overall benchmark result: **FAIL**.

Prompt bytes and judge calls measure context cost. Reported model tokens are included only when the underlying CLI exposes them. No monetary estimate is invented.
