# Ticket Flow Dashboard design

Date: 26 October 2026. Status: written after a brainstorming session, waiting for approval.

Related files: [dashboard/PRODUCT.md](../../../dashboard/PRODUCT.md) (product record), [dashboard/design-system/](../../../dashboard/design-system/) (tokens, component styles and rules), the published design system at https://claude.ai/artifact/Hd9J6Z5ZPWdVFEoTvVF1vV, and the plugin's run conventions in [plugins/ticket-flow/reference/conventions.md](../../../plugins/ticket-flow/reference/conventions.md).

## Why

The ticket-flow plugin carries a Jira ticket to a merge request inside a Claude Code session. While it runs, the only places to see what it is doing are the terminal and the files it writes in `.ticket-flow/runs/<KEY>/`. Jira and the forge show the ticket and the MR, but not the AI's progress, and nothing gathers in one place what waits on the developer (a plan to approve, an escalated MR) or on the client (a question parked on Jira).

The dashboard gives each developer that one place. Success means a developer can tell, without opening Jira, the forge or a terminal, which tickets need them and how far each local run has gone.

## Goals

- Show every ticket of the configured projects, grouped by state: in progress, waiting, to do, done.
- Show the AI's real progress for runs on this machine, stage by stage, read from the run files.
- Show the PR or MR of each ticket with its CI state, and link to the ticket, the MR, the pipeline, the plan and the run folder.
- Separate what waits on the developer from what waits on the client.
- Let the developer see colleagues' tickets on demand, the way a Jira sprint board does.
- Run on Ubuntu, Windows and macOS.

## Non-goals for version 1

- Estimates. The team rarely fills them in Jira.
- Progress of colleagues' runs. Their run files are on their machines. Showing them would need the plugin to publish progress somewhere shared.
- Any server, shared database or hosted backend.
- A light theme.
- Changing anything in Jira or on the forge (my call, see below).

## Anti-goals

- A progress bar that guesses. If the run files do not say where a run is, the dashboard says so instead of inventing a percentage.
- Setup that asks for more than root folders and a Jira token.
- A dashboard that becomes a second Jira. It shows what the flow adds and links out for the rest.

## Users and usage

Each developer of the team runs the app on their own workstation, the same machine where they run the plugin in Claude Code. They keep it open next to their editor and look at it many times a day. Each sees their own tickets by default and can add colleagues.

## Decided with the user

### Stack

A desktop app with Tauri, a Rust backend and a React with TypeScript front end. Code quality in Rust is not a concern because the code is written with AI.

### Data sources

Local data comes from the run folders. Remote data comes from GitHub or GitLab and from Jira, read only. Nothing else.

### Projects

A project is a Jira key with one or more repositories, and can be a single repository. Repositories are found under root folders the developer points the app at. A repository counts when it holds the committed profile `.claude/ticket-flow.md`, whose frontmatter gives `forge` and `jira_project`. Repositories with the same `jira_project` form one project.

A repository whose profile says `tracker: github` takes its tickets from its own GitHub issues. It is a project on its own, named `owner/name` from its `origin` remote. The sidebar shows the Jira projects and these GitHub projects in two labelled groups, "Jira" and "GitHub", and hides a group with no project. See "GitHub-tracker repositories" below.

### Screen

A left sidebar lists "Tous les projets" (all projects), then the projects with their repositories and a count of tickets waiting on the developer. Settings and the sync state sit at the bottom.

- "All projects" shows one row per project with counts: waiting on you, waiting on the client, in progress (with how many have a red CI), to do, done, last activity.
- A project screen has a top bar with a person filter (avatars) and a sprint filter (chips), then four collapsible sections in this order: En cours, En attente, À faire, Terminé. The first two are open by default.
- A side column on the right holds the "À toi de jouer" card (what waits on the developer) and a small summary of the selected sprints.
- Clicking a ticket replaces the side column with a detail panel: header (type, repository, sprint, assignee, Jira status, triage size and risk, plan model), the eight stages with outcome and time, a plan summary with a link to the full plan, and links (Jira ticket, MR, pipeline, run folder, handover report when it exists). Escape closes it.

### Filters

The person filter defaults to the developer alone. Other assignees and "unassigned" can be added. The sprint filter defaults to the active sprint. Future sprints and the backlog can be added. Closed sprints do not appear. Filters and open sections are remembered per project.

### Waiting

Three cases, shown apart. Waiting on the client means the run ended in `parked_needs_info`. Waiting on the developer covers a plan waiting at the gate, `plan_rejected`, the escalated end states, `timeout` and `failed_permission`. Jira to-do tickets the flow has not picked up go in À faire.

### Colleagues' tickets

They show Jira and forge state only, without a stage track.

### After the run

A run that ends in `ready_for_qa` still waits on the developer: first "MR à tester et merger", then "étapes client à publier" once the MR is merged. It moves to Terminé only once the client steps are published.

### Look

The design system in `dashboard/design-system/` is the authority: dark theme only, Geist and Geist Mono, one lavender accent, state colours with words, the stage track as the signature. The validated mockups are in `.superpowers/brainstorm/` (git-ignored). The last one is `theme-geist.html`.

### Language

The interface is in French and English, chosen per developer.

### Distribution

Builds for the three systems. They will likely be unsigned at first, so macOS and Windows warn on first launch.

## Log contract with the plugin (decided with the user)

The plugin is still a draft with no real run, so it changes first, in a small way, so that the dashboard reads facts instead of guessing. Today each `log.jsonl` line is `{"at", "stage", "event", "outcome"}`, with `event` being `start`, `end` or `end_state`. The plugin adds:

- A `waiting` event with an `on` field, written when a stage starts waiting on something outside the session: `on: "developer"` when the plan gate asks for approval, `on: "ci"` while ci-gate waits for the pipeline, `on: "reviews"` while mr-loop waits for comments. The next `start`, `end` or `end_state` line closes the wait.
- A `model` field on the `start` line of every stage that launches a model (the planner and executor choice the detail panel shows).
- A `published` event when `/ticket-flow:handover <KEY> --publish` posts the client steps on the ticket (Jira or the GitHub issue).
- Triage accepts a run folder whose end state is `parked_needs_info` and starts again, appending to the same log, so a parked ticket can run again once the client answers. Today it refuses any existing run folder.
- `conventions.md` documents these fields, and the "never write a secret" rule still applies.

## How states are read (my calls, to check)

The last meaningful line wins. A `start` after an `end_state` reopens the run.

| Run folder shows | Section | Shown as |
|---|---|---|
| last line `waiting` with `on: "developer"` | En attente, you | "plan à valider" |
| last line `waiting` with `on: "ci"` or `"reviews"` | En cours | stage track, "attend la CI" or "attend les retours" |
| `end_state` `ready_for_qa`, MR not merged | En attente, you | "MR à tester et merger" |
| `end_state` `ready_for_qa`, MR merged, no `published` | En attente, you | "étapes client à publier" |
| `end_state` `ready_for_qa` then `published` | Terminé | `ready_for_qa` |
| `end_state` `split` | Terminé | `split` |
| `end_state` `parked_needs_info` | En attente, client | "client · précision demandée" |
| `end_state` among `plan_rejected`, `escalated_checks`, `escalated_ci`, `escalated_comments`, `timeout`, `failed_permission` | En attente, you | the reason, from the end state |
| any other run with no end state | En cours | stage track at the last started stage |
| last line `start` and nothing new for 30 minutes | En cours, flagged | "sans nouvelle depuis 45 min" (the session may have stopped). Waits do not count. |

Two overrides come before the run folder. A Jira status in the Done category, or a merged MR whose client steps are published, puts the ticket in Terminé whatever the log says. A closed, unmerged MR shows "MR fermée" in Terminé. Decided on 2026-10-29: client steps still to publish outrank the tracker. A run that ended in `ready_for_qa` without a `published` line stays in En attente even when Jira says Done or the issue is closed as completed.

Tickets with no run folder take their section from the Jira status category: To Do goes to À faire, In Progress to En cours, Done to Terminé. They have no track. A colleague's in-progress ticket shows "en cours chez AL", because the dashboard has no cheap way to know whether their flow is running. This replaces the "Run sur le poste de AL" wording of the mockup.

The stage index comes from the fixed stage order in `skills/run/SKILL.md`: triage, plan, execute, open-mr, ci-gate, review, mr-loop, handover. A failed CI shows on the current dot when the forge reports a failed pipeline on the MR.

## Whose tickets (my calls, to check)

- A ticket with a local run is the developer's, whatever its Jira assignee. It shows in the default view and in their "waiting on you" counts.
- Other tickets follow the person filter on the Jira assignee.
- The "All projects" counts follow the person filter and each project's default sprint selection.

## Data and refresh (my calls, to check)

### Local files

The Rust side watches each repository's `.ticket-flow/runs/` and re-reads a run when its files change. Reading is tolerant: a half-written last line is skipped and read again on the next change.

### Forge

The Rust side calls the `gh` and `glab` command-line tools, as the plugin's `forge.sh` does, so it reuses the developer's existing login and stores no forge token. The MR comes from `mr.json` (`number`, `url`, `branch`) when a run exists, else from a search by branch name containing the ticket key. CI state comes from the MR's head pipeline.

### Jira

The app calls the Jira Cloud REST API with the developer's email and an API token, kept in the OS keychain. The Jira site URL, the email and the token are the only settings to type. "Me" is the token's account. Sprints come from the boards of each project.

### Jira scope

For each project: tickets in open and future sprints, backlog tickets not done, and tickets done in the last 14 days. Older history is never fetched.

### Projects without one active sprint

A project with no sprint (a Kanban board, or no board) hides the sprint chips and shows all fetched tickets. A project with several active sprints selects all of them by default, each chip named after its sprint.

### Refresh

Local changes show within a few seconds. Jira and the forge are polled every 2 minutes while the window is visible, and a refresh button forces it. The sidebar shows the time of the last sync and a warning when a source fails.

### Failures

When Jira or the forge cannot be reached, the dashboard keeps the last data it had, marks it as stale with its age, and says which source failed. Local data keeps working.

### GitHub-tracker repositories

Decided on 2026-10-27, see [the GitHub issues design](2026-10-27-github-issues-tracker-design.md#dashboard). The tickets of a GitHub project are its issues, read with `gh` in the repository folder: open issues, and issues closed in the last 14 days, pull requests excluded. The run folder rules do not change. Where a Jira project uses the Jira status, a GitHub project uses the issue state:

- An issue closed as completed puts the ticket in Terminé, like the Jira Done override.
- An issue closed as not planned goes to Terminé with the label "fermée sans suite".
- An open issue without a run goes to À faire. GitHub has no "in progress" state, so an assigned issue without a run stays in À faire.

The person filter uses the issue assignees, and "me" is the `gh` login. GitHub projects hide the sprint chips. The detail panel shows the issue state and labels instead of the Jira status, links to the issue instead of the Jira ticket, and shows a "public" tag when the repository is public, because the flow asks for approval before posting there.

## Version 1 scope (my calls, to check)

- Read only. Every action goes through a link: open the ticket, the MR, the pipeline, the plan file or the run folder in the system's default app. The "À toi de jouer" button opens the first waiting item.
- No system notifications.
- No search beyond the search field filtering the visible list by key or summary.

## Constraints

- Never write in a repository or a run folder. The dashboard only reads.
- Never show or log a token. Never copy log contents beyond the fields listed above.
- Contrast and focus rules of the design system apply: text at 4.5:1 or more, control borders and focus ring at 3:1 or more, state shown with words.
- The fonts ship with the app (no network needed to render).
- The app keeps working with no Jira token: it then shows local runs only for Jira projects and says Jira is not configured. GitHub projects need no Jira token and work fully.

## Left to the builder

- The Rust crates (file watching, keychain, YAML frontmatter), the front-end state and data-fetching libraries, the i18n library.
- Folder layout under `dashboard/`, as long as `PRODUCT.md` and `design-system/` stay where they are.
- How far below the root folders to look for repositories, and which folders to skip.
- How the board of a Jira project is found.
- Avatar colours (a stable hash of the account id is fine).
- The build pipeline for the three systems. The repository is on GitHub, so GitHub Actions is the obvious choice.
- Test strategy, with one requirement: the state table and the overrides above are covered by tests on fixture run folders.
- The order of work, with one constraint: the plugin's log contract lands before the dashboard reads it.
