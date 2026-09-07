---
name: nya
description: Use standalone Not You Again repository knowledge before editing and its explicit semantic gate before completion.
---

# Not You Again

This skill is available only because the Git root contains `.nya/SKILL.md`
and does not contain `csm.toml`. If CSM is adopted, use only the CSM integration;
do not invoke the standalone adapter and duplicate retrieval or checks.

Before editing, retrieve relevant scars:

```bash
"${NYA_BIN:-nya}" recall --task="<goal>" --path <expected-path>
```

The Prime extension injects recall automatically when enabled. When reviewing a versioned specification, run `nya spec --file <spec> --task="<goal>" --path <expected-path>` explicitly.

Before completion, run:

```bash
"${NYA_BIN:-nya}" check --task="<completed work>" --base HEAD
```

Exit code 1 means repository findings remain; fix or report them and rerun. Exit
code 2 or a killed, failed, or truncated provider means the operation did not
complete and must never be reported as a pass.

Never run `nya init`, `nya setup`, `nya collect`, `nya remember`, or `nya replay` unless the user explicitly requests the corresponding administrative or recording operation.
