# Not You Again Persisted Scar Detection Benchmark

Run from `2026-07-24T23:22:22.400553+00:00` to `2026-07-24T23:23:47.992960+00:00` with `codex-cli 0.144.0 (gpt-5.6-sol)` on `Windows-11-10.0.26200-SP0`.

NYA binary: `nya 1.0.1` with SHA256 `de4b4db529015be49322ef1d84d541b41e182b2e0de1fe4a19c203b7a1d82e46`.

Source archive: `https://github.com/lucasrgt/not-you-again/releases/download/v1.0.1/nya-v1.0.1-x86_64-pc-windows-msvc.zip` with SHA256 `ed812d6f5d4f752f2437a0379256d5718cd411e609b5462715b650bba45778e8`.

Each fresh repository contains one committed scar. The runner then injects a concrete recurrence into the matching scope and invokes the real `nya check` two-stage judge.

| Case | Recalled | Scars checked | Exit | Matching finding | Detected |
| --- | --- | ---: | ---: | ---: | --- |
| `design-token` | yes | 1 | 1 | 1 | yes |
| `localized-string` | yes | 1 | 1 | 1 | yes |
| `shell-arguments` | yes | 1 | 1 | 1 | yes |
| `aware-datetime` | yes | 1 | 1 | 1 | yes |
| `api-compatibility` | yes | 1 | 1 | 1 | yes |

Persisted recurrences detected and blocked: **5 of 5**.

A case passes only when the persisted scar is recalled, `nya check` exits with code 1, and the structured verdict contains that exact scar ID, changed path, and verbatim diff evidence.

This benchmark measures known-recurrence detection, not agent behavior or a general prevention rate. The seeded scars and injected recurrences are synthetic and are retained as auditable fixtures.
