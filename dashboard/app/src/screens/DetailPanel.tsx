import { Button, Icon, StatusPill, StepList } from '../ui';
import { t, type Lang } from '../i18n';
import type { ProjectView, Ticket } from '../types';
import { open } from './util';

const ISSUE_STATE: Record<string, string> = { open: 'issue_state_open', completed: 'issue_state_completed', notPlanned: 'issue_state_not_planned' };

interface Props { lang: Lang; project: ProjectView; ticket: Ticket; onClose: () => void }

export function DetailPanel({ lang, project, ticket: tk, onClose }: Props) {
  const github = project.tracker === 'github';
  const sprint = project.sprints.find((s) => s.id === tk.sprintId)?.name ?? (project.hasSprints ? t(lang, 'backlog') : null);
  const status = github ? (tk.ticketStatus && t(lang, ISSUE_STATE[tk.ticketStatus] ?? tk.ticketStatus)) : tk.ticketStatus;
  const meta = [tk.summary, tk.issueType, tk.repo, sprint, tk.assignees.map((a) => a.displayName).join(', ') || t(lang, 'unassigned')];
  const mr = tk.mr && `${project.forge === 'github' ? 'PR #' : 'MR !'}${tk.mr.number}`;
  const links: [string, string | null | undefined][] = [
    [t(lang, github ? 'link_issue' : 'link_jira'), tk.ticketUrl],
    [mr ?? '', tk.mr?.url],
    [t(lang, 'link_pipeline'), tk.mr?.pipelineUrl],
    [t(lang, 'link_run'), tk.runDir],
    [t(lang, 'link_handover'), tk.handoverPath],
  ];

  return (
    <section className="panel" aria-labelledby="panel-title">
      <button type="button" className="icon-btn panel-close" aria-label={t(lang, 'close')} title={t(lang, 'close')} onClick={onClose} autoFocus>
        <Icon name="X" />
      </button>
      <h2 id="panel-title" className="panel-title">{tk.key}</h2>
      <p className="panel-meta">{meta.filter(Boolean).join(' · ')}</p>
      <div className="tags">
        {status && <StatusPill>{status}</StatusPill>}
        {github && tk.labels.map((l) => <StatusPill key={l}>{l}</StatusPill>)}
        {project.public && <StatusPill>{t(lang, 'public')}</StatusPill>}
        {tk.triage?.size && <StatusPill>{t(lang, 'size', { size: tk.triage.size })}</StatusPill>}
        {tk.triage?.risk.map((r) => <StatusPill key={r}>{t(lang, 'risk', { risk: r })}</StatusPill>)}
        {tk.plan?.planner && <StatusPill tone="run">plan {tk.plan.planner}</StatusPill>}
        {tk.plan?.executor && <StatusPill tone="run">execute {tk.plan.executor}</StatusPill>}
      </div>

      {tk.localRun && tk.steps.length > 0 && (
        <>
          <h3>{tk.track ? t(lang, 'progress', { current: tk.track.current, total: tk.track.total }) : t(lang, 'col_progress')}</h3>
          <StepList steps={tk.steps.map((s) => ({ name: s.name, state: s.state, note: s.note ?? undefined }))} />
        </>
      )}

      {tk.plan && (
        <>
          <h3>{t(lang, 'plan')}</h3>
          <div className="plan">
            {tk.plan.behaviour && <p>{tk.plan.behaviour}</p>}
            {tk.plan.files.length > 0 && <ul>{tk.plan.files.map((f) => <li key={f}><code className="code">{f}</code></li>)}</ul>}
            {tk.planPath && <button type="button" className="link" onClick={() => open(tk.planPath as string)}>{t(lang, 'plan_full')}</button>}
          </div>
        </>
      )}

      {links.some(([, to]) => to) && (
        <>
          <h3>{t(lang, 'links')}</h3>
          <div className="links">
            {links.map(([label, to]) => to && <Button key={label} onClick={() => open(to)}>{label}</Button>)}
          </div>
        </>
      )}
    </section>
  );
}
