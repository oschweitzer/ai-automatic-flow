import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Snapshot } from './types';

export function useSnapshot(): { snapshot: Snapshot | null; refresh(): Promise<void> } {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  useEffect(() => {
    let off: (() => void) | undefined;
    let gone = false;
    listen<Snapshot>('snapshot', (e) => setSnapshot(e.payload)).then((f) => (gone ? f() : (off = f)));
    invoke<Snapshot>('get_snapshot').then((s) => !gone && setSnapshot((cur) => cur ?? s));
    return () => { gone = true; off?.(); };
  }, []);
  const refresh = useCallback(() => invoke<void>('refresh'), []);
  return { snapshot, refresh };
}
