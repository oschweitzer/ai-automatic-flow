use crate::classify::MrState;
use crate::forge::{self, Runner, SystemRunner};
use crate::jira::JiraClient;
use crate::repos::{self, ProjectDef, Tracker};
use crate::runs::{read_runs, RunState};
use crate::settings::{self, Settings};
use crate::snapshot::{self, mr_key, pick_run, run_of, snapshot, Remote, Snapshot, SourceState};
use chrono::{DateTime, Utc};
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const POLL: Duration = Duration::from_secs(120);
const DEBOUNCE: Duration = Duration::from_millis(500);
const NOT_CONFIGURED: &str = "jira: not configured";

pub struct Inner {
    pub settings: Settings,
    projects: Vec<ProjectDef>,
    runs: Vec<(String, RunState)>,
    remote: Remote,
    pub snapshot: Snapshot,
}

pub struct AppState {
    pub config_dir: PathBuf,
    // Never held across a CLI call or an HTTP request.
    pub inner: tokio::sync::Mutex<Inner>,
    // One remote fetch at a time: each starts from the last stored Remote, so none overwrites a newer one.
    fetching: tokio::sync::Mutex<()>,
    // One local scan at a time, so an older scan never lands after a newer one.
    scanning: tokio::sync::Mutex<()>,
    watcher: std::sync::Mutex<Option<(Vec<PathBuf>, Debouncer<RecommendedWatcher>)>>,
}

impl AppState {
    pub fn new(config_dir: PathBuf) -> Self {
        let settings = settings::load(&config_dir);
        let remote = Remote::default();
        let snapshot = snapshot(&[], &[], &remote, Utc::now());
        AppState {
            config_dir,
            inner: tokio::sync::Mutex::new(Inner { settings, projects: vec![], runs: vec![], remote, snapshot }),
            fetching: tokio::sync::Mutex::new(()),
            scanning: tokio::sync::Mutex::new(()),
            watcher: std::sync::Mutex::new(None),
        }
    }
}

/// Calls `on_change` after file changes below `paths`, debounced. Dropping the result stops watching.
pub fn watch(paths: &[PathBuf], on_change: impl Fn() + Send + 'static) -> Result<Debouncer<RecommendedWatcher>, String> {
    // An error event (lost events, rescan needed) also triggers a re-read.
    let mut d = new_debouncer(DEBOUNCE, move |_: DebounceEventResult| on_change()).map_err(|e| format!("watch: {e}"))?;
    for p in paths {
        // A folder that cannot be watched is still re-read by the poll.
        let _ = d.watcher().watch(p, RecursiveMode::Recursive);
    }
    Ok(d)
}

fn read_all(projects: &[ProjectDef]) -> Vec<(String, RunState)> {
    projects.iter().flat_map(|p| &p.repos)
        .flat_map(|r| read_runs(&r.path).into_iter().map(|run| (r.name.clone(), run)))
        .collect()
}

fn rebuild(i: &mut Inner) -> Snapshot {
    i.snapshot = snapshot(&i.projects, &i.runs, &i.remote, Utc::now());
    i.snapshot.clone()
}

fn emit(app: &AppHandle, snap: Snapshot) {
    let _ = app.emit("snapshot", snap);
}

async fn blocking<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> Option<R> {
    tauri::async_runtime::spawn_blocking(f).await.ok()
}

/// Rediscovers repositories under the roots, re-reads their runs and re-watches their runs folders.
pub async fn rescan(app: &AppHandle) {
    let st = app.state::<AppState>();
    let _one = st.scanning.lock().await;
    let used = st.inner.lock().await.settings.roots.clone();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    let roots: Vec<PathBuf> = used.iter().map(|r| repos::expand_home(r, home.as_deref())).collect();
    let Some((projects, runs)) = blocking(move || {
        let projects = repos::group(repos::discover(&roots));
        let runs = read_all(&projects);
        (projects, runs)
    }).await else { return };
    let snap = {
        let mut i = st.inner.lock().await;
        // Settings saved meanwhile: their own rescan follows, this result is for the old roots.
        if i.settings.roots != used {
            return;
        }
        i.projects = projects.clone();
        i.runs = runs;
        rebuild(&mut i)
    };
    watch_runs(app, &st, &projects);
    emit(app, snap);
}

async fn reload_runs(app: AppHandle) {
    let st = app.state::<AppState>();
    // Serialised with rescan, so the projects read here are still current when the runs are stored.
    let _one = st.scanning.lock().await;
    let projects = st.inner.lock().await.projects.clone();
    let Some(runs) = blocking(move || read_all(&projects)).await else { return };
    let snap = {
        let mut i = st.inner.lock().await;
        i.runs = runs;
        rebuild(&mut i)
    };
    emit(&app, snap);
}

// shortcut: a repo with no runs folder yet is not watched, its first run shows at the next poll or refresh.
fn watch_runs(app: &AppHandle, st: &AppState, projects: &[ProjectDef]) {
    let paths: Vec<PathBuf> = projects.iter().flat_map(|p| &p.repos)
        .map(|r| r.path.join(".ticket-flow").join("runs"))
        .filter(|p| p.is_dir())
        .collect();
    let mut w = st.watcher.lock().unwrap_or_else(|e| e.into_inner());
    if w.as_ref().is_some_and(|(old, _)| *old == paths) {
        return;
    }
    let handle = app.clone();
    *w = watch(&paths, move || {
        tauri::async_runtime::spawn(reload_runs(handle.clone()));
    }).ok().map(|d| (paths, d));
}

/// Fetches Jira and the forge, keeping the last data of any source or item that fails.
pub async fn fetch(app: &AppHandle) {
    let st = app.state::<AppState>();
    let _one = st.fetching.lock().await;
    let (s, projects, runs, mut remote) = {
        let i = st.inner.lock().await;
        (i.settings.clone(), i.projects.clone(), i.runs.clone(), i.remote.clone())
    };
    let email = s.jira_email.clone();
    let token = if email.is_empty() { None } else { blocking(move || settings::token(&email)).await.flatten() };
    fetch_jira(&mut remote, &s, token, &projects).await;
    let Some(remote) = blocking(move || {
        fetch_forge(&SystemRunner, &projects, &runs, &mut remote, Utc::now());
        remote
    }).await else { return };
    let snap = {
        let mut i = st.inner.lock().await;
        i.remote = remote;
        rebuild(&mut i)
    };
    emit(app, snap);
}

fn after(errors: Vec<String>, configured: bool, prev: &SourceState, now: DateTime<Utc>) -> SourceState {
    let ok = errors.is_empty();
    SourceState { configured, ok, error: errors.into_iter().next(), last_ok_at: if ok { Some(now) } else { prev.last_ok_at } }
}

async fn fetch_jira(remote: &mut Remote, s: &Settings, token: Option<String>, projects: &[ProjectDef]) {
    let client = match token.filter(|t| !t.is_empty()) {
        Some(t) if !s.jira_email.is_empty() => JiraClient::new(&s.jira_url, &s.jira_email, &t),
        _ => Err(NOT_CONFIGURED.to_string()),
    };
    let client = match client {
        Ok(c) => c,
        Err(e) if e == NOT_CONFIGURED => {
            (remote.jira, remote.me_jira) = (vec![], None);
            remote.jira_state = SourceState { configured: false, ok: false, error: Some(e), last_ok_at: None };
            return;
        }
        Err(e) => {
            remote.jira_state = after(vec![e], true, &remote.jira_state, Utc::now());
            return;
        }
    };
    // A 401 or an unreachable site fails here once, not once per project.
    match client.myself().await {
        Ok(me) => remote.me_jira = Some(me),
        Err(e) => {
            remote.jira_state = after(vec![e], true, &remote.jira_state, Utc::now());
            return;
        }
    }
    let mut errors = vec![];
    let keys: Vec<&str> = projects.iter().filter(|p| p.tracker == Tracker::Jira).map(|p| p.key.as_str()).collect();
    for key in &keys {
        match client.project(key).await {
            Ok(j) => {
                remote.jira.retain(|x| x.key != *key);
                remote.jira.push(j);
            }
            Err(e) => errors.push(e),
        }
    }
    remote.jira.retain(|x| keys.contains(&x.key.as_str()));
    remote.jira_state = after(errors, true, &remote.jira_state, Utc::now());
}

/// GitHub issues, visibility and login, then one MR list per repository. Each failing item keeps its last value.
pub fn fetch_forge(r: &dyn Runner, projects: &[ProjectDef], runs: &[(String, RunState)], remote: &mut Remote, now: DateTime<Utc>) {
    let mut errors = vec![];
    for p in projects.iter().filter(|p| p.tracker == Tracker::Github) {
        // The repos of a GitHub project share its remote, so one of them answers for all.
        let Some(repo) = p.repos.iter().find(|r| r.remote.is_some()).or(p.repos.first()) else { continue };
        match forge::issues(r, repo, now) {
            Ok(v) => { remote.issues.insert(p.key.clone(), v); }
            Err(e) => errors.push(e),
        }
        if repo.remote.is_some() {
            match forge::visibility(r, repo) {
                Ok(v) => { remote.public.insert(p.key.clone(), v == "public"); }
                Err(e) => errors.push(e),
            }
        }
    }
    let github = projects.iter().any(|p| p.tracker == Tracker::Github);
    remote.issues.retain(|k, _| projects.iter().any(|p| p.tracker == Tracker::Github && p.key == *k));
    if let Some(repo) = projects.iter().flat_map(|p| &p.repos).find(|r| r.forge == Some(repos::Forge::Github)) {
        match forge::gh_login(r, &repo.path) {
            Ok(me) => remote.me_github = Some(me),
            Err(e) => errors.push(e),
        }
    }

    // One MR list per repository, matched here to every ticket of the project.
    let (mut mrs, mut mr_repos) = (HashMap::new(), HashMap::new());
    for p in projects {
        let trackers = snapshot::trackers(p, &remote.jira, &remote.issues);
        let own: Vec<&(String, RunState)> = runs.iter().filter(|(name, run)| p.repos.iter().any(|r| run_of(r, name, run))).collect();
        let with_run: HashSet<&String> = own.iter().map(|(_, run)| &run.key).collect();
        // The run the snapshot shows, from the last data known: its repo and its `mr.json` decide the MR.
        let shown: Vec<&(String, RunState)> = with_run.iter().filter_map(|key| {
            let cands: Vec<&(String, RunState)> = own.iter().copied().filter(|(_, run)| &run.key == *key).collect();
            pick_run(&cands, trackers.get(*key), remote.mrs.get(&mr_key(&p.key, key)), now)
        }).collect();
        let tracker_only: Vec<&String> = trackers.keys().filter(|k| !with_run.contains(k)).collect();
        for repo in p.repos.iter().filter(|r| r.forge.is_some()) {
            let list = match forge::mr_list(r, repo) {
                Ok(l) => l,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            // Run tickets in their own repo only, tracker-only tickets in the first repo that has their MR.
            let wanted = shown.iter().filter(|(name, run)| run_of(repo, name, run)).map(|(_, run)| (&run.key, run.mr.as_ref().map(|m| m.number), true))
                .chain(tracker_only.iter().map(|k| (*k, None, false)));
            for (key, number, has_run) in wanted {
                let k = mr_key(&p.key, key);
                if mrs.contains_key(&k) { continue; }
                let Some(found) = forge::match_mr(&list, number, key) else { continue };
                let mut m = found.clone();
                if repo.forge == Some(repos::Forge::Gitlab) && m.state == MrState::Open {
                    match forge::pipeline(r, repo, m.number) {
                        Ok(Some(full)) => m = full,
                        Ok(None) => {}
                        Err(e) => {
                            errors.push(e);
                            if let Some(old) = remote.mrs.get(&k).filter(|o| o.number == m.number) {
                                (m.ci, m.pipeline_url) = (old.ci.clone(), old.pipeline_url.clone());
                            }
                        }
                    }
                }
                if !has_run { mr_repos.insert(k.clone(), repo.name.clone()); }
                mrs.insert(k, m);
            }
        }
        // An MR older than the list window, or in a list that failed, keeps its last status.
        for key in with_run.iter().chain(&tracker_only) {
            let k = mr_key(&p.key, key);
            if mrs.contains_key(&k) { continue; }
            if let Some(old) = remote.mrs.get(&k) { mrs.insert(k.clone(), old.clone()); }
            if let Some(old) = remote.mr_repos.get(&k) { mr_repos.insert(k, old.clone()); }
        }
    }
    (remote.mrs, remote.mr_repos) = (mrs, mr_repos);
    let configured = github || projects.iter().flat_map(|p| &p.repos).any(|r| r.forge.is_some());
    remote.forge_state = after(errors, configured, &remote.forge_state, now);
}

fn visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_none_or(|w| w.is_visible().unwrap_or(true) && !w.is_minimized().unwrap_or(false))
}

/// First scan and fetch, then a poll every 120 s. Remote fetches skip while the window is hidden or minimised.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        rescan(&app).await;
        fetch(&app).await;
        loop {
            tokio::time::sleep(POLL).await;
            rescan(&app).await;
            if visible(&app) {
                fetch(&app).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{Ci, GithubIssue, IssueState, MrState, MrStatus, Person, StatusCategory};
    use crate::repos::{Forge, Repo};
    use chrono::TimeZone;
    use std::io::Write;
    use std::path::Path;
    use std::sync::mpsc;

    #[test]
    fn watcher_sees_a_log_append_within_3_seconds() {
        let t = tempfile::tempdir().unwrap();
        let runs = t.path().join("runs");
        let run = runs.join("SHOP-1");
        std::fs::create_dir_all(&run).unwrap();
        let log = run.join("log.jsonl");
        std::fs::write(&log, "{\"at\":\"2026-10-26T10:00:00Z\",\"stage\":\"triage\",\"event\":\"start\"}\n").unwrap();
        let (tx, rx) = mpsc::channel();
        let _w = watch(&[runs], move || { let _ = tx.send(()); }).unwrap();
        // Let the backend settle, then drop anything it reported about the setup.
        std::thread::sleep(Duration::from_millis(700));
        while rx.try_recv().is_ok() {}
        let mut f = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
        writeln!(f, "{{\"at\":\"2026-10-26T10:01:00Z\",\"stage\":\"triage\",\"event\":\"end\"}}").unwrap();
        f.sync_all().unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok(), "no change seen within 3 s");
    }

    use crate::forge::tests::Fake;
    use crate::jira::JiraProject;
    use crate::classify::JiraTicket;

    fn now() -> DateTime<Utc> { Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap() }
    fn runs_dir() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs") }
    fn run(key: &str) -> RunState { crate::runs::read_run(&runs_dir().join(key)).unwrap() }
    fn with_mr(mut r: RunState, number: u64) -> RunState {
        r.mr.as_mut().unwrap().number = number;
        r
    }
    fn repo(name: &str, path: PathBuf, forge: Forge) -> Repo {
        Repo { name: name.into(), path, forge: Some(forge), tracker: Tracker::Jira, jira_project: None, remote: None }
    }
    /// Project SHOP with one repository holding every fixture run.
    fn shop(forge: Forge) -> Vec<ProjectDef> {
        vec![ProjectDef { key: "SHOP".into(), tracker: Tracker::Jira, repos: vec![repo("shop", runs_dir(), forge)] }]
    }
    /// Project SHOP with repo "a" holding run SHOP-142 and repo "b" holding run SHOP-133.
    fn two_repos() -> Vec<ProjectDef> {
        let repos = vec![repo("a", runs_dir().join("SHOP-142"), Forge::Github), repo("b", runs_dir().join("SHOP-133"), Forge::Github)];
        vec![ProjectDef { key: "SHOP".into(), tracker: Tracker::Jira, repos }]
    }
    fn tickets(keys: &[String], c: StatusCategory) -> Vec<JiraProject> {
        let t = |k: &String| JiraTicket { key: k.clone(), summary: "s".into(), issue_type: "Story".into(), status: "s".into(),
            status_category: c.clone(), assignee: None, sprint_id: None, url: "u".into() };
        vec![JiraProject { key: "SHOP".into(), sprints: vec![], has_sprints: false, tickets: keys.iter().map(t).collect() }]
    }
    fn pr(n: u64, state: &str, branch: &str) -> String {
        format!(r#"{{"number":{n},"url":"https://github.com/acme/shop/pull/{n}","state":"{state}","isDraft":false,"headRefName":"{branch}","statusCheckRollup":[]}}"#)
    }
    fn list(prs: &[String]) -> String { format!("[{}]", prs.join(",")) }
    /// Answers `gh pr list` with `prs` and `gh api user` with a login.
    fn gh(prs: String) -> Fake {
        Fake::new(move |_, _, a| Ok(if a[0] == "api" { r#"{"login":"al"}"#.into() } else { prs.clone() }))
    }
    fn mr(remote: &Remote, key: &str) -> Option<(u64, MrState, Ci)> {
        remote.mrs.get(&mr_key("SHOP", key)).map(|m| (m.number, m.state.clone(), m.ci.clone()))
    }
    const GH_OPEN: &str = "gh pr list --state open --limit 100 --json number,url,state,isDraft,headRefName,statusCheckRollup";
    const GH_ALL: &str = "gh pr list --state all --limit 200 --json number,url,state,isDraft,headRefName";

    #[test]
    fn forge_error_keeps_last_data() {
        let now = Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap();
        let repo = Repo { name: "shop".into(), path: PathBuf::from(env!("CARGO_MANIFEST_DIR")), forge: Some(Forge::Github), tracker: Tracker::Github, jira_project: None, remote: Some("acme/shop".into()) };
        let projects = vec![ProjectDef { key: "acme/shop".into(), tracker: Tracker::Github, repos: vec![repo] }];
        let run = crate::runs::read_run(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs/SHOP-142")).unwrap();
        let runs = vec![("shop".to_string(), RunState { key: "GH-142".into(), ..run })];
        let issue = |n| GithubIssue { number: n, title: "t".into(), state: IssueState::Open, assignees: vec![], labels: vec![], url: "u".into() };
        let mr = |n| MrStatus { number: n, url: "u".into(), state: MrState::Open, draft: false, ci: Ci::Green, pipeline_url: None };
        let before = Utc.with_ymd_and_hms(2026, 10, 28, 11, 0, 0).unwrap();
        let mut remote = Remote {
            me_github: Some(Person { account_id: "al".into(), display_name: "al".into() }),
            issues: HashMap::from([("acme/shop".to_string(), vec![issue(142), issue(7)])]),
            mrs: HashMap::from([(mr_key("acme/shop", "GH-142"), mr(87)), (mr_key("acme/shop", "GH-7"), mr(9))]),
            mr_repos: HashMap::from([(mr_key("acme/shop", "GH-7"), "shop".to_string())]),
            public: HashMap::from([("acme/shop".to_string(), true)]),
            forge_state: SourceState { configured: true, ok: true, error: None, last_ok_at: Some(before) },
            ..Remote::default()
        };
        fetch_forge(&Fake::new(|_, p, _| Err(format!("{p}: offline"))), &projects, &runs, &mut remote, now);
        assert_eq!(remote.issues["acme/shop"].len(), 2);
        assert_eq!(remote.mrs[&mr_key("acme/shop", "GH-142")].number, 87);
        assert_eq!(remote.mrs[&mr_key("acme/shop", "GH-7")].number, 9);
        assert_eq!(remote.mr_repos.get(&mr_key("acme/shop", "GH-7")).map(String::as_str), Some("shop"));
        assert_eq!(remote.public.get("acme/shop"), Some(&true));
        assert!(remote.me_github.is_some());
        let s = &remote.forge_state;
        assert!(s.configured && !s.ok);
        assert_eq!((s.error.as_deref(), s.last_ok_at), (Some("gh: offline"), Some(before)));
    }

    #[test]
    fn reopened_mr_shows_open_next_cycle() {
        let runs = vec![("shop".to_string(), run("SHOP-142"))];
        let mut remote = Remote::default();
        fetch_forge(&gh(list(&[pr(87, "CLOSED", "feat/SHOP-142-discount-code")])), &shop(Forge::Github), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((87, MrState::Closed, Ci::None)));
        fetch_forge(&gh(list(&[pr(87, "OPEN", "feat/SHOP-142-discount-code")])), &shop(Forge::Github), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((87, MrState::Open, Ci::None)));
    }

    #[test]
    fn run_shows_its_new_mr_after_the_old_one_merged() {
        let mut remote = Remote::default();
        let runs = vec![("shop".to_string(), run("SHOP-142"))];
        fetch_forge(&gh(list(&[pr(87, "MERGED", "feat/SHOP-142-discount-code")])), &shop(Forge::Github), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((87, MrState::Merged, Ci::None)));
        let runs = vec![("shop".to_string(), with_mr(run("SHOP-142"), 91))];
        let both = list(&[pr(91, "OPEN", "feat/SHOP-142-again"), pr(87, "MERGED", "feat/SHOP-142-discount-code")]);
        fetch_forge(&gh(both), &shop(Forge::Github), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((91, MrState::Open, Ci::None)));
    }

    #[test]
    fn mr_missing_from_the_list_keeps_its_last_status() {
        let runs = vec![("shop".to_string(), run("SHOP-142"))];
        let old = MrStatus { number: 87, url: "u".into(), state: MrState::Open, draft: false, ci: Ci::Green, pipeline_url: None };
        let mut remote = Remote { mrs: HashMap::from([(mr_key("SHOP", "SHOP-142"), old)]), ..Remote::default() };
        fetch_forge(&gh(list(&[pr(5, "OPEN", "feat/other")])), &shop(Forge::Github), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((87, MrState::Open, Ci::Green)));
        assert!(remote.forge_state.ok);
    }

    #[test]
    fn fifty_tickets_cost_two_github_list_calls() {
        let keys: Vec<String> = (1..=50).map(|n| format!("SHOP-{n}")).collect();
        let mut remote = Remote { jira: tickets(&keys, StatusCategory::Todo), ..Remote::default() };
        let f = gh(list(&[pr(70, "OPEN", "feat/SHOP-70-x"), pr(7, "OPEN", "feat/SHOP-7-x")]));
        fetch_forge(&f, &shop(Forge::Github), &[], &mut remote, now());
        assert_eq!(f.calls(), vec!["gh api user", GH_OPEN, GH_ALL]);
        // A to-do ticket is matched too.
        assert_eq!(mr(&remote, "SHOP-7"), Some((7, MrState::Open, Ci::None)));
        assert_eq!(remote.mrs.len(), 1);
    }

    #[test]
    fn gitlab_reads_the_pipeline_of_open_matched_mrs_only() {
        let runs = vec![("shop".to_string(), with_mr(run("SHOP-142"), 9)), ("shop".to_string(), run("SHOP-133"))];
        let f = Fake::new(|_, _, a| Ok(if a.get(2) == Some(&"GET") { fx("glab-mr-list.json") } else { fx("glab-mr.json") }));
        let mut remote = Remote::default();
        fetch_forge(&f, &shop(Forge::Gitlab), &runs, &mut remote, now());
        assert_eq!(f.calls(), vec![
            "glab api --method GET projects/:id/merge_requests -f state=all -f per_page=100 -f order_by=updated_at",
            "glab api projects/:id/merge_requests/9",
        ]);
        assert_eq!(mr(&remote, "SHOP-142"), Some((9, MrState::Open, Ci::Running)));
        assert_eq!(mr(&remote, "SHOP-133"), Some((71, MrState::Closed, Ci::None)));
    }

    #[test]
    fn gitlab_pipeline_error_keeps_the_last_ci_of_the_same_mr() {
        let runs = vec![("shop".to_string(), with_mr(run("SHOP-142"), 9))];
        let old = MrStatus { number: 9, url: "u".into(), state: MrState::Open, draft: false, ci: Ci::Red, pipeline_url: Some("p".into()) };
        let mut remote = Remote { mrs: HashMap::from([(mr_key("SHOP", "SHOP-142"), old)]), ..Remote::default() };
        let f = Fake::new(|_, _, a| if a.get(2) == Some(&"GET") { Ok(fx("glab-mr-list.json")) } else { Err("glab: HTTP 500".into()) });
        fetch_forge(&f, &shop(Forge::Gitlab), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((9, MrState::Open, Ci::Red)));
        assert_eq!(remote.mrs[&mr_key("SHOP", "SHOP-142")].pipeline_url.as_deref(), Some("p"));
        assert_eq!(remote.forge_state.error.as_deref(), Some("glab: HTTP 500"));
    }

    #[test]
    fn tracker_ticket_in_two_repos_takes_the_first_repos_mr() {
        let mut remote = Remote { jira: tickets(&["SHOP-150".into()], StatusCategory::InProgress), ..Remote::default() };
        let f = Fake::new(|d, _, a| Ok(match a[0] {
            "api" => r#"{"login":"al"}"#.into(),
            _ if d.ends_with("SHOP-142") => list(&[pr(9, "OPEN", "feat/SHOP-150-a")]),
            _ => list(&[pr(12, "OPEN", "feat/SHOP-150-b")]),
        }));
        fetch_forge(&f, &two_repos(), &[], &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-150"), Some((9, MrState::Open, Ci::None)));
        assert_eq!(remote.mr_repos.get(&mr_key("SHOP", "SHOP-150")).map(String::as_str), Some("a"));
    }

    fn fx(n: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/forge/{n}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn failing_list_keeps_that_repos_statuses_and_others_update() {
        let runs = vec![("a".to_string(), run("SHOP-142")), ("b".to_string(), run("SHOP-133"))];
        let old = |n| MrStatus { number: n, url: "u".into(), state: MrState::Open, draft: false, ci: Ci::Green, pipeline_url: None };
        let mut remote = Remote {
            mrs: HashMap::from([(mr_key("SHOP", "SHOP-142"), old(87)), (mr_key("SHOP", "SHOP-133"), old(71))]),
            ..Remote::default()
        };
        let f = Fake::new(|d, _, a| match a[0] {
            "api" => Ok(r#"{"login":"al"}"#.into()),
            _ if d.ends_with("SHOP-142") => Err("gh: HTTP 502".into()),
            _ => Ok(list(&[pr(71, "MERGED", "fix/SHOP-133-rounding")])),
        });
        fetch_forge(&f, &two_repos(), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-142"), Some((87, MrState::Open, Ci::Green)));
        assert_eq!(mr(&remote, "SHOP-133"), Some((71, MrState::Merged, Ci::None)));
        let s = &remote.forge_state;
        assert_eq!((s.ok, s.error.as_deref()), (false, Some("gh: HTTP 502")));
    }

    #[test]
    fn tracker_ticket_takes_the_first_repo_with_its_mr_and_runs_stay_in_their_repo() {
        let runs = vec![("a".to_string(), run("SHOP-142"))];
        let mut remote = Remote { jira: tickets(&["SHOP-150".into()], StatusCategory::InProgress), ..Remote::default() };
        let f = Fake::new(|d, _, a| Ok(match a[0] {
            "api" => r#"{"login":"al"}"#.into(),
            _ if d.ends_with("SHOP-142") => list(&[pr(3, "OPEN", "feat/SHOP-15-x")]),
            // Same number as run SHOP-142's mr.json, in another repo: not its MR.
            _ => list(&[pr(9, "OPEN", "feat/SHOP-150-y"), pr(87, "OPEN", "feat/SHOP-142-elsewhere")]),
        }));
        fetch_forge(&f, &two_repos(), &runs, &mut remote, now());
        assert_eq!(mr(&remote, "SHOP-150"), Some((9, MrState::Open, Ci::None)));
        assert_eq!(remote.mr_repos.get(&mr_key("SHOP", "SHOP-150")).map(String::as_str), Some("b"));
        assert_eq!(mr(&remote, "SHOP-142"), None);
    }

    #[test]
    fn open_github_issue_without_run_gets_its_pr() {
        let repo = Repo { name: "shop".into(), path: runs_dir(), forge: Some(Forge::Github), tracker: Tracker::Github, jira_project: None, remote: Some("acme/shop".into()) };
        let projects = vec![ProjectDef { key: "acme/shop".into(), tracker: Tracker::Github, repos: vec![repo] }];
        let issue = r#"[{"number":12,"title":"t","state":"OPEN","stateReason":null,"assignees":[],"labels":[],"url":"u"}]"#;
        let f = Fake::new(move |_, _, a| Ok(match (a[0], a[1], a.get(3).copied()) {
            ("issue", "list", Some("open")) => issue.into(),
            ("issue", _, _) => "[]".into(),
            ("repo", _, _) => r#"{"visibility":"PRIVATE"}"#.into(),
            ("api", _, _) => r#"{"login":"al"}"#.into(),
            _ => list(&[pr(13, "OPEN", "fix/gh-12-typo")]),
        }));
        let mut remote = Remote::default();
        fetch_forge(&f, &projects, &[], &mut remote, now());
        assert_eq!(remote.mrs.get(&mr_key("acme/shop", "GH-12")).map(|m| m.number), Some(13));
    }

    #[test]
    fn jira_without_token_keeps_github_issues() {
        let projects = vec![
            ProjectDef { key: "SHOP".into(), tracker: Tracker::Jira, repos: vec![] },
            ProjectDef { key: "acme/shop".into(), tracker: Tracker::Github, repos: vec![] },
        ];
        let issue = GithubIssue { number: 1, title: "t".into(), state: IssueState::Open, assignees: vec![], labels: vec![], url: "u".into() };
        let mut remote = Remote {
            me_jira: Some(Person { account_id: "a".into(), display_name: "A".into() }),
            jira: vec![crate::jira::JiraProject { key: "SHOP".into(), sprints: vec![], has_sprints: false, tickets: vec![] }],
            issues: HashMap::from([("acme/shop".to_string(), vec![issue])]),
            ..Remote::default()
        };
        tauri::async_runtime::block_on(fetch_jira(&mut remote, &Settings::default(), None, &projects));
        assert!(!remote.jira_state.configured);
        assert!(remote.jira.is_empty() && remote.me_jira.is_none());
        assert_eq!(remote.issues["acme/shop"].len(), 1);
        let s = snapshot(&projects, &[], &remote, Utc::now());
        assert!(!s.jira.configured);
        assert_eq!(s.tickets.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(), vec!["GH-1"]);
    }
}
