# Ticket-flow conventions

Every ticket-flow skill follows these rules. `<plugin>` below is the plugin root, two levels above the skill's own folder.

## Inputs

- The ticket key is the skill's first argument, for example `SHOP-142`.
- The repository profile is `.claude/ticket-flow.md` at the repository root. Read its frontmatter for commands, branches and Jira settings, and its body for the module map. If it is missing, stop and tell the user to run `/ticket-flow:profile`.
- The run folder is `.ticket-flow/runs/<KEY>/`. Make sure `.ticket-flow/` is listed in `.git/info/exclude`, so the run folder never shows in `git status` and the repository's `.gitignore` is not touched.

## Run folder

| File | Written by |
|------|------------|
| `ticket.md` | triage, a snapshot of the Jira ticket |
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

The run ends with a line whose `event` is `end_state` and whose `outcome` is one of: `ready_for_qa`, `split`, `parked_needs_info`, `plan_rejected`, `escalated_checks`, `escalated_ci`, `escalated_comments`, `timeout`, `failed_permission`. Never write a secret, a token or a full log into `log.jsonl`.

## Scripts

Run bundled scripts through `bash`, never by bare path:

- `bash <plugin>/scripts/forge.sh <command> ...` for every GitLab or GitHub call. Never call `glab` or `gh` directly.
- `bash <plugin>/scripts/checks.sh` for lint, type check and tests.

## Jira

Jira goes through the Atlassian MCP server. Tool names depend on how the server was added, so look for the operation: search with JQL, read an issue, add a comment, create an issue, link two issues.

Version 1 does not change Jira statuses. It only reads tickets and posts comments. Every Jira comment starts with `[ticket-flow]`.

## Forge comments

Every comment the flow posts on an MR starts with `_ticket-flow_` on its own line, because version 1 posts from the developer's own account.

## Limits

- Two attempts per failing local check.
- Two CI fix rounds.
- One fix attempt per review comment.
- Two comment rounds.

When a limit is reached, write the matching end state and stop. Never raise a limit on your own.

## Text written for humans

Plain language, short sentences, no filler. Jira comments and client steps in the profile's `client_language`, MR comments in `team_language`.
