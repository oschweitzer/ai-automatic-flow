---
name: plan
description: Use when a triaged ticket-flow ticket needs its plan or its split proposal written and approved by the developer before any code is written.
argument-hint: "<JIRA-KEY>"
disable-model-invocation: true
---

# Plan and plan gate

The ticket key is the first argument: `$0`. Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. `triage.json` must exist in the run folder.

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

Post the same summary on the Jira ticket, starting with `[ticket-flow] Proposed plan`, in the client language.

## 4. Gate

Append a `waiting` line with `on: "developer"`, then ask the developer to approve, rework or stop. Wait for the answer. A rework round launches the planner again without a new start line. The next `waiting` line marks the next question. This gate is never skipped in version 1, even when the user asked to run everything.

| Answer | Do |
|--------|----|
| approve a plan | Log the end with outcome `approved`. |
| rework | Write the notes to `plan-notes.md` and go back to step 2. |
| stop | Write the end state `plan_rejected`. |
| approve a split | Create the parts on Jira (below), then write the end state `split`. |

## Creating the parts of a split

For each part, in order: create a Jira issue in the same project, as a sub-task of the story when the project allows it, otherwise as a task linked to the story. Use the part's title, its acceptance criteria as the description, and start the description with `[ticket-flow] Part <n> of <story key>`. Then comment on the story with the list of created keys.

If creating an issue fails, stop, list what was created, and write the end state `failed_permission`.

Tell the developer the keys of the new tickets. Each one goes through the flow on its own with `/ticket-flow:run <KEY>`.
