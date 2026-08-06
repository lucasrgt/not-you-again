# Prime Agent integration

The optional package at `integrations/prime-agent` wraps the standalone `nya`
CLI without reading `.nya` records or reproducing Rust semantics.

Install it after placing `nya` on `PATH`:

```bash
prime-agent package install /absolute/path/to/not-you-again/integrations/prime-agent
```

Run `/reload` in an active Prime session. The adapter activates only when the Git
root contains `.nya/SKILL.md`. A root `csm.toml` always suppresses it;
CSM then owns Prime retrieval and checks while the standalone CLI stays usable.

The adapter exposes `/nya status`, `/nya recall`, explicit `/nya check`,
and a session-only `/nya auto recall on|off` toggle. Automatic
`recall` defaults to on and can be disabled at launch with
`--nya-auto-recall off`. It never exposes repository adoption or semantic
record mutation commands.

All subprocesses use literal argv, the Git root as cwd, cancellation, a timeout,
and a 64 KiB UTF-8 output cap. Nonzero exits, killed processes, and truncation
remain explicit. Injected output is delimited as repository knowledge rather
than higher-priority instructions.
