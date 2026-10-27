---
name: triage
description: Use when a ticket (Jira, or a GitHub issue) is about to enter ticket-flow and has to be checked for an existing run, classified and judged ready or not.
argument-hint: "<TICKET>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *)
---

# Triage

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop.

## 1. Already running?

These checks cost nothing. Run them before reading the ticket.

- The run folder `.ticket-flow/runs/<KEY>/` exists and the last line of its `log.jsonl` is an `end_state` with outcome `parked_needs_info`: the client may have answered. Tell the user the run restarts from triage, then continue with step 2, which overwrites `ticket.md` with a fresh snapshot. Append to the same log, never delete the folder.
- The run folder exists in any other state: read the last lines of its `log.jsonl` and tell the user where the run stopped. Offer to resume from that stage with `/ticket-flow:run <KEY> --resume`. Stop.
- A remote branch carries the key (`git ls-remote --heads origin | grep -i "/<KEY>-"`): run `forge.sh find-mr <branch>`. If an MR is open, tell the user and stop, since the flow keeps one MR per ticket.

## 2. Snapshot the ticket

Read the ticket as the conventions' Tracker section says and write `.ticket-flow/runs/<KEY>/ticket.md`: key, issue type, summary, description, acceptance criteria, comments, the parent and sub-tickets if any, and the names of attachments. Keep the original wording.

With `tracker: github`, `forge.sh issue-read <n>` gives the issue. Refuse a closed issue (`state` is `closed`): tell the user and stop without creating the run folder. Map it this way: issue type from `type`, else the labels. Parent and sub-tickets from `parent` and `sub_issues`, written `#<n>`. Attachments are the file and image links found in the body and the comments. When `dropped_comments` is above 0, add a line saying that many comments from people outside the project were left out.

Log the start of the stage.

## 3. Sort it

Launch `ticket-flow:triager` with the paths of `ticket.md`, the profile and `triage.json`.

Read `triage.json`, then:

| `decision` | Do |
|------------|----|
| `continue` | Log the end with outcome `continue`. |
| `split` | Log the end with outcome `split`. The plan stage writes the split proposal. |
| `needs_info` | Post the ambiguities on the ticket as numbered questions, in the client language, starting with `[ticket-flow]`. With `tracker: github`, follow the Public issues section first. Write the end state `parked_needs_info`. Tell the user. |

Report to the user in three lines: type, size, risks, decision.
