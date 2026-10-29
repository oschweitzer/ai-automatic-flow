use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Forge {
    Github,
    Gitlab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tracker {
    Jira,
    Github,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Repo {
    pub name: String,
    pub path: PathBuf,
    pub forge: Option<Forge>,
    pub tracker: Tracker,
    pub jira_project: Option<String>,
    pub remote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDef {
    pub key: String,
    pub tracker: Tracker,
    pub repos: Vec<Repo>,
}

type Profile = (Option<Forge>, Tracker, Option<String>);

/// None when `.claude/ticket-flow.md` is missing. A malformed frontmatter gives the defaults.
pub fn read_profile(repo: &Path) -> Option<Profile> {
    let text = fs::read_to_string(repo.join(".claude").join("ticket-flow.md")).ok()?;
    let mut lines = text.lines();
    let (mut forge, mut tracker, mut jira) = (None, Tracker::Jira, None);
    if lines.next().map(str::trim_end) != Some("---") {
        return Some((forge, tracker, jira));
    }
    let mut fields = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        fields.push(line);
    }
    if !closed {
        return Some((forge, tracker, jira));
    }
    for line in fields {
        // top-level keys only
        if line.starts_with([' ', '\t', '#']) {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        let mut v = v.trim();
        // a comment starts at whitespace followed by #, as in the plugin's profile_get
        if let Some(i) = v.find(" #").or_else(|| v.find("\t#")) {
            v = v[..i].trim_end();
        }
        let v = v.trim_matches(|c| c == '"' || c == '\'');
        match k.trim() {
            "forge" => {
                forge = match v {
                    "github" => Some(Forge::Github),
                    "gitlab" => Some(Forge::Gitlab),
                    _ => None,
                }
            }
            "tracker" => tracker = if v == "github" { Tracker::Github } else { Tracker::Jira },
            "jira_project" if !v.is_empty() => jira = Some(v.to_string()),
            _ => {}
        }
    }
    Some((forge, tracker, jira))
}

/// owner/name from an https or ssh github.com URL.
pub fn parse_remote(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = ["git@github.com:", "ssh://git@github.com/", "https://github.com/", "http://github.com/"]
        .iter()
        .find_map(|p| url.strip_prefix(p))
        // https with credentials
        .or_else(|| {
            let r = url.strip_prefix("https://")?;
            r.split_once('@')?.1.strip_prefix("github.com/")
        })?;
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let (owner, name) = rest.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| format!("{owner}/{name}"))
}

/// The `url` of `[remote "origin"]`, read from `.git/config`.
fn origin_url(repo: &Path) -> Option<String> {
    let text = fs::read_to_string(repo.join(".git").join("config")).ok()?;
    let mut in_origin = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line.replace(' ', "") == "[remote\"origin\"]";
        } else if in_origin {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "url" {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

const SKIPPED: [&str; 4] = ["node_modules", "target", "dist", "vendor"];

fn walk(dir: &Path, depth: u32, out: &mut Vec<Repo>) {
    // a linked worktree has a `.git` file
    if dir.join(".git").is_file() {
        return;
    }
    if let Some((forge, tracker, jira_project)) = read_profile(dir) {
        out.push(Repo {
            name: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            path: dir.to_path_buf(),
            forge,
            tracker,
            jira_project,
            remote: origin_url(dir).and_then(|u| parse_remote(&u)),
        });
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            !n.starts_with('.') && !SKIPPED.contains(&n.as_str())
        })
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for d in dirs {
        walk(&d, depth - 1, out);
    }
}

/// Repositories with a profile, up to depth 3 below each root.
pub fn discover(roots: &[PathBuf]) -> Vec<Repo> {
    let mut out = Vec::new();
    for r in roots {
        walk(r, 3, &mut out);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

fn key_of(r: &Repo) -> String {
    match r.tracker {
        Tracker::Jira => r.jira_project.clone().unwrap_or_else(|| r.name.to_uppercase()),
        Tracker::Github => r.remote.clone().unwrap_or_else(|| r.name.clone()),
    }
}

/// Group by project key, sorted by key.
pub fn group(repos: Vec<Repo>) -> Vec<ProjectDef> {
    let mut map: BTreeMap<(String, bool), ProjectDef> = BTreeMap::new();
    for r in repos {
        let key = key_of(&r);
        map.entry((key.clone(), r.tracker == Tracker::Github))
            .or_insert_with(|| ProjectDef { key, tracker: r.tracker, repos: Vec::new() })
            .repos
            .push(r);
    }
    map.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(dir: &Path, body: &str) {
        fs::create_dir_all(dir.join(".claude")).unwrap();
        fs::write(dir.join(".claude").join("ticket-flow.md"), body).unwrap();
    }

    #[test]
    fn reads_profile_frontmatter() {
        let t = tempfile::tempdir().unwrap();
        profile(
            t.path(),
            "---\nforge: gitlab   # detected\njira_project: \"SHOP\"\ncommands:\n  jira_project: no\n---\n\njira_project: body\n",
        );
        assert_eq!(read_profile(t.path()), Some((Some(Forge::Gitlab), Tracker::Jira, Some("SHOP".into()))));
    }

    #[test]
    fn profile_without_jira_project() {
        let t = tempfile::tempdir().unwrap();
        profile(t.path(), "---\nforge: github\n---\n");
        assert_eq!(read_profile(t.path()), Some((Some(Forge::Github), Tracker::Jira, None)));
        assert_eq!(read_profile(&t.path().join("nope")), None);
    }

    #[test]
    fn malformed_frontmatter_is_not_fatal() {
        let t = tempfile::tempdir().unwrap();
        profile(t.path(), "---\nforge: gitlab\njira_project: SHOP\n");
        assert_eq!(read_profile(t.path()), Some((None, Tracker::Jira, None)));
    }

    #[test]
    fn discovers_only_repos_with_profile() {
        let t = tempfile::tempdir().unwrap();
        profile(&t.path().join("a"), "---\n---\n");
        fs::create_dir_all(t.path().join("b")).unwrap();
        let repos = discover(&[t.path().to_path_buf()]);
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "a");
    }

    #[test]
    fn skips_heavy_folders() {
        let t = tempfile::tempdir().unwrap();
        profile(&t.path().join("node_modules").join("x"), "---\n---\n");
        profile(&t.path().join(".hidden"), "---\n---\n");
        assert!(discover(&[t.path().to_path_buf()]).is_empty());
    }

    fn repo(name: &str, tracker: Tracker, jira: Option<&str>, remote: Option<&str>) -> Repo {
        Repo {
            name: name.into(),
            path: PathBuf::from(name),
            forge: None,
            tracker,
            jira_project: jira.map(Into::into),
            remote: remote.map(Into::into),
        }
    }

    #[test]
    fn groups_by_jira_project() {
        let p = group(vec![
            repo("front", Tracker::Jira, Some("SHOP"), None),
            repo("back", Tracker::Jira, Some("SHOP"), None),
            repo("solo", Tracker::Jira, None, None),
        ]);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].key, "SHOP");
        assert_eq!(p[0].repos.len(), 2);
        assert_eq!(p[1].key, "SOLO");
    }

    #[test]
    fn tracker_defaults_to_jira() {
        let t = tempfile::tempdir().unwrap();
        profile(t.path(), "---\ntracker: whatever\n---\n");
        assert_eq!(read_profile(t.path()).unwrap().1, Tracker::Jira);
    }

    #[test]
    fn github_tracker_repo_is_its_own_project() {
        let p = group(vec![
            repo("a", Tracker::Github, None, Some("acme/a")),
            repo("b", Tracker::Github, None, None),
        ]);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].key, "acme/a");
        assert_eq!(p[1].key, "b");
    }

    #[test]
    fn ssh_and_https_remotes_parse() {
        assert_eq!(parse_remote("git@github.com:acme/shop.git"), Some("acme/shop".into()));
        assert_eq!(parse_remote("https://github.com/acme/shop"), Some("acme/shop".into()));
        assert_eq!(parse_remote("https://gitlab.com/acme/shop.git"), None);
    }

    #[test]
    fn reads_origin_from_git_config() {
        let t = tempfile::tempdir().unwrap();
        profile(t.path(), "---\ntracker: github\n---\n");
        fs::create_dir_all(t.path().join(".git")).unwrap();
        fs::write(
            t.path().join(".git").join("config"),
            "[remote \"upstream\"]\n\turl = git@github.com:x/y.git\n[remote \"origin\"]\n\turl = git@github.com:acme/shop.git\n",
        )
        .unwrap();
        let r = discover(&[t.path().to_path_buf()]);
        assert_eq!(r[0].remote.as_deref(), Some("acme/shop"));
    }

    #[test]
    fn skips_linked_worktrees() {
        let t = tempfile::tempdir().unwrap();
        let wt = t.path().join("shop.worktrees").join("SHOP-1");
        profile(&wt, "---\n---\n");
        fs::write(wt.join(".git"), "gitdir: /x/shop/.git/worktrees/SHOP-1\n").unwrap();
        assert!(discover(&[t.path().to_path_buf()]).is_empty());
    }
}
