import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import App from '../App';
import { age, reasonText } from './util';
import type { Person, Snapshot, SettingsView, Ticket } from '../types';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(async () => {}), openPath: vi.fn(async () => {}) }));

const ago = (min: number) => new Date(Date.now() - min * 60_000).toISOString();
const me: Person = { accountId: 'me', displayName: 'Olivier Schweitzer' };
const alice: Person = { accountId: 'alice', displayName: 'Alice Lambert' };
const gh: Person = { accountId: 'oschw', displayName: 'oschw' };

const ticket = (o: Partial<Ticket> & Pick<Ticket, 'key' | 'project' | 'placement'>): Ticket => ({
  summary: `${o.key} summary`, issueType: 'Story', repo: null, sprintId: null, assignees: [], localRun: false,
  track: null, steps: [], mr: null, ticketUrl: null, ticketStatus: null, labels: [], triage: null, plan: null,
  runDir: null, planPath: null, handoverPath: null, lastActivity: ago(5), ...o,
});

function fixture(): Snapshot {
  return {
    meJira: me, meGithub: gh, people: [me, alice, gh],
    jira: { configured: true, ok: true, error: null, lastOkAt: ago(1) },
    forge: { configured: true, ok: true, error: null, lastOkAt: ago(1) },
    syncedAt: ago(1),
    projects: [
      { key: 'SHOP', tracker: 'jira', repos: ['shop-front', 'shop-api'], forge: 'gitlab', public: false, hasSprints: true,
        sprints: [{ id: 10, name: 'Sprint 24', state: 'active' }, { id: 11, name: 'Sprint 25', state: 'future' }] },
      { key: 'acme/site', tracker: 'github', repos: ['site'], forge: 'github', public: true, hasSprints: false, sprints: [] },
    ],
    tickets: [
      ticket({ key: 'SHOP-142', project: 'SHOP', summary: 'Bug TVA facture', repo: 'shop-front', sprintId: 10, assignees: [me], localRun: true,
        placement: { section: 'inProgress', waitingOn: null, reason: 'running', staleMinutes: 45 },
        track: { current: 5, total: 8, stage: 'ci-gate', failed: true },
        steps: [{ name: 'triage', state: 'done', note: null }, { name: 'ci-gate', state: 'fail', note: 'test:unit' }],
        mr: { number: 87, url: 'https://gitlab.example/shop/-/merge_requests/87', state: 'open', draft: true, ci: 'red', pipelineUrl: 'https://gitlab.example/p/1' },
        runDir: '/r/shop-front/.ticket-flow/runs/SHOP-142' }),
      ticket({ key: 'SHOP-144', project: 'SHOP', summary: 'Page panier', repo: 'shop-front', sprintId: 10, assignees: [me], localRun: true,
        placement: { section: 'waiting', waitingOn: 'you', reason: 'plan_to_approve', staleMinutes: null },
        planPath: '/r/shop-front/.ticket-flow/runs/SHOP-144/plan.md' }),
      ticket({ key: 'SHOP-146', project: 'SHOP', summary: 'Pagination API', repo: 'shop-api', sprintId: 10, assignees: [alice],
        placement: { section: 'inProgress', waitingOn: null, reason: 'jira_in_progress', staleMinutes: null } }),
      ticket({ key: 'SHOP-147', project: 'SHOP', summary: 'Pair work', repo: 'shop-api', sprintId: 10, assignees: [me, alice],
        placement: { section: 'inProgress', waitingOn: null, reason: 'jira_in_progress', staleMinutes: null } }),
      ticket({ key: 'SHOP-131', project: 'SHOP', summary: 'Logo email', repo: 'shop-front', sprintId: 10, assignees: [me], localRun: true,
        placement: { section: 'done', waitingOn: null, reason: 'ready_for_qa', staleMinutes: null },
        mr: { number: 81, url: 'https://gitlab.example/shop/-/merge_requests/81', state: 'merged', draft: false, ci: 'green', pipelineUrl: null } }),
      ticket({ key: 'SHOP-150', project: 'SHOP', summary: 'Export CSV', sprintId: 10,
        placement: { section: 'todo', waitingOn: null, reason: 'jira_todo', staleMinutes: null } }),
      ticket({ key: 'GH-7', project: 'acme/site', summary: 'Broken footer', repo: 'site', assignees: [gh], ticketStatus: 'open',
        labels: ['bug', 'good first issue'], ticketUrl: 'https://github.com/acme/site/issues/7',
        placement: { section: 'todo', waitingOn: null, reason: 'issue_todo', staleMinutes: null } }),
    ],
  };
}

const settingsView = (o: Partial<SettingsView> = {}): SettingsView =>
  ({ roots: ['/r'], jiraUrl: 'https://x.atlassian.net', jiraEmail: 'o@x.com', language: 'fr', hasToken: true, ...o });

function backend(snapshot: Snapshot, settings: SettingsView) {
  vi.mocked(invoke).mockImplementation(async (cmd: string) => {
    if (cmd === 'get_snapshot') return snapshot;
    if (cmd === 'get_settings') return settings;
    return undefined;
  });
}

async function renderApp(snapshot = fixture(), settings = settingsView()) {
  backend(snapshot, settings);
  render(<App />);
  await act(async () => {});
}

const openProject = (name: RegExp) => fireEvent.click(within(screen.getByRole('complementary')).getByRole('button', { name }));
const row = (key: string) => screen.getByText(key).closest('tr') as HTMLElement;

beforeEach(() => localStorage.clear());
afterEach(() => { cleanup(); vi.clearAllMocks(); });

describe('screens', () => {
  it('first_launch_opens_settings', async () => {
    await renderApp(fixture(), settingsView({ roots: [], hasToken: false }));
    expect(screen.getByRole('heading', { name: 'Réglages' })).toBeTruthy();
    const token = screen.getByLabelText('Jeton d’API Jira') as HTMLInputElement;
    expect(token.type).toBe('password');
    expect(token.value).toBe('');
    expect(screen.queryByText('Jeton enregistré')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Annuler' })).toBeNull();
  });

  it('overview_lists_projects_with_counts', async () => {
    await renderApp();
    expect(screen.getByRole('heading', { level: 1, name: 'Tous les projets' })).toBeTruthy();
    const cells = (key: string) => [...(within(screen.getByRole('main')).getByText(key).closest('tr') as HTMLElement).querySelectorAll('td')]
      .slice(1, 6).map((td) => td.textContent);
    // SHOP: me only, sprint 24. SHOP-146 (Alice) and SHOP-150 (unassigned) are filtered out.
    expect(cells('SHOP')).toEqual(['1', '0', '2 1 CI rouge', '0', '1']);
    expect(cells('acme/site')).toEqual(['0', '0', '0', '1', '0']);
    const nav = within(screen.getByRole('complementary')).getByRole('button', { name: /^SHOP/ });
    expect(nav.textContent).toContain('1');
  });

  it('project_default_filter_is_me', async () => {
    await renderApp();
    openProject(/^SHOP/);
    expect(screen.getByRole('heading', { level: 1, name: 'SHOP' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Moi' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByRole('button', { name: 'Alice Lambert' }).getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByRole('button', { name: 'Sprint 24' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('SHOP-142')).toBeTruthy();
    expect(screen.queryByText('SHOP-146')).toBeNull();
    expect(screen.queryByText('SHOP-150')).toBeNull();
    // Stale run: flag in the stage caption.
    expect(screen.getByLabelText('ci-gate 5/8, failed').textContent).toContain('sans nouvelle depuis 45 min');
    // Search filters the visible list.
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'panier' } });
    expect(screen.queryByText('SHOP-142')).toBeNull();
    expect(screen.getAllByText('SHOP-144').length).toBeGreaterThan(0);
  });

  it('adding_a_person_shows_their_tickets', async () => {
    await renderApp();
    openProject(/^SHOP/);
    fireEvent.click(screen.getByRole('button', { name: 'Alice Lambert' }));
    expect(screen.getByText('SHOP-146')).toBeTruthy();
    expect(within(row('SHOP-146')).getByText('en cours chez AL')).toBeTruthy();
    expect(JSON.parse(localStorage.getItem('tf.prefs.SHOP') ?? '{}').people).toEqual(['__me__', 'alice']);
  });

  it('clicking_a_row_opens_panel_and_escape_closes', async () => {
    await renderApp();
    openProject(/^SHOP/);
    const r = row('SHOP-142');
    expect(r.tabIndex).toBe(0);
    fireEvent.click(r);
    expect(screen.getByRole('heading', { level: 2, name: 'SHOP-142' })).toBeTruthy();
    expect(r.getAttribute('aria-current')).toBe('true');
    expect(screen.queryByRole('heading', { name: 'À toi de jouer' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Pipeline' }));
    expect(openUrl).toHaveBeenCalledWith('https://gitlab.example/p/1');
    fireEvent.click(screen.getByRole('button', { name: 'Dossier du run' }));
    expect(openPath).toHaveBeenCalledWith('/r/shop-front/.ticket-flow/runs/SHOP-142');
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('heading', { level: 2, name: 'SHOP-142' })).toBeNull();
    expect(document.activeElement).toBe(r);
    fireEvent.keyDown(r, { key: 'Enter' });
    expect(screen.getByRole('heading', { level: 2, name: 'SHOP-142' })).toBeTruthy();
  });

  it('action_card_hidden_when_nothing_waits', async () => {
    await renderApp();
    openProject(/^SHOP/);
    expect(screen.getByRole('heading', { name: 'À toi de jouer' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Ouvrir le premier' }));
    expect(openPath).toHaveBeenCalledWith('/r/shop-front/.ticket-flow/runs/SHOP-144/plan.md');
    openProject(/^acme\/site/);
    expect(screen.getByRole('heading', { level: 1, name: 'acme/site' })).toBeTruthy();
    expect(screen.queryByRole('heading', { name: 'À toi de jouer' })).toBeNull();
  });

  it('jira_not_configured_banner', async () => {
    const s = fixture();
    s.jira = { configured: false, ok: false, error: null, lastOkAt: null };
    await renderApp(s);
    openProject(/^SHOP/);
    expect(screen.getByText('Jira non configuré')).toBeTruthy();
    openProject(/^acme\/site/);
    expect(screen.queryByText('Jira non configuré')).toBeNull();
  });

  it('stale_source_shows_age', async () => {
    const s = fixture();
    s.forge = { configured: true, ok: false, error: 'gh: timed out', lastOkAt: ago(12) };
    await renderApp(s);
    const line = within(screen.getByRole('complementary')).getByText(/gh: timed out/);
    expect(line.textContent).toContain('12 min');
    expect(within(screen.getByRole('complementary')).getByText(/Synchronisé il y a/)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Actualiser' }));
    expect(invoke).toHaveBeenCalledWith('refresh');
  });

  it('language_switch_changes_copy', async () => {
    await renderApp();
    fireEvent.click(screen.getByRole('button', { name: 'Réglages' }));
    expect(screen.getByText('Jeton enregistré')).toBeTruthy();
    expect((screen.getByLabelText('Dossiers racines') as HTMLTextAreaElement).value).toBe('/r');
    fireEvent.change(screen.getByLabelText('Langue'), { target: { value: 'en' } });
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Enregistrer' })); });
    expect(invoke).toHaveBeenCalledWith('save_settings', {
      settings: { roots: ['/r'], jiraUrl: 'https://x.atlassian.net', jiraEmail: 'o@x.com', language: 'en' }, token: null,
    });
    expect(screen.getByRole('heading', { level: 1, name: 'All projects' })).toBeTruthy();
  });

  it('projects_grouped_by_tracker', async () => {
    await renderApp();
    const side = within(screen.getByRole('complementary'));
    expect(side.getByRole('navigation', { name: 'Jira' })).toBeTruthy();
    expect(side.getByRole('navigation', { name: 'GitHub' })).toBeTruthy();
    cleanup();
    const s = fixture();
    s.projects = s.projects.filter((p) => p.tracker === 'jira');
    await renderApp(s);
    expect(within(screen.getByRole('complementary')).queryByRole('navigation', { name: 'GitHub' })).toBeNull();
    expect(within(screen.getByRole('main')).queryByText('GitHub')).toBeNull();
  });

  it('github_detail_shows_issue_state_and_public_tag', async () => {
    await renderApp();
    openProject(/^acme\/site/);
    expect(screen.queryByRole('button', { name: 'Backlog' })).toBeNull();
    fireEvent.click(row('GH-7'));
    const panel = within(screen.getByRole('heading', { level: 2, name: 'GH-7' }).closest('section') as HTMLElement);
    expect(panel.getByText('ouverte')).toBeTruthy();
    expect(panel.getByText('bug')).toBeTruthy();
    expect(panel.getByText('good first issue')).toBeTruthy();
    expect(panel.getByText('public')).toBeTruthy();
    fireEvent.click(panel.getByRole('button', { name: 'Issue GitHub' }));
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/site/issues/7');
  });

  it('colleague_in_progress_shows_colleague_initials', async () => {
    await renderApp();
    openProject(/^SHOP/);
    expect(within(row('SHOP-147')).getByText('en cours chez AL')).toBeTruthy();
    const mine = fixture().tickets.find((x) => x.key === 'SHOP-147') as Ticket;
    expect(reasonText('fr', { ...mine, assignees: [me] }, me)).toBe('en cours');
  });

  it('mr_state_is_a_word', async () => {
    await renderApp();
    openProject(/^SHOP/);
    expect(within(row('SHOP-142')).getByText('!87 · brouillon')).toBeTruthy();
    expect(within(row('SHOP-131')).getByText('!81 · mergée')).toBeTruthy();
    fireEvent.click(row('SHOP-131'));
    const panel = within(screen.getByRole('heading', { level: 2, name: 'SHOP-131' }).closest('section') as HTMLElement);
    expect(panel.getByText('!81 · mergée')).toBeTruthy();
  });

  it('age_uses_localised_units', () => {
    const now = Date.now();
    expect(age(now - 40_000, 'fr', now)).toBe('40 s');
    expect(age(now - 12 * 60_000, 'en', now)).toBe('12 min');
    expect(age(now - 2 * 86_400_000, 'fr', now)).toBe('2 j');
    expect(age(now - 2 * 86_400_000, 'en', now)).toBe('2 d');
  });

  it('settings_load_failure_blocks_save', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_settings') throw 'settings: keychain locked';
      if (cmd === 'get_snapshot') return fixture();
      return undefined;
    });
    render(<App />);
    await act(async () => {});
    expect(screen.getByRole('alert').textContent).toContain('settings: keychain locked');
    const save = screen.getByRole('button', { name: 'Enregistrer' }) as HTMLButtonElement;
    expect(save.disabled).toBe(true);
    fireEvent.submit(save.closest('form') as HTMLFormElement);
    await act(async () => {});
    expect(invoke).not.toHaveBeenCalledWith('save_settings', expect.anything());
  });
});
