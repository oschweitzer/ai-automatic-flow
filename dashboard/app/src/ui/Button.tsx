import type { MouseEventHandler, ReactNode } from 'react';

export interface ButtonProps {
  variant?: 'primary' | 'ghost';
  block?: boolean;
  href?: string;
  disabled?: boolean;
  onClick?: MouseEventHandler;
  children: ReactNode;
}

export function Button({ variant = 'ghost', block, href, disabled, onClick, children }: ButtonProps) {
  const cls = `tf-btn tf-btn-${variant}${block ? ' tf-btn-block' : ''}`;
  if (href) return <a className={cls} href={href} onClick={onClick}>{children}</a>;
  return <button type="button" className={cls} disabled={disabled} onClick={onClick}>{children}</button>;
}
