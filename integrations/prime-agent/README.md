# Not You Again for Prime Agent

This optional capability package is a thin adapter around the standalone `nya`
Rust CLI. It adds bounded automatic `recall`, explicit operator commands, and a
conditional model skill without reading semantic records or reimplementing
Not You Again behavior.

## Install

Install `nya` on `PATH`, then run:

```bash
prime-agent package install /absolute/path/to/not-you-again/integrations/prime-agent
```

Use `/reload` in a live Prime session. Set `NYA_BIN` or pass
`--nya-bin /absolute/path/to/nya` when needed.

## Activation and precedence

The package activates only when the Git root contains `.nya/SKILL.md`. It is
fully suppressed when `<git-root>/csm.toml` exists, even if the standalone marker
also remains. CSM then owns Prime retrieval and verification; direct standalone
CLI use remains available. In inactive repositories the package invokes no
`nya` process, exposes no command or skill, and paints no status.

## Surface

- ``/nya recall <task>` and `/nya check [--base=REF] <task>``
- `/nya status`
- `/nya auto recall on|off`

Automatic `recall` is enabled by default and can be disabled at launch with
`--nya-auto-recall off`. Checks are always explicit. The adapter exposes no
repository adoption or semantic-record mutation command.

Every process uses a literal argv array, the resolved Git root as cwd, a
configurable timeout, cancellation, control-sequence sanitization, and a 64 KiB
UTF-8 output cap. Nonzero exits, cancellation, and truncation remain visible.
Repository output is delimited as lower-priority project knowledge.
