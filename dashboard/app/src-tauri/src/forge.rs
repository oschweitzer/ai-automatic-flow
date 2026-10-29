use crate::classify::{Ci, GithubIssue, IssueState, MrState, MrStatus, Person};
use crate::repos::{Forge, Repo};
use crate::runs::MrRef;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub trait Runner: Send + Sync {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(program)
            .args(args)
            .current_dir(dir)
            .output()
            .map_err(|e| format!("{program}: {e}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        // gh pr checks exits non-zero while checks fail or run, its JSON is still valid.
        if out.status.success() || (program == "gh" && args.starts_with(&["pr", "checks"]) && !stdout.trim().is_empty()) {
            Ok(stdout)
        } else {
            Err(format!("{program}: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }
}

fn json(s: &str) -> Option<Value> { serde_json::from_str(s).ok() }

fn text(v: &Value, k: &str) -> String { v[k].as_str().unwrap_or("").to_string() }

/// First list entry whose branch contains the key, case-insensitive.
fn find_by_branch(list: &str, branch_field: &str, key: &str) -> Option<Value> {
    let key = key.to_lowercase();
    json(list)?.as_array()?.iter()
        .find(|e| e[branch_field].as_str().is_some_and(|b| b.to_lowercase().contains(&key)))
        .cloned()
}

pub fn mr_status(r: &dyn Runner, repo: &Repo, mr: Option<&MrRef>, key: &str) -> Result<Option<MrStatus>, String> {
    let dir = repo.path.as_path();
    match repo.forge {
        None => Ok(None),
        Some(Forge::Github) => {
            let number = match mr {
                Some(m) => m.number.to_string(),
                None => {
                    let list = r.run(dir, "gh", &["pr", "list", "--state", "all", "--search", key,
                        "--json", "number,headRefName", "--limit", "20"])?;
                    match find_by_branch(&list, "headRefName", key) {
                        Some(e) => match e["number"].as_u64() {
                            Some(n) => n.to_string(),
                            None => return Ok(None),
                        },
                        None => return Ok(None),
                    }
                }
            };
            let view = r.run(dir, "gh", &["pr", "view", &number, "--json", "number,url,state,isDraft"])?;
            // Best effort: a PR with no checks exits non-zero with empty stdout. A missing gh already failed at pr view.
            let checks = r.run(dir, "gh", &["pr", "checks", &number, "--json", "bucket,link"]).unwrap_or_default();
            Ok(parse_gh(&view, &checks))
        }
        Some(Forge::Gitlab) => {
            let iid = match mr {
                Some(m) => m.number,
                None => {
                    let list = r.run(dir, "glab", &["api", "--method", "GET", "projects/:id/merge_requests",
                        "-f", "state=all", "-f", &format!("search={key}"), "-f", "per_page=20"])?;
                    match find_by_branch(&list, "source_branch", key) {
                        // The list endpoint has no head_pipeline, so fetch the single MR.
                        Some(e) => match e["iid"].as_u64() {
                            Some(n) => n,
                            None => return Ok(None),
                        },
                        None => return Ok(None),
                    }
                }
            };
            Ok(parse_glab(&r.run(dir, "glab", &["api", &format!("projects/:id/merge_requests/{iid}")])?))
        }
    }
}

pub fn parse_gh(view: &str, checks: &str) -> Option<MrStatus> {
    let v = json(view)?;
    let state = match v["state"].as_str()? {
        "OPEN" => MrState::Open,
        "MERGED" => MrState::Merged,
        _ => MrState::Closed,
    };
    let checks = json(checks).and_then(|c| c.as_array().cloned()).unwrap_or_default();
    let buckets: Vec<String> = checks.iter().map(|c| text(c, "bucket")).collect();
    let ci = if buckets.is_empty() { Ci::None }
        else if buckets.iter().any(|b| b == "fail") { Ci::Red }
        else if buckets.iter().any(|b| b == "pending") { Ci::Running }
        else if buckets.iter().all(|b| b == "pass" || b == "skipping") { Ci::Green }
        else { Ci::Red };
    let pipeline_url = checks.iter()
        .find(|c| c["bucket"] == "fail").map(|c| text(c, "link")).filter(|l| !l.is_empty());
    Some(MrStatus { number: v["number"].as_u64()?, url: text(&v, "url"), state, draft: v["isDraft"].as_bool().unwrap_or(false), ci, pipeline_url })
}

pub fn parse_glab(mr: &str) -> Option<MrStatus> {
    let v = json(mr)?;
    let state = match v["state"].as_str()? {
        "opened" => MrState::Open,
        "merged" => MrState::Merged,
        _ => MrState::Closed,
    };
    let p = &v["head_pipeline"];
    let ci = match p["status"].as_str() {
        None => Ci::None,
        Some("success") => Ci::Green,
        Some("running" | "pending" | "created") => Ci::Running,
        Some(_) => Ci::Red,
    };
    Some(MrStatus {
        number: v["iid"].as_u64()?,
        url: text(&v, "web_url"),
        state,
        draft: v["draft"].as_bool().unwrap_or(false),
        ci,
        pipeline_url: p["web_url"].as_str().map(String::from),
    })
}

pub fn issues(r: &dyn Runner, repo: &Repo, now: DateTime<Utc>) -> Result<Vec<GithubIssue>, String> {
    if repo.remote.is_none() { return Ok(vec![]); }
    let dir = repo.path.as_path();
    let since = format!("closed:>={}", (now - Duration::days(14)).format("%Y-%m-%d"));
    let fields = "number,title,state,stateReason,assignees,labels,url";
    let mut all = parse_issues(&r.run(dir, "gh", &["issue", "list", "--state", "open", "--json", fields, "--limit", "500"])?);
    all.extend(parse_issues(&r.run(dir, "gh", &["issue", "list", "--state", "closed", "--search", &since, "--json", fields, "--limit", "500"])?));
    Ok(all)
}

pub fn parse_issues(list: &str) -> Vec<GithubIssue> {
    let Some(Value::Array(items)) = json(list) else { return vec![] };
    items.iter().filter_map(|i| {
        let state = match (i["state"].as_str()?, i["stateReason"].as_str()) {
            ("OPEN", _) => IssueState::Open,
            (_, Some("NOT_PLANNED")) => IssueState::NotPlanned,
            _ => IssueState::Completed,
        };
        Some(GithubIssue {
            number: i["number"].as_u64()?,
            title: text(i, "title"),
            state,
            assignees: i["assignees"].as_array().map(|a| a.iter().map(|p| {
                let login = text(p, "login");
                let name = text(p, "name");
                Person { account_id: login.clone(), display_name: if name.is_empty() { login } else { name } }
            }).collect()).unwrap_or_default(),
            labels: i["labels"].as_array().map(|a| a.iter().map(|l| text(l, "name")).collect()).unwrap_or_default(),
            url: text(i, "url"),
        })
    }).collect()
}

pub fn visibility(r: &dyn Runner, repo: &Repo) -> Result<String, String> {
    let out = r.run(&repo.path, "gh", &["repo", "view", "--json", "visibility"])?;
    json(&out).and_then(|v| v["visibility"].as_str().map(str::to_lowercase)).ok_or_else(|| "gh: unexpected repo view output".into())
}

pub fn gh_login(r: &dyn Runner, dir: &Path) -> Result<Person, String> {
    let out = r.run(dir, "gh", &["api", "user"])?;
    let v = json(&out).ok_or("gh: unexpected api user output")?;
    let login = v["login"].as_str().ok_or("gh: no login in api user output")?.to_string();
    Ok(Person { account_id: login.clone(), display_name: login })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repos::Tracker;
    use std::path::PathBuf;

    fn fx(n: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/forge/{n}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    type Calls = std::sync::Mutex<Vec<String>>;
    struct Fake(Box<dyn Fn(&str, &[&str]) -> Result<String, String> + Send + Sync>, Calls);
    impl Fake {
        fn new(f: impl Fn(&str, &[&str]) -> Result<String, String> + Send + Sync + 'static) -> Self { Fake(Box::new(f), Calls::default()) }
        fn calls(&self) -> Vec<String> { self.1.lock().unwrap().clone() }
    }
    impl Runner for Fake {
        fn run(&self, _: &Path, p: &str, a: &[&str]) -> Result<String, String> {
            self.1.lock().unwrap().push(format!("{p} {}", a.join(" ")));
            (self.0)(p, a)
        }
    }

    fn repo(forge: Option<Forge>) -> Repo {
        Repo { name: "shop".into(), path: PathBuf::from("."), forge, tracker: Tracker::Jira, jira_project: None, remote: Some("acme/shop".into()) }
    }

    #[test]
    fn parses_gh_open_red() {
        let s = parse_gh(&fx("gh-pr-view.json"), &fx("gh-pr-checks-fail.json")).unwrap();
        assert_eq!((s.number, s.state, s.draft, s.ci), (143, MrState::Open, false, Ci::Red));
        assert_eq!(s.pipeline_url.as_deref(), Some("https://github.com/acme/shop/actions/runs/1/job/2"));
    }

    #[test]
    fn parses_gh_merged_green() {
        let s = parse_gh(&fx("gh-pr-view-merged.json"), &fx("gh-pr-checks-pass.json")).unwrap();
        assert_eq!((s.state, s.ci), (MrState::Merged, Ci::Green));
        assert_eq!(parse_gh(&fx("gh-pr-view.json"), "[]").unwrap().ci, Ci::None);
    }

    #[test]
    fn parses_glab_running_draft() {
        let s = parse_glab(&fx("glab-mr.json")).unwrap();
        assert_eq!((s.number, s.state, s.draft, s.ci), (7, MrState::Open, true, Ci::Running));
        assert_eq!(s.pipeline_url.as_deref(), Some("https://gitlab.com/acme/shop/-/pipelines/99"));
    }

    #[test]
    fn finds_mr_by_branch() {
        let gh = Fake::new(|_, a| Ok(match (a[0], a[1]) {
            ("pr", "list") => fx("gh-pr-list.json"),
            ("pr", "view") => fx("gh-pr-view.json"),
            _ => fx("gh-pr-checks-fail.json"),
        }));
        let s = mr_status(&gh, &repo(Some(Forge::Github)), None, "shop-142").unwrap().unwrap();
        assert_eq!(s.number, 143);
        assert_eq!(gh.calls(), vec![
            "gh pr list --state all --search shop-142 --json number,headRefName --limit 20",
            "gh pr view 143 --json number,url,state,isDraft",
            "gh pr checks 143 --json bucket,link",
        ]);
        let gl = Fake::new(|_, a| Ok(if a.last() == Some(&"projects/:id/merge_requests/7") { fx("glab-mr.json") } else { fx("glab-mr-list.json") }));
        let s = mr_status(&gl, &repo(Some(Forge::Gitlab)), None, "shop-142").unwrap().unwrap();
        assert_eq!((s.number, s.ci), (7, Ci::Running));
        assert_eq!(gl.calls(), vec![
            "glab api --method GET projects/:id/merge_requests -f state=all -f search=shop-142 -f per_page=20",
            "glab api projects/:id/merge_requests/7",
        ]);
    }

    #[test]
    fn no_checks_gives_ci_none() {
        let gh = Fake::new(|_, a| if a[1] == "checks" { Err("gh: no checks reported".into()) } else { Ok(fx("gh-pr-view.json")) });
        let mr = MrRef { number: 143, url: String::new(), branch: String::new() };
        let s = mr_status(&gh, &repo(Some(Forge::Github)), Some(&mr), "shop-142").unwrap().unwrap();
        assert_eq!((s.number, s.ci), (143, Ci::None));
    }

    #[test]
    fn issues_runs_open_and_recent_closed() {
        let f = Fake::new(|_, _| Ok(fx("gh-issue-list.json")));
        let now = "2026-10-29T12:00:00Z".parse().unwrap();
        assert_eq!(issues(&f, &repo(None), now).unwrap().len(), 6);
        let fields = "number,title,state,stateReason,assignees,labels,url";
        assert_eq!(f.calls(), vec![
            format!("gh issue list --state open --json {fields} --limit 500"),
            format!("gh issue list --state closed --search closed:>=2026-10-15 --json {fields} --limit 500"),
        ]);
    }

    #[test]
    fn no_match_is_none() {
        let gh = Fake::new(|_, _| Ok(fx("gh-pr-list.json")));
        assert!(mr_status(&gh, &repo(Some(Forge::Github)), None, "shop-999").unwrap().is_none());
        assert!(mr_status(&gh, &repo(None), None, "shop-142").unwrap().is_none());
    }

    #[test]
    fn missing_cli_is_an_error() {
        let f = Fake::new(|_, _| Err("gh: not found".into()));
        let e = mr_status(&f, &repo(Some(Forge::Github)), None, "shop-142").unwrap_err();
        assert!(e.contains("gh"));
    }

    #[test]
    fn parses_issue_list() {
        let v = parse_issues(&fx("gh-issue-list.json"));
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].state, IssueState::Open);
        assert_eq!(v[0].labels, vec!["bug"]);
        assert_eq!(v[0].assignees[0].display_name, "Bob Dev");
        assert_eq!(v[0].assignees[1].display_name, "zed");
        assert_eq!(v[2].state, IssueState::Completed);
    }

    #[test]
    fn not_planned_maps_to_not_planned() {
        assert_eq!(parse_issues(&fx("gh-issue-list.json"))[1].state, IssueState::NotPlanned);
    }
}
