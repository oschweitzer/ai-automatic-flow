use crate::classify::{JiraTicket, Person, StatusCategory};
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

// No derived Debug: the token must never be printed.
pub struct JiraClient {
    base: String,
    email: String,
    token: String,
    http: reqwest::Client,
}

impl std::fmt::Debug for JiraClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JiraClient").field("base", &self.base).field("token", &"<redacted>").finish()
    }
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

pub struct Page {
    pub tickets: Vec<JiraTicket>,
    pub next_token: Option<String>, // /search/jql paging
    pub raw_count: usize,           // agile paging (startAt)
    pub total: Option<u64>,
    pub is_last: bool,
}

pub fn parse_page(v: &Value, base_url: &str, sprint_id: Option<u64>) -> Page {
    let issues = v["issues"].as_array().map(Vec::as_slice).unwrap_or_default();
    Page {
        tickets: issues.iter().filter_map(|i| parse_issue(i, base_url, sprint_id)).collect(),
        next_token: v["nextPageToken"].as_str().map(String::from),
        raw_count: issues.len(),
        total: v["total"].as_u64(),
        is_last: v["isLast"].as_bool().unwrap_or(false),
    }
}

pub fn parse_board_ids(v: &Value) -> Vec<u64> {
    v["values"].as_array().map(|a| a.iter().filter_map(|b| b["id"].as_u64()).collect()).unwrap_or_default()
}

pub fn parse_sprints(v: &Value) -> Vec<Sprint> {
    v["values"]
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

// Basic auth sends the token, so plain http is refused.
pub fn check_url(url: &str) -> Result<String, String> {
    let u = url.trim().trim_end_matches('/');
    if u.is_empty() {
        Err("jira: not configured".into())
    } else if !u.starts_with("https://") {
        Err("jira: https required".into())
    } else {
        Ok(u.to_string())
    }
}

// A site without Jira Software answers 404 on the agile API: treat it as "no board".
pub fn boards_or_none(r: Result<Value, String>) -> Result<Vec<u64>, String> {
    match r {
        Ok(v) => Ok(parse_board_ids(&v)),
        Err(e) if e == "jira: 404" => Ok(vec![]),
        Err(e) => Err(e),
    }
}

// Stop on an empty page, the last page, or a token already seen.
pub fn next_token_to_follow(p: &Page, seen: &[String]) -> Option<String> {
    let t = p.next_token.as_ref()?;
    (p.raw_count > 0 && !p.is_last && !seen.contains(t)).then(|| t.clone())
}

fn project_jql(key: &str) -> String {
    format!("project = \"{}\"", key.replace('"', ""))
}

pub fn search_jql(key: &str, has_sprints: bool) -> String {
    let p = project_jql(key);
    if has_sprints {
        format!("{p} AND statusCategory = Done AND statusCategoryChangedDate >= -14d")
    } else {
        format!("{p} AND (statusCategory != Done OR statusCategoryChangedDate >= -14d)")
    }
}

fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

fn search_url(base: &str, jql: &str, token: Option<&str>) -> String {
    let mut u = format!("{base}/rest/api/3/search/jql?jql={}&fields={FIELDS}&maxResults=100", enc(jql));
    if let Some(t) = token {
        u.push_str(&format!("&nextPageToken={}", enc(t)));
    }
    u
}

impl JiraClient {
    pub fn new(url: &str, email: &str, token: &str) -> Result<Self, String> {
        let base = check_url(url)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|_| "jira: client setup failed".to_string())?;
        Ok(JiraClient { base, email: email.into(), token: token.into(), http })
    }

    async fn get(&self, url: &str) -> Result<Value, String> {
        let resp = self
            .http
            .get(url)
            .basic_auth(&self.email, Some(&self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| if e.is_timeout() { "jira: timeout".to_string() } else { "jira: unreachable".to_string() })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("jira: {}", status.as_u16()));
        }
        resp.json().await.map_err(|_| "jira: unexpected response".to_string())
    }

    pub async fn myself(&self) -> Result<Person, String> {
        let v = self.get(&format!("{}/rest/api/3/myself", self.base)).await?;
        Ok(Person {
            account_id: v["accountId"].as_str().ok_or("jira: unexpected response")?.to_string(),
            display_name: v["displayName"].as_str().unwrap_or_default().to_string(),
        })
    }

    // Agile endpoints page with startAt.
    async fn agile_issues(&self, path: &str, extra: &str, sprint_id: Option<u64>) -> Result<Vec<JiraTicket>, String> {
        let (mut out, mut start) = (vec![], 0usize);
        loop {
            let url = format!("{}{path}?{extra}fields={FIELDS}&maxResults=100&startAt={start}", self.base);
            let p = parse_page(&self.get(&url).await?, &self.base, sprint_id);
            start += p.raw_count;
            out.extend(p.tickets);
            if p.raw_count == 0 || p.total.is_none_or(|t| start as u64 >= t) {
                return Ok(out);
            }
        }
    }

    async fn search(&self, jql: &str) -> Result<Vec<JiraTicket>, String> {
        let (mut out, mut token, mut seen) = (vec![], None::<String>, Vec::<String>::new());
        loop {
            let p = parse_page(&self.get(&search_url(&self.base, jql, token.as_deref())).await?, &self.base, None);
            let next = next_token_to_follow(&p, &seen);
            out.extend(p.tickets);
            match next {
                Some(t) => {
                    seen.push(t.clone());
                    token = Some(t);
                }
                None => return Ok(out),
            }
        }
    }

    pub async fn project(&self, key: &str) -> Result<JiraProject, String> {
        // shortcut: first page only (50), enough for one project's boards and open sprints, page with startAt if a project ever has more
        let boards = boards_or_none(
            self.get(&format!("{}/rest/agile/1.0/board?projectKeyOrId={}&type=scrum", self.base, enc(key))).await,
        )?;
        let scope = project_jql(key);
        let (mut sprints, mut tickets) = (vec![], vec![]);
        for b in &boards {
            // shortcut: first page only (50), enough for one project's boards and open sprints, page with startAt if a project ever has more
            for s in parse_sprints(&self.get(&format!("{}/rest/agile/1.0/board/{b}/sprint?state=active,future", self.base)).await?) {
                let extra = format!("jql={}&", enc(&scope));
                tickets.extend(self.agile_issues(&format!("/rest/agile/1.0/board/{b}/sprint/{}/issue", s.id), &extra, Some(s.id)).await?);
                if !sprints.iter().any(|x: &Sprint| x.id == s.id) {
                    sprints.push(s);
                }
            }
            let extra = format!("jql={}&", enc(&format!("{scope} AND statusCategory != Done")));
            tickets.extend(self.agile_issues(&format!("/rest/agile/1.0/board/{b}/backlog"), &extra, None).await?);
        }
        let has_sprints = !sprints.is_empty();
        tickets.extend(self.search(&search_jql(key, has_sprints)).await?);
        let mut seen = std::collections::HashSet::new();
        tickets.retain(|t| seen.insert(t.key.clone()));
        Ok(JiraProject { key: key.to_string(), sprints, has_sprints, tickets })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = format!("{}/tests/fixtures/jira/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parses_issue_with_assignee() {
        let v = fixture("sprint-issues.json");
        let t = parse_issue(&v["issues"][0], "https://x.atlassian.net/", Some(7)).unwrap();
        assert_eq!(t.key, "DEMO-1");
        assert_eq!(t.summary, "Add login page");
        assert_eq!(t.issue_type, "Story");
        assert_eq!(t.status_category, StatusCategory::InProgress);
        assert_eq!(t.assignee.unwrap().display_name, "Olivier Schweitzer");
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
    fn parses_sprints_and_search_page() {
        let s = parse_sprints(&fixture("sprints.json"));
        assert_eq!((s[0].id, s[1].state.as_str()), (7, "future"));
        let p = parse_page(&fixture("search.json"), "https://x.atlassian.net", None);
        assert_eq!(p.next_token.as_deref(), Some("CAEaAggD"));
        assert_eq!(p.tickets[0].status_category, StatusCategory::Done);
        assert!(parse_page(&fixture("backlog.json"), "https://x", None).next_token.is_none());
    }

    #[test]
    fn search_url_encodes_jql_and_token() {
        let u = search_url("https://x", "project = A AND d >= -14d", Some("a+b"));
        assert!(u.contains("jql=project%20%3D%20A%20AND%20d%20%3E%3D%20-14d"));
        assert!(u.ends_with("&nextPageToken=a%2Bb"));
    }

    #[test]
    fn debug_hides_token() {
        let c = JiraClient::new("https://x", "a@b.c", "SECRET").unwrap();
        assert!(!format!("{c:?}").contains("SECRET"));
    }

    #[test]
    fn refuses_non_https_and_empty_url() {
        assert_eq!(check_url("http://x.atlassian.net").unwrap_err(), "jira: https required");
        assert_eq!(check_url("  ").unwrap_err(), "jira: not configured");
        assert_eq!(check_url(" https://x.atlassian.net/ ").unwrap(), "https://x.atlassian.net");
        assert!(JiraClient::new("http://x", "a", "t").is_err());
    }

    #[test]
    fn search_jql_quotes_key_and_follows_sprints() {
        assert_eq!(search_jql("DEMO", true), "project = \"DEMO\" AND statusCategory = Done AND statusCategoryChangedDate >= -14d");
        assert!(search_jql("DEMO", false).contains("(statusCategory != Done OR"));
    }

    #[test]
    fn board_404_means_no_board_other_errors_fail() {
        assert!(boards_or_none(Err("jira: 404".into())).unwrap().is_empty());
        assert_eq!(boards_or_none(Err("jira: 401".into())).unwrap_err(), "jira: 401");
        assert_eq!(boards_or_none(Ok(fixture("boards.json"))).unwrap(), vec![42]);
    }

    #[test]
    fn page_loop_stops_on_repeat_empty_or_last() {
        let page = |v: Value| parse_page(&v, "https://x", None);
        let issue = serde_json::json!({"key":"A-1","fields":{}});
        let more = page(serde_json::json!({"nextPageToken":"t","issues":[issue.clone()]}));
        assert_eq!(next_token_to_follow(&more, &[]).as_deref(), Some("t"));
        assert!(next_token_to_follow(&more, &["t".to_string()]).is_none());
        assert!(next_token_to_follow(&page(serde_json::json!({"nextPageToken":"t","issues":[]})), &[]).is_none());
        assert!(next_token_to_follow(&page(serde_json::json!({"nextPageToken":"t","isLast":true,"issues":[issue]})), &[]).is_none());
    }
}
