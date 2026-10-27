# GitHub issues tracker implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let ticket-flow take its tickets from GitHub issues in the same repository, with a light, developer-approved version of every post on public repositories, and amend the dashboard design and plan so it shows these repositories apart from Jira projects.

**Architecture:** A new `tracker` key in the profile picks Jira or GitHub. A small script normalises the ticket key (`142`, `#142`, `GH-142` give `GH-142`). Four GitHub-only commands in `forge.sh` read, comment and split issues and report the repository's visibility. `conventions.md` gains a tracker table and a "Public issues" section that every posting skill follows. The dashboard is not built yet, so its part is an amendment to its spec and plan.

**Tech Stack:** bash, jq, `gh`, Markdown skills. Offline tests with stubbed CLIs, as in `tests/test-forge.sh`.

**Spec:** [docs/superpowers/specs/2026-10-27-github-issues-tracker-design.md](../specs/2026-10-27-github-issues-tracker-design.md)

**Depends on:** [2026-10-26-plugin-log-contract.md](2026-10-26-plugin-log-contract.md), done first. Tasks 3 and 4 edit the versions of `conventions.md`, `triage`, `plan`, `handover` and `run` that it leaves.

## Global Constraints

- `tracker` is `jira` or `github`. Missing means `jira`. A profile without the key behaves exactly as today.
- `tracker: github` requires `forge: github`. Any other combination with `tracker: github` stops with a message saying it is not supported.
- The internal key for an issue is `GH-<n>`, `n` without leading zeros. The user may type `142`, `#142` or `GH-142` (any case).
- `forge.sh` is the only place that calls `gh`. Skills never call it directly.
- The flow only reads and comments on issues, plus creating split parts. No labels, assignees, milestones, issue types, closing or reopening.
- Comments kept by `issue-read`: `author_association` in `OWNER`, `MEMBER`, `COLLABORATOR`, plus the issue author's. The rest are counted in `dropped_comments`.
- `public` is the only visibility that triggers the light text and the approval. `private` and `internal` post as with Jira.
- The PR body links with `Issue: #<n>`, never a closing keyword. No comment on the issue with the PR link.
- Every issue comment and part description starts with `[ticket-flow]`, in the profile's `client_language`.
- Profile values stay on one line (the `lib.sh` parser).

## Review Focus

- A profile with no `tracker` key, which is every profile written so far: the key is used unchanged and nothing asks about GitHub. Pinned in Task 1 (`jira_key_unchanged_without_tracker`).
- A Jira-looking key, `#`, `0` or an empty argument on a GitHub-tracker repository: a clear refusal, never a run folder named `GH-` or `GH-0`. Pinned in Task 1.
- An issue number that is a pull request: `issue-read` refuses it instead of snapshotting a PR as a ticket. Pinned in Task 2.
- An issue with no parent: GitHub answers 404 on `/parent`, which must give `parent: null`, not a failure. Pinned in Task 2.
- The attach step of a split failing after the issue was created: the number is still printed so the skill can report an orphan issue. Pinned in Task 2.

---

### Task 1: Tracker in the profile and key normalisation

**Files:**
- Modify: `plugins/ticket-flow/scripts/lib.sh` (add `profile_tracker`)
- Create: `plugins/ticket-flow/scripts/ticket-key.sh`
- Create: `plugins/ticket-flow/tests/test-ticket-key.sh`

**Interfaces:**
- Produces: `profile_tracker <profile-file>` in `lib.sh`, prints `jira` when the key is missing or empty, else the value as written. `bash scripts/ticket-key.sh <arg>`, run from inside the repository, prints the normalised key on stdout and exits 0, or prints a one-line reason on stderr and exits 1. Tasks 3 and 4 call it as the first step of every skill that takes a key.

- [ ] **Step 1: Write the failing tests** in `tests/test-ticket-key.sh`, in the style of `tests/test-forge.sh` (temp git repo, a `check` helper, a profile written per case, exit status = failure count):
  - `no_profile_fails`: no `.claude/ticket-flow.md` gives exit 1.
  - `jira_key_unchanged_without_tracker`: profile `forge: gitlab` only, `SHOP-142` prints `SHOP-142`.
  - `jira_key_unchanged_with_tracker_jira`: `tracker: jira`, `shop-142` prints `shop-142`.
  - `github_number`: `forge: github`, `tracker: github`, `142` prints `GH-142`.
  - `github_hash_number`: `#142` prints `GH-142`.
  - `github_key_any_case`: `gh-142` prints `GH-142`.
  - `github_leading_zeros`: `GH-0142` prints `GH-142`.
  - `github_refuses_jira_key`: `SHOP-142` gives exit 1.
  - `github_refuses_zero_and_empty`: `0`, `#` and `""` each give exit 1.
  - `github_tracker_on_gitlab_refused`: `forge: gitlab`, `tracker: github`, `142` gives exit 1 and stderr contains `not supported`.
  - `unknown_tracker_refused`: `tracker: linear` gives exit 1.
- [ ] **Step 2: Run** `bash plugins/ticket-flow/tests/test-ticket-key.sh`. Expected: FAIL (script missing).
- [ ] **Step 3: Implement** `profile_tracker` with `profile_get`, and `ticket-key.sh` sourcing `lib.sh` and using `profile_path "$PWD"`. Forge for the combination check comes from `profile_get forge`. Use `set -euo pipefail` and a usage line like `forge.sh`.
- [ ] **Step 4: Run** the same command. Expected: every line `ok`, exit 0. Also run `bash plugins/ticket-flow/tests/test-forge.sh` and `test-checks.sh` to see `lib.sh` still works for them.
- [ ] **Step 5: Commit** `feat(ticket-flow): tracker key and ticket key normalisation`.

### Task 2: GitHub issue commands in forge.sh

**Files:**
- Modify: `plugins/ticket-flow/scripts/forge.sh` (usage text, GitHub section)
- Create: `plugins/ticket-flow/tests/fixtures/gh-issue-142.json`, `gh-issue-142-comments.json`, `gh-issue-142-sub-issues.json`, `gh-issue-150.json`, `gh-issue-150-parent.json`, `gh-issue-143-pr.json`
- Modify: `plugins/ticket-flow/tests/test-forge.sh` (a GitHub block after the GitLab one)

**Interfaces:**
- Consumes: `gh_repo` (existing).
- Produces, all GitHub only (on GitLab the dispatcher's "unknown command" stands):
  - `forge.sh visibility` prints `public`, `private` or `internal`, lower case, from `gh repo view --json visibility`.
  - `forge.sh issue-read <n>` prints one JSON object with exactly these keys: `number`, `url`, `title`, `body`, `author`, `labels` (names), `type` (issue type name or `null`), `state` (`open` or `closed`), `parent` (number or `null`), `sub_issues` (numbers), `comments` (array of `{author, body, at}`), `dropped_comments` (integer). Exits 1 with `forge.sh: #<n> is a pull request` on stderr when the issue object has a `pull_request` key.
  - `forge.sh issue-comment <n> <body-file>` prints the new comment id.
  - `forge.sh issue-create-sub <parent> <title> <body-file>` prints the new issue number. It creates with `POST repos/{repo}/issues`, then attaches with `POST repos/{repo}/issues/<parent>/sub_issues` and `-F sub_issue_id=<id>`, where `<id>` is the `id` field of the created issue. When the attach fails, it still prints the number and exits 1.

Reads use `gh api`: `repos/{repo}/issues/<n>`, `repos/{repo}/issues/<n>/comments` with `--paginate`, `repos/{repo}/issues/<n>/sub_issues`, `repos/{repo}/issues/<n>/parent`. A failing `/parent` call (404 when there is no parent) gives `parent: null`. Bodies are passed from the file as `forge.sh` already does for `gh_upsert_note`.

- [ ] **Step 1: Write the fixtures.** Issue 142: author `client-anna`, labels `bug` and `checkout`, `type: {"name": "Bug"}`, state `open`. Its comments: one by `client-anna` (`author_association: NONE`), one by `dev-bob` (`COLLABORATOR`), one by `random-eve` (`NONE`). Its sub-issues: numbers 150 and 151. Issue 150: `type: null`, no labels, its parent is 142. Issue 143: has a `pull_request` key.
- [ ] **Step 2: Write the failing tests** in the GitHub block. Switch the profile to `forge: github`, add a `gh` stub that logs `$*` to `calls` and answers from the fixtures (`gh repo view --json nameWithOwner -q .nameWithOwner` answers `acme/shop`, `/parent` for 142 exits 1 with a 404 message). Checks:
  - `visibility_maps_public`: stub answers `PUBLIC`, output `public`. Same for `PRIVATE` and `INTERNAL`, through an env var as `PIPELINE_STATUS` does.
  - `issue_read_keeps_author_and_collaborators`: `forge issue-read 142 | jq -r '[.comments[].author] | join(" ")'` is `client-anna dev-bob`, and `.dropped_comments` is `1`.
  - `issue_read_reports_type_labels_state`: `.type`, `.labels | join(",")`, `.state` are `Bug`, `bug,checkout`, `open`.
  - `issue_read_sub_issues`: `.sub_issues | join(" ")` is `150 151`.
  - `issue_without_parent_is_null`: `.parent` for 142 is `null`.
  - `issue_with_parent`: `.parent` for 150 is `142`, and `.type` is `null`.
  - `issue_read_refuses_pr`: `forge issue-read 143` exits 1.
  - `create_sub_sends_database_id`: the stub answers the create call with `{"id": 987654, "number": 152}`. Output is `152`, and `calls` contains `sub_issue_id=987654` and not `sub_issue_id=152`.
  - `failed_attach_still_prints_number`: with `ATTACH_FAIL=1` the stub fails the `sub_issues` POST. Output is `152`, exit status 1.
- [ ] **Step 3: Run** `bash plugins/ticket-flow/tests/test-forge.sh`. Expected: the new checks FAIL, the GitLab ones still pass.
- [ ] **Step 4: Implement** `gh_visibility`, `gh_issue_read`, `gh_issue_comment`, `gh_issue_create_sub`, and add the four lines to `usage`.
- [ ] **Step 5: Run** the same command. Expected: all `ok`, exit 0.
- [ ] **Step 6: Commit** `feat(ticket-flow): read, comment and split GitHub issues in forge.sh`.

### Task 3: Conventions, README and decisions

**Files:**
- Modify: `plugins/ticket-flow/reference/conventions.md` (sections "Inputs", "Jira", "Scripts", "Text written for humans")
- Modify: `plugins/ticket-flow/README.md` (prerequisites, use)
- Modify: `decisions.md` (settled list)

**Interfaces:**
- Consumes: `ticket-key.sh` (Task 1), the four commands (Task 2).
- Produces: the sections Task 4's skills point to by name: "Tracker" and "Public issues".

- [ ] **Step 1: Edit "Inputs".** The ticket key is the result of `bash <plugin>/scripts/ticket-key.sh "$0"`, run first. On exit 1, show its message and stop. `<KEY>` everywhere below means that result. For GitHub, the issue number is `<KEY>` without `GH-`.
- [ ] **Step 2: Replace "Jira" with "Tracker".** Read `tracker` from the profile. The table of the spec ("Tracker calls"), with Jira through the Atlassian MCP as the current section says, and GitHub through `forge.sh`. Keep the rule that version 1 changes no status, extended to issues (no labels, assignees, milestones, types, closing).
- [ ] **Step 3: Add "Public issues".** Before any GitHub issue comment or part creation, run `forge.sh visibility`. Only `public` applies the rest of the section. Copy the never-list and the per-post table from the spec. The approval procedure: write `public-<post>.md` in the run folder (`<post>` is `questions`, `plan`, `part-<n>`, `client-steps`), show it in full, ask post, edit or skip. Edit loops. Skip logs an event with outcome `skipped_public` and continues, except that `needs_info` still parks the run and says the client was not asked. The PR handover comment drops the test environment URL and account details on public repositories.
- [ ] **Step 4: Edit "Scripts"** to list `ticket-key.sh`, and "Text written for humans" to say "ticket comments" instead of "Jira comments".
- [ ] **Step 5: README.** Prerequisites: the Atlassian MCP is needed only with `tracker: jira`, `gh` is enough with `tracker: github`. Use: the three ways to type a GitHub issue, and one line on public repositories asking for approval before posting.
- [ ] **Step 6: decisions.md.** Add to the settled list: GitHub issues as a second tracker, same repository, `tracker` in the profile, light approved posts on public repositories, with a link to the spec.
- [ ] **Step 7: Check** `grep -n "Jira" plugins/ticket-flow/reference/conventions.md` shows only lines inside the tracker table or that name Jira as one of the two trackers.
- [ ] **Step 8: Commit** `docs(ticket-flow): tracker and public issue conventions`.

### Task 4: Skills and agents

**Files:**
- Modify: `plugins/ticket-flow/skills/{run,triage,plan,execute,open-mr,ci-gate,review,mr-loop,handover}/SKILL.md`
- Modify: `plugins/ticket-flow/skills/profile/SKILL.md`
- Modify: `plugins/ticket-flow/agents/triager.md`, `plugins/ticket-flow/agents/planner.md`

**Interfaces:**
- Consumes: `ticket-key.sh` (Task 1), `issue-read`, `issue-comment`, `issue-create-sub`, `visibility` (Task 2), "Tracker" and "Public issues" (Task 3).

- [ ] **Step 1: Every skill that takes a key.** `argument-hint` becomes `"<TICKET>"` (keeping `[--resume]` and `[--publish]` where present). Add `Bash(bash ${CLAUDE_SKILL_DIR}/../../scripts/ticket-key.sh *)` to `allowed-tools`, creating the line where it is missing. `description` says "ticket" instead of "Jira ticket". The first line of the body says the key is the output of `ticket-key.sh` on `$0`. `run` normalises once and passes the normalised key to each stage.
- [ ] **Step 2: triage.** Section 2 reads the ticket as the Tracker section says. With GitHub, map `issue-read` to `ticket.md`: issue type from `type`, else labels. Parent and sub-tickets from `parent` and `sub_issues`. Attachments as the file and image links found in body and comments. A line saying how many comments were dropped when `dropped_comments` is above 0. Refuse a closed issue (`state: closed`) with a message and no run folder. Section 3 `needs_info` posts through the Public issues rule.
- [ ] **Step 3: plan.** Add `allowed-tools` for `forge.sh`. Step 3 posts the summary through the Public issues rule, after the gate's approval on public repositories. "Creating the parts" gets a GitHub branch: `issue-create-sub` per part in order, description starting `[ticket-flow] Part <n> of #<parent>`, then a comment on the parent listing `#<n>` of each part. An exit 1 with a number is reported as "created but not linked". New keys are reported as `GH-<n>`.
- [ ] **Step 4: open-mr.** With GitHub tracker, the body line is `Issue: #<n>` and step 5 is skipped.
- [ ] **Step 5: handover.** The PR comment follows the public rule for the environment URL and accounts. `--publish` posts on the ticket as the Tracker section says, through the Public issues rule.
- [ ] **Step 6: profile.** Add `tracker` to the questions asked in one message. With `github`, do not ask `jira_project` and write it `""`. Add `tracker` to the example frontmatter. With `tracker: github` and a non-GitHub remote, say it is not supported and write `tracker: jira`.
- [ ] **Step 7: Agents.** `triager.md` and `planner.md` say "ticket" instead of "Jira ticket".
- [ ] **Step 8: Check.** `grep -rn "JIRA-KEY" plugins/ticket-flow` prints nothing. `grep -rln "ticket-key.sh" plugins/ticket-flow/skills` lists the nine skills that take a key. `claude plugin validate ./plugins/ticket-flow` passes. Run the three test scripts plus `test-ticket-key.sh` (and `test-log-contract.sh` from the log contract plan). Expected: all pass.
- [ ] **Step 9: Commit** `feat(ticket-flow): run the flow from GitHub issues`.

### Task 5: Dashboard spec and plan amendment

The dashboard is not built yet. This task changes its spec and plan so that building it covers GitHub-tracker repositories. No Rust or TypeScript is written here.

**Files:**
- Modify: `docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md` (sections "Projects", "Screen", the state rules for tickets with no run, "Jira", "Constraints")
- Modify: `docs/superpowers/plans/2026-10-26-ticket-flow-dashboard.md` (Global Constraints, Review Focus, Tasks 3 to 8 and 10)

**Interfaces:**
- Consumes: the "Dashboard" section of the GitHub issues spec.

- [ ] **Step 1: Spec.** Add a "GitHub-tracker repositories" subsection that points to the GitHub issues spec and restates its Dashboard section. In "Projects", say a repository with `tracker: github` is its own project, named `owner/name` from the `origin` remote, and that the sidebar shows a "Jira" group and a "GitHub" group. In the constraints, say GitHub projects need no Jira token.
- [ ] **Step 2: Plan, Global Constraints.** Add: GitHub issues scope is open issues plus issues closed in the last 14 days, pull requests excluded. Issues are read through `gh` in the repository folder.
- [ ] **Step 3: Plan, Task 3.** Add `pub struct GithubIssue { number: u64, title: String, state: IssueState, assignees: Vec<Person>, labels: Vec<String>, url: String }` with `pub enum IssueState { Open, Completed, NotPlanned }`, and `pub enum TrackerTicket { Jira(JiraTicket), Github(GithubIssue) }`. `classify` takes `ticket: Option<&TrackerTicket>` instead of `jira`. Add reasons `IssueTodo`, `IssueDone`, `IssueNotPlanned`. Tests: `issue_completed_overrides_run`, `no_run_open_issue_is_todo`, `no_run_assigned_open_issue_is_still_todo`, `not_planned_is_done_with_its_reason`.
- [ ] **Step 4: Plan, Task 4.** `Repo` gains `tracker: Tracker` (`enum Tracker { Jira, Github }`, missing means `Jira`) and `remote: Option<String>` (`owner/name` parsed from `git remote get-url origin`, read only). `group` puts each GitHub-tracker repo in its own `ProjectDef` keyed `owner/name`, and `ProjectDef` gains `tracker`. Tests: `tracker_defaults_to_jira`, `github_tracker_repo_is_its_own_project`, `ssh_and_https_remotes_parse`.
- [ ] **Step 5: Plan, Task 5.** Add `pub fn issues(r: &dyn Runner, repo: &Repo, now: DateTime<Utc>) -> Result<Vec<GithubIssue>, String>` with two calls, `gh issue list --state open` and `gh issue list --state closed --search "closed:>=<date 14 days back>"`, both `--json number,title,state,stateReason,assignees,labels,url --limit 500`, plus `pub fn visibility(r, repo) -> Result<String, String>` and `pub fn gh_login(r, dir) -> Result<Person, String>`. Note for the implementer: check `stateReason` is accepted by the installed `gh` and fall back to `state` alone if not. Tests: `parses_issue_list`, `not_planned_maps_to_not_planned`.
- [ ] **Step 6: Plan, Task 7.** `Ticket.jira_url` and `jira_status` become `ticket_url` and `ticket_status` (the issue state for GitHub). `ProjectView` gains `tracker` and `public: bool`. GitHub projects get `has_sprints = false`. "Me" for GitHub projects is the `gh` login. Issues are fetched with the forge poll and keep their last data on error like the rest. Tests: `github_project_without_jira_token`, `github_ticket_links_to_issue`.
- [ ] **Step 7: Plan, Tasks 8 and 10.** Sidebar and "Tous les projets" group projects under "Jira" and "GitHub", hiding an empty group. The detail panel shows the issue state and labels and links to the issue for GitHub projects, and a "public" tag when `public` is true. New copy keys in French and English: the two group labels, `issue_todo`, `issue_done`, `issue_not_planned` ("fermée sans suite" / "closed as not planned"), `public`.
- [ ] **Step 8: Plan, Review Focus.** Add: a GitHub-tracker repo whose `origin` is not on github.com is listed under its folder name with local runs only, never a crash.
- [ ] **Step 9: Check** `grep -n "jira_url\|jira_status" docs/superpowers/plans/2026-10-26-ticket-flow-dashboard.md` prints nothing, and every new type name is defined in the task that produces it.
- [ ] **Step 10: Commit** `docs(dashboard): cover GitHub-tracker repositories`.
