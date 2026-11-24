---
name: run
description: Use when a ticket (Jira, or a GitHub issue) should go through the whole ticket-flow, from triage to a merge request ready for manual testing, or when an interrupted run must resume.
argument-hint: "<TICKET> [--resume] [--auto-approve]"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/checks.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/watch-mr-comments.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# Run the flow

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop.

Each stage is a skill of its own. Run a stage by reading its `SKILL.md` and following it with the same ticket key. They live next to this one: `${CLAUDE_SKILL_DIR}/../<stage>/SKILL.md`. In those files, `$0` stands for the normalised ticket key, `${CLAUDE_SKILL_DIR}` for that stage's own folder and the session ID for this session's, `${CLAUDE_SESSION_ID}`, because a file read this way gets no substitution.

| # | Stage | Next when |
|---|-------|-----------|
| 1 | `triage` | outcome `continue` or `split` |
| 2 | `plan` | outcome `approved` (a split ends the run here) |
| 3 | `execute` | checks pass |
| 4 | `open-mr` | MR open |
| 5 | `ci-gate` | outcome `green` or `no_ci` |
| 6 | `review` | findings posted |
| 7 | `mr-loop` | nothing left to handle |
| 8 | `handover` | end state `ready_for_qa` |

Rules:

- Stop as soon as a stage writes an end state, and report it.
- The plan gate in stage 2 waits for the developer, even if they asked to run everything. Only `--auto-approve` lets it pass on its own: run the plan stage as if it had been given that flag.
- With `--resume`, read `log.jsonl`, find the last stage that ended without an end state, and continue from the stage after it. A run that has ended (its last `end_state` comes after its last `start`, see the conventions) does not resume: tell the user, who can run a single stage by hand. A run parked with `parked_needs_info` restarts through triage instead: run `/ticket-flow:run <KEY>` without `--resume` once the client has answered. A parked run that restarted and then stopped again resumes normally.
- Keep the session's context small: read the run files a stage needs, not the agents' full output.

## Report

When the run stops, report in five lines at most: the end state, the MR URL if any, the open threads, what the developer must do next, and the estimated cost and tokens of the run, the total row of `bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh total <run folder>`. On an `escalated_*` or `timeout` end state, what the developer must do next comes from `escalation.md`, and the report gives its path.
