---
name: open-mr
description: Use when a ticket-flow branch has passed its local checks and needs to be pushed and opened as a draft merge request.
argument-hint: "<JIRA-KEY>"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *)
---

# Open the draft MR

The ticket key is the first argument: `$0`. Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. All forge calls go through `bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh`.

1. Push the branch from the worktree: `git -C <worktree> push -u origin <branch>`. The git guard refuses protected branches. If it refuses this push, the branch name is wrong: stop and tell the user.
2. Run `forge.sh find-mr <branch>`. If an MR exists, reuse it and skip to step 4.
3. Write the description to `mr-body.md` in the run folder, in the team language:

   ```markdown
   _ticket-flow_

   Jira: <ticket URL>

   <behaviour from plan.json, in one or two sentences>

   Tested: <tests from report.md>
   Not tested: <not_tested from report.md>

   A handover comment with test steps follows once CI and the review are done.
   ```

   Then `forge.sh open-mr <branch> <base> "<KEY>: <summary from triage.json>" <run>/mr-body.md`. It prints the number and the URL.
4. Write `mr.json`: `{"number": 12, "url": "...", "branch": "...", "ci_rounds": 0, "comment_rounds": 0}`.
5. Comment on the Jira ticket: `[ticket-flow] Draft merge request: <url>`.

If the push or the MR creation is refused for lack of rights, write the end state `failed_permission`. Log the end with the MR URL.
