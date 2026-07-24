<!-- nya:instructions:start -->
## Not You Again

This repository uses Not You Again (`nya`) as a required recurrence-prevention gate for every task that changes tracked files.

1. Before editing, read `.nya/SKILL.md`, then run `nya recall` with the current task and expected paths. Treat every relevant scar as a constraint for the task.
2. Use `nya remember` only after a real failure has been corrected and its lesson is reusable. Never record hypothetical issues, preferences, or generic best practices.
3. After the final diff is ready and the repository's tests and checks pass, run `nya check` before committing, pushing, or reporting completion.
4. Do not report the task as complete until `nya check` exits with code 0.
5. Exit code 1 means a known scar was repeated. Fix every confirmed recurrence and run `nya check` again.
6. Exit code 2 means the audit could not be completed. Report the failure explicitly and never claim that the recurrence gate passed.
7. Tests, linters, and prior review do not replace `nya check`. Never skip the gate because the change appears small.
<!-- nya:instructions:end -->
