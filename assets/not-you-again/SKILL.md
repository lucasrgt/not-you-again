---
name: not-you-again
description: Prevent recurrence of repository-specific mistakes with the nya CLI. Use for every task in a repository containing .nya, before changing tracked files, after correcting a real reusable failure, and before reporting implementation complete.
---

# Not You Again

1. Before changing tracked files, run `nya recall` with the current task and expected paths. Treat relevant scars as task constraints.
2. After correcting a real reusable failure, run `nya remember` exactly once. Never record hypotheses, preferences, general knowledge, or generic best practices.
3. After the final diff and repository checks are ready, run `nya check` before committing, pushing, or reporting completion.
4. Exit code 1 means a known scar recurred. Fix every confirmed recurrence and rerun.
5. Exit code 2 means the audit failed. Report the failure and never claim the gate passed.

Do not report a task complete until `nya check` exits with code 0.
