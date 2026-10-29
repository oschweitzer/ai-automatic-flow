import { Avatar, Chip, Icon } from '../ui';
import { t, type Lang } from '../i18n';
import type { ProjectPrefs } from '../prefs';
import type { Person, ProjectView } from '../types';
import { short } from './util';

// Avatar grounds: non-state tokens dark enough for light initials. A stable hash of the account id picks one.
const GROUNDS = ['var(--violet)', 'var(--violet-deep)', 'var(--accent-soft)', 'var(--surface-2)'];
const ground = (id: string) => GROUNDS[[...id].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 0) % GROUNDS.length];

interface Props {
  lang: Lang;
  project: ProjectView;
  me: Person | null;
  people: Person[]; // the project's assignees other than me
  prefs: ProjectPrefs;
  onPrefs: (p: ProjectPrefs) => void;
  query: string;
  onQuery: (q: string) => void;
}

const toggle = <T,>(xs: T[], x: T) => (xs.includes(x) ? xs.filter((y) => y !== x) : [...xs, x]);

export function TopBar({ lang, project, me, people, prefs, onPrefs, query, onQuery }: Props) {
  const person = (id: string) => () => onPrefs({ ...prefs, people: toggle(prefs.people, id) });
  const sprint = (id: number | 'backlog') => () => onPrefs({ ...prefs, sprints: toggle(prefs.sprints, id) });
  return (
    <div className="top">
      <div className="top-group" role="group" aria-label={t(lang, 'people_filter')}>
        <Avatar initials={me ? short(me.displayName) : '?'} label={t(lang, 'me')} color={ground(me?.accountId ?? '')}
          selected={prefs.people.includes('__me__')} onClick={person('__me__')} />
        {people.map((p) => (
          <Avatar key={p.accountId} initials={short(p.displayName)} label={p.displayName} color={ground(p.accountId)}
            selected={prefs.people.includes(p.accountId)} onClick={person(p.accountId)} />
        ))}
        <Avatar initials="?" label={t(lang, 'unassigned')} color="var(--surface-2)"
          selected={prefs.people.includes('__unassigned__')} onClick={person('__unassigned__')} />
      </div>
      {project.hasSprints && (
        <div className="top-group" role="group" aria-label={t(lang, 'sprint_filter')}>
          {project.sprints.map((s) => (
            <Chip key={s.id} selected={prefs.sprints.includes(s.id)} onClick={sprint(s.id)}>{s.name}</Chip>
          ))}
          <Chip selected={prefs.sprints.includes('backlog')} onClick={sprint('backlog')}>{t(lang, 'backlog')}</Chip>
        </div>
      )}
      <label className="search">
        <Icon name="Search" />
        <input type="search" className="body" value={query} placeholder={t(lang, 'search')} aria-label={t(lang, 'search')}
          onChange={(e) => onQuery(e.target.value)} />
      </label>
    </div>
  );
}
