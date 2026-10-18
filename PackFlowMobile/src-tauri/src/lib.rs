use packflow_mobile_core::{
    error::{DomainError, Result},
    models::{Mutation, Query},
    Engine,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
type Core = Arc<Mutex<Engine>>;
async fn operation<T: Send + 'static>(
    state: &tauri::State<'_, Core>,
    f: impl FnOnce(&mut Engine) -> Result<T> + Send + 'static,
) -> Result<T> {
    let core = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = core
            .lock()
            .map_err(|_| DomainError::storage("Restart the app to reopen this workspace."))?;
        f(&mut engine)
    })
    .await
    .map_err(|_| DomainError::storage("The local operation failed."))?
}
#[tauri::command]
async fn mobile_query(
    _app: tauri::AppHandle,
    state: tauri::State<'_, Core>,
    request: Query,
) -> Result<Value> {
    #[cfg(debug_assertions)]
    let smoke =
        matches!(&request, Query::Snapshot) && std::env::var_os("PACKFLOW_MOBILE_SMOKE").is_some();
    let result = operation(&state, move |e| e.query(request)).await?;
    #[cfg(debug_assertions)]
    if smoke {
        let counts = serde_json::json!({
            "clients": result["clients"].as_array().map(Vec::len),
            "orders": result["orders"].as_array().map(Vec::len),
            "payments": result["payments"].as_array().map(Vec::len),
            "templates": result["templates"].as_array().map(Vec::len)
        });
        let root = _app
            .path()
            .app_data_dir()
            .map_err(|_| DomainError::storage("Workspace path unavailable."))?;
        std::fs::write(root.join("mobile-smoke-ok.json"), counts.to_string())?;
    }
    Ok(result)
}
#[tauri::command]
async fn mobile_mutate(state: tauri::State<'_, Core>, request: Mutation) -> Result<Value> {
    operation(&state, move |e| e.mutate(request)).await
}
#[tauri::command]
async fn mobile_backup(state: tauri::State<'_, Core>) -> Result<Value> {
    operation(&state, |e| e.save_backup()).await
}
#[tauri::command]
async fn mobile_restore(
    state: tauri::State<'_, Core>,
    data: String,
    confirmation: String,
) -> Result<Value> {
    operation(&state, move |e| e.restore_encoded(&data, &confirmation)).await
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let backups = app.path().document_dir()?.join("PackFlow Mobile Backups");
            let core = Engine::open(&root.join("mobile.sqlite"), &backups)?;
            app.manage(Arc::new(Mutex::new(core)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            mobile_query,
            mobile_mutate,
            mobile_backup,
            mobile_restore
        ])
        .run(tauri::generate_context!())
        .expect("Could not open the mobile workspace. Existing data was not replaced.");
}
