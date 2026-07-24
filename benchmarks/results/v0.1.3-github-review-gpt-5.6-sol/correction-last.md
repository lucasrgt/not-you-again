Addressed the sole unresolved line-level review comment on PR #1.

- Cache keys now include `tenant_id`.
- Added cross-tenant isolation coverage for identical user IDs.
- Recorded the [review discussion](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1#discussion_r3647349286) in `.nya/scars`.
- `python -m unittest -v`: 2 tests passed.
- `nya check`: passed, 1 scar checked.
- No commit or push performed.