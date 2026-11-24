# GitHub issues as the ticket source

Date: 2026-10-27. Status: proposal, not approved yet.

## Why

Some clients do not use Jira. Their tickets are GitHub issues, in the same repository as the code. The client writes the issues and reads what is posted on them. Today ticket-flow can only read tickets from Jira, so it cannot be used on these projects, even though the forge side (PR, CI, review) already supports GitHub. The dashboard has the same gap: it only knows Jira projects.

## Goals

- Run the whole flow, and each stage alone, from a GitHub issue number.
- What the flow posts on a Jira ticket today goes to the issue instead: the triage questions, the proposed plan and the client test steps. Same `[ticket-flow]` prefix, same client language. The PR link is the exception (see `open-mr` below).
- When triage says a story must be split and the developer approves, create the parts as GitHub sub-issues of the story.
- On a private repository, behave exactly as with Jira.
- On a public repository, post a light version of each issue comment, with nothing sensitive in it, and only after the developer has approved the exact text.
- Show these repositories in the dashboard, kept apart from the Jira projects.
- Keep the Jira path working exactly as it does now. A repository whose profile says nothing new keeps using Jira.

## Non-goals

- GitLab issues.
- Issues in another repository, or tickets kept only in a GitHub Project (including Project iterations as sprints).
- A repository that uses Jira and GitHub issues at the same time.
- Changing issues beyond reading and commenting: no labels, assignees, milestones, issue types, closing or reopening. Version 1 already leaves Jira statuses alone, and this keeps the same rule.

## Anti-goals

- Leaking sensitive details on a public repository: the files and risks of a security fix before it is merged, test environment URLs, account names, internal hostnames, logs.
- Acting on comments from people outside the project. On a public repository anyone can comment on an issue, and those comments must not reach the triager or the planner.
- Any change in behaviour for Jira users, in the plugin or in the dashboard.

## Constraints

- `forge.sh` is the only place in the plugin that calls `gh` or `glab` (its header and `decisions.md` say so). Issue calls go there too.
- The profile is read by a simple frontmatter parser (`lib.sh`): flat keys or one level of nesting, one value per line.
- Agents get file paths, never pasted content. The triager and planner already read `ticket.md`, so they need no change beyond wording, as long as `ticket.md` keeps the same shape.
- The scripts have offline tests with stubbed CLIs (`tests/test-forge.sh` stubs `glab`). The new commands need the same kind of test, with a stubbed `gh`.
- The dashboard only reads, calls the forge through `gh` and `glab` with the developer's existing login, and keeps working without Jira (no `acli` login) (dashboard spec, 2026-10-26).

## Plugin

### Profile

A new frontmatter key, `tracker`, with the value `jira` or `github`. When it is missing, the tracker is `jira`, so existing profiles keep working.

```markdown
---
forge: github
tracker: github
jira_project: ""
...
---
```

`tracker: github` requires `forge: github`. Any skill that reads the profile and finds `tracker: github` with `forge: gitlab` stops and tells the user it is not supported.

The `profile` skill adds `tracker` to the questions it asks the user in its single message. It cannot be detected reliably, because a GitHub repository may have issues switched on and still use Jira. When the answer is `github`, it does not ask for `jira_project` and writes it empty.

Visibility is not stored in the profile. A repository can go public after the profile is written, so the flow checks it live before each issue comment (see below).

### Ticket key

With `tracker: github`, the user can type `142`, `#142` or `GH-142`. The key used everywhere inside the flow is `GH-142`: the run folder `.ticket-flow/runs/GH-142/`, the worktree `<repo>.worktrees/GH-142`, the `{key}` part of the branch pattern, the PR title prefix and the log. A bare number would make branch names and run folders ambiguous, which is why it is not used as the key.

Each skill normalises its first argument as its first step, following one rule in `conventions.md`. With `tracker: jira` the argument is used as it is today.

The `argument-hint` of every skill that takes a key changes from `<JIRA-KEY>` to `<TICKET>`, and the skill and agent descriptions say "ticket" instead of "Jira ticket". This is the change that started the request: the autocompletion should not suggest a Jira key on a GitHub project.

### Tracker calls

`conventions.md` replaces its "Jira" section with a "Tracker" section that lists the operations the flow needs and how each tracker does them:

| Operation | Jira | GitHub |
|-----------|------|--------|
| read a ticket | Atlassian MCP | `forge.sh issue-read <n>` |
| comment on a ticket | Atlassian MCP | `forge.sh issue-comment <n> <body-file>`, through the public check below |
| create a part of a split | Atlassian MCP (sub-task or linked task) | `forge.sh issue-create-sub <parent> <title> <body-file>`, through the public check below |
| link to the ticket from the PR | `Jira: <url>` line | `Issue: #<n>` line |

The skills stop naming Jira and say "read the ticket", "comment on the ticket", "create the part", pointing to this table.

### New `forge.sh` commands, GitHub only

These go in the GitHub section of `forge.sh` and appear in its usage text. On GitLab the dispatcher already fails with "unknown command", which is fine since that combination is refused earlier.

- `visibility` prints `public`, `private` or `internal`, from `gh repo view --json visibility`.
- `issue-read <n>` prints one JSON object: `number`, `url`, `title`, `body`, `author`, `labels`, `type` (the issue type name, or null), `state`, `parent` (number or null), `sub_issues` (numbers), and `comments` as `{author, body, at}`. It reads the issue with `gh api repos/{repo}/issues/<n>`, its comments with `--paginate`, and its sub-issues with `repos/{repo}/issues/<n>/sub_issues`. It refuses a number that is a pull request.
- `issue-comment <n> <body-file>` posts a comment and prints its id.
- `issue-create-sub <parent> <title> <body-file>` creates an issue, then attaches it to the parent with `POST repos/{repo}/issues/<parent>/sub_issues`. That endpoint wants the new issue's `id` (the database id), not its number, sent as an integer (`-F sub_issue_id=<id>`). It prints the new issue's number. If the attach step fails after the issue was created, it prints the number anyway and exits non-zero, so the skill can report an issue that exists but is not linked.

Comment filtering happens in `issue-read`, not in a skill, so it cannot be forgotten. It keeps comments whose `author_association` is `OWNER`, `MEMBER` or `COLLABORATOR`, plus comments by the issue's author. It adds `dropped_comments` with the number of comments it left out, so the snapshot can say that some were ignored.

### Public repositories

Before anything is posted on an issue, the skill runs `forge.sh visibility`. With `private` or `internal`, it posts as it would on Jira, with no extra step. With `public`, two rules apply.

The text is a light version. A new section of `conventions.md`, "Public issues", lists what never goes in it: file paths and code, the risks section of the plan and anything that describes a weakness, test environment URLs, account names or the file that holds them, internal hostnames and IP addresses, logs and stack traces, and the names of people other than the issue's author. Per post:

| Post | Light version |
|------|---------------|
| triage questions | the questions, rephrased without file or code references |
| plan summary | the expected behaviour in a few sentences, and the open points the client can answer. No files, tests, configuration or risks |
| parts of a split | title and acceptance criteria, as today, checked against the list |
| comment listing the parts | unchanged (issue numbers only) |
| client test steps | the steps, with "the test environment" in place of its URL and "your usual test account" in place of account details |

The developer approves the exact text first. The skill writes the light text to a file in the run folder (`public-<post>.md`), shows it in full, and asks to post, edit or skip. On edit, it takes the developer's version and asks again. On skip, nothing is posted, the skip is logged, and the flow goes on, except for `needs_info`, which still parks the run and tells the developer the client has not been asked. This approval is on top of the plan gate, not in place of it. For the plan, the developer approves the plan first, then the public text.

The PR is public too on a public repository. The handover comment on the PR follows the same list for the test environment URL and account details. Its other sections (files, risks, checklist) stay, because the developer needs them and the code itself is public once the PR is open.

### Stage by stage

- `triage` writes `ticket.md` from `issue-read`, with the same sections as for Jira. "Issue type" becomes the issue type if set, otherwise the labels. "Parent and sub-tickets" come from `parent` and `sub_issues`. "Attachments" become the list of file and image links found in the body and comments, since GitHub has no separate attachment list. A closed issue is refused with a message. A `needs_info` decision posts the questions on the issue.
- `plan` posts the plan summary on the issue. An approved split creates each part with `issue-create-sub`, in order, with the description starting `[ticket-flow] Part <n> of #<parent>`, then comments on the parent with the list of new issue numbers. It reports the new keys as `GH-<n>`. A failure keeps the current rule: stop, list what was created, write `failed_permission`. `plan` gets an `allowed-tools` entry for `forge.sh`, which it lacks today.
- `open-mr` writes `Issue: #142` in the PR body instead of the Jira line. It does not comment on the issue with the PR link, because GitHub already shows the PR in the issue's timeline when the PR body mentions it.
- `handover` applies the public rule to the PR comment. `handover --publish` posts the client steps on the issue. The reminder about attaching files by hand stays.
- `execute`, `ci-gate`, `review` and `mr-loop` change only in their `argument-hint` and in normalising the key.

### Tests

`tests/test-forge.sh` gains a GitHub block with a stubbed `gh` and fixtures, in the style of the GitLab block. It checks at least these cases:

- `issue-read` keeps the author's and collaborators' comments, drops the others and counts them in `dropped_comments`.
- `issue-read` reports `parent` and `sub_issues`, and refuses a pull request.
- `issue-create-sub` sends the database id, not the number, to the sub-issues endpoint.
- A failed attach still prints the new number and exits non-zero.
- `visibility` maps the three values.

A small test covers the key normalisation if it ends up in a script rather than in skill text.

## Dashboard

This amends the dashboard spec of 2026-10-26. Everything not named here stays as written there.

### Projects

A repository with `tracker: github` is a project on its own, named after the repository (`owner/name`). Issues belong to one repository, so there is nothing to group. Repositories with `tracker: jira` (or no `tracker`) are grouped by `jira_project` as today.

The sidebar keeps "Tous les projets" at the top, then two labelled groups: "Jira" with the Jira projects, "GitHub" with the GitHub-tracker repositories. A group with no project is hidden. "Tous les projets" lists both, with the same group labels.

### Tickets and sections

The tickets of a GitHub project are its issues, read with `gh`, so no new login or token. Scope: open issues, and issues closed in the last 14 days, pull requests excluded. Older history is never fetched.

The run folder rules and their overrides do not change. Where the Jira spec uses the Jira status, a GitHub project uses the issue state:

| Jira rule | GitHub version |
|-----------|----------------|
| Jira status in the Done category puts the ticket in Terminé | issue closed as completed puts it in Terminé |
| no run, status To Do, goes to À faire | no run, open issue, goes to À faire |
| no run, status In Progress, goes to En cours | no equivalent: an open issue without a run stays in À faire |
| no run, status Done, goes to Terminé | no run, closed as completed, goes to Terminé |

An issue closed as not planned goes to Terminé with the label "fermée sans suite". The merged-MR override (merged MR with published client steps puts the ticket in Terminé) applies as is.

### Filters and detail panel

- The person filter uses the issue assignees. "Me" is the `gh` login. Avatars come from GitHub logins.
- GitHub projects hide the sprint chips, like a Jira project without a sprint.
- The detail panel shows the issue state and labels where the Jira version shows the Jira status, and links to the issue where it links to the Jira ticket. A small "public" tag shows when the repository is public, so the developer knows posts there need approval.

### Without Jira

GitHub projects work fully without Jira (no `acli` login). The "Jira is not configured" message only concerns the Jira group.

## Build order

The plugin log contract plan (`docs/superpowers/plans/2026-10-26-plugin-log-contract.md`) is not implemented yet and edits the same sections of `conventions.md`, `triage`, `plan` and `handover`. This change is built after it, on top of its version of those sections. The dashboard part is built with the dashboard, after the log contract as the dashboard spec already requires.

## Calls I made that you should check

- No closing keyword in the PR. I first said `Closes #142` could go in the PR body. I changed my mind: closing the issue at merge is a status change, version 1 never changes statuses, and the client test steps are posted after the merge. `Issue: #142` links without closing. Also, `Closes` only acts when the PR targets the default branch, which `develop` usually is not.
- No comment on the issue with the PR link, because GitHub's timeline already shows it.
- Comments from people without write access are dropped, except the issue author's. On a public repository, this also drops comments from the client's colleagues if they are not collaborators.
- `internal` repositories (GitHub Enterprise) are treated as private.
- Visibility is checked live before each post, not stored in the profile.
- On a public repository the PR handover drops only the environment URL and account details, and keeps files and risks. The PR and its code are public anyway once opened.
- In the dashboard, an open issue without a run never shows as En cours, because GitHub has no "in progress" state. Assigned issues stay in À faire.
- No sprint filter for GitHub projects. Milestones could serve later if a client uses them.
- The internal key is `GH-<n>`, and the user can type it three ways.
- `tracker` defaults to `jira` when missing.

## Left to the builder

- Whether key normalisation lives in each skill's text or in a small helper in `lib.sh`. A helper is easier to test.
- The exact `gh api` calls and `jq` filters, as long as the output shapes above hold. Check live: `author_association` for org members with private membership, how the parent of an issue is returned, and sub-issue support on GitHub Enterprise Server.
- The wording changes in the skill and agent descriptions.
- Updating `plugins/ticket-flow/README.md` (prerequisites: `gh` is enough, the Atlassian MCP is only needed with Jira), `decisions.md` (GitHub issues as a second tracker) and the dashboard spec (a pointer to this document).
