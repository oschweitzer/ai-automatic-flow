---
name: ci-gate
description: Use when a ticket-flow merge request is waiting for its CI pipeline, or the pipeline is red and needs a targeted fix.
argument-hint: "<TICKET>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# CI gate

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. This session's ID is `${CLAUDE_SESSION_ID}`. `mr.json` must exist. `forge.sh` below means `bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh`.

## Wait

Append a `waiting` line with `on: "ci"`. After a fix is pushed and the skill waits again, append another.

Wait for the pipeline with the Monitor tool, which costs nothing while CI runs. Poll every 60 seconds until the status is not `running`, with a 30-minute deadline:

```bash
while s=$(bash "${CLAUDE_SKILL_DIR}/../../scripts/forge.sh" ci-status <mr>) && [ "$s" = running ]; do sleep 60; done; echo "CI_STATUS $s"
```

A pipeline can take a few seconds to appear after a push, so treat `none` right after a push as `running` for two minutes.

| Status | Do |
|--------|----|
| `success` | Write `ci_sha` in `mr.json` (see the conventions, Results belong to a commit). Log the end with outcome `green`. |
| `none` | The repository has no CI. Write `ci_sha` all the same, then log the end with outcome `no_ci`, so the handover says so. |
| `blocked` | No result: a manual job, a skipped or a canceled pipeline. It is not a success, and not a bug to fix in the code. Tell the user, then write the end state `escalated_ci` with that reason. |
| `failed` | Diagnose it, then fix it if the diagnosis says so, below. |
| deadline reached | Write the end state `timeout`. |

## Fix a red pipeline

1. If `ci_rounds` in `mr.json` is already 2, write the end state `escalated_ci`, with the failed jobs' names, and stop.
2. Run `forge.sh ci-failed-log <mr> 80` and save it to `ci-failure.txt` in the run folder.
3. Read the log before touching the code. A red cross covers different cases, and only one of them calls for a code change:

   | The log shows | Diagnosis | Do |
   |---------------|-----------|----|
   | a test ran and found a wrong result | `code` | step 4 |
   | a test or a job did not start: missing dependency, service, variable, runner | `environment` | If the change itself broke the environment (a lockfile, a CI file, a new variable the plan lists), step 4 on that file only. Otherwise no fix: the behaviour has not been tested yet. Escalate. |
   | a test ran but its expected result is the thing in doubt | `expectation` | No fix: the business rule needs a human. Escalate. |
   | no log, or a log that does not say why | `missing` | No fix. Escalate and say which result is missing. |

   Write the diagnosis and the line of the log that shows it as the first line of `ci-failure.txt`. To escalate, write the end state `escalated_ci` with the diagnosis as the reason.
4. Launch `ticket-flow:implementer` on the worktree with `ci-failure.txt` as its task and `ci-fix-report.md` as its report. Ask for the smallest fix that makes the failing job pass, without weakening the test.
5. Run `checks.sh <worktree>`, push the branch, add one to `ci_rounds` in `mr.json`, and wait again.

A failure that the change cannot have caused (a runner crash, a network error, a job that also fails on the base branch) is an `environment` failure too: tell the user, write the end state `escalated_ci`, and name the cause.
