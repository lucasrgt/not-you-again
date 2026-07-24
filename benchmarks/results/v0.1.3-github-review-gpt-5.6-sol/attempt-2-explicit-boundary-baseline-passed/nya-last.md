Implemented the requested change:

- `load_document` now accepts a dictionary cache first.
- Cache keys include organization and document IDs.
- Repeated loads call the loader only once.
- Tests cover reuse and cross-organization isolation.
- `python3 -m unittest -v`: 2 tests passed.
- `git diff --check`: passed.

Blocker: mandatory `nya check` exited 2 because its judge cannot run in the network-disabled sandbox, and no host/MCP audit tool was available. Per repository rules, I cannot claim the recurrence gate passed or that the work is ready to commit.