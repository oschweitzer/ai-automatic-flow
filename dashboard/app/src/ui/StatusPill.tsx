import type { ReactNode } from 'react';

export type Tone = 'you' | 'client' | 'ok' | 'bad' | 'run' | 'mute';

export interface StatusPillProps {
  tone?: Tone;
  children: ReactNode;
}

export function StatusPill({ tone = 'mute', children }: StatusPillProps) {
  return <span className={`tf-pill tf-pill-${tone}`}>{children}</span>;
}
