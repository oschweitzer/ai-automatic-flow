import { useEffect, useRef, useState } from 'react';
import { ActionCard, Icon, Section } from '../ui';
import { t, type Lang } from '../i18n';
import { bySection, visible } from '../filter';
import { loadPrefs, savePrefs, type ProjectPrefs } from '../prefs';
import { ticketId, type Person, type ProjectView, type SectionKey, type Snapshot, type Ticket } from '../types';
import { TopBar } from './TopBar';
import { TicketTable } from './TicketTable';
import { DetailPanel } from './DetailPanel';
import { age, meOf, open, reasonText, target } from './util';

const SECTIONS: SectionKey[] = ['inProgress', 'waiting', 'todo', 'done'];
const FAILED = new Set(['plan_rejected', 'escalated_checks', 'escalated_ci', 'escalated_comments', 'timeout', 'failed_permission']);

interface Props { lang: Lang; snapshot: Snapshot; project: ProjectView; onPrefs: () => void }

export function ProjectScreen({ lang, snapshot, project, onPrefs }: Props) {
  const [prefs, setPrefsState] = useState(() => loadPrefs(project.key, project));
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const opener = useRef<HTMLElement | null>(null);

  const setPrefs = (p: ProjectPrefs) => { setPrefsState(p); savePrefs(project.key, p); onPrefs(); };
  const me = meOf(snapshot, project);
  const all = snapshot.tickets.filter((x) => x.project === project.key);
  const people = [...new Map(all.flatMap((x) => x.assignees).filter((a) => a.accountId !== me?.accountId)
    .map((a): [string, Person] => [a.accountId, a])).values()];
  const shown = visible(snapshot.tickets, project, prefs, me, query);
  const sections = bySection(shown);
  const yours = sections.waiting.filter((x) => x.placement.waitingOn === 'you');
  const ticket = selected ? all.find((x) => ticketId(x) === selected) : undefined;

  const close = () => { setSelected(null); opener.current?.focus(); };
  useEffect(() => {
    if (!ticket) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  });

  const select = (tk: Ticket, row: HTMLElement) => { opener.current = row; setSelected(ticketId(tk)); };
  const toggle = (s: SectionKey) =>
    setPrefs({ ...prefs, open: prefs.open.includes(s) ? prefs.open.filter((x) => x !== s) : [...prefs.open, s] });
  const openFirst = (tk: Ticket) => { const to = target(tk); if (to) open(to); };
  const sprintNames = [
    ...project.sprints.filter((s) => prefs.sprints.includes(s.id)).map((s) => s.name),
    ...(prefs.sprints.includes('backlog') ? [t(lang, 'backlog')] : []),
  ];

  return (
    <>
      <TopBar lang={lang} project={project} me={me} people={people} prefs={prefs} onPrefs={setPrefs} query={query} onQuery={setQuery} />
      <div className="screen">
        <header className="screen-head">
          <h1 className="page-title">{project.key}</h1>
          <p className="tf-sub">{[...project.repos, project.forge === 'github' ? 'GitHub' : project.forge === 'gitlab' ? 'GitLab' : null]
            .filter(Boolean).join(' · ')}</p>
        </header>
        {project.tracker === 'jira' && !snapshot.jira.configured && (
          <div className="banner" role="status">
            <Icon name="TriangleAlert" />
            <p><strong>{t(lang, 'jira_not_configured')}</strong> {t(lang, 'jira_not_configured_hint')}</p>
          </div>
        )}
        <div className="project-grid">
          <div className="list">
            {SECTIONS.map((s) => (
              <Section key={s} title={t(lang, `section_${s}`)} count={sections[s].length} open={prefs.open.includes(s)} onToggle={() => toggle(s)}>
                <TicketTable lang={lang} tickets={sections[s]} me={me} project={project}
                  selected={selected} onSelect={select} />
              </Section>
            ))}
          </div>
          <div className="side">
            {ticket ? (
              <DetailPanel lang={lang} project={project} ticket={ticket} onClose={close} />
            ) : (
              <>
                {yours.length > 0 && (
                  <ActionCard title={t(lang, 'your_turn')}
                    text={yours.length === 1 ? t(lang, 'your_turn_one') : t(lang, 'your_turn_many', { n: yours.length })}
                    items={yours.map((x) => ({
                      title: `${x.key} · ${x.summary}`,
                      meta: [reasonText(lang, x, me), x.lastActivity && age(x.lastActivity, lang)].filter(Boolean).join(' · '),
                      icon: FAILED.has(x.placement.reason) ? 'alert' : 'check',
                      onClick: () => openFirst(x),
                    }))}
                    actionLabel={t(lang, 'open_first')} onAction={() => openFirst(yours[0])} />
                )}
                <section className="mini">
                  <h2 className="label">{sprintNames.length ? sprintNames.join(' · ') : t(lang, 'summary')}</h2>
                  {SECTIONS.map((s) => (
                    <div className="mini-line body" key={s}><span>{t(lang, `section_${s}`)}</span><span>{sections[s].length}</span></div>
                  ))}
                </section>
              </>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
