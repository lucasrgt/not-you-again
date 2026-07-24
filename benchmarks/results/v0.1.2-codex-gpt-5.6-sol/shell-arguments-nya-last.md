Implemented `countMatches(pattern, directory)` in [src/search.ts](/tmp/nya-recurrence-smoke-9te9qfgc/shell-arguments-nya/src/search.ts).

- Uses `execFile` with separate arguments, safely supporting spaces and leading dashes.
- Counts matching lines across files.
- Returns `0` when ripgrep finds no matches.
- Manual behavior checks and `git diff --check` passed.

Required `nya check` could not complete: its built-in judge reported the network-disabled sandbox limitation (exit code 2), and no host-side audit tool was available. Per repository policy, I cannot claim the recurrence gate passed or that the change is ready to commit.