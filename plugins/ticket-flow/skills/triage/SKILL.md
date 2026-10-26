---
name: triage
description: Use when a Jira ticket is about to enter ticket-flow and has to be checked for an existing run, classified and judged ready or not.
argument-hint: "<JIRA-KEY>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *)
---

# Triage

The ticket key is the first argument: `$0`. Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first.

## 1. Already running?

These checks cost nothing. Run them before reading the ticket.

- The run folder `.ticket-flow/runs/<KEY>/` exists: read the last lines of its `log.jsonl` and tell the user where the run stopped. Offer to resume from that stage with `/ticket-flow:run <KEY> --resume`. Stop.
- A remote branch carries the key (`git ls-remote --heads origin | grep -i "/<KEY>-"`): run `forge.sh find-mr <branch>`. If an MR is open, tell the user and stop, since the flow keeps one MR per ticket.

## 2. Snapshot the ticket

Read the Jira ticket and write `.ticket-flow/runs/<KEY>/ticket.md`: key, issue type, summary, description, acceptance criteria, comments, the parent and sub-tickets if any, and the names of attachments. Keep the original wording.

Log the start of the stage.

## 3. Sort it

Launch `ticket-flow:triager` with the paths of `ticket.md`, the profile and `triage.json`.

Read `triage.json`, then:

| `decision` | Do |
|------------|----|
| `continue` | Log the end with outcome `continue`. |
| `split` | Log the end with outcome `split`. The plan stage writes the split proposal. |
| `needs_info` | Post the ambiguities on the Jira ticket as numbered questions, in the client language, starting with `[ticket-flow]`. Write the end state `parked_needs_info`. Tell the user. |

Report to the user in three lines: type, size, risks, decision.
