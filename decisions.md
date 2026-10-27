# Decisions

## Settled

- Version 1 runs locally in Claude Code, on a developer's machine.
- The flow ships as a Claude Code plugin with its own skills, agents and hooks, installed per repository.
- It is developed and tried against an isolated test repository first.
- It builds on superpowers for the discipline skills (test first, debugging, worktrees, receiving review, verification), without the brainstorming skill.
- The documentation is in English.
- The design merges the two plans compared in [comparison.md](comparison.md). The result is in [architecture.md](architecture.md).
- One ticket per branch and per MR. Lots, parallel mode and multi-ticket runs are left for later.
- Every kind of ticket is accepted, not only maintenance tickets. Triage classifies each ticket as a bug, a feature, a technical change or a story. A story too big for one branch gets a split proposal, and once a human approves it the flow creates the sub-tickets on Jira.
- No category is excluded during the pilot. Risk signals raise the planner to Opus and show up in the handover instead of blocking the ticket.
- No spend cap during the pilot. Cost is measured on every run, and the loop caps and `maxTurns` stay.
- The pilot uses at least one ticket of each kind (see below), to validate the whole flow.
- GitLab and GitHub are both supported, GitLab first. Every forge call goes through one script, `forge.sh`.
- Tickets come from Jira or from GitHub issues in the same repository, chosen by `tracker` in the profile (Jira when missing). On a public repository, every post on an issue is a light version without sensitive details, approved by the developer first. See the [design](docs/superpowers/specs/2026-10-27-github-issues-tracker-design.md).
- The MR opens as a draft right after the local checks. The forge's CI is the gate before the AI review, with a capped fix loop.
- The flow may push to the branch of an MR it opened, and may reply to threads on that MR, without asking a human. It resolves only the threads it fixed and whose fix passed the fix checker. Every other thread stays open for a human. It never pushes to an integration branch (main, master, develop, staging, prod and the like), never force-pushes, and never merges. The `PreToolUse` git guard enforces the push and merge rules.
- The AI review is Claude Code's bundled `/code-review`, run locally in a separate non-interactive session, so it shares nothing with the author's context. The flow posts its findings as threads itself. No personal Claude credential goes into CI. There is no second AI review before the MR.
- Every run ends in a named state.
- Jira statuses are set aside for now. Version 1 reads tickets and posts comments only. The status proposal stays in [architecture.md](architecture.md#jira-statuses) for later.
- The flow ends with a handover report: an MR comment for the developer (plain explanation, manual test steps with screenshots, API requests and test files, points of attention including corner cases accepted on purpose, a checklist before merging) and client test steps for Jira. See [handover-report.md](handover-report.md).
- API requests in the report use the repository's own tool: Bruno if it has Bruno files, else Postman if it has a Postman collection, else a single-line curl that works on every OS.
- The merge checklist lists every local or infrastructure configuration to update, per environment, and only when the change needs it.
- Everything stays in this folder. The personal skills the plugin draws on are copied into `reference/existing-skills/` and the originals are never edited.
- For now there is no test repository and no non-personal identity. Version 1 runs on the developer's own accounts (Claude, GitLab or GitHub, Jira), and every comment the flow posts says so.

## Pilot tickets

At least one of each, on the test repository:

| Kind | What it checks |
|------|----------------|
| Bug on a screen | reproduction test first, screenshots in the handover |
| Bug in an API | reproduction test first, Bruno, Postman or curl request in the handover |
| Feature on a screen | acceptance criteria turned into tests, screenshots, client steps |
| Feature with an API and a migration | risk flag, Opus planner, migration in the checklist |
| Technical change with a new environment variable | configuration lines in the checklist |
| Change touching authentication or permissions | risk flag, critical points in the handover |
| Story with several independent parts | split proposal, sub-tickets created on Jira |
| Vague ticket | questions posted, Needs info |
| The same ticket started twice | resume instead of a second run or MR |

## Calls I made that need checking

- Skills drive version 1. A Workflow script comes later, to run several tickets at once.
- Run state goes in `.ticket-flow/runs/<KEY>/`, git-ignored, in the target repository.
- Per-repository settings live in a committed `.claude/ticket-flow.md`, not in plugin `userConfig`, because `userConfig` values are saved in the user's own settings.
- The Atlassian MCP server is a prerequisite, not bundled in the plugin.
- The plan gate stays on for every ticket in version 1.
- Planner routing: Sonnet for small, low-risk tickets without ambiguity, Opus for the rest and for every split. A guess to check on the pilot.
- Caps: two attempts per failing local check, two CI fix rounds, one fix attempt per comment, two comment rounds.
- A split creates Jira sub-tasks under the story, in Ready for AI, and leaves the story's own status to the team.
- `risk_paths` in the profile flags risky tickets without blocking them.
- Every skill sets `disable-model-invocation: true`, so the plugin adds nothing to the context of a session that does not use it. `run` reads the stage files instead of invoking the stage skills.
- One worktree per ticket, created by `execute` outside the repository at `<repository parent>/<repository name>.worktrees/<KEY>`, instead of `isolation: worktree` on the agents. Later fixes need the same branch.
- No `SubagentStop` hook. It would run the checks after every subagent of every session in the repository. `execute` runs them itself.
- The git guard denies protected branches and lets any other branch through, so it can stay on in every session without getting in the developer's way.
- Local check failures end in their own state, `escalated_checks`, rather than `escalated_ci`.
- The review session is started with `command claude`, so a shell alias named `claude` cannot get in the way.
- Client test steps stay a draft until a human has read them, then one command posts them on Jira.
- On GitLab, screenshots and test files go through the project uploads API. On GitHub, they go to a `ticket-flow-assets` branch that is never merged.
- Test files are synthetic and generated. Real client data never goes into them.

## Open questions

1. Which app becomes the test repository, and which sandbox Jira project goes with it?
2. Later, when the flow runs without a developer, which non-personal identity pays for it? The documented options are a Claude Console service account (API billing, keys or Workload Identity Federation), a cloud provider (Bedrock, Agent Platform, Foundry), or the managed Code Review service on a Team or Enterprise plan. GitLab and GitHub need their own non-personal identity too (a GitLab project or group access token, a GitHub App, or a machine user) so comments are not posted under a developer's name.
3. Later: can a Jira admin create the statuses and transitions on the sandbox project, or should the profile map the flow onto an existing workflow?
