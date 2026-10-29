import type { MouseEventHandler, ReactNode } from 'react';

export interface ChipProps {
  selected?: boolean;
  onClick?: MouseEventHandler;
  children: ReactNode;
}

export function Chip({ selected = false, onClick, children }: ChipProps) {
  return <button type="button" className="tf-chip" aria-pressed={selected} onClick={onClick}>{children}</button>;
}
