import type { ReactNode } from 'react';
import { Icon } from './Icon';

export interface SectionProps {
  title: ReactNode;
  count?: number;
  /** Controlled: the consumer stores the state (prefs.ts). */
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}

export function Section({ title, count, open, onToggle, children }: SectionProps) {
  return (
    <details className="tf-section" open={open}>
      <summary onClick={(e) => { e.preventDefault(); onToggle(); }}>
        <span className="tf-section-chev"><Icon name="ChevronRight" /></span>
        <span>{title}</span>
        {count !== undefined && <span className="tf-section-count">{count}</span>}
      </summary>
      {children}
    </details>
  );
}
