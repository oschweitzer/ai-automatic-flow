import type { MouseEventHandler } from 'react';
import { Button } from './Button';
import { Icon, lucideOf, type IconName } from './Icon';

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

export function ActionCard({ title, text, items = [], actionLabel, onAction }: ActionCardProps) {
  return (
    <section className="tf-action">
      <h2>{title}</h2>
      {text && <p>{text}</p>}
      {items.map((it) => (
        <button key={it.title} type="button" className="tf-action-item" onClick={it.onClick}>
          <span>{it.title}{it.meta && <small>{it.meta}</small>}</span>
          <Icon name={lucideOf[it.icon ?? 'arrow']} />
        </button>
      ))}
      {actionLabel && <Button variant="primary" block onClick={onAction}>{actionLabel}</Button>}
    </section>
  );
}
