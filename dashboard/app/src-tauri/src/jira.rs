use crate::classify::{JiraTicket, Person, StatusCategory};
use crate::forge::Runner;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprint {
    pub id: u64,
    pub name: String,
    pub state: String, // "active" | "future"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraProject {
    pub key: String,
    pub sprints: Vec<Sprint>,
    pub has_sprints: bool,
    pub tickets: Vec<JiraTicket>,
}

const FIELDS: &str = "summary,status,issuetype,assignee";

pub fn status_category(key: &str) -> StatusCategory {
    match key {
        "done" => StatusCategory::Done,
        "indeterminate" => StatusCategory::InProgress,
        _ => StatusCategory::Todo,
    }
}

pub fn parse_issue(v: &Value, base_url: &str, sprint_id: Option<u64>) -> Option<JiraTicket> {
    let key = v["key"].as_str()?;
    let f = &v["fields"];
    let assignee = f["assignee"]["accountId"].as_str().map(|id| Person {
        account_id: id.to_string(),
        display_name: f["assignee"]["displayName"].as_str().unwrap_or(id).to_string(),
    });
    Some(JiraTicket {
        key: key.to_string(),
        summary: f["summary"].as_str().unwrap_or_default().to_string(),
        issue_type: f["issuetype"]["name"].as_str().unwrap_or_default().to_string(),
        status: f["status"]["name"].as_str().unwrap_or_default().to_string(),
        status_category: status_category(f["status"]["statusCategory"]["key"].as_str().unwrap_or_default()),
        assignee,
        sprint_id,
        url: format!("{}/browse/{key}", base_url.trim_end_matches('/')),
    })
}

pub fn parse_board_ids(v: &Value) -> Vec<u64> {
    v["values"].as_array().map(|a| a.iter().filter_map(|b| b["id"].as_u64()).collect()).unwrap_or_default()
}

pub fn parse_sprints(v: &Value) -> Vec<Sprint> {
    v["sprints"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    Some(Sprint { id: s["id"].as_u64()?, name: s["name"].as_str()?.to_string(), state: s["state"].as_str()?.to_string() })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn project_jql(key: &str) -> String {
    format!("project = \"{}\"", key.replace('"', ""))
}

pub fn search_jql(key: &str, has_sprints: bool) -> String {
    let p = project_jql(key);
    if has_sprints {
        // Replaces the agile backlog endpoint, which acli lacks: backlog, kanban-style, plus the recently done.
        format!(
            "{p} AND ((statusCategory != Done AND (sprint is EMPTY OR (sprint not in openSprints() AND sprint not in futureSprints()))) OR (statusCategory = Done AND statusCategoryChangedDate >= -14d))"
        )
    } else {
        format!("{p} AND (statusCategory != Done OR statusCategoryChangedDate >= -14d)")
    }
}

fn acli(r: &dyn Runner, args: &[&str]) -> Result<String, String> {
    r.run(&std::env::temp_dir(), "acli", args)
}

pub fn site(r: &dyn Runner) -> Result<String, String> {
    let out = acli(r, &["jira", "auth", "status"])?;
    let host = out.lines().find_map(|l| l.trim().strip_prefix("Site:")).map(str::trim).filter(|h| !h.is_empty());
    host.map(|h| format!("https://{h}")).ok_or_else(|| "jira: unexpected auth status".into())
}

/// Logged out, or acli not installed.
pub fn not_configured(e: &str) -> bool {
    e.contains("unauthorized") || e.contains("(os error 2)")
}

/// Issues of every JSON document in the output: bare arrays (workitem search) or `{"issues":[..]}` pages (--paginate prints one per page).
fn issues(out: &str) -> Result<Vec<Value>, String> {
    let mut all = vec![];
    for doc in serde_json::Deserializer::from_str(out).into_iter::<Value>() {
        let doc = doc.map_err(|_| "jira: unexpected acli output".to_string())?;
        let items = if doc.is_array() { &doc } else { &doc["issues"] };
        all.extend(items.as_array().cloned().unwrap_or_default());
    }
    Ok(all)
}

pub fn me(r: &dyn Runner) -> Result<Option<Person>, String> {
    for field in ["assignee", "reporter"] {
        let jql = format!("{field} = currentUser() ORDER BY updated DESC");
        let found = issues(&acli(r, &["jira", "workitem", "search", "--jql", &jql, "--fields", field, "--limit", "1", "--json"])?)?;
        if let Some(p) = found.first().map(|i| &i["fields"][field]) {
            if let Some(id) = p["accountId"].as_str() {
                return Ok(Some(Person { account_id: id.into(), display_name: p["displayName"].as_str().unwrap_or(id).into() }));
            }
        }
    }
    Ok(None)
}

fn parse_tickets(out: &str, base: &str, sprint_id: Option<u64>) -> Result<Vec<JiraTicket>, String> {
    Ok(issues(out)?.iter().filter_map(|i| parse_issue(i, base, sprint_id)).collect())
}

pub fn project(r: &dyn Runner, base: &str, key: &str) -> Result<JiraProject, String> {
    // shortcut: first page only (50), enough for one project's boards and open sprints, no paging in acli board search
    let boards = parse_board_ids(&json(&acli(r, &["jira", "board", "search", "--project", key, "--type", "scrum", "--json"])?)?);
    let scope = project_jql(key);
    let (mut sprints, mut tickets_) = (vec![], vec![]);
    for b in &boards {
        let b = b.to_string();
        // shortcut: first page only (50), enough for one project's open sprints, no paging in acli list-sprints
        let out = acli(r, &["jira", "board", "list-sprints", "--id", &b, "--state", "active,future", "--json"])?;
        for s in parse_sprints(&json(&out)?) {
            let id = s.id.to_string();
            let args = ["jira", "sprint", "list-workitems", "--board", &b, "--sprint", &id, "--jql", &scope, "--fields", FIELDS, "--paginate", "--json"];
            tickets_.extend(parse_tickets(&acli(r, &args)?, base, Some(s.id))?);
            if !sprints.iter().any(|x: &Sprint| x.id == s.id) {
                sprints.push(s);
            }
        }
    }
    let has_sprints = !sprints.is_empty();
    let jql = search_jql(key, has_sprints);
    tickets_.extend(parse_tickets(&acli(r, &["jira", "workitem", "search", "--jql", &jql, "--fields", FIELDS, "--paginate", "--json"])?, base, None)?);
    let mut seen = std::collections::HashSet::new();
    tickets_.retain(|t| seen.insert(t.key.clone()));
    Ok(JiraProject { key: key.to_string(), sprints, has_sprints, tickets: tickets_ })
}

fn json(out: &str) -> Result<Value, String> {
    serde_json::from_str(out).map_err(|_| "jira: unexpected acli output".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::tests::Fake;

    fn raw(name: &str) -> String {
        std::fs::read_to_string(format!("{}/tests/fixtures/jira/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn fixture(name: &str) -> Value {
        serde_json::from_str(&raw(name)).unwrap()
    }

    #[test]
    fn parses_issue_with_assignee() {
        let v = fixture("sprint-issues.json");
        let t = parse_issue(&v["issues"][0], "https://x.atlassian.net/", Some(7)).unwrap();
        assert_eq!(t.key, "DEMO-1");
        assert_eq!(t.summary, "Add login page");
        assert_eq!(t.issue_type, "Story");
        assert_eq!(t.status_category, StatusCategory::InProgress);
        assert_eq!(t.assignee.unwrap().display_name, "Dev One");
        assert_eq!(t.sprint_id, Some(7));
        assert_eq!(t.url, "https://x.atlassian.net/browse/DEMO-1");
    }

    #[test]
    fn parses_unassigned_issue() {
        let v = fixture("sprint-issues.json");
        let t = parse_issue(&v["issues"][1], "https://x.atlassian.net", None).unwrap();
        assert!(t.assignee.is_none());
        assert_eq!(t.status_category, StatusCategory::Todo);
    }

    #[test]
    fn maps_status_categories() {
        assert_eq!(status_category("new"), StatusCategory::Todo);
        assert_eq!(status_category("indeterminate"), StatusCategory::InProgress);
        assert_eq!(status_category("done"), StatusCategory::Done);
    }

    #[test]
    fn kanban_project_has_no_sprints() {
        assert_eq!(parse_board_ids(&fixture("boards.json")), vec![42]);
        assert!(parse_board_ids(&serde_json::json!({"values": []})).is_empty());
    }

    #[test]
    fn search_jql_quotes_key_and_follows_sprints() {
        assert_eq!(
            search_jql("DEMO", true),
            "project = \"DEMO\" AND ((statusCategory != Done AND (sprint is EMPTY OR (sprint not in openSprints() AND sprint not in futureSprints()))) OR (statusCategory = Done AND statusCategoryChangedDate >= -14d))"
        );
        assert!(search_jql("DEMO", false).contains("(statusCategory != Done OR"));
    }

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

    #[test]
    fn garbage_output_is_an_error() {
        assert_eq!(issues("not json").unwrap_err(), "jira: unexpected acli output");
    }
}
