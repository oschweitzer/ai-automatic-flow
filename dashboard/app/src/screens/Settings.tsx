import { useState, type FormEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '../ui';
import { t } from '../i18n';
import type { Settings as SettingsData } from '../types';

interface Props { initial: SettingsData; loadError?: string; onSaved: (s: SettingsData) => void; onCancel?: () => void }

export function Settings({ initial, loadError, onSaved, onCancel }: Props) {
  const [roots, setRoots] = useState(initial.roots.join('\n'));
  const [language, setLanguage] = useState(initial.language);
  const [error, setError] = useState<string | null>(null);
  const lang = initial.language;

  const save = async (e: FormEvent) => {
    e.preventDefault();
    if (loadError) return; // saving defaults over unread settings would erase them
    const settings: SettingsData = {
      roots: roots.split('\n').map((r) => r.trim()).filter(Boolean),
      language,
    };
    try {
      await invoke('save_settings', { settings });
      onSaved(settings);
    } catch (err) {
      setError(t(lang, 'settings_saved_error', { error: String(err) }));
    }
  };

  return (
    <div className="screen">
      <header className="screen-head"><h1 className="page-title">{t(lang, 'settings')}</h1></header>
      <form className="settings" onSubmit={save}>
        <div className="field">
          <label className="label" htmlFor="roots">{t(lang, 'settings_roots')}</label>
          <textarea id="roots" className="code" rows={4} value={roots} onChange={(e) => setRoots(e.target.value)} aria-describedby="roots-hint" spellCheck={false} />
          <small id="roots-hint" className="tf-sub">{t(lang, 'settings_roots_hint')}</small>
        </div>
        <p className="tf-sub">{t(lang, 'settings_jira_acli')}</p>
        <div className="field">
          <label className="label" htmlFor="language">{t(lang, 'settings_language')}</label>
          <select id="language" className="body" value={language} onChange={(e) => setLanguage(e.target.value as SettingsData['language'])}>
            <option value="fr">Français</option>
            <option value="en">English</option>
          </select>
        </div>
        {(loadError || error) && (
          <p className="form-error" role="alert">{loadError ? t(lang, 'settings_load_error', { error: loadError }) : error}</p>
        )}
        <div className="form-actions">
          <button type="submit" className="tf-btn tf-btn-primary" disabled={!!loadError}>{t(lang, 'settings_save')}</button>
          {onCancel && <Button onClick={onCancel}>{t(lang, 'settings_cancel')}</Button>}
        </div>
      </form>
    </div>
  );
}
