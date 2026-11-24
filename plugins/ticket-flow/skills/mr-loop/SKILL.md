---
name: mr-loop
description: Use when a ticket-flow merge request has review findings or new comments from people or bots that need sorting, fixing or answering.
argument-hint: "<TICKET>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/watch-mr-comments.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# Comment loop

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. This session's ID is `${CLAUDE_SESSION_ID}`. `forge.sh` below means `bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh`, and the watcher is `bash ${CLAUDE_SKILL_DIR}/../../scripts/watch-mr-comments.sh`.

Act only on the MR recorded in `mr.json`. Never reply on another MR.

## One round

1. Collect the batch, one JSON object per line in `batch-<n>.jsonl`:
   - the findings in `findings.json` not handled yet. They were posted from the developer's account, so the watcher does not see them.
   - new comments from others: run the watcher once, `watcher <mr> <run> --once`, then read each listed comment with `forge.sh list-threads <mr>`.
   If the batch is empty, the loop is done.
2. Launch `ticket-flow:comment-triager` with the batch, `ticket.md`, `plan.json`, the worktree and `triage-<n>.json`.
3. Handle each category:

   | Category | Do | Thread |
   |----------|----|--------|
   | `actionable` | fix, below. With `suggestion: wrong`, the fix follows `fix`, not the comment, and the reply says why | resolve only if the fix checker says yes |
   | `question` | reply with what the code does today, and say a human will answer | leave open |
   | `human_required` | reply that this is a decision for the team, and why. For a contradiction, link the other thread | leave open |
   | `incorrect` | reply with the reason, politely, citing the code | leave open |
   | `duplicate` | reply with a link to the other thread | leave open |

   Every reply starts with `_ticket-flow_`. Post it with `forge.sh reply <mr> <thread> <file>`.
4. Record every comment handled in `comments.json` with its category, reason and outcome. The handover reads it.

## Fixing the actionable comments

All actionable comments of a round go to one `ticket-flow:implementer` run on the worktree, with the triage file as its task and `fix-report-<n>.md` as its report. One attempt per comment: a comment whose fix fails is not retried.

Then run `checks.sh <worktree>`, push, and follow `${CLAUDE_SKILL_DIR}/../ci-gate/SKILL.md` until the pipeline is green.

Launch `ticket-flow:fix-checker` with the worktree, the commit range of this round, and the actionable comments, each with its `fix` from the triage file: the checker judges the change against the expected behaviour, not against the comment's own suggestion. For each:

- yes: reply `Fixed in <short sha>: <what changed>`, then `forge.sh resolve <mr> <thread>`.
- no: reply with what is still missing, and leave the thread open for a human.

## Rounds and waiting

Add one to `comment_rounds` in `mr.json` after each round. Two rounds at most: when a third would start with comments still to handle, write the end state `escalated_comments`.

After the first round, if the profile names a `review_bot`, append a `waiting` line with `on: "reviews"`, then wait for it with the Monitor tool: run the watcher without `--once`, with a 20-minute deadline, and start the next round on its first `BATCH_END`. Without a review bot, do not wait for people and write no `waiting` line: the handover lists the open threads, and the developer can run `/ticket-flow:mr-loop <KEY>` again later.

Log the end with the counts per category and the number of threads resolved in `detail`, for example `actionable 2, question 1, resolved 2`.
