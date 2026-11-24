pub mod classify;
pub mod forge;
pub mod jira;
pub mod repos;
pub mod runs;
pub mod settings;
pub mod snapshot;
pub mod sync;

use settings::Settings;
use snapshot::Snapshot;
use sync::AppState;
use tauri::{AppHandle, Manager, State};

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
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.inner.lock().await.settings.clone())
}

#[tauri::command]
async fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let st = app.state::<AppState>();
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
