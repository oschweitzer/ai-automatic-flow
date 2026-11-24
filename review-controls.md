# Review controls

Florian Bruniaux's talk at Dev With AI (October 2026), "Du code au merge", describes how his team organises the checks between a developer's machine and the merge decision when agents write most of the code. This note maps its ideas to ticket-flow: what the plugin already did, what version 0.4.0 adds, and what it leaves out.

The talk's own summary fits in five lines. Verification capacity is the bottleneck, not code generation. Review comments are proposals to check, diagnosis and fix separately. A fix loop has a goal, a budget and a stop condition, and hands over useful facts when it stops. A result belongs to a commit. Gains are judged on the whole work: human time, waiting, rework, quality and cost.

## What changed, idea by idea

| Idea from the talk | Before 0.4.0 | In 0.4.0 |
|--------------------|--------------|----------|
| Check on the machine before asking for a review | `checks.sh` ran lint, type check and tests before the push | Unchanged. The output is now saved to `checks.txt` so the MR quotes it |
| A PR a human and an agent can understand: the problem, the behaviour after, the checks that actually ran, the limits, with technical details folded | The MR body had the behaviour, tested and not tested | `open-mr` adds the problem, the local checks with their commit, and a folded "AI and details" section |
| Start the checks from what the change can break: a button must be visible and work, a shared component needs its pages tested end to end, access rights need the allowed and the refused case, in the UI and the API | The planner listed tests per file | The planner gets a table of observable checks by kind of change, and marks the ones that need a human as `manual` |
| Route the review by risk (labels "Review AI" and "Review Human" in the talk) | Risks were listed in the handover, nothing decided who reads | The handover picks `Review: human` or `Review: AI` from the final diff, with reasons, and adds one checklist line per reason. The route never allows a merge |
| Several reviewers, one native review always runs, an outside reviewer that is down must not block | `/code-review` always ran, the team bot's comments entered the loop | Unchanged, and now written down. The handover says which reviewer answered |
| Check each comment in the current code, merge duplicates, keep disagreements | The comment triager sorted into five categories and merged duplicates | The triager judges a comment against the current code, keeps contradictions as `human_required` and links them |
| A right diagnosis can come with a wrong fix (the talk's example: one "Refused" message for a missing document and a forbidden access) | One `fix` per actionable comment, taken from the comment | The triager checks the diagnosis and the suggestion separately (`suggestion: right, wrong, none`). With `wrong`, the fix follows the expected behaviour and the reply says why. The fix checker uses that as its reference, and checks the neighbouring cases |
| Diagnose a red CI before changing code: a test that found a bug is not a test that never started | Any red pipeline went to the implementer | `ci-gate` sorts the failure (`code`, `environment`, `expectation`, `missing`) and only `code` gets a code fix |
| Skipped, canceled or pending is not a success | GitHub read "all skipped" as success, and both forges treated "canceled" as a failure to fix | `forge.sh ci-status` reports both as `blocked`, a missing result. Covered by tests |
| A result belongs to a commit: green on A says nothing about B | Nothing recorded which commit was checked | `mr.json` records `ci_sha` and `reviewed_sha`. The review refuses to run on a commit CI has not passed. The handover lists the commits after the review and reruns CI if the last green pipeline is not on the final commit |
| Bounded fix loops with a hand-over that says what was tried, what is unknown and the next action | Limits and end states existed, the reason was one line | A stage that escalates or times out writes `escalation.md` with those five points, and the run report points to it |

## Left out on purpose

- Labels on the MR. The route is in the handover comment and in the `ready_for_qa` log line. A forge label would need a new `forge.sh` command and, on GitHub, a label created beforehand. Worth adding once the dashboard or a filter needs it.
- A plan critique by a second model before the plan gate. The human plan gate stays the control. It can be tried later and measured, as the talk itself suggests.
- Turning recurring review comments into tested checks, and the periodic analysis of fix reports. `comments.json` per run is the raw material. A skill that reads them across runs is the next step, with a human deciding what becomes a rule.
- Measuring the gain. The talk reports a speed-up without a measure that isolates the harness. The run log and `usage.sh` already give cost and waiting time per run. Comparing similar MRs before and after 0.4.0 is left to the pilot.

## Not verified

- None of the 0.4.0 changes has run on a real repository yet, like the rest of the plugin.
- The GitHub bucket names `pass`, `fail`, `pending`, `skipping` and `cancel` are checked against the `gh pr checks` manual (https://cli.github.com/manual/gh_pr_checks). How a real repository's skipped jobs show up has not been tried.

## Sources

- Florian Bruniaux, Dev With AI 2026 report: https://www.florian.bruniaux.com/reports/dev-with-ai-2026.html
- GitHub CLI manual, `gh pr checks`: https://cli.github.com/manual/gh_pr_checks
- Florian Bruniaux, "Du code au merge : organiser le travail des agents en équipe": https://www.florian.bruniaux.com/guides/du-code-au-merge/
