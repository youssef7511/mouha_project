use packflow_core::{auth::LocalAuth, DbWorker, DomainError, Mutation, Query};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::Manager;

struct Session {
    auth: LocalAuth,
    resume_checked: bool,
}
struct AppState {
    session: Mutex<Session>,
    worker: DbWorker,
    service: String,
}
type Shared = Arc<AppState>;
type Result<T> = std::result::Result<T, DomainError>;
fn lock(state: &AppState) -> Result<std::sync::MutexGuard<'_, Session>> {
    state
        .session
        .lock()
        .map_err(|_| DomainError::storage("Account service needs an application restart."))
}
#[cfg(target_os = "macos")]
fn stored(state: &AppState, value: Option<&str>, read: bool) -> Result<Option<String>> {
    let entry = keyring::Entry::new(&state.service, "owner-session")
        .map_err(|_| DomainError::storage("Keychain is unavailable."))?;
    if read {
        return match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(DomainError::storage(
                "Keychain access was not available. Sign in with your password.",
            )),
        };
    }
    if let Some(v) = value {
        entry
            .set_password(v)
            .map_err(|_| DomainError::storage("Could not remember this session in Keychain."))?;
    } else {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(_) => return Err(DomainError::storage("Could not remove the Keychain entry.")),
        }
    }
    Ok(None)
}
#[cfg(not(target_os = "macos"))]
fn stored(state: &AppState, value: Option<&str>, _read: bool) -> Result<Option<String>> {
    let _ = &state.service;
    if value.is_some() {
        return Err(DomainError::storage(
            "Remember me is available in the Mac build. You are signed in for this session.",
        ));
    }
    Ok(None)
}
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| DomainError::storage("Local operation failed."))?
}
#[tauri::command]
async fn query(state: tauri::State<'_, Shared>, request: Query) -> Result<Value> {
    let state = state.inner().clone();
    blocking(move || {
        let session = lock(&state)?;
        session.auth.require()?;
        state.worker.query(request)
    })
    .await
}
#[tauri::command]
async fn mutate(state: tauri::State<'_, Shared>, mut request: Mutation) -> Result<Value> {
    let state = state.inner().clone();
    blocking(move || {
        let session = lock(&state)?;
        session.auth.require()?;
        request.actor_id = packflow_core::models::MANAGER_ID.into();
        state.worker.mutate(request)
    })
    .await
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AuthRequest {
    Status,
    Setup {
        username: String,
        display_name: String,
        password: String,
    },
    Login {
        username: String,
        password: String,
        remember: bool,
    },
    Logout,
    ChangePassword {
        current: String,
        password: String,
    },
    Recover {
        code: String,
        password: String,
    },
}
#[tauri::command]
async fn auth(state: tauri::State<'_, Shared>, request: AuthRequest) -> Result<Value> {
    let state = state.inner().clone();
    blocking(move || {
        let mut session = lock(&state)?;
        let mut recovery = None;
        let mut warning = None;
        match request {
            AuthRequest::Status => {
                if !session.resume_checked {
                    session.resume_checked = true;
                    if let Ok(Some(token)) = stored(&state, None, true) {
                        let _ = session.auth.resume(&token);
                    }
                }
            }
            AuthRequest::Setup {
                username,
                display_name,
                password,
            } => {
                recovery = Some(session.auth.setup(&username, &display_name, &password)?);
            }
            AuthRequest::Login {
                username,
                password,
                remember,
            } => {
                session.auth.login(&username, &password)?;
                session.resume_checked = true;
                let _ = stored(&state, None, false);
                if remember {
                    let token = session.auth.remember()?;
                    if let Err(e) = stored(&state, Some(&token), false) {
                        warning = Some(e.message);
                    }
                }
            }
            AuthRequest::Logout => {
                session.auth.logout()?;
                session.resume_checked = true;
                let _ = stored(&state, None, false);
            }
            AuthRequest::ChangePassword { current, password } => {
                session.auth.change_password(&current, &password)?;
                session.resume_checked = true;
                let _ = stored(&state, None, false);
            }
            AuthRequest::Recover { code, password } => {
                recovery = Some(session.auth.recover(&code, &password)?);
                session.resume_checked = true;
                let _ = stored(&state, None, false);
            }
        }
        let mut result = session.auth.status()?;
        result["recovery_code"] = json!(recovery);
        result["warning"] = json!(warning);
        Ok(result)
    })
    .await
}
#[tauri::command]
async fn workspace_backup(state: tauri::State<'_, Shared>) -> Result<Value> {
    {
        lock(&state)?.auth.require()?;
    }
    let file = rfd::AsyncFileDialog::new()
        .add_filter("PackFlow backup", &["pfbackup"])
        .set_file_name(format!(
            "PackFlow-{}.pfbackup",
            packflow_core::service::business_today()
        ))
        .save_file()
        .await;
    let Some(file) = file else {
        return Ok(json!({"cancelled":true}));
    };
    let path = file.path().to_owned();
    let state = state.inner().clone();
    blocking(move || {
        let session = lock(&state)?;
        session.auth.require()?;
        state.worker.backup(path)
    })
    .await
}
#[tauri::command]
async fn workspace_restore(state: tauri::State<'_, Shared>, confirmation: String) -> Result<Value> {
    {
        lock(&state)?.auth.require()?;
    }
    if confirmation != "RESTORE" {
        return Err(DomainError::validation(
            "confirmation",
            "Type RESTORE to replace workspace data.",
        ));
    }
    let file = rfd::AsyncFileDialog::new()
        .add_filter("PackFlow backup", &["pfbackup"])
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(json!({"cancelled":true}));
    };
    let path = file.path().to_owned();
    let state = state.inner().clone();
    blocking(move || {
        let mut session = lock(&state)?;
        session.auth.require()?;
        let result = state.worker.restore(path)?;
        // Invalidate the remembered token as well as the process session.
        session.auth.logout()?;
        session.resume_checked = true;
        let _ = stored(&state, None, false);
        Ok(result)
    })
    .await
}

#[tauri::command]
async fn workspace_export(
    state: tauri::State<'_, Shared>,
    request: packflow_core::excel::ExportRequest,
) -> Result<Value> {
    {
        lock(&state)?.auth.require()?;
    }
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Excel workbook", &["xlsx"])
        .set_file_name(format!("PackFlow-{}.xlsx", chrono_filename()))
        .save_file()
        .await;
    let Some(file) = file else {
        return Ok(json!({"cancelled":true}));
    };
    let path = file.path().to_owned();
    let state = state.inner().clone();
    blocking(move || {
        let session = lock(&state)?;
        session.auth.require()?;
        let excel = state.worker.export_excel(request)?;
        excel.save(&path)?;
        Ok(json!({"path":path.to_string_lossy(),"cancelled":false}))
    })
    .await
}
fn chrono_filename() -> String {
    format!(
        "{}-{}",
        packflow_core::service::business_today(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let service = if cfg!(debug_assertions) {
                format!("{}.dev", app.config().identifier)
            } else {
                app.config().identifier.clone()
            };
            let root = if cfg!(debug_assertions) {
                app.path().app_data_dir()?.with_file_name(&service)
            } else {
                app.path().app_data_dir()?
            };
            let worker = DbWorker::start(root.join("data/packflow.sqlite"))?;
            let auth = LocalAuth::open(&root.join("account.sqlite"))?;
            app.manage(Arc::new(AppState {
                session: Mutex::new(Session {
                    auth,
                    resume_checked: false,
                }),
                worker,
                service,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            query,
            mutate,
            auth,
            workspace_backup,
            workspace_restore,
            workspace_export
        ])
        .run(tauri::generate_context!())
        .expect("PackFlow could not start. Business data was not replaced.");
}
