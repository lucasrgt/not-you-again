# Not You Again Engineering Guide

All repository artifacts must be written in English.

## Product contract

Not You Again exposes one public concept, the scar, and three agent actions:

1. `nya remember`
2. `nya recall`
3. `nya check`

Do not introduce antibodies, candidates, graduation states, generic memory
types, or deterministic checker definitions.

`nya check` is a scar-scoped recurrence audit. It is not a generic AI review.

## Engineering constitution

1. Production code under `src/` must remain at or below 500 code lines as
   measured by `tokei`.
2. Line coverage must remain at or above 95 percent without rounding.
3. Test code is unlimited and must live under `tests/`.
4. Production behavior may not be moved into scripts, generated files,
   integrations, or test helpers to evade the line budget.
5. Git is the durable source of truth for scars.
6. SQLite is a disposable projection with no unique knowledge.
7. CLI and MCP must call the same core operations.
8. Judge failure must fail closed.

## Change discipline

Prefer the smallest complete implementation. Add a dependency or abstraction
only when it removes more maintained behavior than it introduces.

Before reporting implementation work complete, run the production line gate,
the complete test suite, and the coverage gate.
