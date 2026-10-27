# Ticket Flow Dashboard implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build version 1 of the read-only desktop dashboard that shows each developer where their ticket-flow tickets stand.

**Architecture:** A Tauri 2 app in `dashboard/app/`. The Rust side does all reading and deciding: it finds repositories, parses run folders, asks `gh`/`glab` and the Jira REST API, classifies every ticket into a section, and pushes one `Snapshot` to the front end through a command and an event. The React front end only filters (person, sprint), groups and renders, using the design system in `dashboard/design-system/`.

**Tech Stack:** Tauri 2, Rust (serde, chrono, reqwest with rustls, notify, keyring, tempfile for tests), React 18 + TypeScript + Vite, Vitest + Testing Library, pnpm, `@tauri-apps/plugin-opener`, Geist fonts from npm.

**Spec:** [docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md](../specs/2026-10-26-ticket-flow-dashboard-design.md). Product record: [dashboard/PRODUCT.md](../../../dashboard/PRODUCT.md). Design rules: [dashboard/design-system/README.md](../../../dashboard/design-system/README.md).

**Amended by:** [the GitHub issues design](../specs/2026-10-27-github-issues-tracker-design.md#dashboard), which adds GitHub-tracker repositories (Tasks 3 to 8 and 10 below already include it).

**Depends on:** [2026-10-26-plugin-log-contract.md](2026-10-26-plugin-log-contract.md), done first. Its fixture `plugins/ticket-flow/tests/fixtures/log-contract.jsonl` is this plan's first run fixture.

## Global Constraints

- Read only. The app never writes in a repository, a run folder, Jira or the forge.
- Never show, log or send a token. The Jira token lives only in the OS keychain (service `ticket-flow-dashboard`, account = Jira email).
- Forge access goes through the `gh` and `glab` CLIs run in the repository's folder. No forge token is stored.
- Dark theme only. Every colour, size and radius comes from `dashboard/design-system/tokens.css` and `components/components.css`. No literal colours in TSX or new CSS.
- Text contrast 4.5:1 or more, control borders and focus ring 3:1 or more, state always shown with a word.
- Fonts ship with the app. No request to Google Fonts at runtime.
- UI copy in French and English, chosen per developer. French is the default. Stage names and end states stay in English, lower case.
- Polling Jira and the forge every 120 seconds while the window is visible. Local files re-read within 3 seconds of a change.
- A run with no new log line for 30 minutes, whose last line is a `start`, is flagged stale. Waits never count as stale.
- Jira scope per project: open and future sprints, backlog not done, and tickets done in the last 14 days.
- GitHub issues scope per GitHub-tracker repository: open issues, and issues closed in the last 14 days, pull requests excluded. Issues are read through `gh` in the repository folder.
- Section order: En cours, En attente, À faire, Terminé. The first two open by default.

## Review Focus

- A log line cut in half (the plugin writing while the watcher reads): skip it, keep the rest, re-read on the next change. Pinned in Task 2.
- A repository whose profile has no `jira_project`, or a malformed frontmatter: list the repository's local runs under a project named after the folder, never crash. Pinned in Task 4.
- `gh` or `glab` not installed or not logged in: the forge source shows an error with the CLI name, tickets still render without MR data. Pinned in Task 5.
- A GitHub-tracker repository whose `origin` is not on github.com, or has no `origin`: it is listed under its folder name with its local runs only, never a crash. Pinned in Task 4.
- Jira returns 401 or the site is unreachable: keep the last snapshot, mark Jira stale with its age, keep local data live. Pinned in Task 7.
- A ticket key present locally but unknown to Jira (deleted, other project, no token): it still shows with its run, summary taken from `ticket.md`'s first heading. Pinned in Task 7.

---

### Task 1: Scaffold the app with the design system

**Files:**
- Create: `dashboard/app/` (from create-tauri-app, React + TypeScript, pnpm)
- Modify: `dashboard/app/vite.config.ts` (allow `..` in `server.fs.allow`, Vitest config with `jsdom`)
- Modify: `dashboard/app/src-tauri/tauri.conf.json` (product name `Ticket Flow Dashboard`, identifier `dev.oschweitzer.ticketflow.dashboard`, window 1440×900, min 1100×700, title `Ticket Flow`)
- Modify: `dashboard/design-system/components/components.css` (delete the Google Fonts `@import` line)
- Modify: `dashboard/design-system/README.md` (the fonts paragraph: fonts now come from npm packages in the app)
- Create: `dashboard/app/src/main.tsx`, `dashboard/app/src/App.tsx`, `dashboard/app/src/App.test.tsx`
- Modify: `.gitignore` (add `dashboard/app/node_modules/`, `dashboard/app/dist/`, `dashboard/app/src-tauri/target/`)

**Interfaces:**
- Produces: `pnpm test` (Vitest) and `cargo test --manifest-path dashboard/app/src-tauri/Cargo.toml` both runnable. `main.tsx` imports, in order: the Geist and Geist Mono font packages, `../../design-system/tokens.css`, `../../design-system/components/components.css`.

- [ ] **Step 1: Scaffold.** From `dashboard/`, run `pnpm create tauri-app app --template react-ts --manager pnpm --identifier dev.oschweitzer.ticketflow.dashboard --yes` (check `pnpm create tauri-app --help` if a flag was renamed). Then `pnpm install` in `dashboard/app`. Delete the template's demo content, CSS and logo assets.
- [ ] **Step 2: Fonts.** Add `@fontsource-variable/geist` and `@fontsource-variable/geist-mono` (if either name does not resolve on npm, use the `geist` package from Vercel and import its CSS). Update `tokens.json`'s families only if the family names differ from `Geist` and `Geist Mono`, then regenerate the two lines in `tokens.css` by hand.
- [ ] **Step 3: Write the failing test** `App.test.tsx`: renders `<App />` and expects `screen.getByText('Ticket Flow')`.
- [ ] **Step 4: Run** `pnpm test` in `dashboard/app`. Expected: FAIL (App still renders the template or nothing).
- [ ] **Step 5: Implement** `App()` returning a `<div className="app">` with the brand text `Ticket Flow`. Add Vitest, jsdom and `@testing-library/react` as dev dependencies.
- [ ] **Step 6: Run** `pnpm test` (PASS), `cargo test --manifest-path src-tauri/Cargo.toml` (PASS, zero tests), and `pnpm tauri dev` once by hand: the window opens on the dark ground with "Ticket Flow" in Geist.
- [ ] **Step 7: Commit** `feat(dashboard): scaffold Tauri app with design system and fonts`.

### Task 2: Parse a run folder

**Files:**
- Create: `dashboard/app/src-tauri/src/runs.rs`
- Create: `dashboard/app/src-tauri/tests/fixtures/runs/` with folders `SHOP-142/` (copy of the plugin fixture as `log.jsonl`, plus `mr.json`, `triage.json`, `plan.json`, `ticket.md`), `SHOP-139/` (parked), `SHOP-133/` (escalated_ci), `SHOP-146/` (execute started, no end), `SHOP-144/` (plan waiting on developer), `SHOP-150/` (last line cut in half)
- Test: `dashboard/app/src-tauri/src/runs.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces, all `#[derive(Debug, Clone, Serialize)]` with `#[serde(rename_all = "camelCase")]`:

```rust
pub const STAGES: [&str; 8] = ["triage", "plan", "execute", "open-mr", "ci-gate", "review", "mr-loop", "handover"];

pub enum EventKind { Start, End, Waiting, EndState, Published }          // from "start" ... "published"
pub struct LogLine { pub at: DateTime<Utc>, pub stage: String, pub event: EventKind,
                     pub outcome: Option<String>, pub on: Option<String>, pub model: Option<String> }
pub struct StepInfo { pub name: String, pub state: StepState, pub note: Option<String> } // note: "outcome · HH:MM"
pub enum StepState { Done, Now, Fail, Todo }
pub struct MrRef { pub number: u64, pub url: String, pub branch: String }
pub struct TriageInfo { pub size: Option<String>, pub risk: Vec<String> }
pub struct PlanSummary { pub behaviour: Option<String>, pub files: Vec<String>, pub planner: Option<String>, pub executor: Option<String> }
pub struct RunState {
    pub key: String, pub run_dir: PathBuf, pub lines: Vec<LogLine>,
    pub stage_index: u8,            // 1..=8, index of the last started stage
    pub end_state: Option<String>,  // outcome of the last end_state line, cleared by a later start
    pub waiting_on: Option<String>, // `on` of a final waiting line
    pub published: bool,            // a published line after the last end_state
    pub last_at: DateTime<Utc>, pub last_is_start: bool,
    pub mr: Option<MrRef>, pub triage: Option<TriageInfo>, pub plan: Option<PlanSummary>,
    pub summary: Option<String>,    // first "# " heading of ticket.md, without the "# "
}
pub fn read_run(dir: &Path) -> Option<RunState>;           // None when log.jsonl is missing or has no valid line
pub fn read_runs(repo: &Path) -> Vec<RunState>;            // every folder under <repo>/.ticket-flow/runs/
pub fn steps(run: &RunState, ci_failed: bool) -> Vec<StepInfo>;
```

- [ ] **Step 1: Write the fixtures** from the plugin contract. `SHOP-150/log.jsonl` ends with `{"at":"2026-10-26T11:00:00Z","stage":"exe` and no newline.
- [ ] **Step 2: Write the failing tests:** `reads_full_run` (SHOP-142: `stage_index == 8`, `end_state == Some("ready_for_qa")`, `published`, `mr.number == 87`, `plan.planner == Some("sonnet")`), `reads_parked` (`end_state == Some("parked_needs_info")`), `reads_waiting_developer` (SHOP-144: `waiting_on == Some("developer")`, `stage_index == 2`), `reads_running` (SHOP-146: `stage_index == 3`, `last_is_start`), `skips_cut_line` (SHOP-150: one line fewer than the file has, no panic), `start_after_end_state_reopens` (inline log: `end_state parked_needs_info` then `triage start` gives `end_state == None`), `steps_mark_failure` (`steps(&shop146, true)[2].state == StepState::Fail`, earlier steps `Done`, later `Todo`), `summary_from_ticket_md`.
- [ ] **Step 3: Run** `cargo test runs::` and see them fail (module missing).
- [ ] **Step 4: Implement** with `serde_json::from_str` per line, skipping lines that fail to parse. `planner` is the `model` of the `plan` start line, falling back to `plan.json`'s `planner`. `executor` is the `execute` start line's `model`, falling back to `plan.json`'s `executor`. `waiting_on` is set only when the last line is `waiting`.
- [ ] **Step 5: Run** `cargo test runs::`. Expected: all PASS.
- [ ] **Step 6: Commit** `feat(dashboard): parse ticket-flow run folders`.

### Task 3: Classify a ticket into a section

**Files:**
- Create: `dashboard/app/src-tauri/src/classify.rs`
- Test: same file

**Interfaces:**
- Consumes: `RunState` (Task 2). `MrStatus`, `JiraTicket`, `GithubIssue` and `TrackerTicket` are defined here as plain structs and reused by Tasks 5 and 6:

```rust
pub enum MrState { Open, Merged, Closed }
pub enum Ci { Green, Red, Running, None }
pub struct MrStatus { pub number: u64, pub url: String, pub state: MrState, pub draft: bool, pub ci: Ci, pub pipeline_url: Option<String> }
pub enum StatusCategory { Todo, InProgress, Done }
pub struct Person { pub account_id: String, pub display_name: String }
pub struct JiraTicket { pub key: String, pub summary: String, pub issue_type: String, pub status: String,
                        pub status_category: StatusCategory, pub assignee: Option<Person>,
                        pub sprint_id: Option<u64>, pub url: String }
pub enum IssueState { Open, Completed, NotPlanned }
pub struct GithubIssue { pub number: u64, pub title: String, pub state: IssueState, pub assignees: Vec<Person>,
                         pub labels: Vec<String>, pub url: String }
pub enum TrackerTicket { Jira(JiraTicket), Github(GithubIssue) }

pub enum Section { InProgress, Waiting, Todo, Done }
pub enum WaitingOn { You, Client }
pub enum Reason { PlanToApprove, MrToMerge, ClientStepsToPublish, ParkedNeedsInfo, PlanRejected,
                  EscalatedChecks, EscalatedCi, EscalatedComments, Timeout, FailedPermission,
                  Running, WaitingCi, WaitingReviews, ReadyForQa, Split, MrClosed,
                  JiraTodo, JiraInProgress, JiraDone, IssueTodo, IssueDone, IssueNotPlanned }        // serde snake_case: these are i18n keys
// serde: Section, WaitingOn, MrState, Ci, StatusCategory, IssueState use rename_all = "camelCase", Reason uses "snake_case"
pub struct Placement { pub section: Section, pub waiting_on: Option<WaitingOn>, pub reason: Reason, pub stale_minutes: Option<i64> }

pub fn classify(run: Option<&RunState>, ticket: Option<&TrackerTicket>, mr: Option<&MrStatus>, now: DateTime<Utc>) -> Placement;
```

- [ ] **Step 1: Write one failing test per row of the spec's state table** (`docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md`, "How states are read") and per override: `waiting_developer_is_plan_to_approve`, `waiting_ci_is_in_progress`, `ready_for_qa_open_mr_is_mr_to_merge`, `ready_for_qa_merged_is_client_steps`, `ready_for_qa_published_is_done`, `split_is_done`, `parked_is_waiting_client`, `each_blocked_end_state_waits_on_you` (loop over the six), `running_is_in_progress`, `stale_after_30_minutes` (last `start` 45 minutes before `now` gives `stale_minutes == Some(45)`), `waits_are_never_stale`, `jira_done_overrides_run`, `closed_mr_is_mr_closed`, `no_run_follows_jira_category` (three cases), `issue_completed_overrides_run`, `no_run_open_issue_is_todo`, `no_run_assigned_open_issue_is_still_todo`, `not_planned_is_done_with_its_reason`. Build `RunState` values inline with a small test helper.
- [ ] **Step 2: Run** `cargo test classify::`. Expected: FAIL.
- [ ] **Step 3: Implement** `classify`. Order: Jira Done override or issue closed as completed, then merged-and-published, then closed MR, then the run table, then, when there is no run, the Jira category or the issue state (open gives `IssueTodo` in À faire, completed `IssueDone` and not planned `IssueNotPlanned`, both in Terminé).
- [ ] **Step 4: Run** `cargo test classify::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): classify tickets into sections`.

### Task 4: Find repositories and projects

**Files:**
- Create: `dashboard/app/src-tauri/src/repos.rs`
- Test: same file, with `tempfile::TempDir`

**Interfaces:**
- Produces:

```rust
pub enum Forge { Github, Gitlab }
pub enum Tracker { Jira, Github }                                 // missing or unknown in the profile gives Jira
pub struct Repo { pub name: String, pub path: PathBuf, pub forge: Option<Forge>, pub tracker: Tracker,
                  pub jira_project: Option<String>, pub remote: Option<String> } // remote = "owner/name" from origin, github.com only
pub struct ProjectDef { pub key: String, pub tracker: Tracker, pub repos: Vec<Repo> } // key = jira_project, else the folder name upper-cased. GitHub tracker: remote, else the folder name
pub fn read_profile(repo: &Path) -> Option<(Option<Forge>, Tracker, Option<String>)>; // None when .claude/ticket-flow.md is missing
pub fn parse_remote(url: &str) -> Option<String>;                 // owner/name from an https or ssh github.com URL
pub fn discover(roots: &[PathBuf]) -> Vec<Repo>;                  // depth 3 below each root
pub fn group(repos: Vec<Repo>) -> Vec<ProjectDef>;                // sorted by key
```

- [ ] **Step 1: Write the failing tests:** `reads_profile_frontmatter` (frontmatter with `forge: gitlab   # detected` and `jira_project: SHOP` gives `(Some(Gitlab), Some("SHOP"))`, using the same comment-stripping case as `plugins/ticket-flow/tests/test-checks.sh`), `profile_without_jira_project`, `malformed_frontmatter_is_not_fatal` (no closing `---`: returns `(None, None)`), `discovers_only_repos_with_profile`, `skips_heavy_folders` (a profile inside `node_modules/x/` is ignored), `groups_by_jira_project` (two repos with `SHOP` give one project with two repos), `tracker_defaults_to_jira`, `github_tracker_repo_is_its_own_project` (two GitHub-tracker repos with the same empty `jira_project` give two projects), `ssh_and_https_remotes_parse` (`git@github.com:acme/shop.git` and `https://github.com/acme/shop` give `acme/shop`, a gitlab.com URL gives `None`).
- [ ] **Step 2: Run** `cargo test repos::`. Expected: FAIL.
- [ ] **Step 3: Implement.** Read only the lines between the first two `---` lines, take top-level `key: value` pairs, strip a trailing `# comment` and surrounding quotes. No YAML crate. Skip folders named `node_modules`, `target`, `dist`, `vendor`, and any starting with `.` except `.claude` when reading the profile. A folder that holds a profile is not searched further. `remote` comes from reading `.git/config` (or the `gitdir` a `.git` file points to), never from running git.
- [ ] **Step 4: Run** `cargo test repos::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): discover repositories and group projects`.

### Task 5: Read MR state through gh and glab

**Files:**
- Create: `dashboard/app/src-tauri/src/forge.rs`
- Create: `dashboard/app/src-tauri/tests/fixtures/forge/` (`gh-pr-view.json`, `gh-pr-checks-fail.json`, `gh-pr-list.json`, `glab-mr.json`, `glab-mr-list.json`, `gh-issue-list.json`)
- Test: same file

**Interfaces:**
- Consumes: `MrStatus`, `MrState`, `Ci` (Task 3), `Forge`, `Repo` (Task 4), `MrRef` (Task 2).
- Produces:

```rust
pub trait Runner: Send + Sync { fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String>; }
pub struct SystemRunner;                                      // std::process::Command, stdout on success, "<program>: <stderr>" on failure
pub fn mr_status(r: &dyn Runner, repo: &Repo, mr: Option<&MrRef>, key: &str) -> Result<Option<MrStatus>, String>;
pub fn parse_gh(view: &str, checks: &str) -> Option<MrStatus>;
pub fn parse_glab(mr: &str) -> Option<MrStatus>;
pub fn issues(r: &dyn Runner, repo: &Repo, now: DateTime<Utc>) -> Result<Vec<GithubIssue>, String>;
pub fn parse_issues(list: &str) -> Vec<GithubIssue>;
pub fn visibility(r: &dyn Runner, repo: &Repo) -> Result<String, String>;   // gh repo view --json visibility, lower case
pub fn gh_login(r: &dyn Runner, dir: &Path) -> Result<Person, String>;     // gh api user, account_id = login
```

Commands, run in the repository folder. GitHub: `gh pr view <n> --json number,url,state,isDraft` and `gh pr checks <n> --json bucket,link` (it exits non-zero while checks fail or run, keep its stdout), or without `MrRef`: `gh pr list --state all --search <KEY> --json number,headRefName --limit 20`, keeping the first whose `headRefName` contains the key, case-insensitive. GitLab: `glab api projects/:id/merge_requests/<n>` (`state`, `draft`, `web_url`, `head_pipeline.status`, `head_pipeline.web_url`), or without `MrRef`: `glab api --method GET projects/:id/merge_requests -f state=all -f search=<KEY> -f per_page=20`, same branch rule on `source_branch`. CI mapping: GitHub buckets with any `fail` give Red, any `pending` Running, all `pass` or `skipping` Green, none at all None. GitLab `success` Green, `failed` Red, `running`, `pending`, `created` Running, missing pipeline None, anything else Red.

`issues` runs `gh issue list --state open` and `gh issue list --state closed --search "closed:>=<date 14 days before now>"`, both with `--json number,title,state,stateReason,assignees,labels,url --limit 500`. `stateReason` `NOT_PLANNED` gives `NotPlanned`, any other closed issue `Completed`. Check that the installed `gh` accepts `stateReason`. If it does not, drop the field and treat every closed issue as `Completed`.

- [ ] **Step 1: Write the failing tests:** `parses_gh_open_red`, `parses_gh_merged_green`, `parses_glab_running_draft`, `finds_mr_by_branch` (a fake `Runner` returning the list fixture, key `shop-142` matches branch `feat/SHOP-142-tva`), `no_match_is_none`, `missing_cli_is_an_error` (fake runner returning `Err("gh: not found")` gives `Err` containing `gh`), `parses_issue_list`, `not_planned_maps_to_not_planned`.
- [ ] **Step 2: Run** `cargo test forge::`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test forge::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): read MR and CI state through gh and glab`.

### Task 6: Read Jira and keep the settings

**Files:**
- Create: `dashboard/app/src-tauri/src/jira.rs`, `dashboard/app/src-tauri/src/settings.rs`
- Create: `dashboard/app/src-tauri/tests/fixtures/jira/` (`myself.json`, `boards.json`, `sprints.json`, `sprint-issues.json`, `backlog.json`, `search.json`)
- Test: both files

**Interfaces:**
- Consumes: `JiraTicket`, `Person`, `StatusCategory` (Task 3).
- Produces:

```rust
// settings.rs
pub struct Settings { pub roots: Vec<String>, pub jira_url: String, pub jira_email: String, pub language: String } // "fr" | "en", default "fr"
pub fn load(config_dir: &Path) -> Settings;                     // settings.json, defaults when missing or invalid
pub fn save(config_dir: &Path, s: &Settings) -> Result<(), String>;
pub fn token(email: &str) -> Option<String>;                    // keyring service "ticket-flow-dashboard"
pub fn set_token(email: &str, token: &str) -> Result<(), String>;

// jira.rs
pub struct Sprint { pub id: u64, pub name: String, pub state: String }   // "active" | "future"
pub struct JiraProject { pub key: String, pub sprints: Vec<Sprint>, pub has_sprints: bool, pub tickets: Vec<JiraTicket> }
pub struct JiraClient { /* base url, email, token, reqwest::Client */ }
impl JiraClient {
    pub fn new(url: &str, email: &str, token: &str) -> Self;
    pub async fn myself(&self) -> Result<Person, String>;                   // GET /rest/api/3/myself
    pub async fn project(&self, key: &str) -> Result<JiraProject, String>;
}
pub fn parse_issue(v: &serde_json::Value, base_url: &str, sprint_id: Option<u64>) -> Option<JiraTicket>;
```

`project(key)`: `GET /rest/agile/1.0/board?projectKeyOrId=<key>&type=scrum`. With scrum boards: for each, `GET /rest/agile/1.0/board/<id>/sprint?state=active,future`, then `GET .../sprint/<sid>/issue?fields=summary,status,issuetype,assignee`, then `GET .../board/<id>/backlog?jql=statusCategory != Done&fields=...`. Every board's results are merged and deduplicated by key. Then, for every project, `GET /rest/api/3/search/jql?jql=project = <key> AND statusCategory = Done AND statusCategoryChangedDate >= -14d&fields=...` (paging with `nextPageToken`). With no scrum board: `has_sprints = false` and one search `project = <key> AND (statusCategory != Done OR statusCategoryChangedDate >= -14d)`. `status_category` maps `new` to Todo, `indeterminate` to InProgress, `done` to Done. `url` is `<base>/browse/<KEY>`. Basic auth with email and token. A 401 gives `Err("jira: 401")`.

- [ ] **Step 1: Write the failing tests:** `parses_issue_with_assignee`, `parses_unassigned_issue`, `maps_status_categories`, `settings_default_when_missing` (language `fr`, no roots), `settings_round_trip` (temp dir), `kanban_project_has_no_sprints` (parse the boards fixture with an empty `values`). HTTP calls themselves are not unit-tested. Keep request building and response parsing in separate functions so the parsing is.
- [ ] **Step 2: Run** `cargo test jira:: settings::`. Expected: FAIL.
- [ ] **Step 3: Implement.** `reqwest` with `rustls-tls` and `json` features, no default features.
- [ ] **Step 4: Run** `cargo test jira:: settings::`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): Jira client and settings with keychain token`.

### Task 7: Build the snapshot, watch, poll, expose commands

**Files:**
- Create: `dashboard/app/src-tauri/src/snapshot.rs`, `dashboard/app/src-tauri/src/sync.rs`
- Modify: `dashboard/app/src-tauri/src/lib.rs` (modules, commands, setup, opener plugin)
- Modify: `dashboard/app/src-tauri/capabilities/default.json` (opener: `opener:allow-open-url` for `https://*`, `opener:allow-open-path` for `**`)
- Test: `snapshot.rs`

**Interfaces:**
- Consumes: everything from Tasks 2 to 6.
- Produces the payload the front end reads (serde camelCase):

```rust
pub struct Track { pub current: u8, pub total: u8, pub stage: String, pub failed: bool }
pub struct Ticket {
    pub key: String, pub project: String, pub summary: String, pub issue_type: Option<String>,
    pub repo: Option<String>, pub sprint_id: Option<u64>, pub assignee: Option<Person>,
    pub local_run: bool, pub placement: Placement, pub track: Option<Track>, pub steps: Vec<StepInfo>,
    pub mr: Option<MrStatus>, pub ticket_url: Option<String>, pub ticket_status: Option<String>, // Jira status, or the issue state
    pub labels: Vec<String>,                                                                     // GitHub issues only
    pub triage: Option<TriageInfo>, pub plan: Option<PlanSummary>,
    pub run_dir: Option<String>, pub plan_path: Option<String>, pub handover_path: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
}
pub struct ProjectView { pub key: String, pub tracker: Tracker, pub repos: Vec<String>, pub forge: Option<Forge>, pub public: bool,
                         pub sprints: Vec<Sprint>, pub has_sprints: bool } // GitHub tracker: has_sprints false
pub struct SourceState { pub configured: bool, pub ok: bool, pub error: Option<String>, pub last_ok_at: Option<DateTime<Utc>> }
pub struct Snapshot { pub me: Option<Person>, pub projects: Vec<ProjectView>, pub tickets: Vec<Ticket>,
                      pub people: Vec<Person>, pub jira: SourceState, pub forge: SourceState, pub synced_at: DateTime<Utc> }

pub fn build(projects: &[ProjectDef], runs: &[(String, RunState)],      // (repo name, run)
             jira: &[JiraProject], issues: &HashMap<String, Vec<GithubIssue>>, // issues by project key
             mrs: &HashMap<String, MrStatus>,                            // by ticket key
             me: Option<&Person>, now: DateTime<Utc>) -> Vec<Ticket>;
```

Tauri commands: `get_snapshot() -> Snapshot`, `refresh()`, `get_settings() -> SettingsView` (`Settings` plus `has_token: bool`), `save_settings(settings: Settings, token: Option<String>)`. Event `snapshot` with a `Snapshot` payload, emitted after every rebuild.

`sync.rs` holds the state in a `tokio::sync::Mutex`. Local rebuilds come from a `notify` watcher on each repo's `.ticket-flow/runs/` (recursive, debounced 500 ms). Remote fetches run every 120 s, and on `refresh` and `save_settings`. Remote fetches pause while the main window is hidden or minimised. On a remote error, the previous Jira or forge data is kept and only the `SourceState` changes. Without settings or token, `jira.configured = false` and only local runs are listed for Jira projects. GitHub projects fetch their issues and visibility with the forge poll whatever the Jira settings, and keep their last data on error like the rest. Their ticket keys are `GH-<number>`, matching the run folders, and "me" for them is the `gh` login.

- [ ] **Step 1: Write the failing tests:** `local_run_without_jira_uses_ticket_md_summary`, `local_run_is_mine_whatever_the_assignee` (the ticket's `assignee` stays the Jira one and `local_run` is true. The front end relies on that flag), `jira_only_ticket_has_no_track`, `track_marks_failed_when_ci_red` (run at ci-gate and MR `Ci::Red` gives `track.failed`), `people_are_unique_and_sorted`, `paths_point_into_run_dir` (`plan_path` ends with `plan.json`, `handover_path` only when `handover.md` exists), `github_project_without_jira_token` (a GitHub-tracker project lists its issues with `jira.configured = false`), `github_ticket_links_to_issue` (`ticket_url` is the issue URL, `ticket_status` is `open`, the key is `GH-142`).
- [ ] **Step 2: Run** `cargo test snapshot::`. Expected: FAIL.
- [ ] **Step 3: Implement** `build`, then `sync.rs` and the commands. Register `tauri_plugin_opener::init()`.
- [ ] **Step 4: Run** `cargo test`. Expected: all Rust tests PASS.
- [ ] **Step 5: Manual check.** Create a temp root holding one git repo with a `.claude/ticket-flow.md` and a copy of `tests/fixtures/runs/SHOP-146`, point the settings at it through `save_settings` from the dev tools console, and watch `get_snapshot` return the ticket. Append a line to its `log.jsonl` and see a `snapshot` event within 3 seconds.
- [ ] **Step 6: Commit** `feat(dashboard): snapshot, file watching, polling and commands`.

### Task 8: Front-end data, filters and copy

**Files:**
- Create: `dashboard/app/src/types.ts` (TypeScript mirror of the Task 7 payload, same names in camelCase)
- Create: `dashboard/app/src/i18n.ts`, `dashboard/app/src/prefs.ts`, `dashboard/app/src/filter.ts`, `dashboard/app/src/useSnapshot.ts`
- Test: `dashboard/app/src/filter.test.ts`, `dashboard/app/src/prefs.test.ts`

**Interfaces:**
- Consumes: `Snapshot` (Task 7), via `invoke('get_snapshot')` and `listen('snapshot')`.
- Produces:

```ts
export type Lang = 'fr' | 'en';
export function t(lang: Lang, key: string, vars?: Record<string, string | number>): string; // unknown key returns the key
export interface ProjectPrefs { people: string[]; sprints: (number | 'backlog')[]; open: Section[] } // people: accountIds, '__me__', '__unassigned__'
export function loadPrefs(project: string, p: ProjectView): ProjectPrefs;   // defaults: ['__me__'], active sprints (all, or [] when !hasSprints), ['inProgress','waiting']
export function savePrefs(project: string, prefs: ProjectPrefs): void;      // localStorage key "tf.prefs.<project>", try/catch
export function visible(tickets: Ticket[], project: string, prefs: ProjectPrefs, me: Person | null, query: string): Ticket[]; // query: case-insensitive match on key or summary, '' matches all
export function bySection(tickets: Ticket[]): Record<Section, Ticket[]>;   // order inside: waiting on you first, then by lastActivity desc
export function counts(tickets: Ticket[]): { you: number; client: number; inProgress: number; ciRed: number; todo: number; done: number };
export function useSnapshot(): { snapshot: Snapshot | null; refresh(): Promise<void> };
```

Rules in `visible`: a ticket with `localRun` passes the person filter whenever `'__me__'` is selected. `'__unassigned__'` matches `assignee == null` without a local run. Sprint filter: `sprintId` in the selection, or `'backlog'` selected and `sprintId == null`. When `hasSprints` is false, the sprint filter is skipped.

Copy keys: one per `Reason` (French and English, French text exactly as in the spec: "plan à valider", "MR à tester et merger", "étapes client à publier", "client · précision demandée", "attend la CI", "attend les retours", "sans nouvelle depuis {n} min", "en cours chez {initials}", "MR fermée"), `issue_todo`, `issue_done`, `issue_not_planned` ("fermée sans suite" / "closed as not planned"), the group labels "Jira" and "GitHub", `public` ("public" in both languages), the section titles, "Tous les projets", "À toi de jouer", "Synchronisé il y a {age}", "Jira non configuré", and the settings labels.

- [ ] **Step 1: Write the failing tests:** `default_shows_only_mine_and_local_runs`, `search_matches_key_or_summary`, `unassigned_filter`, `backlog_filter`, `kanban_project_ignores_sprints`, `sections_put_you_first`, `counts_match_overview_columns`, `prefs_default_and_round_trip`, `prefs_survive_broken_storage` (localStorage throwing gives defaults), `t_falls_back_to_key`.
- [ ] **Step 2: Run** `pnpm test`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): front-end data, filters, preferences and copy`.

### Task 9: Design system components in React

**Files:**
- Create: `dashboard/app/src/ui/` with one file per component: `Button.tsx`, `Chip.tsx`, `Avatar.tsx`, `StatusPill.tsx`, `StageTrack.tsx`, `StepList.tsx`, `Section.tsx`, `NavItem.tsx`, `ActionCard.tsx`, `Icon.tsx`, and `index.ts`
- Test: `dashboard/app/src/ui/ui.test.tsx`

**Interfaces:**
- Consumes: `dashboard/design-system/components/index.d.ts` (props, copy them exactly), `components.css` classes (`tf-*`).
- Produces: the nine components with those props, plus `Icon` taking `name` from `lucide-react` (`ChevronRight`, `ArrowUpRight`, `Folder`, `LayoutGrid`, `SquareCheck`, `TriangleAlert`, `Search`, `Settings`, `RefreshCw`, `X`) at size 16, stroke 1.6.

Behaviour comes from the design-system notes (`dashboard/design-system/components/<Name>.md`). `Section` persists nothing itself. It takes `open` and `onToggle` so `prefs.ts` stays the single store.

- [ ] **Step 1: Write the failing tests:** `chip_exposes_pressed`, `avatar_has_accessible_name`, `stage_track_label` (`current=5`, `stage="ci-gate"`, `failed` gives an element with `aria-label` "ci-gate 5/8, failed"), `status_pill_always_has_text`, `section_toggles` (clicking the summary calls `onToggle`), `nav_item_hides_zero_count`.
- [ ] **Step 2: Run** `pnpm test`. Expected: FAIL.
- [ ] **Step 3: Implement.** Add `lucide-react`.
- [ ] **Step 4: Run** `pnpm test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(dashboard): design system components in React`.

### Task 10: Screens

**Files:**
- Create: `dashboard/app/src/screens/Sidebar.tsx`, `TopBar.tsx`, `Overview.tsx`, `ProjectScreen.tsx`, `TicketTable.tsx`, `DetailPanel.tsx`, `Settings.tsx`, `dashboard/app/src/app.css` (layout only, tokens only)
- Modify: `dashboard/app/src/App.tsx`
- Test: `dashboard/app/src/screens/screens.test.tsx` (mock `@tauri-apps/api/core`, `@tauri-apps/api/event` and `@tauri-apps/plugin-opener` with `vi.mock`, and feed a snapshot built from a fixture object)

**Interfaces:**
- Consumes: Tasks 8 and 9. Opening links uses `openUrl` for http(s) and `openPath` for `runDir`, `planPath`, `handoverPath`.

The layout follows `.superpowers/brainstorm/595432-1793018224/content/theme-geist.html` (git-ignored, on the author's machine) and the "Layout" paragraph of `dashboard/design-system/README.md`: 248 px sidebar, top bar (avatars, sprint chips, search), list column, 340 px side column. The detail panel replaces the side column and Escape closes it. The side column shows the action card, then a small summary of the selected sprints (count per section). The search field in the top bar feeds `visible`'s `query`. The sync line at the bottom of the sidebar shows the age of the last sync and a refresh button calling `refresh`. A colleague's in-progress ticket shows "en cours chez <initials>" in the progress cell. A stale run shows its flag in the stage caption. `Settings` asks for root folders (text list, one per line), Jira URL, email, token (password field, never pre-filled, "enregistré" when `hasToken`), and language. The first launch with no roots opens Settings. The sidebar and the overview group projects under "Jira" and "GitHub" by `tracker`, hiding an empty group. For a GitHub project, the detail panel shows `ticketStatus` and `labels` where a Jira project shows the Jira status, its ticket link opens the issue, and a "public" tag shows when the project is `public`.

- [ ] **Step 1: Write the failing tests:** `first_launch_opens_settings`, `overview_lists_projects_with_counts`, `project_default_filter_is_me`, `adding_a_person_shows_their_tickets`, `clicking_a_row_opens_panel_and_escape_closes`, `action_card_hidden_when_nothing_waits`, `jira_not_configured_banner`, `stale_source_shows_age`, `language_switch_changes_copy`, `projects_grouped_by_tracker`, `github_detail_shows_issue_state_and_public_tag`.
- [ ] **Step 2: Run** `pnpm test`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `pnpm test`. Expected: PASS.
- [ ] **Step 5: Visual check.** `pnpm tauri dev` against the temp root from Task 7, then compare with the mockup at 1440×900 and at 1100×700 (the minimum window). Check focus rings by tabbing through the top bar, and that nothing overflows at 1100 px.
- [ ] **Step 6: Commit** `feat(dashboard): screens`.

### Task 11: Builds for Ubuntu, Windows and macOS

**Files:**
- Create: `.github/workflows/dashboard-release.yml`
- Create: `dashboard/README.md` (what it is, how to run in dev, how to cut a release, the unsigned-build warning per OS, prerequisites: `gh` or `glab` logged in)

**Interfaces:**
- Consumes: the app as built by Tasks 1 to 10.

The workflow runs on tags `dashboard-v*` and by hand. It uses a matrix of `ubuntu-22.04`, `windows-latest`, `macos-latest` (universal target `universal-apple-darwin`), installs pnpm and Rust, installs the WebKitGTK packages on Ubuntu that the Tauri prerequisites page lists, runs `pnpm test` and `cargo test`, then builds with `tauri-apps/tauri-action` (check its current major version) with `projectPath: dashboard/app`, creating a draft GitHub release with the installers.

- [ ] **Step 1: Write the workflow and the README.**
- [ ] **Step 2: Lint the workflow** with `actionlint .github/workflows/dashboard-release.yml` if available, else `gh workflow view` after pushing the branch.
- [ ] **Step 3: Run it** by hand on the branch (`gh workflow run dashboard-release.yml --ref <branch>`) and check that the three jobs pass and a draft release holds a `.deb` or `.AppImage`, a `.msi` or `.exe`, and a `.dmg`.
- [ ] **Step 4: Commit** `ci(dashboard): build installers for Ubuntu, Windows and macOS`.
