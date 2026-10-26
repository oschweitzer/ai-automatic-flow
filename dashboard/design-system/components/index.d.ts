import type { ReactNode, MouseEventHandler } from 'react';

export type Tone = 'you' | 'client' | 'ok' | 'bad' | 'run' | 'mute';
export type IconName = 'chevron' | 'arrow' | 'folder' | 'grid' | 'check' | 'alert';

export interface ButtonProps {
  /** primary: lavender fill, one per area. ghost: bordered, for links and secondary actions. */
  variant?: 'primary' | 'ghost';
  /** Full width of its container. */
  block?: boolean;
  /** Renders an <a> instead of a <button>. */
  href?: string;
  disabled?: boolean;
  onClick?: MouseEventHandler;
  children: ReactNode;
}

export interface ChipProps {
  /** Sprint or backlog filter state. */
  selected?: boolean;
  onClick?: MouseEventHandler;
  children: ReactNode;
}

export interface AvatarProps {
  /** Two letters. */
  initials: string;
  /** Full name, used as tooltip and accessible name. */
  label: string;
  /** Person colour from Jira or a hash of the account id. White text must stay readable on it. */
  color?: string;
  selected?: boolean;
  onClick?: MouseEventHandler;
}

export interface StatusPillProps {
  /** you: waits on the developer. client: waits on the client. ok, bad: CI and end states. run: AI model or stage. mute: Jira status. */
  tone?: Tone;
  children: ReactNode;
}

export interface StageTrackProps {
  /** 1-based index of the running or last stage. */
  current: number;
  /** Number of stages in the flow. */
  total?: number;
  /** Stage name shown under the track, like ci-gate. */
  stage?: string;
  /** Extra caption, like "depuis 12 min". */
  detail?: string;
  /** The current stage is blocked or failed. */
  failed?: boolean;
}

export interface Step {
  name: string;
  state?: 'done' | 'now' | 'fail' | 'todo';
  /** Outcome and time, like "approuvé · 10:31". */
  note?: string;
}

export interface StepListProps {
  steps: Step[];
}

export interface SectionProps {
  title: ReactNode;
  count?: number;
  defaultOpen?: boolean;
  /** Usually a <table>. Rows that open the panel take className "tf-clickable". */
  children: ReactNode;
}

export interface NavItemProps {
  label: string;
  /** Repositories under a project name. */
  sub?: string;
  /** Tickets waiting on the developer. Hidden at 0. */
  count?: number;
  icon?: IconName;
  selected?: boolean;
  onClick?: MouseEventHandler;
}

export interface ActionItem {
  title: string;
  meta?: string;
  icon?: IconName;
  onClick?: MouseEventHandler;
}

export interface ActionCardProps {
  title: string;
  text?: string;
  items?: ActionItem[];
  actionLabel?: string;
  onAction?: MouseEventHandler;
}

export declare function Button(props: ButtonProps): JSX.Element;
export declare function Chip(props: ChipProps): JSX.Element;
export declare function Avatar(props: AvatarProps): JSX.Element;
export declare function StatusPill(props: StatusPillProps): JSX.Element;
export declare function StageTrack(props: StageTrackProps): JSX.Element;
export declare function StepList(props: StepListProps): JSX.Element;
export declare function Section(props: SectionProps): JSX.Element;
export declare function NavItem(props: NavItemProps): JSX.Element;
export declare function ActionCard(props: ActionCardProps): JSX.Element;
