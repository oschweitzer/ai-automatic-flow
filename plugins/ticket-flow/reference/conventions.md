# Ticket-flow conventions

Every ticket-flow skill follows these rules. `<plugin>` below is the plugin root, two levels above the skill's own folder.

## Inputs

- The repository profile is `.claude/ticket-flow.md` at the repository root. Read its frontmatter for commands, branches and tracker settings, and its body for the module map. If it is missing, stop and tell the user to run `/ticket-flow:profile`.
- The ticket key is the output of `bash <plugin>/scripts/ticket-key.sh "<first argument>"`, run first. On exit 1, show its message and stop. A Jira key is kept as typed, for example `SHOP-142`. A GitHub issue typed `142`, `#142` or `GH-142` becomes `GH-142`. `<KEY>` everywhere below means that output. For GitHub, the issue number is `<KEY>` without `GH-`.
- The run folder is `.ticket-flow/runs/<KEY>/`. Make sure `.ticket-flow/` is listed in `.git/info/exclude`, so the run folder never shows in `git status` and the repository's `.gitignore` is not touched.

## Run folder

| File | Written by |
|------|------------|
| `ticket.md` | triage, a snapshot of the ticket |
| `triage.json` | triage |
| `plan.json` | plan |
| `report.md` | execute |
| `review.txt`, `findings.json` | review |
| `mr.json` | open-mr, then ci-gate and mr-loop |
| `comments.json` | mr-loop, one entry per comment handled |
| `handover.md`, `client-steps.md`, `assets/` | handover |
| `log.jsonl` | every skill |

Agents get file paths, never pasted file content.

## Worktree

The code changes happen in one git worktree per ticket, outside the repository so that the repository's own tools never scan it: `<repository parent>/<repository name>.worktrees/<KEY>`. The path is derived, never stored. Every agent that changes code for the ticket works there, on the branch named in `plan.json`.

## Agents

The plugin's agents are `ticket-flow:triager`, `ticket-flow:planner`, `ticket-flow:implementer`, `ticket-flow:implementer-light`, `ticket-flow:comment-triager` and `ticket-flow:fix-checker`. Pass the `model` parameter only when a skill says to override the agent's own model. Launch an agent in the background only when a skill says so. Otherwise wait for its one-line answer.

## Log

Each skill appends one JSON line to `log.jsonl` when it starts and one when it ends:

```json
{"at":"2026-10-26T10:12:00Z","stage":"triage","event":"end","outcome":"continue"}
```

Lines are appended, never rewritten. `at` is UTC, RFC 3339 with `Z`. `stage` is one of `triage`, `plan`, `execute`, `open-mr`, `ci-gate`, `review`, `mr-loop`, `handover`. `event` is one of these:

| `event` | Written when | Extra fields |
|---------|--------------|--------------|
| `start` | the stage starts | `model` on `plan` and `execute`: `haiku`, `sonnet` or `opus` |
| `end` | the stage ends without ending the run | `outcome` |
| `waiting` | the stage starts waiting on someone else | `on`: `developer` (plan gate), `ci` (ci-gate), `reviews` (mr-loop waiting for the review bot) |
| `end_state` | the run ends | `outcome`, see below |
| `published` | `handover --publish` posted the client steps | none |

The next `start`, `end` or `end_state` line closes a wait. A `start` line after an `end_state` reopens the run: a run has ended only when its last `end_state` line comes after its last `start` line.

The run ends with a line whose `event` is `end_state` and whose `outcome` is one of: `ready_for_qa`, `split`, `parked_needs_info`, `plan_rejected`, `escalated_checks`, `escalated_ci`, `escalated_comments`, `timeout`, `failed_permission`.

Any line may carry `detail`, a string of 200 characters at most, for counts or a URL, for example `3 commits, checks passed`. Never put a comment body, a check output or a CI log in it. A line carries no field beyond the ones above: when a skill says to log something with an end, it goes in `detail`. Never write a secret, a token or a full log into `log.jsonl`.

The ticket-flow dashboard reads this file. `tests/fixtures/log-contract.jsonl` is a full example run and `tests/test-log-contract.sh` checks a log against these rules. Changing the contract means changing that fixture and the dashboard together.

## Scripts

Run bundled scripts through `bash`, never by bare path:

- `bash <plugin>/scripts/forge.sh <command> ...` for every GitLab or GitHub call. Never call `glab` or `gh` directly.
- `bash <plugin>/scripts/checks.sh` for lint, type check and tests.
- `bash <plugin>/scripts/ticket-key.sh <ticket>` for the ticket key, see Inputs.

## Tracker

The profile's `tracker` says where tickets live: `jira` (the default when the key is missing) or `github`, for GitHub issues in the same repository. `tracker: github` needs `forge: github`.

| Operation | Jira | GitHub |
|-----------|------|--------|
| read a ticket | Atlassian MCP | `forge.sh issue-read <n>` |
| comment on a ticket | Atlassian MCP | `forge.sh issue-comment <n> <body-file>`, after the Public issues check |
| create a part of a split | Atlassian MCP (sub-task, or task linked to the story) | `forge.sh issue-create-sub <parent> <title> <body-file>`, after the Public issues check |
| link to the ticket from the MR | `Jira: <ticket URL>` line | `Issue: #<n>` line, never a closing keyword |

Jira goes through the Atlassian MCP server. Tool names depend on how the server was added, so look for the operation: search with JQL, read an issue, add a comment, create an issue, link two issues.

`issue-read` drops comments from people without write access, except the issue author's, and counts them in `dropped_comments`.

Version 1 changes no status. With `tracker: jira` it only reads tickets, posts comments and creates the parts of a split. With `tracker: github` it never touches labels, assignees, milestones, issue types, and never closes or reopens an issue. Every ticket comment starts with `[ticket-flow]`.

## Public issues

With `tracker: github`, before any issue comment or part creation, run `forge.sh visibility`. With `private` or `internal`, post as with Jira. With `public`, anyone can read the issue, so two rules apply. Any other output, or a failure of the command, counts as `public`.

The text is a light version. It never contains file paths or code, the risks of the plan or anything that describes a weakness, test environment URLs, account names or the file that holds them, internal hostnames or IP addresses, logs or stack traces, or the names of people other than the issue's author.

| Post | Light version |
|------|---------------|
| triage questions | the questions, without file or code references |
| plan summary | the expected behaviour in a few sentences, and the open points the client can answer. No files, tests, configuration or risks |
| part of a split | title and acceptance criteria, checked against the list above |
| comment listing the parts | unchanged, issue numbers only |
| client test steps | the steps, with "the test environment" in place of its URL and "your usual test account" in place of account details |

The developer approves the exact text first. Write it to `public-<post>.md` in the run folder (`<post>` is `questions`, `plan`, `part-<n>` or `client-steps`), show it in full, and ask: post, edit or skip. On edit, take the developer's version and ask again. On skip, post nothing, add `public post skipped: <post>` to the `detail` of the stage's next log line, and go on. A skipped `questions` post still parks the run: tell the developer the client has not been asked. A skipped `client-steps` post writes no `published` line: tell the developer the steps are still unpublished. This approval comes on top of the plan gate. For the plan, the developer approves the plan first, then the public text.

On a public GitHub repository the MR is public too, whatever the tracker. The handover comment on the MR, including the client steps draft it embeds, leaves out the test environment URL and account details, and keeps its other sections.

## Forge comments

Every comment the flow posts on an MR starts with `_ticket-flow_` on its own line, because version 1 posts from the developer's own account.

## Limits

- Two attempts per failing local check.
- Two CI fix rounds.
- One fix attempt per review comment.
- Two comment rounds.

When a limit is reached, write the matching end state and stop. Never raise a limit on your own.

## Text written for humans

Plain language, short sentences, no filler. Ticket comments and client steps in the profile's `client_language`, MR comments in `team_language`.
