import type { MouseEvent, KeyboardEvent } from 'react';
import { StageTrack, StatusPill, type Tone } from '../ui';
import { t, type Lang } from '../i18n';
import { ticketId, type Person, type ProjectView, type Ticket } from '../types';
import { mrText, reasonText, short } from './util';

const CI_TONE = { green: 'ok', red: 'bad', running: 'run' } as const;

function tone(tk: Ticket): Tone {
  const { section, waitingOn, reason } = tk.placement;
  if (section === 'waiting') return waitingOn === 'client' ? 'client' : waitingOn === 'you' ? 'you' : 'mute';
  if (section === 'inProgress') return 'run';
  if (section === 'done') return reason === 'ready_for_qa' || reason === 'jira_done' || reason === 'issue_done' ? 'ok' : 'mute';
  return 'mute';
}

interface Props {
  lang: Lang;
  tickets: Ticket[];
  me: Person | null;
  project: ProjectView;
  selected: string | null;
  onSelect: (tk: Ticket, row: HTMLElement) => void;
}

export function TicketTable({ lang, tickets, me, project, selected, onSelect }: Props) {
  if (!tickets.length) return null;
  const pick = (tk: Ticket) => (e: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>) => {
    if (e.type === 'click' || (e as KeyboardEvent).key === 'Enter') onSelect(tk, e.currentTarget);
  };
  return (
    <table className="ticket-table">
      <colgroup><col /><col className="col-progress" /><col className="col-mr" /></colgroup>
      <thead>
        <tr>{['col_ticket', 'col_progress', 'col_mr'].map((k) => <th key={k}>{t(lang, k)}</th>)}</tr>
      </thead>
      <tbody>
        {tickets.map((tk) => {
          const others = tk.assignees.filter((a) => a.accountId !== me?.accountId).map((a) => short(a.displayName));
          const stale = tk.placement.staleMinutes;
          const detail = [
            tk.track && `${tk.track.current}/${tk.track.total}`,
            stale != null ? t(lang, 'stale', { n: stale }) : tk.placement.reason !== 'running' && reasonText(lang, tk, me),
          ].filter(Boolean).join(' · ');
          return (
            <tr key={ticketId(tk)} className="tf-clickable" tabIndex={0} aria-current={selected === ticketId(tk) ? 'true' : undefined}
              onClick={pick(tk)} onKeyDown={pick(tk)}>
              <td>
                <span className="ellipsis"><span className="tf-key">{tk.key}</span> {tk.summary}</span>
                <span className="tf-sub ellipsis">{[tk.repo, ...others].filter(Boolean).join(' · ')}</span>
              </td>
              <td>
                {tk.localRun && tk.track && tk.placement.section === 'inProgress'
                  ? <StageTrack current={tk.track.current} total={tk.track.total} stage={tk.track.stage} failed={tk.track.failed} detail={detail} />
                  : <StatusPill tone={tone(tk)}>{reasonText(lang, tk, me)}</StatusPill>}
              </td>
              <td>
                {tk.mr && (
                  <div className="mr">
                    <span className="tf-sub">{mrText(lang, project, tk.mr)}</span>
                    {tk.mr.ci !== 'none' && <StatusPill tone={CI_TONE[tk.mr.ci]}>{t(lang, `ci_${tk.mr.ci}`)}</StatusPill>}
                  </div>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
