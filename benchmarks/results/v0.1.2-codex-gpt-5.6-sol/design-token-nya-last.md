Implemented the destructive button tone:

- Added `"destructive"` while preserving the existing `buttonStyle(tone)` API.
- Added semantic destructive background/foreground tokens.
- Red `#b42318` with white text provides a 6.57:1 contrast ratio.
- `git diff --check` passes.

The required `nya check` could not complete: its host judge is unavailable inside this network-disabled sandbox (exit code 2), and no host/MCP audit endpoint is available. Per repository policy, I cannot claim the recurrence gate passed or that the change is ready to commit.