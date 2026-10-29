import { describe, expect, it } from 'vitest';
import { bySection, counts, visible } from './filter';
import type { ProjectPrefs } from './prefs';
import type { Person, ProjectView, Ticket } from './types';

const me: Person = { accountId: 'me', displayName: 'Me Myself' };
const bob: Person = { accountId: 'bob', displayName: 'Bob' };
const shop: ProjectView = { key: 'SHOP', tracker: 'jira', repos: ['front'], forge: 'gitlab', public: false, sprints: [{ id: 7, name: 'S7', state: 'active' }], hasSprints: true };
const kanban: ProjectView = { ...shop, key: 'KAN', hasSprints: false, sprints: [] };

const tk = (key: string, o: Partial<Ticket> = {}, p: Partial<Ticket['placement']> = {}): Ticket => ({
  key, project: 'SHOP', summary: `${key} summary`, issueType: null, repo: null, sprintId: 7, assignees: [], localRun: false,
  placement: { section: 'todo', waitingOn: null, reason: 'jira_todo', staleMinutes: null, ...p },
  track: null, steps: [], mr: null, ticketUrl: null, ticketStatus: null, labels: [], triage: null, plan: null,
  runDir: null, planPath: null, handoverPath: null, lastActivity: null, ...o,
});
const prefs = (o: Partial<ProjectPrefs> = {}): ProjectPrefs => ({ people: ['__me__'], sprints: [7], open: ['inProgress', 'waiting'], ...o });
const keys = (ts: Ticket[]) => ts.map((t) => t.key);

describe('visible', () => {
  const all = [tk('A-1', { assignees: [me] }), tk('A-2', { assignees: [bob], localRun: true }), tk('A-3', { assignees: [bob] }), tk('A-4')];

  it('default_shows_only_mine_and_local_runs', () => {
    expect(keys(visible(all, shop, prefs(), me, ''))).toEqual(['A-1', 'A-2']);
    expect(keys(visible(all, shop, prefs(), null, ''))).toEqual(['A-2']);
  });

  it('search_matches_key_or_summary', () => {
    const ts = [tk('SHOP-1', { summary: 'Add Cart field', localRun: true }), tk('SHOP-2', { summary: 'other', localRun: true })];
    expect(keys(visible(ts, shop, prefs(), me, 'cart'))).toEqual(['SHOP-1']);
    expect(keys(visible(ts, shop, prefs(), me, 'shop-2'))).toEqual(['SHOP-2']);
    expect(visible(ts, shop, prefs(), me, '')).toHaveLength(2);
  });

  it('unassigned_filter', () => {
    const ts = [...all, tk('A-5', { localRun: true })];
    expect(keys(visible(ts, shop, prefs({ people: ['__unassigned__'] }), me, ''))).toEqual(['A-4']);
    expect(keys(visible(ts, shop, prefs({ people: ['bob'] }), me, ''))).toEqual(['A-2', 'A-3']);
  });

  it('search_ignores_accents_and_other_projects', () => {
    const ts = [tk('SHOP-1', { summary: 'Publier une étape', localRun: true }), tk('PAY-1', { project: 'PAY', summary: 'étape', localRun: true })];
    expect(keys(visible(ts, shop, prefs(), me, 'ETAPE'))).toEqual(['SHOP-1']);
  });

  it('two_people_filter', () => {
    const ts = [tk('A-1', { assignees: [me] }), tk('A-2', { assignees: [bob] }), tk('A-3', { assignees: [{ accountId: 'c', displayName: 'C' }] })];
    expect(keys(visible(ts, shop, prefs({ people: ['__me__', 'bob'] }), me, ''))).toEqual(['A-1', 'A-2']);
  });

  it('backlog_filter', () => {
    const ts = [tk('B-1', { localRun: true, sprintId: null }), tk('B-2', { localRun: true }), tk('B-3', { localRun: true, sprintId: 9 })];
    expect(keys(visible(ts, shop, prefs(), me, ''))).toEqual(['B-2']);
    expect(keys(visible(ts, shop, prefs({ sprints: [7, 'backlog'] }), me, ''))).toEqual(['B-1', 'B-2']);
  });

  it('kanban_project_ignores_sprints', () => {
    const ts = [tk('K-1', { project: 'KAN', localRun: true, sprintId: null })];
    expect(visible(ts, kanban, prefs({ sprints: [] }), me, '')).toHaveLength(1);
  });
});

describe('order and counts', () => {
  it('sections_put_you_first', () => {
    const w = (waitingOn: 'you' | 'client' | null) => ({ section: 'waiting' as const, waitingOn });
    const ts = [
      tk('GH-142', { lastActivity: '2026-10-27T10:00:00Z' }, w('client')),
      tk('GH-7', { lastActivity: '2026-10-27T10:00:00Z' }, w('client')),
      tk('GH-9', { lastActivity: '2026-10-28T10:00:00Z' }, w('client')),
      tk('GH-1', { lastActivity: '2026-10-20T10:00:00Z' }, w('you')),
      tk('GH-2', {}, { section: 'done' }),
    ];
    const s = bySection(ts);
    expect(keys(s.waiting)).toEqual(['GH-1', 'GH-9', 'GH-7', 'GH-142']);
    expect(keys(s.done)).toEqual(['GH-2']);
    expect(s.inProgress).toEqual([]);
  });

  it('counts_match_overview_columns', () => {
    const mr = (ci: 'red' | 'green') => ({ number: 1, url: '', state: 'open' as const, draft: false, ci, pipelineUrl: null });
    const ts = [
      tk('1', {}, { section: 'waiting', waitingOn: 'you' }),
      tk('2', {}, { section: 'waiting', waitingOn: 'client' }),
      tk('3', { mr: mr('red') }, { section: 'inProgress' }),
      tk('4', { mr: mr('green') }, { section: 'inProgress' }),
      tk('5', { mr: mr('red') }, { section: 'done' }),
      tk('6'),
    ];
    expect(counts(ts)).toEqual({ you: 1, client: 1, inProgress: 2, ciRed: 1, todo: 1, done: 1 });
  });
});
