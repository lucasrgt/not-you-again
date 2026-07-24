Implemented the cache-first `load_document` API.

- Caches by `(org_id, document_id)` to preserve tenant isolation.
- Repeated loads invoke the loader only once.
- Added caching and cross-organization regression tests.
- All 3 tests pass; `git diff --check` passes.

Blocking gate: `nya check` exited 2 because the built-in judge cannot run in the network-disabled sandbox, and no host/MCP audit tool was available. Per repository policy, I cannot claim the recurrence gate passed or that the change is ready to commit.