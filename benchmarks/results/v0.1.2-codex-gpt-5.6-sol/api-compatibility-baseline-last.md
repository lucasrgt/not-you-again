Implemented preferred display name support in `serialize_user`:

- Returns `display_name` when provided.
- Falls back to `username` when absent.
- Preserves the existing `username` field.

Focused assertions, compilation, and `git diff --check` all pass. Ready to commit.