import type { Person, ProjectView, SectionKey, Ticket } from './types';
import type { ProjectPrefs } from './prefs';

const fold = (s: string) => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();

/** Tickets of `project` that pass the person filter, the sprint filter (Jira projects with sprints) and the search. */
export function visible(tickets: Ticket[], project: ProjectView, prefs: ProjectPrefs, me: Person | null, query: string): Ticket[] {
  const q = fold(query.trim());
  return tickets.filter((t) => {
    if (t.project !== project.key) return false;
    const mine = t.localRun || (me != null && t.assignees.some((a) => a.accountId === me.accountId));
    const person = prefs.people.some((p) =>
      p === '__me__' ? mine
        : p === '__unassigned__' ? t.assignees.length === 0 && !t.localRun
        : t.assignees.some((a) => a.accountId === p));
    if (!person) return false;
    // A local run outside any sprint (backlog, future sprint, key unknown to Jira) is the developer's: it always shows.
    if (project.hasSprints && !(t.localRun && t.sprintId == null)) {
      const sprint = t.sprintId == null ? prefs.sprints.includes('backlog') : prefs.sprints.includes(t.sprintId);
      if (!sprint) return false;
    }
    return !q || fold(t.key).includes(q) || fold(t.summary).includes(q);
  });
}

const time = (t: Ticket) => (t.lastActivity ? Date.parse(t.lastActivity) : 0);
const natural = new Intl.Collator('en', { numeric: true });

/** Waiting on you first, then latest activity, then key in natural order (GH-7 before GH-142). */
function order(a: Ticket, b: Ticket): number {
  const you = Number(b.placement.waitingOn === 'you') - Number(a.placement.waitingOn === 'you');
  return you || time(b) - time(a) || natural.compare(a.key, b.key);
}

export function bySection(tickets: Ticket[]): Record<SectionKey, Ticket[]> {
  const out: Record<SectionKey, Ticket[]> = { inProgress: [], waiting: [], todo: [], done: [] };
  for (const t of [...tickets].sort(order)) out[t.placement.section].push(t);
  return out;
}

export function counts(tickets: Ticket[]) {
  const n = (f: (t: Ticket) => boolean) => tickets.filter(f).length;
  const sec = (s: SectionKey) => (t: Ticket) => t.placement.section === s;
  return {
    you: n((t) => t.placement.section === 'waiting' && t.placement.waitingOn === 'you'),
    client: n((t) => t.placement.section === 'waiting' && t.placement.waitingOn === 'client'),
    inProgress: n(sec('inProgress')),
    ciRed: n((t) => t.placement.section === 'inProgress' && t.mr?.ci === 'red'),
    todo: n(sec('todo')),
    done: n(sec('done')),
  };
}
