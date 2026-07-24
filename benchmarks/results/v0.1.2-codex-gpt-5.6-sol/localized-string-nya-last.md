Implemented the minimal change:

- Added `"session.expired"` to `src/messages.ts` with exact text: `Your session expired. Sign in again.`
- Exported `sessionExpiredMessage` from `src/session.ts`.
- `git diff --check` passes.

Blocker: mandatory `nya check` exited with code 2 because its judge cannot run in the network-disabled sandbox, and no host-side NYA endpoint is available. Per repository rules, I cannot claim the recurrence gate passed or that the work is ready to commit.