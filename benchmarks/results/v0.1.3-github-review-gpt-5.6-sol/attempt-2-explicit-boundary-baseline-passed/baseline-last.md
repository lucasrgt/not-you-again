Implemented and ready to commit.

- `load_document` now accepts a dictionary cache as its first argument.
- Cache keys combine organization and document IDs.
- Repeated loads call the loader only once.
- Updated tests confirm cache reuse.

Checks passed: 2 unit tests, compilation, and `git diff --check`.