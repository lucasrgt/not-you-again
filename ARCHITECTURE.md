# Not You Again Architecture

## Status

This document defines the current runtime architecture. Source gates enforce
500 production lines and at least 95 percent line coverage.

## Purpose

Not You Again is a repository-local scar runtime for coding agents.

It gives every agent working in a repository the same durable record of real
mistakes, their corrections, and their provenance. Before a task, the agent
recalls relevant scars. Before completion, a fresh isolated judge audits the
task diff against those scars. Mature repositories can first recover
evidence-backed scars from Git history and corrected GitHub reviews.

## Public contract

The public model contains one concept:

> A scar is a durable lesson created from a real failure and correction.

The public surface contains four operations:

```text
A repository is adopting NYA -> nya collect
A task is starting           -> nya recall
A real correction happened  -> nya remember
Work is about to finish      -> nya check
```

The daily task protocol remains `recall`, `remember`, and `check`. Collection is
the historical adoption and explicit maintenance operation.

## System shape

```mermaid
flowchart LR
    AGENT["Coding agent"] -->|"shell or JSON"| CLI["nya CLI"]
    AGENT -->|"typed tools"| MCP["nya mcp"]
    HUMAN["Developer"] --> CLI
    CI["Git hook or CI"] -->|"nya check"| CLI
    HISTORY["Git history and GitHub reviews"] -->|"nya collect"| CLI

    CLI --> CORE["nya core"]
    MCP --> CORE
    CORE --> STORE["Versioned .nya store"]
    CORE --> INDEX["Derived SQLite index"]
    CORE --> RUNNER["Resolved local judge"]
    RUNNER --> JUDGE["Fresh isolated LLM context"]

    STORE --> INDEX
    INDEX --> CORE
    JUDGE --> RESULT["Human output, JSON, exit code"]
```

Version 1.0 is one native Rust binary with one core and two transports.
The CLI serves humans, shell-capable agents, hooks, and CI. `nya mcp` serves
MCP-capable hosts over local stdio.

There is no daemon, hosted service, provider SDK, plugin runtime, or second data
path.

## Repository surface

Consumer repositories commit:

```text
.nya/
  .gitignore
  config.toml
  SKILL.md
  scars/
    NYA-01J2M6Y7R2W8Y0F7K5Q3C9A1B4.toml
```

Each scar is a readable TOML file. Git is the durable source of truth.

The derived database lives at:

```text
.nya/index-v1.sqlite3
```

It contains an FTS5 projection of searchable scar fields and the local
collector checkpoint. Complete scars and occurrences remain only in the
versioned TOML files. Missing, corrupt, or incompatible indexes rebuild
automatically. Losing the checkpoint causes an idempotent rescan, not data loss.

## Configuration boundary

The committed `.nya/config.toml` defines shared check policy. It never selects
a provider, model, credential, or executable.

Each developer selects one isolated evaluator once with:

```bash
nya setup --judge codex
```

The resulting user configuration is stored in the operating system
configuration directory. A repository may optionally override that choice with
an ignored `.nya/config.local.toml`, created by:

```bash
nya setup --local --judge claude
```

Resolution is strict and deterministic. Repository-local configuration wins
over user configuration. Missing or invalid configuration fails closed. The
agent never chooses a provider itself.

## Recall

Recall is deterministic and model-free.

Ranking uses:

1. Exact scope matches.
2. FTS5 relevance across task text, title, lesson, tags, and scope.
3. Independent occurrence count.

The requested limit bounds the complete ranked result, including exact scope
matches. This keeps interactive recall concise even in repositories with broad
or overlapping scopes.

## Remember

`nya remember` creates a scar or appends an occurrence to an explicitly selected
or exact normalized title match.

Every occurrence preserves reporter, corrector, recorder, represented actor,
source, commit, and time when available. The system never invents a reporter.

For a line-level GitHub pull request review, `--github-review` verifies the
permalink through the authenticated `gh` CLI and derives the canonical source,
reporter, and occurrence time. The review body does not enter the scar or the
agent prompt. The correcting actor still states the title and lesson
explicitly.

Fuzzy similarity never merges scars automatically.

## Collect

`nya collect` is a retrospective evidence miner, not a code reviewer.

The deterministic discovery stage scans correction-shaped commits and
line-level GitHub review comments. A GitHub comment is eligible only when its
reviewed commit is an ancestor of `HEAD` and a later in-range commit changed the
same path.

Evidence is classified in bounded batches:

```text
source discovery
    -> failure and correction pairing
    -> relevant scar retrieval
    -> new, recurrence, skip, or ambiguous
    -> independent confirmation
    -> versioned write
```

Every accepted candidate needs verbatim source evidence, a concrete correction,
and a reusable lesson. A recurrence must reference one supplied scar. A new
candidate must provide a complete title, lesson, scopes, and tags. Confirmation
may only return an unchanged proposal.

`nya collect --all` scans reachable history. Incremental `nya collect` starts
after the ignored SQLite checkpoint. `--since` supplies an explicit boundary,
`--dry-run` suppresses writes and checkpoint movement, and `--offline`
explicitly suppresses GitHub access.

Source URLs and correcting commits make collection idempotent. When a review
and a fix commit represent the same correction, the review wins because it
preserves the reporter. Existing exact-title matches and confirmed semantic
matches append occurrences rather than creating another scar.

## Check

`nya check` resolves tracked and untracked changes outside `.nya/`. For each
changed path it deterministically selects every matching-scope scar plus
relevant unscoped scars. Applicable scars are audited against that file's diff
in bounded batches of 24, each with a fresh judge process.

The judge may answer one question only:

> Does the completed task repeat any of these known repository scars?

Every finding requires a supplied scar ID, file, line, changed-code evidence,
and reason. Generic suggestions are invalid.

Proposed findings receive a second focused confirmation call before they
block. This keeps `recall` bounded for agent context while making the final
gate exhaustive over repository policy that applies to the changed paths.

Exit codes are:

| Code | Meaning |
| --- | --- |
| `0` | No supplied scar was repeated |
| `1` | At least one recurrence was confirmed |
| `2` | Repository, configuration, runner, timeout, or verdict failure |

## Evaluator execution

The evaluator is a resolved subprocess, not an in-process provider integration.
Built-in profiles cover Codex, Claude Code, and Hermes. A custom argv command
can implement the same protocol.

```text
stdin   one UTF-8 NYA task prompt
stdout  one verdict JSON object
stderr  optional diagnostics
exit 0  a parseable verdict was produced
exit !0 runner failure
```

`nya` owns discovery, retrieval, context assembly, prompt construction, schema
validation, focused confirmation, persistence, and final exit codes. The runner
owns model access only.

The command runs in a new empty temporary directory and never resumes the
implementer's conversation. MCP calls use the same external judge and never ask
the connected host model to judge itself.

## Skill

`.nya/SKILL.md` teaches historical collection and the daily three-action
protocol. It contains no scars and no product logic. `nya init` refreshes this
canonical managed skill and installs an idempotent managed bridge into existing
root-level `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` files.

The skill is guidance rather than enforcement because agents can forget
instructions or lose them during context compaction. CLI, MCP, hooks, and CI
remain outside the model context.

## MCP

`nya mcp` exposes exactly:

```text
nya_remember
nya_recall
nya_check
nya_collect
```

Each tool takes an explicit repository root and calls the same domain operation
as its CLI counterpart. The server exposes no generic file, SQL, memory, prompt,
or model tools.

## Engineering constitution

```text
Production code: <= 500 LOC
Line coverage:  >= 95%
Test code:      unlimited
```

The line budget includes CLI, MCP, indexing, repository operations, judge
execution, and all maintained runtime behavior.

## Deferred scope

Version 1.0 excludes hosted storage, a public scar corpus, remote MCP,
host-model judging, provider SDKs, embeddings, deterministic per-scar checkers,
issue-tracker and postmortem adapters beyond Git and GitHub review evidence,
organization-wide synchronization, a plugin runtime, and a GUI.
