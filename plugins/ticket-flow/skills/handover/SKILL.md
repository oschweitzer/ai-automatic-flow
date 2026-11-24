---
name: handover
description: Use when a ticket-flow merge request has finished its review loop and the developer needs test steps, points of attention and a merge checklist, or when reviewed client test steps must be published on the ticket.
argument-hint: "<TICKET> [--publish]"
disable-model-invocation: true
allowed-tools: Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/forge.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *) Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh *)
---

# Handover

Read `${CLAUDE_SKILL_DIR}/../../reference/conventions.md` first. The ticket key is the output of `bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh "$0"`, run before anything else. On exit 1, show its message and stop. This session's ID is `${CLAUDE_SESSION_ID}`. With `--publish` as second argument, skip to the last section.

The goal: a developer can test the change, explain it to a colleague or the client, and know what could go wrong, without reading the code. Follow the structure of `${CLAUDE_SKILL_DIR}/example.md`.

## 1. Read what the run recorded

Write from the run folder, not from a fresh exploration: `ticket.md`, `triage.json`, `plan.json`, `report.md` and the fix reports, `findings.json`, `comments.json`, `mr.json`, the log (CI outcome), and `git -C <worktree> diff --stat origin/<base>...HEAD`. Open a changed file only to check a detail the run files do not give: a route, a label on screen, a default value.

Then check which commit each result is on (conventions, Results belong to a commit). HEAD is `git -C <worktree> rev-parse HEAD`.

- `ci_sha` must be HEAD. If it is not, a push came after the last green pipeline: run `${CLAUDE_SKILL_DIR}/../ci-gate/SKILL.md` first.
- When `reviewed_sha` is not HEAD, list the commits after it with `git -C <worktree> log --oneline <reviewed_sha>..HEAD`. They were checked by the fix checker only.

Decide the review route, `human` or `ai`, with its reasons, as the conventions' Review route section says, from the final diff and the run files.

## 2. Build the assets

Only what the change needs:

- Screenshots, when the change touches a screen and the profile has `run_app` and `app_url`. Start the app from the worktree, walk the happy path of the test steps with a throwaway Playwright script written in the run folder, using the seed accounts only. Ask before installing Playwright. Stop the app afterwards.
- API requests, when the change touches an endpoint, in the profile's `api_client`: a `.bru` request for Bruno, a request to import for Postman, or a curl command on one line, with double quotes only, reading its JSON body from a file (`--data "@body.json"`), followed by the note that Windows PowerShell needs `curl.exe`.
- Test files (spreadsheets, images, PDFs), when the test steps need one. Generate small synthetic files with a script. Never put real client data in them. When a realistic file cannot be generated, say which one is needed and add a checklist line.

Save them in `assets/` and upload each with `forge.sh upload <mr> <file>`, which prints the Markdown link to use.

## 3. Write the MR comment

`handover.md`, in the team language, first line `<!-- ticket-flow:handover <KEY> -->`, with these sections:

1. One status line: CI outcome and its short sha, review findings fixed and left open, plan approval (say auto-approved when the plan's end line has that `detail`). Then the review line: the route (`Review: human` with its reasons, or `Review: AI`), the reviewers that answered (`/code-review` on its short sha, and the review bot or "review bot did not answer"), and the commits after the review if any, saying they were checked by the fix checker only. Then one line for what the run has used so far, from `bash ${CLAUDE_SKILL_DIR}/../../scripts/usage.sh total <run folder>`: the estimated cost in USD at API rates, the normal tokens (input and output), the cached tokens apart (cache write and cache read), and the cost per model. On exit 2, say the count is partial, and why from the script's message. On exit 1, leave the line out.
2. What this MR does: plain language, the problem before, one everyday analogy for how it works, one sentence for a client.
3. What changed: a table by area (screen, API, database, config, tests).
4. How to test: prerequisites with exact commands, numbered steps each with its expected result and screenshot, API requests, and how to tell it is broken.
5. Test files, if any.
6. Points of attention: critical points (every risk from the triage lands here), known limits accepted on purpose with the reason, things to watch later, what was not tested, threads left open for a human.
7. Before merging: a checklist. Always CI on the final commit, tests, manual test, open threads answered, client steps read. With the `human` route, one line per reason, asking a person to read that part of the diff (for example `A person has read the changes under src/auth/`). With commits after the review, a line asking a person to read them. The route never removes a line: the checklist and the human merge decision stay. Add one line per configuration change, saying what to set, where, and on which environments (local, test environment, production). When a default exists, the line asks to confirm the default is fine. Last line: `/ticket-flow:handover <KEY> --publish` after deployment to the test environment.
8. The client steps from section 4 below, inside `<details>`, marked as a draft.

On a GitHub forge, whatever the tracker, run `forge.sh visibility` first. When it prints `public` (or fails), the MR is public too: leave the test environment URL and account details out of `handover.md`, including the client steps draft of section 8, as the Public issues section says, and keep the other sections.

Post it with `forge.sh upsert-note <mr> "ticket-flow:handover <KEY>" handover.md`, which updates the earlier comment on a rerun.

## 4. Draft the client steps

`client-steps.md`, in the client language: the test environment URL, the account or data to use (never a password), numbered steps with the words the client sees on screen and the expected result, the files to use, the known limits in neutral terms, and how to report a problem. No technical detail, no internal links.

## 5. Finish

Run `forge.sh mark-ready <mr>`. Write the end state `ready_for_qa`, with the route in `detail`, for example `review: human (src/auth/)`. Tell the developer the MR is ready for manual testing, and that the client steps stay a draft until they publish them.

## Publishing the client steps

Only with `--publish`. Show `client-steps.md` and ask the developer to confirm they have read it and that the change is deployed on the test environment. On yes, post it on the ticket, starting with `[ticket-flow]`. With `tracker: github`, follow the Public issues section first. If the developer skips the post, write no `published` line and tell them the steps are still unpublished. If it mentions files, remind the developer to attach them to the ticket by hand. Once posted, append a line with `stage: "handover"` and `event: "published"`.
