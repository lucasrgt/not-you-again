Implemented and ready to commit.

- `load_document` now accepts a dictionary cache as its first argument.
- Documents are cached by document ID.
- Repeated loads invoke the loader only once.
- Tests updated and caching behavior covered.
- `python3 -m unittest -v`: 2 tests passed.
- `git diff --check`: clean.