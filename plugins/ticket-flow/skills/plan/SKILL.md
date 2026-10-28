---
name: plan
description: Use when a triaged ticket-flow ticket needs its plan or its split proposal written and approved by the developer before any code is written.
argument-hint: "<TICKET> [--auto-approve]"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# Plan and plan gate

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. This session's ID is `${CLAUDE_SESSION_ID}`. `triage.json` must exist in the run folder.

## 1. Pick the planner's model

| Triage says | Model |
|-------------|-------|
| `decision` is `split` | `opus` |
| `size` is `small`, `risk` is empty, `ambiguities` is empty | `sonnet` |
| anything else | `opus` |

This rule is a first guess. Write the `plan` start line with `model` set to the chosen model, so the pilot can check it.

## 2. Write the plan

Launch `ticket-flow:planner` with the chosen `model` and the paths of `ticket.md`, `triage.json`, the profile and `plan.json`. On a rework round, also pass a file holding the developer's notes.

## 3. Show it

Summarise `plan.json` in the session, in this order:

- For a plan: the branch, the executor model, each file with its change, the tests, the configuration changes, the risks and what the plan does about them, the open points.
- For a split: one line per part (title, order, dependencies) and its acceptance criteria.

Post the same summary on the ticket, starting with `[ticket-flow] Proposed plan`, in the client language. With `tracker: github`, follow the Public issues section first. On a public repository, post the light version only after the gate below has approved the plan.

## 4. Gate

With `--auto-approve`, approve the plan without asking when it is a plan, not a split, and its `open_points` is empty: write no `waiting` line, log the end with outcome `approved` and `detail` `auto-approved`, and tell the developer in one line. Otherwise, or without the flag, ask as below. A split always waits, because approving it creates tickets. The public post approval of the Public issues section still applies.

Append a `waiting` line with `on: "developer"`, then ask the developer to approve, rework or stop. Wait for the answer. On a rework answer, append a new `plan` start line with `model` before launching the planner again, so the log no longer says the developer is the one blocking. The next `waiting` line marks the next question.

| Answer | Do |
|--------|----|
| approve a plan | Log the end with outcome `approved`. |
| rework | Write the notes to `plan-notes.md` and go back to step 2. |
| stop | Write the end state `plan_rejected`. |
| approve a split | Create the parts on the ticket's tracker (below), then write the end state `split`. |

## Creating the parts of a split

With `tracker: jira`, for each part, in order: create a Jira issue in the same project, as a sub-task of the story when the project allows it, otherwise as a task linked to the story. Use the part's title, its acceptance criteria as the description, and start the description with `[ticket-flow] Part <n> of <story key>`. Then comment on the story with the list of created keys.

With `tracker: github`, for each part, in order, after the Public issues check: `forge.sh issue-create-sub <story number> "<title>" <body-file>`, with the acceptance criteria as the body, starting with `[ticket-flow] Part <n> of #<story number>`. It prints the new issue number. If it exits 1 after printing a number, the issue exists but is not linked to the story: say so in the list. Then comment on the story with the list of `#<n>`. New keys are `GH-<n>`.

If creating an issue fails, stop, list what was created, and write the end state `failed_permission`.

Tell the developer the keys of the new tickets. Each one goes through the flow on its own with `/ticket-flow:run <KEY>`.
