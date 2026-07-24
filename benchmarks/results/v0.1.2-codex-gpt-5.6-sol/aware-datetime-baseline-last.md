Implemented `is_expired(expires_at: str)` in [src/lease.py](/tmp/nya-recurrence-smoke-9te9qfgc/aware-datetime-baseline/src/lease.py).

It parses ISO 8601 timestamps, including `Z` and explicit UTC offsets, and compares them against the current UTC time.

Validation passed:

- Python compilation
- Past/future timestamp checks
- `Z` timestamp check
- `git diff --check`