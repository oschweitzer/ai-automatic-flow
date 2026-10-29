use crate::classify::{Ci, GithubIssue, IssueState, MrState, MrStatus, Person};
use crate::repos::{Forge, Repo};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::Instant;

const CLI_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub trait Runner: Send + Sync {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, dir: &Path, program: &str, args: &[&str]) -> Result<String, String> {
        let mut cmd = Command::new(program);
        cmd.args(args).current_dir(dir);
        // The app has no console: without this, Windows flashes a console window for every call.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        // Started from Finder or the Dock, the app gets launchd's PATH, which lacks Homebrew's folders.
        #[cfg(target_os = "macos")]
        {
            let mut dirs: Vec<std::path::PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
            dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(std::path::PathBuf::from));
            if let Ok(path) = std::env::join_paths(dirs) {
                cmd.env("PATH", path);
            }
        }
        let (success, stdout, stderr) = output(cmd, program, CLI_TIMEOUT)?;
        if success {
            Ok(stdout)
        } else {
            Err(format!("{program}: {}", short_err(&stderr)))
        }
    }
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    })
}

/// Runs `cmd` like `Command::output`, but kills it after `limit`: a hung CLI must not hold the fetch forever.
fn output(mut cmd: Command, program: &str, limit: std::time::Duration) -> Result<(bool, String, String), String> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("{program}: {e}"))?;
    // Read both pipes while waiting, or a child filling one would block forever.
    let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait().map_err(|e| format!("{program}: {e}"))? {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{program}: timed out"));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    Ok((status.success(), out.join().unwrap_or_default(), err.join().unwrap_or_default()))
}

/// The sidebar shows the error: its first line, at most 200 characters.
fn short_err(s: &str) -> String { s.trim().lines().next().unwrap_or("").chars().take(200).collect() }

fn json(s: &str) -> Option<Value> { serde_json::from_str(s).ok() }

fn text(v: &Value, k: &str) -> String { v[k].as_str().unwrap_or("").to_string() }

/// True when `branch` holds `key` as a whole key, case-insensitive: "feat/shop-14-fix" holds SHOP-14, "feat/shop-142-x" does not.
fn has_key(branch: &str, key: &str) -> bool {
    let (b, k) = (branch.to_lowercase(), key.to_lowercase());
    b.match_indices(&k).any(|(i, _)| {
        !b[..i].chars().next_back().is_some_and(char::is_alphanumeric)
            && !b[i + k.len()..].chars().next().is_some_and(|c| c.is_ascii_digit())
    })
}

/// A repository's MR with its head branch.
pub type Listed = (String, MrStatus);

/// The repository's MRs, in no useful order. GitLab's list has no pipeline: see `pipeline`.
pub fn mr_list(r: &dyn Runner, repo: &Repo) -> Result<Vec<Listed>, String> {
    let dir = repo.path.as_path();
    let gh = |args: &[&str]| parse_gh_list(&r.run(dir, "gh", args)?).ok_or_else(|| "gh: unexpected pr list output".to_string());
    match repo.forge {
        None => Ok(vec![]),
        Some(Forge::Github) => {
            // Checks are slow to list, so only open PRs carry them. An open PR older than the 100 newest open ones
            // is left out of both and keeps its last status.
            let open = gh(&["pr", "list", "--state", "open", "--limit", "100", "--json", "number,url,state,isDraft,headRefName,statusCheckRollup"])?;
            let mut done: Vec<Listed> = gh(&["pr", "list", "--state", "all", "--limit", "200", "--json", "number,url,state,isDraft,headRefName"])?
                .into_iter().filter(|(_, m)| m.state != MrState::Open).collect();
            // A PR merged between the two calls is in both: the later read wins.
            let gone: Vec<u64> = done.iter().map(|(_, m)| m.number).collect();
            done.extend(open.into_iter().filter(|(_, o)| !gone.contains(&o.number)));
            Ok(done)
        }
        Some(Forge::Gitlab) => parse_glab_list(&r.run(dir, "glab", &["api", "--method", "GET", "projects/:id/merge_requests",
            "-f", "state=all", "-f", "per_page=100", "-f", "order_by=updated_at"])?).ok_or_else(|| "glab: unexpected merge requests output".into()),
    }
}

/// The single GitLab MR, which carries its head pipeline.
pub fn pipeline(r: &dyn Runner, repo: &Repo, iid: u64) -> Result<Option<MrStatus>, String> {
    Ok(parse_glab(&r.run(&repo.path, "glab", &["api", &format!("projects/:id/merge_requests/{iid}")])?))
}

/// The run's `mr.json` number, else the newest MR whose branch holds the key, an open one first.
/// List order is not used: GitLab's updated_at order moves an old MR up when someone comments on it.
pub fn match_mr<'a>(list: &'a [Listed], number: Option<u64>, key: &str) -> Option<&'a MrStatus> {
    match number {
        Some(n) => list.iter().map(|(_, m)| m).find(|m| m.number == n),
        None => list.iter().filter(|(b, _)| has_key(b, key)).map(|(_, m)| m).max_by_key(|m| (m.state == MrState::Open, m.number)),
    }
}

pub fn parse_gh_list(list: &str) -> Option<Vec<Listed>> {
    Some(json(list)?.as_array()?.iter().filter_map(|v| {
        let state = match v["state"].as_str()? {
            "OPEN" => MrState::Open,
            "MERGED" => MrState::Merged,
            _ => MrState::Closed,
        };
        let checks = v["statusCheckRollup"].as_array().cloned().unwrap_or_default();
        // A CheckRun has a conclusion once completed, else a status. A StatusContext has a state.
        let word = |c: &Value| ["conclusion", "state", "status"].iter().find_map(|k| c[*k].as_str().filter(|s| !s.is_empty())).unwrap_or("").to_string();
        let fail = |c: &Value| ["FAILURE", "ERROR", "TIMED_OUT", "CANCELLED", "ACTION_REQUIRED"].contains(&word(c).as_str());
        let words: Vec<String> = checks.iter().map(word).collect();
        let ci = if checks.is_empty() { Ci::None }
            else if checks.iter().any(fail) { Ci::Red }
            else if words.iter().any(|w| ["PENDING", "QUEUED", "IN_PROGRESS", "WAITING", "REQUESTED", "EXPECTED"].contains(&w.as_str())) { Ci::Running }
            else if words.iter().all(|w| ["SUCCESS", "NEUTRAL", "SKIPPED"].contains(&w.as_str())) { Ci::Green }
            else { Ci::Red }; // deliberate: an unknown word (STALE, a null conclusion with no status) counts as failed
        let link = |c: &Value| ["detailsUrl", "targetUrl"].iter().find_map(|k| c[*k].as_str().filter(|s| !s.is_empty())).map(String::from);
        let pipeline_url = checks.iter().find(|c| fail(c)).or(checks.first()).and_then(link);
        Some((text(v, "headRefName"), MrStatus { number: v["number"].as_u64()?, url: text(v, "url"), state, draft: v["isDraft"].as_bool().unwrap_or(false), ci, pipeline_url }))
    }).collect())
}

pub fn parse_glab_list(list: &str) -> Option<Vec<Listed>> {
    Some(json(list)?.as_array()?.iter().filter_map(|v| Some((text(v, "source_branch"), glab_status(v)?))).collect())
}

pub fn parse_glab(mr: &str) -> Option<MrStatus> { glab_status(&json(mr)?) }

fn glab_status(v: &Value) -> Option<MrStatus> {
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
        url: text(v, "web_url"),
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
pub(crate) mod tests {
    use super::*;
    use crate::repos::Tracker;
    use std::path::PathBuf;

    // Does nothing in a normal run. The timeout test starts this test binary on it with the variable set.
    #[test]
    fn sleeper() {
        if std::env::var_os("DASHBOARD_TEST_SLEEP").is_some() {
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    }

    fn self_on_sleeper() -> Command {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args(["--exact", "forge::tests::sleeper", "--test-threads=1"]);
        cmd
    }

    #[test]
    fn slow_cli_is_killed_after_the_deadline() {
        let mut cmd = self_on_sleeper();
        cmd.env("DASHBOARD_TEST_SLEEP", "1");
        let start = Instant::now();
        assert_eq!(output(cmd, "slow", std::time::Duration::from_secs(1)), Err("slow: timed out".to_string()));
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        let (ok, stdout, _) = output(self_on_sleeper(), "fast", std::time::Duration::from_secs(30)).unwrap();
        assert!(ok && stdout.contains("1 passed"), "{stdout}");
    }

    fn fx(n: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/forge/{n}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    type Calls = std::sync::Mutex<Vec<String>>;
    type Answer = Box<dyn Fn(&Path, &str, &[&str]) -> Result<String, String> + Send + Sync>;
    /// Records each call as "program args" and answers it from the closure, given the folder it runs in.
    pub(crate) struct Fake(Answer, Calls);
    impl Fake {
        pub(crate) fn new(f: impl Fn(&Path, &str, &[&str]) -> Result<String, String> + Send + Sync + 'static) -> Self { Fake(Box::new(f), Calls::default()) }
        pub(crate) fn calls(&self) -> Vec<String> { self.1.lock().unwrap().clone() }
    }
    impl Runner for Fake {
        fn run(&self, d: &Path, p: &str, a: &[&str]) -> Result<String, String> {
            self.1.lock().unwrap().push(format!("{p} {}", a.join(" ")));
            (self.0)(d, p, a)
        }
    }

    fn repo(forge: Option<Forge>) -> Repo {
        Repo { name: "shop".into(), path: PathBuf::from("."), forge, tracker: Tracker::Jira, jira_project: None, remote: Some("acme/shop".into()) }
    }

    #[test]
    fn parses_gh_list_with_each_ci_case() {
        let v = parse_gh_list(&fx("gh-pr-list-open.json")).unwrap();
        let row = |n: u64| v.iter().find(|(_, m)| m.number == n).map(|(_, m)| m).unwrap();
        let ci = |n: u64| row(n).ci.clone();
        let url = |n: u64| row(n).pipeline_url.clone();
        assert_eq!(v.len(), 10);
        assert_eq!(v[0].0, "feat/SHOP-150-draft");
        assert_eq!((row(150).state.clone(), row(150).draft, ci(150), url(150)), (MrState::Open, true, Ci::None, None));
        assert_eq!(row(150).url, "https://github.com/acme/shop/pull/150");
        assert_eq!((ci(149), url(149).as_deref()), (Ci::Red, Some("https://github.com/acme/shop/actions/runs/1/job/2")));
        assert_eq!((ci(148), url(148).as_deref()), (Ci::Red, Some("https://ci.example.com/build/3")));
        assert_eq!((ci(147), url(147).as_deref()), (Ci::Running, Some("https://github.com/acme/shop/actions/runs/4/job/5")));
        assert_eq!((ci(146), url(146)), (Ci::Running, None));
        assert_eq!(ci(145), Ci::Green);
        assert_eq!(ci(144), Ci::Red);
        assert_eq!((ci(143), ci(142), ci(141)), (Ci::Red, Ci::Red, Ci::Running));
        assert!(parse_gh_list("not json").is_none());
        // The all-states list has no checks field.
        let all = parse_gh_list(&fx("gh-pr-list-all.json")).unwrap();
        let st: Vec<(u64, MrState, Ci)> = all.iter().map(|(_, m)| (m.number, m.state.clone(), m.ci.clone())).collect();
        assert_eq!(st[2..4], [(131, MrState::Closed, Ci::None), (130, MrState::Merged, Ci::None)]);
    }

    #[test]
    fn parses_glab_list_without_pipeline() {
        let v = parse_glab_list(&fx("glab-mr-list.json")).unwrap();
        let got: Vec<(&str, u64, MrState, bool, Ci)> = v.iter().map(|(b, m)| (b.as_str(), m.number, m.state.clone(), m.draft, m.ci.clone())).collect();
        assert_eq!(got, vec![
            ("feat/SHOP-142-tva", 9, MrState::Open, false, Ci::None),
            ("feat/SHOP-142-first", 7, MrState::Merged, false, Ci::None),
            ("fix/SHOP-133-rounding", 71, MrState::Closed, false, Ci::None),
            ("fix/unrelated", 3, MrState::Open, true, Ci::None),
        ]);
        assert_eq!(v[0].1.url, "https://gitlab.com/acme/shop/-/merge_requests/9");
    }

    const GH_OPEN: &str = "gh pr list --state open --limit 100 --json number,url,state,isDraft,headRefName,statusCheckRollup";
    const GH_ALL: &str = "gh pr list --state all --limit 200 --json number,url,state,isDraft,headRefName";

    #[test]
    fn list_and_pipeline_calls() {
        let gh = Fake::new(|_, _, a| Ok(if a[3] == "open" { fx("gh-pr-list-open.json") } else { fx("gh-pr-list-all.json") }));
        let v = mr_list(&gh, &repo(Some(Forge::Github))).unwrap();
        assert_eq!(gh.calls(), vec![GH_OPEN, GH_ALL]);
        let mut got: Vec<(u64, MrState, Ci)> = v.iter().map(|(_, m)| (m.number, m.state.clone(), m.ci.clone())).collect();
        got.sort_by_key(|g| g.0);
        // Open PRs come from the open list with their checks. #120, open but outside it, is left out to keep its last status.
        assert_eq!(got.len(), 12);
        assert_eq!(got[..3], [(130, MrState::Merged, Ci::None), (131, MrState::Closed, Ci::None), (141, MrState::Open, Ci::Running)]);
        assert_eq!(got[11], (150, MrState::Open, Ci::None));
        assert!(v.iter().find(|(_, m)| m.number == 150).unwrap().1.draft);
        let gl = Fake::new(|_, _, a| Ok(if a.get(2) == Some(&"GET") { fx("glab-mr-list.json") } else { fx("glab-mr.json") }));
        assert_eq!(mr_list(&gl, &repo(Some(Forge::Gitlab))).unwrap().len(), 4);
        let s = pipeline(&gl, &repo(Some(Forge::Gitlab)), 9).unwrap().unwrap();
        assert_eq!((s.number, s.ci, s.pipeline_url.as_deref()), (9, Ci::Running, Some("https://gitlab.com/acme/shop/-/pipelines/99")));
        assert_eq!(gl.calls(), vec![
            "glab api --method GET projects/:id/merge_requests -f state=all -f per_page=100 -f order_by=updated_at",
            "glab api projects/:id/merge_requests/9",
        ]);
        let bad = Fake::new(|_, _, _| Ok("oops".into()));
        assert!(mr_list(&bad, &repo(Some(Forge::Github))).unwrap_err().contains("gh"));
        let off = Fake::new(|_, _, _| Err("glab: not found".into()));
        assert!(mr_list(&off, &repo(Some(Forge::Gitlab))).unwrap_err().contains("glab"));
        // The open list failing fails the repo, even when the all-states list answers.
        let half = Fake::new(|_, _, a| if a[3] == "open" { Err("gh: HTTP 502".into()) } else { Ok(fx("gh-pr-list-all.json")) });
        assert_eq!(mr_list(&half, &repo(Some(Forge::Github))).unwrap_err(), "gh: HTTP 502");
    }

    fn listed(rows: &[(&str, u64, MrState)]) -> Vec<Listed> {
        rows.iter().map(|(b, n, s)| (b.to_string(), MrStatus { number: *n, url: String::new(), state: s.clone(), draft: false, ci: Ci::None, pipeline_url: None })).collect()
    }

    #[test]
    fn matches_by_number_else_newest_open_branch_with_the_key() {
        use MrState::*;
        let l = listed(&[("feat/shop-142-x", 5, Open), ("feat/shop-14-old", 4, Open), ("feat/SHOP-14-fix", 6, Open), ("feat/xshop-14", 7, Open)]);
        assert_eq!(match_mr(&l, Some(4), "SHOP-14").map(|m| m.number), Some(4));
        assert_eq!(match_mr(&l, Some(99), "SHOP-14").map(|m| m.number), None);
        // Whatever the list order, the highest number wins among branches holding the key.
        assert_eq!(match_mr(&l, None, "SHOP-14").map(|m| m.number), Some(6));
        assert!(match_mr(&l[..1], None, "SHOP-14").is_none());
        assert!(match_mr(&l[3..], None, "SHOP-14").is_none());
        assert_eq!(match_mr(&l, None, "shop-142").map(|m| m.number), Some(5));
        // An open MR beats a newer merged one.
        let l = listed(&[("feat/SHOP-14-b", 8, Merged), ("feat/SHOP-14-a", 6, Open)]);
        assert_eq!(match_mr(&l, None, "SHOP-14").map(|m| m.number), Some(6));
    }

    #[test]
    fn an_old_merged_mr_listed_first_loses_to_the_newer_open_one() {
        let gh = parse_gh_list(r#"[{"number":87,"url":"u","state":"MERGED","isDraft":false,"headRefName":"feat/SHOP-142-a"},
            {"number":91,"url":"u","state":"OPEN","isDraft":false,"headRefName":"feat/SHOP-142-b"}]"#).unwrap();
        assert_eq!(match_mr(&gh, None, "SHOP-142").map(|m| m.number), Some(91));
        // GitLab orders by updated_at: a comment on the old merged MR moves it up.
        let gl = parse_glab_list(r#"[{"iid":7,"state":"merged","source_branch":"feat/SHOP-142-first"},
            {"iid":9,"state":"opened","source_branch":"feat/SHOP-142-tva"}]"#).unwrap();
        assert_eq!(match_mr(&gl, None, "SHOP-142").map(|m| m.number), Some(9));
    }

    #[test]
    fn cli_error_keeps_first_line_and_200_chars() {
        assert_eq!(short_err("  boom\nsecond line\n"), "boom");
        assert_eq!(short_err(&"x".repeat(500)).chars().count(), 200);
        assert_eq!(short_err(&"é".repeat(300)).chars().count(), 200);
    }

    #[test]
    fn issues_runs_open_and_recent_closed() {
        let f = Fake::new(|_, _, _| Ok(fx("gh-issue-list.json")));
        let now = "2026-10-29T12:00:00Z".parse().unwrap();
        assert_eq!(issues(&f, &repo(None), now).unwrap().len(), 6);
        let fields = "number,title,state,stateReason,assignees,labels,url";
        assert_eq!(f.calls(), vec![
            format!("gh issue list --state open --json {fields} --limit 500"),
            format!("gh issue list --state closed --search closed:>=2026-10-15 --json {fields} --limit 500"),
        ]);
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
