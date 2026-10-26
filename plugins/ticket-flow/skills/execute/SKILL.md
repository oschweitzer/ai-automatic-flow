---
name: execute
description: Use when an approved ticket-flow plan has to be implemented in its worktree and pass the repository's local checks.
argument-hint: "<JIRA-KEY>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh *)
---

# Execute

The ticket key is the first argument: `$0`. Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. `plan.json` must exist and be approved.

## 1. Worktree

Create the worktree at the path the conventions give, on a new branch cut from the base branch. A worktree does not start on the current HEAD, so name the start point:

```bash
git fetch origin <base>
git worktree add -b <branch> <worktree> origin/<base>
```

If the worktree already exists on that branch, reuse it. Copy the git-ignored env files the tests need (`.env`, `.env.local`, `.env.test`) from the main checkout. Run the profile's `install` command in the worktree.

## 2. Implement

Launch `ticket-flow:implementer`, or `ticket-flow:implementer-light` when `plan.json` says `"executor": "haiku"`. Give it the worktree path, the path of `plan.json`, the path of the profile, and `report.md` as its report. Add one line by type:

- bug: start with the test that reproduces the bug, and show it failing before the fix.
- feature: one test per acceptance criterion at least.
- technical: no behaviour change, the existing tests named in the plan must stay green.

## 3. Local checks

Run `bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh <worktree>`. It prints one line per check and only the tail of a failure.

On a failure, send the output to the same agent with SendMessage so it keeps its context, and ask for a fix. Two attempts, then write the end state `escalated_checks` with the failing check as the reason.

## 4. What the agent left

Run the steps listed under `left_to_orchestrator` in `report.md` yourself, in the worktree, one at a time: generating a migration, integration tests, anything that needs the database or a server. Commit their output on the branch. Stop every service you started.

## 5. Done

Log the end with the commits and the check results. Report in three lines: commits, checks, anything not tested.
