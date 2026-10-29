use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const STAGES: [&str; 8] = ["triage", "plan", "execute", "open-mr", "ci-gate", "review", "mr-loop", "handover"];

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    Start,
    End,
    Waiting,
    #[serde(alias = "end_state")]
    EndState,
    Published,
}

// The `detail` field is deliberately absent: it is never read.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub at: DateTime<Utc>,
    pub stage: String,
    pub event: EventKind,
    pub outcome: Option<String>,
    pub on: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Done,
    Now,
    Fail,
    Todo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInfo {
    pub name: String,
    pub state: StepState,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrRef {
    pub number: u64,
    pub url: String,
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageInfo {
    pub size: Option<String>,
    #[serde(default)]
    pub risk: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub behaviour: Option<String>,
    pub files: Vec<String>,
    pub planner: Option<String>,
    pub executor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub key: String,
    pub run_dir: PathBuf,
    pub lines: Vec<LogLine>,
    pub stage_index: u8,
    pub end_state: Option<String>,
    pub waiting_on: Option<String>,
    pub published: bool,
    pub last_at: DateTime<Utc>,
    pub last_is_start: bool,
    pub mr: Option<MrRef>,
    pub triage: Option<TriageInfo>,
    pub plan: Option<PlanSummary>,
    pub summary: Option<String>,
    pub has_handover: bool,
}

#[derive(Deserialize)]
struct RawPlan {
    behaviour: Option<String>,
    #[serde(default)]
    files: Vec<RawFile>,
    planner: Option<String>,
    executor: Option<String>,
}

#[derive(Deserialize)]
struct RawFile {
    path: String,
}

fn json<T: serde::de::DeserializeOwned>(dir: &Path, name: &str) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(dir.join(name)).ok()?).ok()
}

pub fn read_run(dir: &Path) -> Option<RunState> {
    let text = std::fs::read_to_string(dir.join("log.jsonl")).ok()?;
    // A line cut in half by a writer fails to parse and is skipped.
    let lines: Vec<LogLine> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let last = lines.last()?;
    let mut end_state = None;
    let mut published = false;
    for l in &lines {
        match l.event {
            EventKind::Start => (end_state, published) = (None, false),
            EventKind::EndState => (end_state, published) = (l.outcome.clone(), false),
            EventKind::Published => published = end_state.is_some(),
            _ => {}
        }
    }
    let stage_of = |l: &LogLine| STAGES.iter().position(|s| *s == l.stage).map(|i| i as u8 + 1);
    let stage_index = lines.iter().rev().find(|l| l.event == EventKind::Start).and_then(stage_of).or_else(|| stage_of(last)).unwrap_or(1);
    let started = |stage: &str| lines.iter().find(|l| l.stage == stage && l.event == EventKind::Start).and_then(|l| l.model.clone());
    let raw: Option<RawPlan> = json(dir, "plan.json");
    let plan = raw.map(|p| PlanSummary {
        behaviour: p.behaviour,
        files: p.files.into_iter().map(|f| f.path).collect(),
        planner: started("plan").or(p.planner),
        executor: started("execute").or(p.executor),
    });
    let summary = std::fs::read_to_string(dir.join("ticket.md"))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("# ").map(|h| h.trim().to_string())));
    Some(RunState {
        key: dir.file_name()?.to_string_lossy().into_owned(),
        run_dir: dir.to_path_buf(),
        stage_index,
        end_state,
        waiting_on: if last.event == EventKind::Waiting { last.on.clone() } else { None },
        published,
        last_at: last.at,
        last_is_start: last.event == EventKind::Start,
        mr: json(dir, "mr.json"),
        triage: json(dir, "triage.json"),
        plan,
        summary,
        has_handover: dir.join("handover.md").is_file(),
        lines,
    })
}

pub fn read_runs(repo: &Path) -> Vec<RunState> {
    let Ok(entries) = std::fs::read_dir(repo.join(".ticket-flow/runs")) else { return vec![] };
    let mut runs: Vec<RunState> = entries.filter_map(|e| read_run(&e.ok()?.path())).collect();
    runs.sort_by(|a, b| a.key.cmp(&b.key));
    runs
}

const FAILED_ENDS: [&str; 6] = ["plan_rejected", "escalated_checks", "escalated_ci", "escalated_comments", "timeout", "failed_permission"];

pub fn steps(run: &RunState, ci_failed: bool) -> Vec<StepInfo> {
    STAGES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let n = i as u8 + 1;
            let own: Vec<&LogLine> = run.lines.iter().filter(|l| l.stage == *name).collect();
            let outcome = own.iter().rev().find_map(|l| l.outcome.clone());
            let time = own.last().map(|l| l.at.with_timezone(&Local).format("%H:%M").to_string());
            let note = |o: Option<String>| match (o, &time) {
                (Some(o), Some(t)) => Some(format!("{o} · {t}")),
                (Some(o), None) => Some(o),
                _ => None,
            };
            let state = if n > run.stage_index {
                StepState::Todo
            } else if n < run.stage_index {
                StepState::Done
            } else if run.end_state.as_deref().is_some_and(|e| FAILED_ENDS.contains(&e)) || (ci_failed && run.end_state.is_none()) {
                StepState::Fail
            } else if run.end_state.is_some() || own.last().is_some_and(|l| l.event == EventKind::End) {
                StepState::Done
            } else {
                StepState::Now
            };
            let note = match state {
                StepState::Todo | StepState::Now => None,
                StepState::Done => note(outcome),
                StepState::Fail => note(Some(outcome.unwrap_or_else(|| "failed".into()))),
            };
            StepInfo { name: name.to_string(), state, note }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(key: &str) -> RunState {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs").join(key);
        read_run(&dir).unwrap()
    }

    #[test]
    fn reads_full_run() {
        let r = fx("SHOP-142");
        assert_eq!(r.key, "SHOP-142");
        assert_eq!(r.stage_index, 8);
        assert_eq!(r.end_state.as_deref(), Some("ready_for_qa"));
        assert!(r.published);
        assert_eq!(r.mr.as_ref().unwrap().number, 87);
        assert_eq!(r.plan.as_ref().unwrap().planner.as_deref(), Some("sonnet"));
        assert_eq!(r.plan.as_ref().unwrap().files.len(), 2);
        assert_eq!(r.triage.as_ref().unwrap().size.as_deref(), Some("medium"));
    }

    #[test]
    fn reads_parked() {
        assert_eq!(fx("SHOP-139").end_state.as_deref(), Some("parked_needs_info"));
    }

    #[test]
    fn reads_waiting_developer() {
        let r = fx("SHOP-144");
        assert_eq!(r.waiting_on.as_deref(), Some("developer"));
        assert_eq!(r.stage_index, 2);
    }

    #[test]
    fn reads_running() {
        let r = fx("SHOP-146");
        assert_eq!(r.stage_index, 3);
        assert!(r.last_is_start);
    }

    #[test]
    fn skips_cut_line() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs/SHOP-150");
        let total = std::fs::read_to_string(dir.join("log.jsonl")).unwrap().lines().count();
        assert_eq!(read_run(&dir).unwrap().lines.len(), total - 1);
    }

    #[test]
    fn start_after_end_state_reopens() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(
            t.path().join("log.jsonl"),
            "{\"at\":\"2026-10-26T09:00:00Z\",\"stage\":\"triage\",\"event\":\"start\"}\n\
             {\"at\":\"2026-10-26T09:01:00Z\",\"stage\":\"triage\",\"event\":\"end_state\",\"outcome\":\"parked_needs_info\"}\n\
             {\"at\":\"2026-10-26T09:02:00Z\",\"stage\":\"handover\",\"event\":\"published\"}\n\
             {\"at\":\"2026-10-26T12:00:00Z\",\"stage\":\"triage\",\"event\":\"start\"}\n",
        )
        .unwrap();
        let r = read_run(t.path()).unwrap();
        assert_eq!(r.end_state, None);
        assert!(!r.published);
    }

    #[test]
    fn only_a_cut_line_gives_none() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("log.jsonl"), "{\"at\":\"2026-10-26T11:00:00Z\",\"stage\":\"exe").unwrap();
        assert!(read_run(t.path()).is_none());
    }

    #[test]
    fn steps_mark_failure() {
        let s = steps(&fx("SHOP-146"), true);
        assert_eq!(s.len(), 8);
        assert_eq!(s[2].state, StepState::Fail);
        assert!(s[2].note.as_deref().unwrap().starts_with("failed · "));
        assert!(s[..2].iter().all(|x| x.state == StepState::Done));
        assert!(s[3..].iter().all(|x| x.state == StepState::Todo));
        let esc = steps(&fx("SHOP-133"), false);
        assert_eq!(esc[4].state, StepState::Fail);
        assert!(esc[4].note.as_deref().unwrap().starts_with("escalated_ci · "));
        assert_eq!(steps(&fx("SHOP-146"), false)[2].state, StepState::Now);
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let target = to.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copy_dir(&e.path(), &target);
            } else {
                std::fs::copy(e.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn read_runs_lists_every_folder() {
        let repo = tempfile::tempdir().unwrap();
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs"), &repo.path().join(".ticket-flow/runs"));
        assert_eq!(read_runs(repo.path()).len(), 6);
    }

    #[test]
    fn summary_from_ticket_md() {
        assert_eq!(fx("SHOP-142").summary.as_deref(), Some("Add a discount code field to the cart"));
        assert_eq!(fx("SHOP-139").summary, None);
    }
}
