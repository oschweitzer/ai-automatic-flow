use crate::runs::RunState;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MrState { Open, Merged, Closed }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Ci { Green, Red, Running, None }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MrStatus {
    pub number: u64,
    pub url: String,
    pub state: MrState,
    pub draft: bool,
    pub ci: Ci,
    pub pipeline_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusCategory { Todo, InProgress, Done }

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub account_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraTicket {
    pub key: String,
    pub summary: String,
    pub issue_type: String,
    pub status: String,
    pub status_category: StatusCategory,
    pub assignee: Option<Person>,
    pub sprint_id: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueState { Open, Completed, NotPlanned }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubIssue {
    pub number: u64,
    pub title: String,
    pub state: IssueState,
    pub assignees: Vec<Person>,
    pub labels: Vec<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackerTicket { Jira(JiraTicket), Github(GithubIssue) }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Section { InProgress, Waiting, Todo, Done }

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WaitingOn { You, Client }

// These are i18n keys on the front end.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    PlanToApprove, MrToMerge, ClientStepsToPublish, ParkedNeedsInfo, PlanRejected,
    EscalatedChecks, EscalatedCi, EscalatedComments, Timeout, FailedPermission,
    Running, WaitingCi, WaitingReviews, ReadyForQa, Split, MrClosed,
    JiraTodo, JiraInProgress, JiraDone, IssueTodo, IssueDone, IssueNotPlanned,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub section: Section,
    pub waiting_on: Option<WaitingOn>,
    pub reason: Reason,
    pub stale_minutes: Option<i64>,
}

const STALE_AFTER_MINUTES: i64 = 30;

fn at(section: Section, waiting_on: Option<WaitingOn>, reason: Reason) -> Placement {
    Placement { section, waiting_on, reason, stale_minutes: None }
}

fn blocked(reason: Reason) -> Placement {
    at(Section::Waiting, Some(WaitingOn::You), reason)
}

pub fn classify(run: Option<&RunState>, ticket: Option<&TrackerTicket>, mr: Option<&MrStatus>, now: DateTime<Utc>) -> Placement {
    let done = |reason| at(Section::Done, None, reason);
    // Overrides, in order: ticket closed, merged and published, MR closed.
    match ticket {
        Some(TrackerTicket::Jira(j)) if j.status_category == StatusCategory::Done => return done(Reason::JiraDone),
        Some(TrackerTicket::Github(g)) if g.state == IssueState::Completed => return done(Reason::IssueDone),
        Some(TrackerTicket::Github(g)) if g.state == IssueState::NotPlanned => return done(Reason::IssueNotPlanned),
        _ => {}
    }
    let mr_state = mr.map(|m| &m.state);
    if mr_state == Some(&MrState::Merged) && run.is_some_and(|r| r.published) {
        return done(Reason::ReadyForQa);
    }
    if mr_state == Some(&MrState::Closed) {
        return done(Reason::MrClosed);
    }
    let Some(run) = run else {
        return match ticket {
            Some(TrackerTicket::Jira(j)) => match j.status_category {
                StatusCategory::InProgress => at(Section::InProgress, None, Reason::JiraInProgress),
                _ => at(Section::Todo, None, Reason::JiraTodo),
            },
            Some(TrackerTicket::Github(_)) => at(Section::Todo, None, Reason::IssueTodo),
            // Unreachable from the snapshot builder: every ticket has a run or a tracker ticket.
            None => at(Section::Todo, None, Reason::JiraTodo),
        };
    };
    match run.end_state.as_deref() {
        Some("ready_for_qa") if run.published => done(Reason::ReadyForQa),
        Some("ready_for_qa") if mr_state == Some(&MrState::Merged) => blocked(Reason::ClientStepsToPublish),
        // No MR status known (forge unreachable) counts as not merged.
        Some("ready_for_qa") => blocked(Reason::MrToMerge),
        Some("split") => done(Reason::Split),
        Some("parked_needs_info") => at(Section::Waiting, Some(WaitingOn::Client), Reason::ParkedNeedsInfo),
        Some("plan_rejected") => blocked(Reason::PlanRejected),
        Some("escalated_checks") => blocked(Reason::EscalatedChecks),
        Some("escalated_ci") => blocked(Reason::EscalatedCi),
        Some("escalated_comments") => blocked(Reason::EscalatedComments),
        Some("timeout") => blocked(Reason::Timeout),
        Some("failed_permission") => blocked(Reason::FailedPermission),
        // shortcut: an unknown end state is treated as a run with none, revisit if the plugin adds end states.
        _ => match run.waiting_on.as_deref() {
            Some("developer") => blocked(Reason::PlanToApprove),
            Some("ci") => at(Section::InProgress, None, Reason::WaitingCi),
            Some("reviews") => at(Section::InProgress, None, Reason::WaitingReviews),
            _ => {
                let gap = (now - run.last_at).num_minutes();
                let stale = run.last_is_start && run.end_state.is_none() && run.waiting_on.is_none() && gap >= STALE_AFTER_MINUTES;
                Placement { stale_minutes: stale.then_some(gap), ..at(Section::InProgress, None, Reason::Running) }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use std::path::PathBuf;

    fn now() -> DateTime<Utc> { Utc.with_ymd_and_hms(2026, 10, 26, 12, 0, 0).unwrap() }

    fn run() -> RunState {
        RunState {
            key: "SHOP-1".into(), run_dir: PathBuf::new(), lines: vec![], stage_index: 3, end_state: None,
            waiting_on: None, published: false, last_at: now(), last_is_start: false, mr: None, triage: None,
            plan: None, summary: None, has_handover: false,
        }
    }
    fn ended(e: &str) -> RunState { RunState { end_state: Some(e.into()), ..run() } }
    fn waiting(on: &str) -> RunState { RunState { waiting_on: Some(on.into()), ..run() } }
    fn mr(state: MrState) -> MrStatus {
        MrStatus { number: 1, url: String::new(), state, draft: false, ci: Ci::None, pipeline_url: None }
    }
    fn jira(c: StatusCategory) -> TrackerTicket {
        TrackerTicket::Jira(JiraTicket {
            key: "SHOP-1".into(), summary: String::new(), issue_type: "Task".into(), status: String::new(),
            status_category: c, assignee: None, sprint_id: None, url: String::new(),
        })
    }
    fn issue(state: IssueState, assigned: bool) -> TrackerTicket {
        let assignees = if assigned { vec![Person { account_id: "a".into(), display_name: "A".into() }] } else { vec![] };
        TrackerTicket::Github(GithubIssue { number: 1, title: String::new(), state, assignees, labels: vec![], url: String::new() })
    }
    fn p(section: Section, w: Option<WaitingOn>, reason: Reason) -> Placement { at(section, w, reason) }
    fn go(r: &RunState) -> Placement { classify(Some(r), None, None, now()) }

    #[test]
    fn waiting_developer_is_plan_to_approve() {
        assert_eq!(go(&waiting("developer")), p(Section::Waiting, Some(WaitingOn::You), Reason::PlanToApprove));
    }

    #[test]
    fn waiting_ci_is_in_progress() {
        assert_eq!(go(&waiting("ci")), p(Section::InProgress, None, Reason::WaitingCi));
        assert_eq!(go(&waiting("reviews")), p(Section::InProgress, None, Reason::WaitingReviews));
    }

    #[test]
    fn ready_for_qa_open_mr_is_mr_to_merge() {
        let r = ended("ready_for_qa");
        let want = p(Section::Waiting, Some(WaitingOn::You), Reason::MrToMerge);
        assert_eq!(classify(Some(&r), None, Some(&mr(MrState::Open)), now()), want);
        assert_eq!(classify(Some(&r), None, None, now()), want);
    }

    #[test]
    fn ready_for_qa_merged_is_client_steps() {
        let r = ended("ready_for_qa");
        assert_eq!(
            classify(Some(&r), None, Some(&mr(MrState::Merged)), now()),
            p(Section::Waiting, Some(WaitingOn::You), Reason::ClientStepsToPublish)
        );
    }

    #[test]
    fn ready_for_qa_published_is_done() {
        let r = RunState { published: true, ..ended("ready_for_qa") };
        let want = p(Section::Done, None, Reason::ReadyForQa);
        assert_eq!(go(&r), want);
        assert_eq!(classify(Some(&r), None, Some(&mr(MrState::Merged)), now()), want);
        assert_eq!(classify(Some(&r), None, Some(&mr(MrState::Open)), now()), want);
    }

    #[test]
    fn split_is_done() {
        assert_eq!(go(&ended("split")), p(Section::Done, None, Reason::Split));
    }

    #[test]
    fn parked_is_waiting_client() {
        assert_eq!(go(&ended("parked_needs_info")), p(Section::Waiting, Some(WaitingOn::Client), Reason::ParkedNeedsInfo));
    }

    #[test]
    fn each_blocked_end_state_waits_on_you() {
        for (e, reason) in [
            ("plan_rejected", Reason::PlanRejected), ("escalated_checks", Reason::EscalatedChecks),
            ("escalated_ci", Reason::EscalatedCi), ("escalated_comments", Reason::EscalatedComments),
            ("timeout", Reason::Timeout), ("failed_permission", Reason::FailedPermission),
        ] {
            assert_eq!(go(&ended(e)), p(Section::Waiting, Some(WaitingOn::You), reason), "{e}");
        }
    }

    #[test]
    fn running_is_in_progress() {
        assert_eq!(go(&run()), p(Section::InProgress, None, Reason::Running));
        assert_eq!(go(&ended("something_new")), p(Section::InProgress, None, Reason::Running));
    }

    #[test]
    fn stale_after_30_minutes() {
        let r = RunState { last_is_start: true, last_at: now() - Duration::minutes(45), ..run() };
        assert_eq!(go(&r).stale_minutes, Some(45));
        let fresh = RunState { last_at: now() - Duration::minutes(29), ..r.clone() };
        assert_eq!(go(&fresh).stale_minutes, None);
        let not_start = RunState { last_is_start: false, ..r };
        assert_eq!(go(&not_start).stale_minutes, None);
        let full = go(&RunState { last_is_start: true, last_at: now() - Duration::minutes(45), ..run() });
        assert_eq!((full.section, full.reason), (Section::InProgress, Reason::Running));
        let edge = RunState { last_is_start: true, last_at: now() - Duration::minutes(30), ..run() };
        assert_eq!(go(&edge).stale_minutes, Some(30));
        let future = RunState { last_is_start: true, last_at: now() + Duration::minutes(5), ..run() };
        assert_eq!(go(&future).stale_minutes, None);
    }

    #[test]
    fn merged_and_published_overrides_the_run_table() {
        let r = RunState { published: true, ..ended("parked_needs_info") };
        assert_eq!(
            classify(Some(&r), None, Some(&mr(MrState::Merged)), now()),
            p(Section::Done, None, Reason::ReadyForQa)
        );
        assert_eq!(go(&r), p(Section::Waiting, Some(WaitingOn::Client), Reason::ParkedNeedsInfo));
    }

    #[test]
    fn override_precedence() {
        let closed = mr(MrState::Closed);
        let parked = ended("parked_needs_info");
        assert_eq!(
            classify(Some(&run()), Some(&jira(StatusCategory::Done)), Some(&closed), now()),
            p(Section::Done, None, Reason::JiraDone)
        );
        assert_eq!(classify(Some(&parked), None, Some(&closed), now()), p(Section::Done, None, Reason::MrClosed));
        assert_eq!(classify(None, None, Some(&closed), now()), p(Section::Done, None, Reason::MrClosed));
        assert_eq!(
            classify(None, Some(&issue(IssueState::NotPlanned, false)), Some(&closed), now()),
            p(Section::Done, None, Reason::IssueNotPlanned)
        );
    }

    #[test]
    fn placement_serialises_with_expected_keys() {
        let a = serde_json::to_string(&p(Section::InProgress, None, Reason::PlanToApprove)).unwrap();
        assert!(a.contains("\"section\":\"inProgress\""), "{a}");
        assert!(a.contains("\"reason\":\"plan_to_approve\""), "{a}");
        assert!(a.contains("\"staleMinutes\":null"), "{a}");
        let b = serde_json::to_string(&p(Section::Waiting, Some(WaitingOn::You), Reason::IssueNotPlanned)).unwrap();
        assert!(b.contains("\"waitingOn\":\"you\"") && b.contains("\"reason\":\"issue_not_planned\""), "{b}");
    }

    #[test]
    fn waits_are_never_stale() {
        // As runs.rs reads it: the wait is the last line, so the last line is not a start.
        let old = now() - Duration::minutes(300);
        for on in ["developer", "ci", "reviews"] {
            let r = RunState { last_at: old, ..waiting(on) };
            assert_eq!(go(&r).stale_minutes, None, "{on}");
        }
    }

    /// The spec's state table and its overrides, row by row, from fixture run folders.
    #[test]
    fn state_table_from_fixture_runs() {
        let fx = |key: &str| crate::runs::read_run(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runs").join(key)).unwrap();
        let soon = |r: &RunState| r.last_at + Duration::minutes(10);
        let you = |reason| p(Section::Waiting, Some(WaitingOn::You), reason);
        let rows: Vec<(&str, Option<TrackerTicket>, Option<MrStatus>, Placement)> = vec![
            ("SHOP-144", None, None, you(Reason::PlanToApprove)),
            ("SHOP-160", None, None, p(Section::InProgress, None, Reason::WaitingCi)),
            ("SHOP-161", None, None, p(Section::InProgress, None, Reason::WaitingReviews)),
            ("SHOP-162", None, Some(mr(MrState::Open)), you(Reason::MrToMerge)),
            ("SHOP-162", None, Some(mr(MrState::Merged)), you(Reason::ClientStepsToPublish)),
            ("SHOP-142", None, None, p(Section::Done, None, Reason::ReadyForQa)),
            ("SHOP-163", None, None, p(Section::Done, None, Reason::Split)),
            ("SHOP-139", None, None, p(Section::Waiting, Some(WaitingOn::Client), Reason::ParkedNeedsInfo)),
            ("SHOP-133", None, None, you(Reason::EscalatedCi)),
            ("SHOP-146", None, None, p(Section::InProgress, None, Reason::Running)),
            ("SHOP-164", None, None, p(Section::InProgress, None, Reason::Running)), // a start after parked reopens
            // Overrides: Jira done, merged and published, MR closed.
            ("SHOP-144", Some(jira(StatusCategory::Done)), None, p(Section::Done, None, Reason::JiraDone)),
            ("SHOP-139", None, Some(mr(MrState::Merged)), p(Section::Waiting, Some(WaitingOn::Client), Reason::ParkedNeedsInfo)),
            ("SHOP-142", None, Some(mr(MrState::Merged)), p(Section::Done, None, Reason::ReadyForQa)),
            ("SHOP-146", None, Some(mr(MrState::Closed)), p(Section::Done, None, Reason::MrClosed)),
        ];
        for (key, ticket, m, want) in rows {
            let r = fx(key);
            assert_eq!(classify(Some(&r), ticket.as_ref(), m.as_ref(), soon(&r)), want, "{key}");
        }
        // Last line `start`, nothing new for 45 minutes: running, flagged.
        let r = fx("SHOP-165");
        let stale = classify(Some(&r), None, None, r.last_at + Duration::minutes(45));
        assert_eq!(stale, Placement { stale_minutes: Some(45), ..p(Section::InProgress, None, Reason::Running) });
        // A wait does not count, however old.
        let w = fx("SHOP-160");
        assert_eq!(classify(Some(&w), None, None, w.last_at + Duration::minutes(300)).stale_minutes, None);
    }

    #[test]
    fn jira_done_overrides_run() {
        let r = waiting("developer");
        assert_eq!(
            classify(Some(&r), Some(&jira(StatusCategory::Done)), None, now()),
            p(Section::Done, None, Reason::JiraDone)
        );
    }

    #[test]
    fn closed_mr_is_mr_closed() {
        let r = run();
        assert_eq!(classify(Some(&r), None, Some(&mr(MrState::Closed)), now()), p(Section::Done, None, Reason::MrClosed));
    }

    #[test]
    fn no_run_follows_jira_category() {
        for (c, section, reason) in [
            (StatusCategory::Todo, Section::Todo, Reason::JiraTodo),
            (StatusCategory::InProgress, Section::InProgress, Reason::JiraInProgress),
            (StatusCategory::Done, Section::Done, Reason::JiraDone),
        ] {
            assert_eq!(classify(None, Some(&jira(c)), None, now()), p(section, None, reason));
        }
    }

    #[test]
    fn issue_completed_overrides_run() {
        let r = waiting("developer");
        assert_eq!(
            classify(Some(&r), Some(&issue(IssueState::Completed, false)), None, now()),
            p(Section::Done, None, Reason::IssueDone)
        );
        assert_eq!(
            classify(None, Some(&issue(IssueState::Completed, false)), None, now()),
            p(Section::Done, None, Reason::IssueDone)
        );
    }

    #[test]
    fn no_run_open_issue_is_todo() {
        assert_eq!(classify(None, Some(&issue(IssueState::Open, false)), None, now()), p(Section::Todo, None, Reason::IssueTodo));
    }

    #[test]
    fn no_run_assigned_open_issue_is_still_todo() {
        assert_eq!(classify(None, Some(&issue(IssueState::Open, true)), None, now()), p(Section::Todo, None, Reason::IssueTodo));
    }

    #[test]
    fn not_planned_is_done_with_its_reason() {
        assert_eq!(
            classify(None, Some(&issue(IssueState::NotPlanned, false)), None, now()),
            p(Section::Done, None, Reason::IssueNotPlanned)
        );
    }

    #[test]
    fn not_planned_overrides_a_parked_run() {
        let r = ended("parked_needs_info");
        assert_eq!(
            classify(Some(&r), Some(&issue(IssueState::NotPlanned, false)), None, now()),
            p(Section::Done, None, Reason::IssueNotPlanned)
        );
    }
}
