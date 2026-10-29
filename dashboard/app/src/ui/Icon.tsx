import { ArrowUpRight, ChevronRight, Folder, LayoutGrid, RefreshCw, Search, Settings, SquareCheck, TriangleAlert, X } from 'lucide-react';

const icons = { ChevronRight, ArrowUpRight, Folder, LayoutGrid, SquareCheck, TriangleAlert, Search, Settings, RefreshCw, X };

export type LucideName = keyof typeof icons;
export type IconName = 'chevron' | 'arrow' | 'folder' | 'grid' | 'check' | 'alert';

/** Maps the design-system icon names to their lucide icon. */
export const lucideOf: Record<IconName, LucideName> = {
  chevron: 'ChevronRight', arrow: 'ArrowUpRight', folder: 'Folder', grid: 'LayoutGrid', check: 'SquareCheck', alert: 'TriangleAlert',
};

export function Icon({ name }: { name: LucideName }) {
  const C = icons[name];
  return <C className="tf-icon" size={16} strokeWidth={1.6} aria-hidden="true" />;
}
