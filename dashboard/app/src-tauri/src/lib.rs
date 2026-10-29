pub mod classify;
pub mod forge;
pub mod jira;
pub mod repos;
pub mod runs;
pub mod settings;
pub mod snapshot;
pub mod sync;

use serde::Serialize;
use settings::Settings;
use snapshot::Snapshot;
use sync::AppState;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    has_token: bool,
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(state.inner.lock().await.snapshot.clone())
}

#[tauri::command]
async fn refresh(app: AppHandle) -> Result<(), String> {
    sync::rescan(&app).await;
    sync::fetch(&app).await;
    Ok(())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    let settings = state.inner.lock().await.settings.clone();
    let email = settings.jira_email.clone();
    let has_token = !email.is_empty()
        && tauri::async_runtime::spawn_blocking(move || settings::token(&email).is_some()).await.unwrap_or(false);
    Ok(SettingsView { settings, has_token })
}

// The token goes to the keychain only. It is never stored elsewhere, returned or put in an error.
#[tauri::command]
async fn save_settings(app: AppHandle, settings: Settings, token: Option<String>) -> Result<(), String> {
    let st = app.state::<AppState>();
    let token = token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    if token.is_some() && settings.jira_email.trim().is_empty() {
        return Err("settings: a Jira email is needed to store the token".into());
    }
    // Keychain first, then disk, then memory: a failure leaves disk and memory as they were.
    if let Some(token) = token {
        let email = settings.jira_email.clone();
        tauri::async_runtime::spawn_blocking(move || settings::set_token(&email, &token))
            .await
            .map_err(|_| "settings: keychain unavailable".to_string())??;
    }
    settings::save(&st.config_dir, &settings)?;
    st.inner.lock().await.settings = settings;
    // The dialog does not wait for the network: the result comes as a `snapshot` event.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        sync::rescan(&app).await;
        sync::fetch(&app).await;
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState::new(app.path().app_config_dir()?));
            sync::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_snapshot, refresh, get_settings, save_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
