import { useState } from 'react';
import { Section, StatusPill } from '../ui';
import { t, type Lang } from '../i18n';
import type { Snapshot, Tracker } from '../types';
import { age, projectCounts } from './util';

const TRACKERS: Tracker[] = ['jira', 'github'];

interface Props { lang: Lang; snapshot: Snapshot; onOpen: (key: string) => void }

export function Overview({ lang, snapshot, onOpen }: Props) {
  const [closed, setClosed] = useState<Tracker[]>([]);
  const last = (key: string) => Math.max(0, ...snapshot.tickets.filter((x) => x.project === key && x.lastActivity)
    .map((x) => Date.parse(x.lastActivity as string)));
  const big = (n: number, tone?: 'you' | 'client') => <td className={`count${n === 0 ? ' zero' : tone ? ` ${tone}` : ''}`}>{n}</td>;

  return (
    <div className="screen">
      <header className="screen-head"><h1 className="page-title">{t(lang, 'all_projects')}</h1></header>
      {TRACKERS.map((tr) => {
        const projects = snapshot.projects.filter((p) => p.tracker === tr);
        if (!projects.length) return null;
        const open = !closed.includes(tr);
        return (
          <Section key={tr} title={t(lang, `group_${tr}`)} count={projects.length} open={open}
            onToggle={() => setClosed(open ? [...closed, tr] : closed.filter((x) => x !== tr))}>
            <table className="overview-table">
              <thead>
                <tr>
                  {['col_project', 'your_turn', 'col_client', 'section_inProgress', 'section_todo', 'section_done', 'col_activity']
                    .map((k) => <th key={k}>{t(lang, k)}</th>)}
                </tr>
              </thead>
              <tbody>
                {projects.map((p) => {
                  const c = projectCounts(snapshot, p);
                  const at = last(p.key);
                  return (
                    <tr key={p.key} className="tf-clickable" tabIndex={0} onClick={() => onOpen(p.key)}
                      onKeyDown={(e) => e.key === 'Enter' && onOpen(p.key)}>
                      <td className="cell-main"><span className="tf-key">{p.key}</span><span className="tf-sub ellipsis">{p.repos.join(', ')}</span></td>
                      {big(c.you, 'you')}
                      {big(c.client, 'client')}
                      <td className="nowrap">
                        <span className={`count${c.inProgress === 0 ? ' zero' : ''}`}>{c.inProgress}</span>
                        {c.ciRed > 0 && <> <StatusPill tone="bad">{t(lang, 'ci_red_count', { n: c.ciRed })}</StatusPill></>}
                      </td>
                      {big(c.todo)}
                      {big(c.done)}
                      <td className="tf-sub nowrap">{at ? t(lang, 'ago', { age: age(at, lang) }) : ''}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </Section>
        );
      })}
    </div>
  );
}
