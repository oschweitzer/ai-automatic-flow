import { afterEach, describe, expect, it, vi } from 'vitest';
import { initials, t } from './i18n';
import { loadPrefs, savePrefs } from './prefs';
import type { ProjectView } from './types';

const project: ProjectView = {
  key: 'SHOP', tracker: 'jira', repos: [], forge: null, public: false, hasSprints: true,
  sprints: [{ id: 7, name: 'S7', state: 'active' }, { id: 8, name: 'S8', state: 'future' }],
};

afterEach(() => { localStorage.clear(); vi.restoreAllMocks(); });

describe('prefs', () => {
  it('prefs_default_and_round_trip', () => {
    const d = loadPrefs('SHOP', project);
    expect(d).toEqual({ people: ['__me__'], sprints: [7], open: ['inProgress', 'waiting'] });
    expect(loadPrefs('KAN', { ...project, hasSprints: false }).sprints).toEqual([]);
    const next = { people: ['__me__', 'bob'], sprints: [7, 'backlog' as const], open: ['done' as const] };
    savePrefs('SHOP', next);
    expect(localStorage.getItem('tf.prefs.SHOP')).toBe(JSON.stringify(next));
    expect(loadPrefs('SHOP', project)).toEqual(next);
    expect(loadPrefs('OTHER', project)).toEqual(d);
  });

  it('prefs_survive_broken_storage', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked'); });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('blocked'); });
    expect(loadPrefs('SHOP', project).people).toEqual(['__me__']);
    expect(() => savePrefs('SHOP', loadPrefs('SHOP', project))).not.toThrow();
    vi.restoreAllMocks();
    localStorage.setItem('tf.prefs.SHOP', '{nope');
    expect(loadPrefs('SHOP', project).sprints).toEqual([7]);
    localStorage.setItem('tf.prefs.SHOP', JSON.stringify({ people: ['bob'], open: 'x' }));
    expect(loadPrefs('SHOP', project)).toEqual({ people: ['bob'], sprints: [7], open: ['inProgress', 'waiting'] });
  });
});

describe('i18n', () => {
  it('t_falls_back_to_key', () => {
    expect(t('fr', 'nope')).toBe('nope');
    expect(t('en', 'nope', { n: 1 })).toBe('nope');
    expect(t('fr', 'stale', { n: 45 })).toBe('sans nouvelle depuis 45 min');
    expect(t('en', 'public')).toBe('public');
  });

  it('initials_take_two_words', () => {
    expect(initials('alice lambert')).toBe('AL');
    expect(initials('  Bob ')).toBe('B');
    expect(initials('Jean Pierre Dupont')).toBe('JP');
    expect(t('fr', 'jira_in_progress', { initials: initials('Al Lee') })).toBe('en cours chez AL');
  });
});
