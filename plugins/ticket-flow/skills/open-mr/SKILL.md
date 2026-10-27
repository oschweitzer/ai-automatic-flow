---
name: open-mr
description: Use when a ticket-flow branch has passed its local checks and needs to be pushed and opened as a draft merge request.
argument-hint: "<TICKET>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *)
---

# Open the draft MR

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. All forge calls go through `bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh`.

1. Push the branch from the worktree: `git -C <worktree> push -u origin <branch>`. The git guard refuses protected branches. If it refuses this push, the branch name is wrong: stop and tell the user.
2. Run `forge.sh find-mr <branch>`. If an MR exists, reuse it and skip to step 4.
3. Write the description to `mr-body.md` in the run folder, in the team language:

   ```markdown
   _ticket-flow_

   <link to the ticket, from the Tracker section: `Jira: <ticket URL>` or `Issue: #<n>`>

   <behaviour from plan.json, in one or two sentences>

   Tested: <tests from report.md>
   Not tested: <not_tested from report.md>

   A handover comment with test steps follows once CI and the review are done.
   ```

   Then `forge.sh open-mr <branch> <base> "<KEY>: <summary from triage.json>" <run>/mr-body.md`. It prints the number and the URL.
4. Write `mr.json`: `{"number": 12, "url": "...", "branch": "...", "ci_rounds": 0, "comment_rounds": 0}`.
5. On Jira, comment on the ticket: `[ticket-flow] Draft merge request: <url>`. On GitHub, skip this step: the issue's timeline already shows the MR.

If the push or the MR creation is refused for lack of rights, write the end state `failed_permission`. Log the end with the MR URL.
