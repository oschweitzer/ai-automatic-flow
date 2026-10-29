import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import { initials, t, type Lang } from '../i18n';
import { counts, visible } from '../filter';
import { loadPrefs } from '../prefs';
import type { Person, ProjectView, Snapshot, Ticket } from '../types';

/** Links open in the system's default app: http(s) in the browser, local paths in the file manager or editor. */
export function open(target: string) {
  (/^https?:\/\//.test(target) ? openUrl(target) : openPath(target)).catch(console.error);
}

/** "40 s", "12 min", "3 h", then days. Spoken around it by the `synced`, `ago` and `data_age` copy. */
export function age(at: string | number, now = Date.now()): string {
  const s = Math.max(0, Math.floor((now - (typeof at === 'number' ? at : Date.parse(at))) / 1000));
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} d`;
}

export const meOf = (s: Snapshot, p: ProjectView): Person | null => (p.tracker === 'github' ? s.meGithub : s.meJira);

/** Overview and sidebar counts: the project's remembered person filter, its default sprint selection, no search. */
export function projectCounts(s: Snapshot, p: ProjectView) {
  const prefs = { ...loadPrefs(p.key, p), sprints: p.sprints.filter((x) => x.state === 'active').map((x) => x.id) };
  return counts(visible(s.tickets, p, prefs, meOf(s, p), ''));
}

/** What a waiting ticket asks for: the plan, the MR, the pipeline, the client steps, else the ticket. */
export function target(tk: Ticket): string | null {
  const r = tk.placement.reason;
  const first = (...xs: (string | null | undefined)[]) => xs.find((x) => x) ?? null;
  if (r === 'plan_to_approve' || r === 'plan_rejected') return first(tk.planPath, tk.runDir);
  if (r === 'escalated_ci') return first(tk.mr?.pipelineUrl, tk.mr?.url, tk.runDir);
  if (r === 'client_steps_to_publish') return first(tk.handoverPath, tk.ticketUrl);
  if (r === 'parked_needs_info') return first(tk.ticketUrl, tk.runDir);
  return first(tk.mr?.url, tk.runDir, tk.ticketUrl);
}

/** Initials, falling back to the first two letters of a one-word login ("oschw" gives "OS"). */
export function short(name: string): string {
  const i = initials(name);
  return i.length > 1 ? i : name.slice(0, 2).toUpperCase();
}

/** The words of a ticket's state: "à toi · plan à valider", "en cours chez AL". */
export function reasonText(lang: Lang, tk: Ticket): string {
  const { reason, waitingOn } = tk.placement;
  if (reason === 'jira_in_progress') {
    return tk.assignees.length ? t(lang, reason, { initials: short(tk.assignees[0].displayName) }) : t(lang, 'running');
  }
  if (waitingOn === 'you') return `${t(lang, 'waiting_on_you')} · ${t(lang, reason)}`;
  return t(lang, reason);
}
