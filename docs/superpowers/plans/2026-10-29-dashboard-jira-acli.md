# Dashboard Jira through acli implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The dashboard reads Jira through the Atlassian CLI (`acli`) and the developer's existing OAuth login, so it stores no Jira URL, email or token.

**Architecture:** `jira.rs` stops using reqwest and runs `acli` through the existing `forge::Runner` trait, like `gh` and `glab`. `fetch_jira` becomes synchronous and runs in the same blocking task as `fetch_forge`, before it. Settings lose `jiraUrl`, `jiraEmail` and the keychain token, so the settings screen keeps root folders and language only.

**Tech Stack:** Rust (Tauri 2, serde_json), React 18 + TypeScript, Vitest, `acli` 1.3.39-stable.

**Spec:** `docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md` (Jira section, amended by Task 2 of this plan).

## Global Constraints

- The app stays read-only. Only these `acli` subcommands may be run: `jira auth status`, `jira workitem search`, `jira board search`, `jira board list-sprints`, `jira sprint list-workitems`.
- Every `acli` call goes through `forge::Runner` (60 s timeout, no console window on Windows, Homebrew PATH on macOS). No new crate.
- JQL and project keys are passed as separate arguments, never through a shell.
- Ticket links use the site from `acli jira auth status` (`https://<site>/browse/<KEY>`), never the `self` URLs in the JSON (they point to an internal host).
- Jira scope is unchanged: tickets in open and future sprints, backlog tickets not done, tickets done in the last 14 days.
- Tests use hand-written fixtures in the real `acli` shape. No real names, emails or account ids in fixtures.
- UI copy follows the existing i18n style (French tutoiement, typographic apostrophe ’).

## Review Focus

- `acli` not installed. The Jira group says Jira is not configured with the install hint, and GitHub projects keep working. Pinned in Task 1 (`fetch_jira_without_acli_is_not_configured`).
- `acli` installed but logged out (`acli: ✗ Error: unauthorized: ...` on stderr, exit 1). Same as not installed. Pinned in Task 1 (`fetch_jira_logged_out_is_not_configured`).
- A developer with no assigned ticket. "Me" falls back to the reporter, then to none, without an error. Pinned in Task 1 (`me_falls_back_to_reporter_then_none`).
- `--paginate` prints one JSON document per page, concatenated. All pages are read. Pinned in Task 1 (`reads_every_concatenated_page`).
- An old `settings.json` holding `jiraUrl` and `jiraEmail` still loads its roots and language. Pinned in Task 2 (`old_settings_with_jira_fields_still_load`).

---

### Task 1: Jira through acli (jira.rs and sync.rs)

**Files:**
- Modify: `dashboard/app/src-tauri/src/jira.rs` (whole file: client, parsers, tests)
- Modify: `dashboard/app/src-tauri/tests/fixtures/jira/` (see Step 1)
- Modify: `dashboard/app/src-tauri/src/sync.rs` (`fetch`, `fetch_jira`, the `NOT_CONFIGURED` const and the test `jira_without_token_keeps_github_issues`)

**Interfaces:**
- Consumes: `crate::forge::Runner`, `crate::forge::SystemRunner` (`fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String>`), and in tests `crate::forge::tests::Fake` (`Fake::new(|dir, program, args| ...)`, `fake.calls()` returns `"program arg1 arg2 ..."` strings).
- Produces, all `pub` in `crate::jira`:
  - `fn site(r: &dyn Runner) -> Result<String, String>` returns `https://<site>` from `acli jira auth status`. Error `"jira: unexpected auth status"` if no `Site:` line.
  - `fn not_configured(e: &str) -> bool` is true when `e` contains `unauthorized` or `(os error 2)` (logged out or `acli` missing).
  - `fn me(r: &dyn Runner) -> Result<Option<Person>, String>`
  - `fn project(r: &dyn Runner, base: &str, key: &str) -> Result<JiraProject, String>`
  - `Sprint`, `JiraProject`, `status_category`, `parse_issue` and `search_jql` keep their current signatures.
  - In `sync.rs`, `fn fetch_jira(r: &dyn Runner, remote: &mut Remote, projects: &[ProjectDef])` (sync, private as now).
- Removed: `JiraClient`, `Page`, `parse_page`, `check_url`, `boards_or_none`, `next_token_to_follow`, `search_url`, `enc`.

Exact commands. `<dir>` is `std::env::temp_dir()`, and FIELDS stays `summary,status,issuetype,assignee`.

| Purpose | Args after `acli` |
|---|---|
| site | `jira auth status` |
| me | `jira workitem search --jql "assignee = currentUser() ORDER BY updated DESC" --fields assignee --limit 1 --json` |
| me, fallback | `jira workitem search --jql "reporter = currentUser() ORDER BY updated DESC" --fields reporter --limit 1 --json` |
| boards | `jira board search --project <KEY> --type scrum --json` |
| sprints | `jira board list-sprints --id <BOARD> --state active,future --json` |
| sprint tickets | `jira sprint list-workitems --board <BOARD> --sprint <SPRINT> --jql <project_jql(KEY)> --fields <FIELDS> --paginate --json` |
| the rest | `jira workitem search --jql <search_jql(KEY, has_sprints)> --fields <FIELDS> --paginate --json` |

Real output shapes, checked on 2026-10-29:
- `auth status` prints `✓ Authenticated`, `  Site: example.atlassian.net`, `  Email: ...` and `  Authentication Type: oauth`.
- `board search` prints `{"values":[{"id":9175,...}],...}`.
- `list-sprints` prints `{"sprints":[{"id":..,"name":..,"state":"active"},...],...}`, with the key `sprints` and not `values`.
- `list-workitems --paginate --json` prints one `{"issues":[...],"startAt":..,"total":..}` object per page, concatenated with no separator.
- `workitem search --json` prints one bare array of issues, also with `--paginate`. Issues keep the REST shape (`key`, `fields.summary`, `fields.status.statusCategory.key`, ...).

`search_jql(key, true)` becomes one query for the backlog and the recently done, replacing the agile backlog endpoint, which `acli` lacks. `search_jql(key, false)` is unchanged.

```
project = "<KEY>" AND ((statusCategory != Done AND (sprint is EMPTY OR (sprint not in openSprints() AND sprint not in futureSprints()))) OR (statusCategory = Done AND statusCategoryChangedDate >= -14d))
```

On IPC32603 this query returned 430, which is 409 backlog plus 21 recently done. The open sprints held 22 open tickets and the future sprints 72, and 409 + 22 + 72 equals the project's 503 open tickets.

- [ ] **Step 1: Replace the fixtures**

Keep `sprint-issues.json` and `boards.json`. Delete `backlog.json`, `myself.json` and `search.json`. Rewrite `sprints.json` as `{"isLast":true,"maxResults":50,"sprints":[{"id":7,"name":"Sprint 7","state":"active"},{"id":8,"name":"Sprint 8","state":"future"}]}`. Add:
- `auth-status.txt`, holding the four lines above with `Site: example.atlassian.net` and `Email: dev@example.com`.
- `search.json`, a bare array of two issues: `DEMO-3` done and `DEMO-4` not done and unassigned.
- `sprint-paged.json`, two `{"issues":[...]}` objects one after the other: page one holds `DEMO-1`, page two holds `DEMO-2`.

- [ ] **Step 2: Write the failing tests**

Replace the tests module's tests that cover removed functions. Keep `parses_issue_with_assignee`, `parses_unassigned_issue`, `maps_status_categories`, `kanban_project_has_no_sprints` and `search_jql_quotes_key_and_follows_sprints`, with the new `true` string. Add:

```rust
#[test]
fn reads_site_from_auth_status() {
    let r = Fake::new(|_, _, _| Ok(raw("auth-status.txt")));
    assert_eq!(site(&r).unwrap(), "https://example.atlassian.net");
    assert_eq!(r.calls(), vec!["acli jira auth status"]);
    assert_eq!(site(&Fake::new(|_, _, _| Ok("✓ Authenticated\n".into()))).unwrap_err(), "jira: unexpected auth status");
}

#[test]
fn spots_logged_out_or_missing_acli() {
    assert!(not_configured("acli: ✗ Error: unauthorized: use 'acli jira auth login' to authenticate"));
    assert!(not_configured("acli: No such file or directory (os error 2)"));
    assert!(!not_configured("acli: timed out"));
}

#[test]
fn me_falls_back_to_reporter_then_none() {
    let person = |f: &str| format!(r#"[{{"key":"A-1","fields":{{"{f}":{{"accountId":"id-1","displayName":"Dev One"}}}}}}]"#);
    let assignee = Fake::new(move |_, _, a| Ok(if a[4].starts_with("assignee") { person("assignee") } else { "[]".into() }));
    assert_eq!(me(&assignee).unwrap().unwrap().account_id, "id-1");
    assert_eq!(assignee.calls().len(), 1);
    let reporter = Fake::new(move |_, _, a| Ok(if a[4].starts_with("reporter") { person("reporter") } else { "[]".into() }));
    assert_eq!(me(&reporter).unwrap().unwrap().display_name, "Dev One");
    assert!(me(&Fake::new(|_, _, _| Ok("[]".into()))).unwrap().is_none());
}

#[test]
fn reads_every_concatenated_page() {
    let r = Fake::new(|_, _, a| Ok(match a[1] {
        "board" if a[2] == "search" => raw("boards.json"),
        "board" => raw("sprints.json"),
        "sprint" => raw("sprint-paged.json"),
        _ => raw("search.json"),
    }));
    let p = project(&r, "https://x.atlassian.net", "DEMO").unwrap();
    assert!(p.has_sprints);
    // DEMO-1 and DEMO-2 come from both sprints' pages, deduplicated. DEMO-3 and DEMO-4 come from the search.
    assert_eq!(p.tickets.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(), vec!["DEMO-1", "DEMO-2", "DEMO-3", "DEMO-4"]);
    assert_eq!(p.tickets[0].sprint_id, Some(7));
    assert_eq!(p.tickets[3].url, "https://x.atlassian.net/browse/DEMO-4");
    assert!(r.calls().contains(&"acli jira board list-sprints --id 42 --state active,future --json".to_string()));
    assert!(r.calls().last().unwrap().contains("--paginate --json"));
}

#[test]
fn kanban_project_searches_without_sprint_calls() {
    let r = Fake::new(|_, _, a| Ok(if a[1] == "board" { r#"{"values":[]}"#.into() } else { raw("search.json") }));
    let p = project(&r, "https://x", "DEMO").unwrap();
    assert!(!p.has_sprints && p.sprints.is_empty());
    assert_eq!(r.calls().len(), 2);
    assert!(r.calls()[1].contains("statusCategory != Done OR"));
}
```

In `sync.rs` tests, replace `jira_without_token_keeps_github_issues` with `fetch_jira_without_acli_is_not_configured`. It has the same body and assertions, but calls `fetch_jira(&Fake::new(|_, p, _| Err(format!("{p}: No such file or directory (os error 2)"))), &mut remote, &projects)` and also asserts `remote.jira_state.error.as_deref().unwrap().contains("os error 2")`. Add:

```rust
#[test]
fn fetch_jira_logged_out_is_not_configured() {
    let mut remote = Remote::default();
    fetch_jira(&Fake::new(|_, _, _| Err("acli: ✗ Error: unauthorized: use 'acli jira auth login' to authenticate".into())), &mut remote, &shop_jira());
    assert!(!remote.jira_state.configured);
}

#[test]
fn fetch_jira_timeout_keeps_last_data() {
    let mut remote = Remote { jira: tickets(&["SHOP-1".into()], StatusCategory::Todo), ..Remote::default() };
    fetch_jira(&Fake::new(|_, _, _| Err("acli: timed out".into())), &mut remote, &shop_jira());
    assert!(remote.jira_state.configured && !remote.jira_state.ok);
    assert_eq!(remote.jira.len(), 1);
}
```

`shop_jira()` is one `ProjectDef { key: "SHOP", tracker: Tracker::Jira, repos: vec![] }`. Reuse an existing helper if one fits.

`raw(name)` reads `tests/fixtures/jira/<name>` as a `String`, next to the existing `fixture()`. The issue order follows the sprints first, then the search. Adapt the expected order only if the dedup keeps the first occurrence differently, never by dropping an assertion.

- [ ] **Step 3: Run them to see them fail**

Run `cd dashboard/app/src-tauri && cargo test`. It is expected to fail to compile (`site`, `me`, `project` and `not_configured` are not defined yet).

- [ ] **Step 4: Implement**

Write the functions of the Interfaces block. `project` keeps its current logic and both shortcut comments, with the agile backlog call removed. One private helper turns `acli` output into issue values. It reads the output as a stream of JSON documents with `serde_json::Deserializer::from_str(out).into_iter::<Value>()`. A document that is an array gives its items, and an object gives its `issues` array. If the output is not valid JSON, the call returns `Err("jira: unexpected acli output")`. `parse_sprints` reads `sprints`.

`fetch_jira` goes in this order:
1. Run `site(r)`. If `not_configured(&e)` is true, clear `remote.jira` and `remote.me_jira` and set `SourceState { configured: false, ok: false, error: Some(e), last_ok_at: None }`. Any other error goes through `after(vec![e], true, ...)` and keeps the last data.
2. Run `me(r)`. An error goes through `after` and returns. `Ok(None)` sets `me_jira = None`.
3. Run each project with the existing per-project keep-last-data loop.

In `fetch`, drop the token lookup and call `fetch_jira(&SystemRunner, &mut remote, &projects)` inside the existing `blocking` closure, just before `fetch_forge`. `lib.rs` still reads the token for `has_token` until Task 2.

- [ ] **Step 5: Run the tests to see them pass**

Run `cargo test` and `cargo build`. Every test passes and the build gives no warnings.

- [ ] **Step 6: Commit**

```bash
git add dashboard/app/src-tauri/src/jira.rs dashboard/app/src-tauri/src/sync.rs dashboard/app/src-tauri/tests/fixtures/jira
git commit -m "feat(dashboard): read Jira through acli"
```

### Task 2: Settings without Jira fields, front end and docs

**Files:**
- Modify: `dashboard/app/src-tauri/src/settings.rs` (drop `jira_url`, `jira_email`, `SERVICE`, `token` and `set_token`)
- Modify: `dashboard/app/src-tauri/src/lib.rs` (`get_settings` returns `Settings`, `save_settings(app, settings)`, delete `SettingsView`)
- Modify: `dashboard/app/src-tauri/Cargo.toml` (remove `reqwest`, `keyring` and `urlencoding`), plus `Cargo.lock` through cargo
- Modify: `dashboard/app/src/types.ts` (`Settings` becomes `{ roots: string[]; language: 'fr' | 'en' }`, and `SettingsView` is deleted)
- Modify: `dashboard/app/src/App.tsx` (`NO_SETTINGS`, the type uses)
- Modify: `dashboard/app/src/screens/Settings.tsx` (remove the URL, email and token fields and their state)
- Modify: `dashboard/app/src/i18n.ts` (both languages)
- Modify: `dashboard/app/src/screens/screens.test.tsx`
- Modify: `dashboard/README.md`, `dashboard/PRODUCT.md`, `docs/superpowers/specs/2026-10-26-ticket-flow-dashboard-design.md` and `docs/superpowers/specs/2026-10-27-github-issues-tracker-design.md`

**Interfaces:**
- Consumes: Task 1's `fetch_jira`, which no longer needs settings.
- Produces: Tauri commands `get_settings() -> Settings` and `save_settings(settings: Settings)`. The JSON `Settings` is `{ roots: string[], language: "fr" | "en" }`. On the front end, `invoke('get_settings')` returns `Settings`, and saving is `invoke('save_settings', { settings })`.

Exact copy:
- Delete the i18n keys `settings_jira_url`, `settings_jira_email`, `settings_token` and `settings_token_saved`.
- Add `settings_jira_acli`:
  - fr: `Jira passe par l’Atlassian CLI. Connecte-toi une fois avec « acli jira auth login --web ».`
  - en: `Jira goes through the Atlassian CLI. Log in once with “acli jira auth login --web”.`
- Change `jira_not_configured_hint`:
  - fr: `Seuls les runs locaux s’affichent. Installe l’Atlassian CLI (acli) puis lance « acli jira auth login --web ».`
  - en: `Only local runs show. Install the Atlassian CLI (acli), then run “acli jira auth login --web”.`

- [ ] **Step 1: Update the tests**

In `settings.rs` tests, update `settings_round_trip` to the two fields and add:

```rust
#[test]
fn old_settings_with_jira_fields_still_load() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("settings.json"), r#"{"roots":["/w"],"jiraUrl":"https://x","jiraEmail":"a@b.c","language":"en"}"#).unwrap();
    let s = load(dir.path());
    assert_eq!((s.roots, s.language.as_str()), (vec!["/w".to_string()], "en"));
}
```

In `screens.test.tsx`:
- Drop `jiraUrl`, `jiraEmail` and `hasToken` from the settings helper.
- Delete the token-field and "Jeton enregistré" tests.
- Make the save test expect `invoke('save_settings', { settings: { roots: ['/r'], language: 'en' } })`.
- Add `settings_shows_the_acli_login_hint`, which renders Settings and finds the fr `settings_jira_acli` text.
- Keep the `get_settings` failure test, with its thrown message changed to `'settings: unreadable'`.

- [ ] **Step 2: Run them to see them fail**

Run `cd dashboard/app/src-tauri && cargo test`, which fails to compile, then `cd dashboard/app && pnpm test`, where the new and changed tests fail.

- [ ] **Step 3: Implement**

In `lib.rs`, `save_settings` only validates and saves the settings, then keeps whatever it did after saving (rescan or fetch). Remove the three dependencies and the now unused `settings::token`. Settings shows the `settings_jira_acli` line as `<p className="tf-sub">` where the Jira fields were.

- [ ] **Step 4: Update the docs**

- README prerequisites: remove the Secret Service line and add "`acli` (Atlassian CLI) logged in with `acli jira auth login --web`, for Jira projects". Mention it in the macOS PATH sentence next to `gh` and `glab`.
- README first launch: replace the Jira URL, email and token paragraph with one sentence saying Jira needs no setting, only the `acli` login. Rewrite "With no Jira settings" as "Without `acli` logged in".
- PRODUCT.md: replace the line about the Atlassian MCP server and the token with "The app reads Jira through the Atlassian CLI (`acli`) and its OAuth login, so it stores no Jira token." Change the zero-setup line to "Zero setup beyond pointing at folders and logging in to `gh`, `glab` and `acli`."
- Main spec, Jira section: replace the paragraph with one saying the app runs `acli` like `gh` and `glab`, that "me" is the account found with `assignee = currentUser()` then `reporter = currentUser()`, that the site comes from `acli jira auth status`, and that the backlog is read by JQL. Add a sentence dated 2026-10-29 saying that this replaced the REST API and keychain token because the developer cannot get an API token. Update the lines that say "Jira token" in the out-of-scope list, the no-token behaviour and the crate list. The keychain goes away.
- GitHub-issues spec: in the two places that say "without a Jira token", write "without Jira (no `acli` login)".

- [ ] **Step 5: Verify**

Run `pnpm test` and `pnpm build`, then `cargo test` and `cargo build` in `src-tauri`. Everything passes and the build gives no warnings. `grep -rn "reqwest\|keyring\|urlencoding" dashboard/app/src-tauri/src` prints nothing. `grep -rn "hasToken\|jiraUrl\|jiraEmail\|Jeton" dashboard/app/src` prints nothing.

- [ ] **Step 6: Commit**

```bash
git add dashboard/app/src-tauri dashboard/app/src dashboard/README.md dashboard/PRODUCT.md docs/superpowers/specs
git commit -m "feat(dashboard): no Jira settings or keychain, docs for acli"
```
