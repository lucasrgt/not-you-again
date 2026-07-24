# Real GitHub Review Recurrence Smoke

The source scar was created from a real line-level review on
[fixture PR #1](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1) using
[this exact comment](https://github.com/lucasrgt/nya-github-review-benchmark/pull/1#discussion_r3647349286).

| Arm | Outcome | Task complete | Isolation preserved | Recall observed | Host gate |
| --- | --- | --- | --- | --- | --- |
| baseline | incomplete | False | False | False | n/a |
| nya | incomplete | False | False | True | 0 |

Prevention evidence: **false**.

This one paired smoke proves or disproves the tested causal example only. It is
not a general prevention rate.
