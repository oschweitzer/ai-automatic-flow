import { useState, type FormEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '../ui';
import { t } from '../i18n';
import type { Settings as SettingsData, SettingsView } from '../types';

interface Props { initial: SettingsView; onSaved: (s: SettingsView) => void; onCancel?: () => void }

export function Settings({ initial, onSaved, onCancel }: Props) {
  const [roots, setRoots] = useState(initial.roots.join('\n'));
  const [jiraUrl, setJiraUrl] = useState(initial.jiraUrl);
  const [jiraEmail, setJiraEmail] = useState(initial.jiraEmail);
  const [token, setToken] = useState(''); // never pre-filled: the token never comes back from the backend
  const [language, setLanguage] = useState(initial.language);
  const [error, setError] = useState<string | null>(null);
  const lang = initial.language;

  const save = async (e: FormEvent) => {
    e.preventDefault();
    const settings: SettingsData = {
      roots: roots.split('\n').map((r) => r.trim()).filter(Boolean),
      jiraUrl: jiraUrl.trim(), jiraEmail: jiraEmail.trim(), language,
    };
    try {
      // An empty token leaves the stored one alone.
      await invoke('save_settings', { settings, token: token.trim() || null });
      // The stored token is keyed by email: a new email without a new token has none.
      onSaved({ ...settings, hasToken: !!token.trim() || (initial.hasToken && settings.jiraEmail === initial.jiraEmail) });
    } catch (err) {
      setError(t(lang, 'settings_saved_error', { error: String(err) }));
    }
  };

  return (
    <div className="screen">
      <header className="screen-head"><h1 className="page-title">{t(lang, 'settings')}</h1></header>
      <form className="settings" onSubmit={save}>
        <div className="field">
          <label htmlFor="roots">{t(lang, 'settings_roots')}</label>
          <textarea id="roots" rows={4} value={roots} onChange={(e) => setRoots(e.target.value)} aria-describedby="roots-hint" spellCheck={false} />
          <small id="roots-hint" className="tf-sub">{t(lang, 'settings_roots_hint')}</small>
        </div>
        <div className="field">
          <label htmlFor="jira-url">{t(lang, 'settings_jira_url')}</label>
          <input id="jira-url" type="url" value={jiraUrl} onChange={(e) => setJiraUrl(e.target.value)} placeholder="https://example.atlassian.net" />
        </div>
        <div className="field">
          <label htmlFor="jira-email">{t(lang, 'settings_jira_email')}</label>
          <input id="jira-email" type="email" value={jiraEmail} onChange={(e) => setJiraEmail(e.target.value)} />
        </div>
        <div className="field">
          <label htmlFor="token">{t(lang, 'settings_token')}</label>
          <input id="token" type="password" value={token} onChange={(e) => setToken(e.target.value)} autoComplete="off"
            aria-describedby={initial.hasToken ? 'token-saved' : undefined} />
          {initial.hasToken && <small id="token-saved" className="tf-sub">{t(lang, 'settings_token_saved')}</small>}
        </div>
        <div className="field">
          <label htmlFor="language">{t(lang, 'settings_language')}</label>
          <select id="language" value={language} onChange={(e) => setLanguage(e.target.value as SettingsData['language'])}>
            <option value="fr">Français</option>
            <option value="en">English</option>
          </select>
        </div>
        {error && <p className="form-error" role="alert">{error}</p>}
        <div className="form-actions">
          <button type="submit" className="tf-btn tf-btn-primary">{t(lang, 'settings_save')}</button>
          {onCancel && <Button onClick={onCancel}>{t(lang, 'settings_cancel')}</Button>}
        </div>
      </form>
    </div>
  );
}
