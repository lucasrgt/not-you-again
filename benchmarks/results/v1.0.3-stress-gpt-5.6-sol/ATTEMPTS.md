# Stress benchmark development attempts

The passing result was not selected from repeated identical runs. Each prior
attempt exposed a concrete problem, was preserved, and led to a documented
product or protocol correction.

| Attempt | Matching scars audited | Positive detections | Unexpected positive findings | Negative findings | What it exposed |
| --- | ---: | ---: | ---: | ---: | --- |
| [1](../v1.0.3-stress-gpt-5.6-sol-attempt-1/REPORT.md) | 1,024 | 15 of 16 | 4 | 3 | One global candidate set could miss a target, and several controls were incomplete |
| [2](../v1.0.3-stress-gpt-5.6-sol-attempt-2/REPORT.md) | 1,024 | 16 of 16 | 3 | 4 | Generated cross-domain scopes were unrealistically broad and caused false positives |
| [3](../v1.0.3-stress-gpt-5.6-sol-attempt-3/REPORT.md) | 128 | 16 of 16 | 1 | 1 | Domain scopes worked, but the accessibility control still contained the prohibited hardcoded text |
| [Final](REPORT.md) | 128 | 16 of 16 | 0 | 0 | Pass |

The final implementation keeps interactive recall bounded, audits every scar
whose scope matches a changed path in batches of 24, and lets only unscoped
scars enter a path through relevance search. The final fixtures use
domain-specific scopes and isolate one intended recurrence per changed file.

All attempt directories include their generated summary, report, diffs,
verdicts, stderr logs, and binary metadata.
