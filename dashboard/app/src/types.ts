// Mirror of the Rust payload (src-tauri/src/snapshot.rs and friends). Keep both in step.
export type SectionKey = 'inProgress' | 'waiting' | 'todo' | 'done';
export type WaitingOn = 'you' | 'client';
export type Reason =
  | 'plan_to_approve' | 'mr_to_merge' | 'client_steps_to_publish' | 'parked_needs_info' | 'plan_rejected'
  | 'escalated_checks' | 'escalated_ci' | 'escalated_comments' | 'timeout' | 'failed_permission'
  | 'running' | 'waiting_ci' | 'waiting_reviews' | 'ready_for_qa' | 'split' | 'mr_closed'
  | 'jira_todo' | 'jira_in_progress' | 'jira_done' | 'issue_todo' | 'issue_done' | 'issue_not_planned';
export type MrState = 'open' | 'merged' | 'closed';
export type Ci = 'green' | 'red' | 'running' | 'none';
export type StepState = 'done' | 'now' | 'fail' | 'todo';
export type Tracker = 'jira' | 'github';
export type Forge = 'github' | 'gitlab';
/** GitHub issue states as `ticketStatus` carries them. A Jira ticket carries its Jira status instead. */
export type IssueState = 'open' | 'completed' | 'notPlanned';

export interface Person { accountId: string; displayName: string }
export interface Placement { section: SectionKey; waitingOn: WaitingOn | null; reason: Reason; staleMinutes: number | null }
export interface MrStatus { number: number; url: string; state: MrState; draft: boolean; ci: Ci; pipelineUrl: string | null }
export interface Track { current: number; total: number; stage: string; failed: boolean }
export interface StepInfo { name: string; state: StepState; note: string | null }
export interface TriageInfo { size: string | null; risk: string[] }
export interface PlanSummary { behaviour: string | null; files: string[]; planner: string | null; executor: string | null }
export interface Sprint { id: number; name: string; state: string } // "active" | "future"

export interface Ticket {
  key: string; project: string; summary: string; issueType: string | null; repo: string | null;
  sprintId: number | null; assignees: Person[]; localRun: boolean; placement: Placement;
  track: Track | null; steps: StepInfo[]; mr: MrStatus | null; ticketUrl: string | null;
  ticketStatus: string | null; labels: string[]; triage: TriageInfo | null; plan: PlanSummary | null;
  runDir: string | null; planPath: string | null; handoverPath: string | null;
  lastActivity: string | null; // ISO 8601
}

export interface ProjectView {
  key: string; tracker: Tracker; repos: string[]; forge: Forge | null; public: boolean;
  sprints: Sprint[]; hasSprints: boolean;
}

export interface SourceState { configured: boolean; ok: boolean; error: string | null; lastOkAt: string | null }

export interface Snapshot {
  meJira: Person | null; meGithub: Person | null; projects: ProjectView[]; tickets: Ticket[];
  people: Person[]; jira: SourceState; forge: SourceState; syncedAt: string;
}

export interface Settings { roots: string[]; jiraUrl: string; jiraEmail: string; language: 'fr' | 'en' }
/** What `get_settings` returns. The token itself never comes back. */
export interface SettingsView extends Settings { hasToken: boolean }

/** Tickets are unique by project and key: `GH-<n>` repeats across GitHub repositories. */
export const ticketId = (t: Pick<Ticket, 'project' | 'key'>) => `${t.project}\n${t.key}`;
