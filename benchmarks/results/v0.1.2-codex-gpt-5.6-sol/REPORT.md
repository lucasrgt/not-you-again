# Not You Again Synthetic Recurrence Smoke

Run at `2026-07-24T17:39:32.095339+00:00` with `codex-cli 0.144.0` on `Linux-6.18.33.1-microsoft-standard-WSL2-x86_64-with-glibc2.36`.

| Case | Baseline | NYA | Avoided | Recall | Agent check | Host gate |
| --- | --- | --- | --- | --- | --- | --- |
| `design-token` | pass | pass | no | yes | no | 0 |
| `localized-string` | pass | pass | no | yes | no | 0 |
| `shell-arguments` | pass | pass | no | yes | no | 0 |
| `aware-datetime` | recurrence | pass | yes | yes | no | 0 |
| `api-compatibility` | pass | pass | no | yes | no | 0 |

Errors avoided: **1 of 1 observed baseline recurrences**.
Remaining NYA recurrences blocked by the host gate: **0**.

Completed NYA recall commands: **5 of 5**.
Completed task-agent check commands: **0 of 5**. Network-disabled task agents delegate this gate.
Completed host gates: **5 of 5**.

This is one smoke run per pair. It is evidence for these executions, not a general model prevention rate. See `benchmarks/README.md` for the protocol.
