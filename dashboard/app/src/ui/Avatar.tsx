import type { MouseEventHandler } from 'react';

export interface AvatarProps {
  initials: string;
  label: string;
  color?: string;
  selected?: boolean;
  onClick?: MouseEventHandler;
}

export function Avatar({ initials, label, color, selected = false, onClick }: AvatarProps) {
  return (
    <button type="button" className="tf-avatar" title={label} aria-label={label} aria-pressed={selected}
      style={color ? { background: color } : undefined} onClick={onClick}>
      {initials}
    </button>
  );
}
