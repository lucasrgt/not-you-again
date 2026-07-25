# Production-scale attempt 1

This run completed the deterministic 10,000-scar phase, both `gpt-5.6-sol`
runs, and the first `gpt-5.3-codex-spark` run. It was manually interrupted
during the final negative check after buffered output was mistakenly
interpreted as a stalled process.

The completed Spark run detected all four injected recurrences but incorrectly
flagged this corrected design-token use:

```text
background: "var(--color-danger)"
```

The lesson said to reference role-based design tokens but did not explicitly
state that CSS custom-property references implement that remedy. The catalog
was clarified before the next full run. Raw scale, diff, check, and judge
metrics are preserved in this directory; there is no aggregate `summary.json`
because the process was interrupted before final rendering.
