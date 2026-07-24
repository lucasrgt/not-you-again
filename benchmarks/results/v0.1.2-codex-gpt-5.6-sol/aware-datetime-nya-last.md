Implemented `is_expired()` in [src/lease.py](/tmp/nya-recurrence-smoke-9te9qfgc/aware-datetime-nya/src/lease.py:8).

It supports:

- `Z` timestamps
- Explicit UTC offsets
- Missing offsets, interpreted as UTC
- Aware UTC comparisons

Compilation, timestamp checks, and `git diff --check` passed.

Blocking gate: `nya check` exited 2 because its judge cannot run in the network-disabled sandbox, and no host-side recurrence tool was available. Per repository policy, I cannot claim the recurrence gate passed or that the change is ready to commit.