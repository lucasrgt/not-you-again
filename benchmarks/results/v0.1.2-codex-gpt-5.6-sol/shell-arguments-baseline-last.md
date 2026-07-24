Implemented `countMatches(pattern, directory)` in `src/search.ts`.

- Invokes `rg` safely with separate arguments, supporting spaces.
- Counts matching lines across files.
- Returns `0` when no matches exist.
- Propagates other ripgrep errors.
- `git diff --check` passes; shell smoke test passed.