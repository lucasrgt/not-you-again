Implemented `serialize_user` display-name support:

- Preserves the existing `username` field.
- Adds `display_name`, preferring the provided value and falling back to `username`.
- Manual behavior assertions and `git diff --check` passed.

`nya check` could not complete: its built-in judge reported a network-disabled sandbox (exit code 2), and no host-side NYA integration was available. Per repository policy, the change cannot be declared ready to commit until that gate runs successfully externally.