# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

A desktop app for Ubuntu, Windows and macOS. The window wraps a web front end, so the design language is web, not native.

## Stack

Delegated: Tauri with React and TypeScript, chosen during brainstorming on 26 October 2026. Tauri ships one light binary per OS and its Rust side watches local files, calls the forge and Jira command-line tools. React was picked because the code will be written with AI and the team are web developers. The team said Rust skills do not matter since the code is AI-written.

## Users

Every developer of the team, each on their own workstation, where they also run the ticket-flow plugin in Claude Code. They open the dashboard to see where their tickets stand, what the AI is doing on them right now, and what is waiting on them or on the client.

## Product Purpose

Ticket Flow Dashboard tracks the tickets handled by the ticket-flow plugin (Jira ticket to merge request). It shows tickets in progress, waiting and done, the state of each pull or merge request on GitHub or GitLab with its CI, links to the content, and a progress bar for each AI run on the local machine.

Success means a developer can tell, without opening Jira, the forge or a terminal, which tickets need them and how far each local run has gone.

## Positioning

It reads the plugin's own run folders (`.ticket-flow/runs/<KEY>/`), so it shows the AI's progress stage by stage, which neither Jira nor the forge can show.

## Operating Context

- Local data comes from `.ticket-flow/runs/<KEY>/` in each repository: `log.jsonl` (one start and one end line per stage, then an end state), `triage.json`, `plan.json`, `mr.json`, `handover.md`. The run has 8 stages: triage, plan, execute, open-mr, ci-gate, review, mr-loop, handover.
- Remote data comes from read calls to GitHub or GitLab (PR/MR, CI) and Jira (tickets, statuses, sprints, assignees). Nothing else is remote. There is no server to host.
- Repositories are found from root folders the developer points the app at. A repository counts when it holds the committed profile `.claude/ticket-flow.md`, which gives its Jira key and forge.
- The app reads Jira through the Atlassian CLI (`acli`) and its OAuth login, so it stores no Jira token.
- Builds for the three OS are meant to be published as releases of this repository. They will likely be unsigned at first, so macOS and Windows show a warning on first launch.

## Capabilities and Constraints

- A project is a Jira key with one or more repositories, and can be a single repository.
- The first tab, "all projects", shows one row per project with counts per state. Then one tab per project.
- A person filter works like a Jira active-sprint board. The default is the developer alone. Other people and "unassigned" can be added.
- All tickets are listed, not only the active sprint. Sprint chips at the top choose the sprints and the backlog, with the active sprint selected by default.
- Under the chips, collapsible sections by state: in progress, waiting, to do, done.
- "Waiting" covers three cases, shown apart: waiting on client information (`parked_needs_info`), waiting on a human action (plan to approve, plan rejected, escalated MR, report to read), and Jira to-do tickets the flow has not picked up.
- A ticket row shows the ticket, its repository, its sprint, the AI progress for local runs only, and the PR/MR with its CI state.
- A colleague's run happens on their machine, so their tickets show Jira and forge state without a progress bar.
- Clicking a ticket opens a detail panel with progress, plan and links.
- The interface is available in French and English, chosen by each developer.
- Undecided: whether version 1 stays read-only (my assumption), refresh rates, notifications, whether done tickets in the backlog are capped, and the exact content of the detail panel.
- Out of scope: estimates (rarely filled in Jira for this team), and the progress of colleagues' runs, which would need the plugin to publish its progress somewhere shared.

## Brand Commitments

The name is Ticket Flow Dashboard. No logo, palette or other identity asset exists.

## Evidence on Hand

Clickable mockups from the brainstorming session are in `.superpowers/brainstorm/` (git-ignored): layout options, the tabbed view with collapsible sections, the person filter, the sprint chips and the detail panel. No real run folder exists yet, because the plugin has not run on a test repository.

## Product Principles

- What needs the developer comes first, and comes apart from what waits on the client.
- Show the AI's real progress from its run files, never a guessed one.
- Zero setup beyond pointing at folders and logging in to `gh`, `glab` and `acli`. Everything else is read from the repositories and the trackers.
- Each developer's own tickets by default, the team one click away.
