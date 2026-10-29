import { useEffect, useReducer, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSnapshot } from './useSnapshot';
import type { SettingsView } from './types';
import { Sidebar, type View } from './screens/Sidebar';
import { Overview } from './screens/Overview';
import { ProjectScreen } from './screens/ProjectScreen';
import { Settings } from './screens/Settings';
import './app.css';

const NO_SETTINGS: SettingsView = { roots: [], jiraUrl: '', jiraEmail: '', language: 'fr', hasToken: false };

export default function App() {
  const { snapshot, refresh } = useSnapshot();
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [view, setView] = useState<View>({ kind: 'all' });
  const [loadError, setLoadError] = useState<string>();
  // Bumped when a project's filters change, so the sidebar and overview counts follow.
  const [, prefsChanged] = useReducer((n: number) => n + 1, 0);

  useEffect(() => {
    invoke<SettingsView>('get_settings')
      .catch((e) => { setLoadError(String(e)); return NO_SETTINGS; })
      .then((s) => { setSettings(s); if (!s.roots.length) setView({ kind: 'settings' }); });
  }, []);

  if (!settings) return null;
  const lang = settings.language;
  const project = view.kind === 'project' ? snapshot?.projects.find((p) => p.key === view.key) : undefined;

  return (
    <div className="app">
      <Sidebar lang={lang} snapshot={snapshot} view={view} onView={setView} onRefresh={() => refresh().catch(console.error)} />
      <main className="main">
        {view.kind === 'settings' ? (
          <Settings initial={settings} loadError={loadError}
            onSaved={(s) => { setSettings(s); if (s.roots.length) setView({ kind: 'all' }); }}
            onCancel={settings.roots.length ? () => setView({ kind: 'all' }) : undefined} />
        ) : !snapshot ? null : project ? (
          <ProjectScreen key={project.key} lang={lang} snapshot={snapshot} project={project} onPrefs={prefsChanged} />
        ) : (
          <Overview lang={lang} snapshot={snapshot} onOpen={(key) => setView({ kind: 'project', key })} />
        )}
      </main>
    </div>
  );
}
