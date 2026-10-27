---
name: ci-gate
description: Use when a ticket-flow merge request is waiting for its CI pipeline, or the pipeline is red and needs a targeted fix.
argument-hint: "<JIRA-KEY>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh *)
---

# CI gate

The ticket key is the first argument: `$0`. Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. `mr.json` must exist. `forge.sh` below means `bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh`.

## Wait

Append a `waiting` line with `on: "ci"`. After a fix is pushed and the skill waits again, append another.

Wait for the pipeline with the Monitor tool, which costs nothing while CI runs. Poll every 60 seconds until the status is not `running`, with a 30-minute deadline:

```bash
while s=$(bash "${CLAUDE_SKILL_DIR}/../../scripts/forge.sh" ci-status <mr>) && [ "$s" = running ]; do sleep 60; done; echo "CI_STATUS $s"
```

A pipeline can take a few seconds to appear after a push, so treat `none` right after a push as `running` for two minutes.

| Status | Do |
|--------|----|
| `success` | Log the end with outcome `green`. |
| `none` | The repository has no CI. Log the end with outcome `no_ci`, so the handover says so. |
| `blocked` | A manual job or a skipped pipeline. Tell the user, then write the end state `escalated_ci` with that reason. |
| `failed` | Fix it, below. |
| deadline reached | Write the end state `timeout`. |

## Fix a red pipeline

1. If `ci_rounds` in `mr.json` is already 2, write the end state `escalated_ci`, with the failed jobs' names, and stop.
2. Run `forge.sh ci-failed-log <mr> 80` and save it to `ci-failure.txt` in the run folder.
3. Launch `ticket-flow:implementer` on the worktree with `ci-failure.txt` as its task and `ci-fix-report.md` as its report. Ask for the smallest fix that makes the failing job pass.
4. Run `checks.sh <worktree>`, push the branch, add one to `ci_rounds` in `mr.json`, and wait again.

A failure that the change cannot have caused (a runner crash, a network error, a job that also fails on the base branch) gets no fix. Tell the user, write the end state `escalated_ci`, and name the cause.
