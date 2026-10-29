import { useEffect, useState } from 'react';
import { Icon, NavItem } from '../ui';
import { t, type Lang } from '../i18n';
import type { Snapshot, SourceState, Tracker } from '../types';
import { age, projectCounts } from './util';

export type View = { kind: 'all' } | { kind: 'project'; key: string } | { kind: 'settings' };

const TRACKERS: Tracker[] = ['jira', 'github'];

interface Props {
  lang: Lang;
  snapshot: Snapshot | null;
  view: View;
  onView: (v: View) => void;
  onRefresh: () => void;
}

export function Sidebar({ lang, snapshot, view, onView, onRefresh }: Props) {
  // Re-render the sync age every 15 s.
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 15_000);
    return () => clearInterval(id);
  }, []);

  const failing = (name: string, s: SourceState) =>
    s.configured && !s.ok && (
      <p className="sync sync-bad" key={name}>
        <Icon name="TriangleAlert" />
        <span>{[t(lang, 'source_failed', { source: name }), s.lastOkAt && t(lang, 'data_age', { age: age(s.lastOkAt, now) }), s.error]
          .filter(Boolean).join(' · ')}</span>
      </p>
    );

  return (
    <aside className="rail">
      <div className="brand"><b>Ticket Flow</b><small>Dashboard</small></div>
      <nav className="rail-nav">
        <NavItem label={t(lang, 'all_projects')} icon="grid" selected={view.kind === 'all'} onClick={() => onView({ kind: 'all' })} />
      </nav>
      {snapshot && TRACKERS.map((tr) => {
        const projects = snapshot.projects.filter((p) => p.tracker === tr);
        if (!projects.length) return null;
        return (
          <nav className="rail-nav" key={tr} aria-labelledby={`group-${tr}`}>
            <h6 id={`group-${tr}`}>{t(lang, `group_${tr}`)}</h6>
            {projects.map((p) => (
              <NavItem key={p.key} label={p.key} sub={p.repos.join(', ')} icon="folder" count={projectCounts(snapshot, p).you}
                selected={view.kind === 'project' && view.key === p.key} onClick={() => onView({ kind: 'project', key: p.key })} />
            ))}
          </nav>
        );
      })}
      <div className="rail-foot">
        {snapshot && (
          <>
            {failing('Jira', snapshot.jira)}
            {failing('GitHub/GitLab', snapshot.forge)}
            <div className="sync">
              <span>{t(lang, 'synced', { age: age(snapshot.syncedAt, now) })}</span>
              <button type="button" className="icon-btn" aria-label={t(lang, 'refresh')} title={t(lang, 'refresh')} onClick={onRefresh}>
                <Icon name="RefreshCw" />
              </button>
            </div>
          </>
        )}
        <button type="button" className="tf-nav" aria-current={view.kind === 'settings' ? 'page' : undefined} onClick={() => onView({ kind: 'settings' })}>
          <Icon name="Settings" /><span>{t(lang, 'settings')}</span>
        </button>
      </div>
    </aside>
  );
}
