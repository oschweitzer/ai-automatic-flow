---
name: parallel-tickets
description: Use when the user asks to implement several tickets, issues or features at once or in parallel ("implémente ces tickets en parallèle", "lance ces 5 tickets"), or invokes /parallel-tickets.
---

# Parallel tickets

One shared analysis in this session, then one agent per lot, each in its own worktree. A lot that needs the analysis runs as a fork, which reuses this session's prompt cache instead of exploring again. A lot whose brief can be written in full runs as a fresh agent on Sonnet.

## 1. Analyse once, in this session

Do the analysis yourself, not in a subagent: forks inherit what this session has read. For each ticket, read it (Jira, issue or the user's text), then find the files it touches, the helpers to reuse, the tests that cover it, and whether it is already done.

Group tickets into lots. Tickets that touch the same files, or depend on each other, share a lot. A lot that needs another lot's output waits for it. A ticket whose code lives in another repository gets no lot: flag it in the table.

For each lot, list the steps that need a shared service, like generating a migration against the database. The agent leaves those to you.

### Decide: inline or parallel

Every agent rebuilds its own context and every reviewer reads the diff again, so going parallel costs tokens. It only pays off when lots really run at the same time and are big enough to save exploration in this session. Count the lots that can start right away, with no dependency on another lot.

- **inline** when fewer than two lots can start right away, or when you can already write every change yourself (the analysis read all the files, and the whole job is a few hundred changed lines at most). Implement the tickets yourself in this session, on one branch cut from the base branch, with one commit per ticket. Then spawn one cold reviewer (step 3) for the whole diff, and fix its real findings yourself. Skip steps 2 and 4.
- **parallel** when at least two lots can start right away and each still needs reading or design work the analysis did not cover. Continue with the mode of each lot below.

On four small overlapping tickets, parallel cost 1.8 times as much as inline with one reviewer and took twice as long, for the same result. The single reviewer cost a seventh of the inline run. The user can force either mode. Forcing parallel keeps the lots as grouped above, so tickets that share files still share a lot. Put the choice and its reason on the first line of the plan.

In parallel, pick each lot's mode with this rule:

- **sonnet**: you can write the whole brief, naming every file to change, the helper to reuse, the expected behaviour and the test to add or run.
- **fork**: otherwise, when the work depends on what the analysis found without writing it down, or needs design judgement. A fork always runs on this session's model, so it runs on Opus if the session does. When every lot is simple, the user can switch the session to Sonnet (`/model sonnet`) before starting, and the forks follow.

Find the base branch features start from (CLAUDE.md, memory, or ask). A worktree starts on the repo's default branch, not on the current HEAD.

Show the user the plan in your reply: this table, then one short section per lot (files, helpers to reuse, expected behaviour, tests, steps left to you). Wait for their go, even when the request said "implement".

| Lot | Tickets | Files | Mode | Branch |
| --- | ------- | ----- | ---- | ------ |

## 2. Launch every lot in one message

Right after the go, send all Agent calls in a single message, so the forks read the cache while it is warm.

| Mode   | subagent_type     | model    | isolation  |
| ------ | ----------------- | -------- | ---------- |
| fork   | `fork`            | omit it  | `worktree` |
| sonnet | `general-purpose` | `sonnet` | `worktree` |

Each prompt has these parts, in this order:

1. The lot. Fork: "You are lot N of the plan above." Sonnet: that lot's section of the plan, pasted in full.
2. Setup: `git fetch origin <base>`, `git switch -c <branch> --no-track origin/<base>`, then install dependencies, and copy the git-ignored env files from `<main checkout>` if the tests need them.
3. Scope: change the lot's files, plus what the repo's conventions require alongside them (docs, shared types, changelog). Run unit tests and type checks only, nothing that uses a shared service (database, fixed port, dev server). Leave the steps that need one and name them in the report. Commit locally following the repo's conventions, never push.
4. Retry cap: after two failed attempts at the same test, stop and report the failure.
5. Report, five lines at most: commits, tests run with their result, what was not tested, steps left to you, risks.

## 3. Review cold

For each finished lot, spawn one reviewer: `general-purpose`, model `sonnet`, no isolation since it only reads, and never a fork, since a fork shares the author's assumptions. Give it the ticket text and the branch names, and ask it to review `git diff origin/<base>...<branch>` against the ticket, with ranked findings in ten lines at most. A lot whose diff changes fewer than 20 lines gets no reviewer: read the diff yourself.

Send real findings to the lot's agent with SendMessage for one fix round. If the second review still fails, park the lot and report why.

## 4. Finish here

- Run the steps the agents left, and whatever needs a shared service (integration tests, migrations), yourself in each worktree, one branch after another. Commit them on the lot's branch, then stop anything you started.
- Report per lot: branch, worktree path, commits, tests run and not run, review outcome, and the push command for the user. Do not push.

## Common mistakes

| Mistake                                         | What it costs                                        |
| ----------------------------------------------- | ---------------------------------------------------- |
| Parallel when lots run one after another        | Several times the tokens and time, same result       |
| Fresh agent for a lot that needs the analysis   | It explores from zero, the cost this skill avoids    |
| `model` set on a fork                           | Ignored, the fork runs on the session's model        |
| Lots launched one by one, or after a long pause | Cold cache, each fork pays full input price          |
| Fork as reviewer                                | It agrees with the author                            |
| Integration tests inside the agents             | Agents collide on the shared database and ports      |
| Assuming the worktree starts on the base branch | The branch is cut from the default branch instead    |
