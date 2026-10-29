use crate::classify::{classify, Ci, GithubIssue, IssueState, MrStatus, Person, Placement, TrackerTicket};
use crate::jira::{JiraProject, Sprint};
use crate::repos::{Forge, ProjectDef, Repo, Tracker};
use crate::runs::{steps, PlanSummary, RunState, StepInfo, StepState, TriageInfo, STAGES};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track { pub current: u8, pub total: u8, pub stage: String, pub failed: bool }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    pub key: String, pub project: String, pub summary: String, pub issue_type: Option<String>,
    pub repo: Option<String>, pub sprint_id: Option<u64>, pub assignees: Vec<Person>, // Jira: zero or one
    pub local_run: bool, pub placement: Placement, pub track: Option<Track>, pub steps: Vec<StepInfo>,
    pub mr: Option<MrStatus>, pub ticket_url: Option<String>, pub ticket_status: Option<String>, // Jira status, or the issue state
    pub labels: Vec<String>, // GitHub issues only
    pub triage: Option<TriageInfo>, pub plan: Option<PlanSummary>,
    pub run_dir: Option<String>, pub plan_path: Option<String>, pub handover_path: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub key: String, pub tracker: Tracker, pub repos: Vec<String>, pub forge: Option<Forge>, pub public: bool,
    pub sprints: Vec<Sprint>, pub has_sprints: bool, // GitHub tracker: has_sprints false
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceState { pub configured: bool, pub ok: bool, pub error: Option<String>, pub last_ok_at: Option<DateTime<Utc>> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub me_jira: Option<Person>, pub me_github: Option<Person>, pub projects: Vec<ProjectView>, pub tickets: Vec<Ticket>,
    pub people: Vec<Person>, pub jira: SourceState, pub forge: SourceState, pub synced_at: DateTime<Utc>,
}

/// The last remote data, kept as is when a fetch fails.
#[derive(Debug, Clone, Default)]
pub struct Remote {
    pub me_jira: Option<Person>,
    pub me_github: Option<Person>,
    pub jira: Vec<JiraProject>,
    pub issues: HashMap<String, Vec<GithubIssue>>, // by project key
    pub mrs: HashMap<String, MrStatus>,            // by mr_key
    pub mr_repos: HashMap<String, String>,         // by mr_key: the repo the MR was found in, tickets without a run only
    pub public: HashMap<String, bool>,             // by project key, GitHub-tracker projects
    pub jira_state: SourceState,
    pub forge_state: SourceState,
}

/// MR map key. GitHub keys (`GH-<n>`) repeat across repositories, so the project qualifies them.
pub fn mr_key(project: &str, ticket: &str) -> String { format!("{project}\n{ticket}") }

pub fn issue_key(number: u64) -> String { format!("GH-{number}") }

/// Whether a run read from repository `name` belongs to `repo`. Folder names repeat across roots, so the path decides.
pub fn run_of(repo: &Repo, name: &str, run: &RunState) -> bool { repo.name == name && run.run_dir.starts_with(&repo.path) }

#[allow(clippy::too_many_arguments)] // the signature the plan fixed
pub fn build(projects: &[ProjectDef], runs: &[(String, RunState)],      // (repo name, run)
             jira: &[JiraProject], issues: &HashMap<String, Vec<GithubIssue>>, // issues by project key
             mrs: &HashMap<String, MrStatus>,                            // by mr_key(project, ticket key)
             me_jira: Option<&Person>, me_github: Option<&Person>, now: DateTime<Utc>) -> Vec<Ticket> {
    // "Me" is a front end filter: a local run is mine whatever the assignee, and the `local_run` flag says so.
    let _ = (me_jira, me_github);
    let mut out = Vec::new();
    for p in projects {
        let trackers: HashMap<String, TrackerTicket> = match p.tracker {
            Tracker::Jira => jira.iter().filter(|j| j.key == p.key).flat_map(|j| &j.tickets)
                .map(|t| (t.key.clone(), TrackerTicket::Jira(t.clone()))).collect(),
            Tracker::Github => issues.get(&p.key).into_iter().flatten()
                .map(|i| (issue_key(i.number), TrackerTicket::Github(i.clone()))).collect(),
        };
        let own: Vec<&(String, RunState)> = runs.iter().filter(|(name, run)| p.repos.iter().any(|r| run_of(r, name, run))).collect();
        let keys: BTreeSet<&String> = trackers.keys().chain(own.iter().map(|(_, r)| &r.key)).collect();
        for key in keys {
            let run = own.iter().find(|(_, r)| &r.key == key).map(|(repo, r)| (repo, r));
            out.push(ticket(p, key, run, trackers.get(key), mrs.get(&mr_key(&p.key, key)), now));
        }
    }
    out
}

fn ticket(p: &ProjectDef, key: &str, run: Option<(&String, &RunState)>, tracker: Option<&TrackerTicket>, mr: Option<&MrStatus>, now: DateTime<Utc>) -> Ticket {
    let (summary, issue_type, sprint_id, assignees, ticket_url, ticket_status, labels) = match tracker {
        Some(TrackerTicket::Jira(j)) => (Some(j.summary.clone()), Some(j.issue_type.clone()), j.sprint_id,
            j.assignee.iter().cloned().collect(), Some(j.url.clone()), Some(j.status.clone()), vec![]),
        Some(TrackerTicket::Github(g)) => (Some(g.title.clone()), None, None,
            g.assignees.clone(), Some(g.url.clone()), Some(issue_state(&g.state).to_string()), g.labels.clone()),
        None => (None, None, None, vec![], None, None, vec![]),
    };
    let r = run.map(|(_, r)| r);
    let steps = r.map(|r| steps(r, mr.is_some_and(|m| m.ci == Ci::Red))).unwrap_or_default();
    let track = r.map(|r| Track {
        current: r.stage_index,
        total: STAGES.len() as u8,
        stage: STAGES.get(usize::from(r.stage_index).saturating_sub(1)).unwrap_or(&"").to_string(),
        failed: steps.iter().any(|s| s.state == StepState::Fail),
    });
    let in_run = |name: &str| r.map(|r| r.run_dir.join(name).to_string_lossy().into_owned());
    Ticket {
        key: key.to_string(),
        project: p.key.clone(),
        summary: summary.or_else(|| r.and_then(|r| r.summary.clone())).unwrap_or_else(|| key.to_string()),
        issue_type,
        // A tracker-only ticket gets the project's only repo here, `snapshot` puts the MR's repo over it.
        repo: run.map(|(repo, _)| repo.clone()).or_else(|| (p.repos.len() == 1).then(|| p.repos[0].name.clone())),
        sprint_id,
        assignees,
        local_run: r.is_some(),
        placement: classify(r, tracker, mr, now),
        track,
        steps,
        mr: mr.cloned(),
        ticket_url,
        ticket_status,
        labels,
        triage: r.and_then(|r| r.triage.clone()),
        plan: r.and_then(|r| r.plan.clone()),
        run_dir: r.map(|r| r.run_dir.to_string_lossy().into_owned()),
        plan_path: r.filter(|r| r.plan.is_some()).and_then(|_| in_run("plan.json")),
        handover_path: r.filter(|r| r.has_handover).and_then(|_| in_run("handover.md")),
        last_activity: r.map(|r| r.last_at),
    }
}

fn issue_state(s: &IssueState) -> &'static str {
    match s {
        IssueState::Open => "open",
        IssueState::Completed => "completed",
        IssueState::NotPlanned => "notPlanned",
    }
}

pub fn snapshot(projects: &[ProjectDef], runs: &[(String, RunState)], remote: &Remote, now: DateTime<Utc>) -> Snapshot {
    let mut tickets = build(projects, runs, &remote.jira, &remote.issues, &remote.mrs,
        remote.me_jira.as_ref(), remote.me_github.as_ref(), now);
    for t in tickets.iter_mut().filter(|t| !t.local_run) {
        if let Some(repo) = remote.mr_repos.get(&mr_key(&t.project, &t.key)) {
            t.repo = Some(repo.clone());
        }
    }
    let unique: BTreeSet<&Person> = tickets.iter().flat_map(|t| &t.assignees).chain(&remote.me_jira).chain(&remote.me_github).collect();
    let mut people: Vec<Person> = unique.into_iter().cloned().collect();
    people.sort_by_cached_key(|p| p.display_name.to_lowercase());
    let projects = projects.iter().map(|p| {
        let j = remote.jira.iter().find(|j| p.tracker == Tracker::Jira && j.key == p.key);
        ProjectView {
            key: p.key.clone(),
            tracker: p.tracker,
            repos: p.repos.iter().map(|r| r.name.clone()).collect(),
            forge: p.repos.iter().find_map(|r| r.forge),
            public: p.tracker == Tracker::Github && remote.public.get(&p.key) == Some(&true),
            sprints: j.map(|j| j.sprints.clone()).unwrap_or_default(),
            has_sprints: j.is_some_and(|j| j.has_sprints),
        }
    }).collect();
    Snapshot {
        me_jira: remote.me_jira.clone(),
        me_github: remote.me_github.clone(),
        projects,
        tickets,
        people,
        jira: remote.jira_state.clone(),
        forge: remote.forge_state.clone(),
        synced_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{JiraTicket, MrState, Section, StatusCategory};
    use crate::repos::Repo;
    use crate::runs::read_run;
    use chrono::TimeZone;
    use std::path::{Path, PathBuf};

    fn now() -> DateTime<Utc> { Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap() }
    fn fixture(key: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs").join(key) }
    fn person(id: &str, name: &str) -> Person { Person { account_id: id.into(), display_name: name.into() } }
    fn repo(name: &str, tracker: Tracker, forge: Option<Forge>) -> Repo {
        // The fixture runs live below the crate folder, so it stands for the repository.
        Repo { name: name.into(), path: PathBuf::from(env!("CARGO_MANIFEST_DIR")), forge, tracker, jira_project: None, remote: None }
    }
    fn project(key: &str, repo: Repo) -> ProjectDef { ProjectDef { key: key.into(), tracker: Tracker::Jira, repos: vec![repo] } }
    fn shop() -> Vec<ProjectDef> { vec![project("SHOP", repo("front", Tracker::Jira, Some(Forge::Gitlab)))] }
    fn acme() -> Vec<ProjectDef> {
        vec![ProjectDef { key: "acme/shop".into(), tracker: Tracker::Github, repos: vec![repo("shop", Tracker::Github, Some(Forge::Github))] }]
    }
    fn jira_ticket(key: &str, assignee: Option<Person>) -> JiraTicket {
        JiraTicket {
            key: key.into(), summary: format!("{key} from Jira"), issue_type: "Story".into(), status: "In Review".into(),
            status_category: StatusCategory::InProgress, assignee, sprint_id: Some(7), url: format!("https://x.atlassian.net/browse/{key}"),
        }
    }
    fn jira_project(tickets: Vec<JiraTicket>) -> Vec<JiraProject> {
        vec![JiraProject { key: "SHOP".into(), sprints: vec![], has_sprints: false, tickets }]
    }
    fn issue(number: u64, state: IssueState, assignees: Vec<Person>) -> GithubIssue {
        GithubIssue { number, title: format!("Issue {number}"), state, assignees, labels: vec!["bug".into()], url: format!("https://github.com/acme/shop/issues/{number}") }
    }
    fn go(projects: &[ProjectDef], runs: &[(String, RunState)], jira: &[JiraProject], mrs: &HashMap<String, MrStatus>) -> Vec<Ticket> {
        build(projects, runs, jira, &HashMap::new(), mrs, None, None, now())
    }

    #[test]
    fn local_run_without_jira_uses_ticket_md_summary() {
        let runs = vec![("front".to_string(), read_run(&fixture("SHOP-142")).unwrap())];
        let t = go(&shop(), &runs, &[], &HashMap::new());
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].key.as_str(), t[0].project.as_str()), ("SHOP-142", "SHOP"));
        assert_eq!(t[0].summary, "Add a discount code field to the cart");
        assert_eq!(t[0].repo.as_deref(), Some("front"));
        assert!(t[0].local_run);
        assert_eq!((t[0].ticket_url.as_ref(), t[0].issue_type.as_ref()), (None, None));
    }

    #[test]
    fn local_run_is_mine_whatever_the_assignee() {
        let bob = person("bob", "Bob");
        let runs = vec![("front".to_string(), read_run(&fixture("SHOP-142")).unwrap())];
        let jira = jira_project(vec![jira_ticket("SHOP-142", Some(bob.clone()))]);
        let t = build(&shop(), &runs, &jira, &HashMap::new(), &HashMap::new(), Some(&person("me", "Me")), None, now());
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].assignees, vec![bob]);
        assert!(t[0].local_run);
        assert_eq!(t[0].summary, "SHOP-142 from Jira");
        assert_eq!(t[0].ticket_status.as_deref(), Some("In Review"));
    }

    #[test]
    fn jira_only_ticket_has_no_track() {
        let t = go(&shop(), &[], &jira_project(vec![jira_ticket("SHOP-200", None)]), &HashMap::new());
        assert_eq!(t.len(), 1);
        assert!(!t[0].local_run);
        assert!(t[0].track.is_none() && t[0].steps.is_empty() && t[0].run_dir.is_none());
        assert_eq!(t[0].sprint_id, Some(7));
        assert_eq!(t[0].issue_type.as_deref(), Some("Story"));
        assert_eq!(t[0].ticket_url.as_deref(), Some("https://x.atlassian.net/browse/SHOP-200"));
        // The project's only repo stands in when no run or MR says otherwise.
        assert_eq!(t[0].repo.as_deref(), Some("front"));
        assert_eq!(t[0].placement.section, Section::InProgress);
    }

    #[test]
    fn track_marks_failed_when_ci_red() {
        let mut run = read_run(&fixture("SHOP-146")).unwrap();
        run.stage_index = 5; // ci-gate, no end state
        let runs = vec![("front".to_string(), run)];
        let mr = |ci| MrStatus { number: 3, url: String::new(), state: MrState::Open, draft: false, ci, pipeline_url: None };
        let red = HashMap::from([(mr_key("SHOP", "SHOP-146"), mr(Ci::Red))]);
        let t = go(&shop(), &runs, &[], &red);
        assert_eq!(t[0].track, Some(Track { current: 5, total: 8, stage: "ci-gate".into(), failed: true }));
        assert_eq!(t[0].mr.as_ref().map(|m| m.number), Some(3));
        let green = HashMap::from([(mr_key("SHOP", "SHOP-146"), mr(Ci::Green))]);
        assert!(!go(&shop(), &runs, &[], &green)[0].track.as_ref().unwrap().failed);
    }

    #[test]
    fn people_are_unique_and_sorted() {
        let (zoe, bob, al) = (person("z", "Zoe"), person("b", "Bob"), person("a", "Al"));
        let mut projects = shop();
        projects.extend(acme());
        let remote = Remote {
            me_jira: Some(al.clone()),
            me_github: Some(bob.clone()),
            jira: jira_project(vec![jira_ticket("SHOP-1", Some(zoe.clone())), jira_ticket("SHOP-2", Some(bob.clone()))]),
            issues: HashMap::from([("acme/shop".to_string(), vec![issue(1, IssueState::Open, vec![bob.clone(), zoe.clone()])])]),
            ..Remote::default()
        };
        assert_eq!(snapshot(&projects, &[], &remote, now()).people, vec![al, bob, zoe]);
    }

    #[test]
    fn paths_point_into_run_dir() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("SHOP-142");
        std::fs::create_dir(&dir).unwrap();
        for f in std::fs::read_dir(fixture("SHOP-142")).unwrap() {
            let f = f.unwrap();
            std::fs::copy(f.path(), dir.join(f.file_name())).unwrap();
        }
        let shop = vec![project("SHOP", Repo { path: t.path().to_path_buf(), ..repo("front", Tracker::Jira, None) })];
        let ticket = |runs: &[(String, RunState)]| go(&shop, runs, &[], &HashMap::new()).remove(0);
        let a = ticket(&[("front".into(), read_run(&dir).unwrap())]);
        assert_eq!(a.run_dir.as_deref(), Some(dir.to_string_lossy().as_ref()));
        assert_eq!(a.plan_path.as_deref().map(PathBuf::from), Some(dir.join("plan.json")));
        assert_eq!(a.handover_path, None);
        std::fs::write(dir.join("handover.md"), "# handover").unwrap();
        let b = ticket(&[("front".into(), read_run(&dir).unwrap())]);
        assert_eq!(b.handover_path.as_deref().map(PathBuf::from), Some(dir.join("handover.md")));
        assert_eq!(b.last_activity, Some(Utc.with_ymd_and_hms(2026, 10, 27, 14, 20, 0).unwrap()));
    }

    #[test]
    fn github_project_without_jira_token() {
        let remote = Remote {
            issues: HashMap::from([("acme/shop".to_string(), vec![issue(142, IssueState::Open, vec![]), issue(7, IssueState::Completed, vec![])])]),
            ..Remote::default()
        };
        let s = snapshot(&acme(), &[], &remote, now());
        assert!(!s.jira.configured);
        let keys: Vec<&str> = s.tickets.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys, vec!["GH-142", "GH-7"]);
        assert_eq!(s.tickets[1].placement.section, Section::Done);
        assert_eq!(s.projects.len(), 1);
        assert_eq!((s.projects[0].tracker, s.projects[0].has_sprints), (Tracker::Github, false));
        assert_eq!(s.projects[0].repos, vec!["shop".to_string()]);
    }

    #[test]
    fn same_repo_name_in_two_roots_keeps_runs_apart() {
        let at = |root: &str| Repo { path: PathBuf::from(root).join("front"), ..repo("front", Tracker::Jira, None) };
        let projects = vec![project("PAY", at("b")), project("SHOP", at("a"))];
        let run = |root: &str, key: &str| {
            let r = read_run(&fixture("SHOP-146")).unwrap();
            ("front".to_string(), RunState { key: key.into(), run_dir: PathBuf::from(root).join("front/.ticket-flow/runs").join(key), ..r })
        };
        let t = go(&projects, &[run("a", "SHOP-1"), run("b", "PAY-12")], &[], &HashMap::new());
        let got: Vec<(&str, &str)> = t.iter().map(|t| (t.project.as_str(), t.key.as_str())).collect();
        assert_eq!(got, vec![("PAY", "PAY-12"), ("SHOP", "SHOP-1")]);
    }

    #[test]
    fn github_ticket_links_to_issue() {
        let al = person("al", "al");
        let issues = HashMap::from([("acme/shop".to_string(), vec![issue(142, IssueState::Open, vec![al.clone()])])]);
        let t = build(&acme(), &[], &[], &issues, &HashMap::new(), None, Some(&al), now());
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].key, "GH-142");
        assert_eq!(t[0].ticket_url.as_deref(), Some("https://github.com/acme/shop/issues/142"));
        assert_eq!(t[0].ticket_status.as_deref(), Some("open"));
        assert_eq!((t[0].summary.as_str(), t[0].labels.clone(), t[0].assignees.clone()), ("Issue 142", vec!["bug".to_string()], vec![al]));
        assert_eq!(t[0].issue_type, None);
    }
}
