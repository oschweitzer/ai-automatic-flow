---
name: review
description: Use when a ticket-flow merge request has a green pipeline and needs its independent AI review posted as threads.
argument-hint: "<TICKET>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# AI review

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. This session's ID is `${CLAUDE_SESSION_ID}`. The pipeline must be green, or absent (`no_ci` in the log).

The review runs in a separate, non-interactive Claude Code session, so it shares nothing with the session that wrote the code. It reads the repository's `CLAUDE.md`, not the plan.

## 1. Run it

The review session's tokens count for the run, so give it an ID first: `bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh new <run>` records one and prints it. Then, from the worktree, with a 15-minute timeout:

```bash
cd <worktree> && command claude -p --model sonnet --session-id <id> \
  --allowedTools "Read,Grep,Glob,Bash(git diff *),Bash(git log *),Bash(git show *)" \
  "/code-review medium origin/<base>...HEAD" > <run>/review.txt
```

`command` skips any shell alias named `claude`. The new session inherits `CLAUDE_CONFIG_DIR`, so it uses the same account as this one.

Do not pass `--comment`: on GitLab it writes one note for the whole review, which cannot be resolved finding by finding. The flow posts the threads itself.

If the command fails or `review.txt` is empty, tell the user and stop without an end state, so the stage can be rerun.

## 2. Extract the findings

Read `review.txt` and write `findings.json`, one entry per finding:

```json
[{"id": "f1", "severity": "high | medium | low", "path": "src/cart/discount.ts", "line": 18, "title": "...", "evidence": "what in the code shows it", "action": "what to change", "thread": null}]
```

A finding with no evidence keeps `"evidence": ""`. It is posted as a remark and never blocks.

## 3. Post them

For each finding, write its body to a temporary file, in the team language:

```markdown
_ticket-flow_ AI review (`/code-review`), severity: <severity>

**<title>**

<evidence>

Suggested: <action>
```

For a finding without evidence, the first line ends with `remark, does not block` instead of the severity.

Run `forge.sh post-thread <mr> <path> <line> <file>` and store the printed id as `thread` in `findings.json`.

Log the end with the number of findings per severity in `detail`. If there are none, say so: the comment loop then only waits for human or bot comments.
