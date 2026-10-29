import type { ProjectView, SectionKey } from './types';

export interface ProjectPrefs { people: string[]; sprints: (number | 'backlog')[]; open: SectionKey[] } // people: accountIds, '__me__', '__unassigned__'

const SECTIONS: SectionKey[] = ['inProgress', 'waiting', 'todo', 'done'];
const KEY = (project: string) => `tf.prefs.${project}`;

const defaults = (p: ProjectView): ProjectPrefs => ({
  people: ['__me__'],
  sprints: p.hasSprints ? p.sprints.filter((s) => s.state === 'active').map((s) => s.id) : [],
  open: ['inProgress', 'waiting'],
});

export function loadPrefs(project: string, p: ProjectView): ProjectPrefs {
  const d = defaults(p);
  let raw: Partial<Record<keyof ProjectPrefs, unknown>> = {};
  try {
    const parsed = JSON.parse(localStorage.getItem(KEY(project)) ?? '{}');
    if (parsed && typeof parsed === 'object') raw = parsed;
  } catch { /* broken storage or JSON: defaults */ }
  const list = <T,>(v: unknown, ok: (x: unknown) => x is T, fallback: T[]) =>
    Array.isArray(v) && v.every(ok) ? v : fallback;
  return {
    people: list(raw.people, (x): x is string => typeof x === 'string', d.people),
    sprints: list(raw.sprints, (x): x is number | 'backlog' => typeof x === 'number' || x === 'backlog', d.sprints),
    open: list(raw.open, (x): x is SectionKey => SECTIONS.includes(x as SectionKey), d.open),
  };
}

export function savePrefs(project: string, prefs: ProjectPrefs): void {
  try { localStorage.setItem(KEY(project), JSON.stringify(prefs)); } catch { /* not remembered */ }
}
