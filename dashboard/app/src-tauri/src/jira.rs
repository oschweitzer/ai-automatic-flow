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
}

pub fn parse_page(v: &Value, base_url: &str, sprint_id: Option<u64>) -> Page {
    let issues = v["issues"].as_array().map(Vec::as_slice).unwrap_or_default();
    Page {
        tickets: issues.iter().filter_map(|i| parse_issue(i, base_url, sprint_id)).collect(),
        next_token: v["nextPageToken"].as_str().map(String::from),
        raw_count: issues.len(),
        total: v["total"].as_u64(),
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
    pub fn new(url: &str, email: &str, token: &str) -> Self {
        JiraClient { base: url.trim_end_matches('/').to_string(), email: email.into(), token: token.into(), http: reqwest::Client::new() }
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
        let (mut out, mut token) = (vec![], None::<String>);
        loop {
            let p = parse_page(&self.get(&search_url(&self.base, jql, token.as_deref())).await?, &self.base, None);
            out.extend(p.tickets);
            match p.next_token {
                Some(t) => token = Some(t),
                None => return Ok(out),
            }
        }
    }

    pub async fn project(&self, key: &str) -> Result<JiraProject, String> {
        let boards = parse_board_ids(
            &self.get(&format!("{}/rest/agile/1.0/board?projectKeyOrId={}&type=scrum", self.base, enc(key))).await?,
        );
        let (mut sprints, mut tickets) = (vec![], vec![]);
        for b in &boards {
            for s in parse_sprints(&self.get(&format!("{}/rest/agile/1.0/board/{b}/sprint?state=active,future", self.base)).await?) {
                tickets.extend(self.agile_issues(&format!("/rest/agile/1.0/board/{b}/sprint/{}/issue", s.id), "", Some(s.id)).await?);
                if !sprints.iter().any(|x: &Sprint| x.id == s.id) {
                    sprints.push(s);
                }
            }
            let extra = format!("jql={}&", enc("statusCategory != Done"));
            tickets.extend(self.agile_issues(&format!("/rest/agile/1.0/board/{b}/backlog"), &extra, None).await?);
        }
        let jql = if boards.is_empty() {
            format!("project = {key} AND (statusCategory != Done OR statusCategoryChangedDate >= -14d)")
        } else {
            format!("project = {key} AND statusCategory = Done AND statusCategoryChangedDate >= -14d")
        };
        tickets.extend(self.search(&jql).await?);
        let mut seen = std::collections::HashSet::new();
        tickets.retain(|t| seen.insert(t.key.clone()));
        Ok(JiraProject { key: key.to_string(), sprints, has_sprints: !boards.is_empty(), tickets })
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
        let c = JiraClient::new("https://x", "a@b.c", "SECRET");
        assert!(!format!("{c:?}").contains("SECRET"));
    }
}
