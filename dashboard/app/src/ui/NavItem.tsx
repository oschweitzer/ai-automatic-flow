import type { MouseEventHandler } from 'react';
import { Icon, lucideOf, type IconName } from './Icon';

export interface NavItemProps {
  label: string;
  sub?: string;
  count?: number;
  icon?: IconName;
  selected?: boolean;
  onClick?: MouseEventHandler;
}

export function NavItem({ label, sub, count, icon, selected, onClick }: NavItemProps) {
  return (
    <button type="button" className="tf-nav" aria-current={selected ? 'page' : undefined} onClick={onClick}>
      {icon && <Icon name={lucideOf[icon]} />}
      <span>
        {label}
        {sub && <span className="tf-nav-sub">{sub}</span>}
      </span>
      {count ? <span className="tf-nav-count">{count}</span> : null}
    </button>
  );
}
