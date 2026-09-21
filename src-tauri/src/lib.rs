mod macos;
mod model;
mod storage;

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use macos::{MacAutomation, PermissionState};
use model::{Action, MacroDocument, PlaybackProgress, RecordingSettings};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub struct AppState {
    automation: Arc<MacAutomation>,
    documents: Mutex<Vec<MacroDocument>>,
    emergency_stop_available: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MacroLibrary {
    documents: Vec<MacroDocument>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    emergency_stop_available: bool,
}

#[tauri::command]
fn runtime_status(state: State<'_, AppState>) -> RuntimeStatus {
    RuntimeStatus {
        emergency_stop_available: state.emergency_stop_available.load(Ordering::SeqCst),
    }
}

#[tauri::command]
fn permission_status(state: State<'_, AppState>) -> PermissionState {
    state.automation.permission_status()
}

#[tauri::command]
fn request_accessibility(state: State<'_, AppState>) -> bool {
    state.automation.request_accessibility()
}

#[tauri::command]
fn request_screen_recording(state: State<'_, AppState>) -> bool {
    state.automation.request_screen_recording()
}

#[tauri::command]
fn request_input_monitoring(state: State<'_, AppState>) -> bool {
    state.automation.request_input_monitoring()
}

#[tauri::command]
fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: RecordingSettings,
) -> Result<(), String> {
    state.automation.ensure_event_listener()?;
    state.automation.start_recording(settings)?;
    let _ = app.emit("recording-state", true);
    Ok(())
}

#[tauri::command]
fn stop_recording(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Action>, String> {
    let actions = state.automation.stop_recording()?;
    let _ = app.emit("recording-state", false);
    let _ = app.emit("recording-actions", &actions);
    Ok(actions)
}

#[tauri::command]
fn playback(
    app: AppHandle,
    state: State<'_, AppState>,
    actions: Vec<Action>,
    start_at: Option<usize>,
    speed: f64,
    pointer_hz: Option<u16>,
) -> Result<(), String> {
    if actions.is_empty() {
        return Err("This macro has no actions to play.".into());
    }
    let start = start_at.unwrap_or(0);
    if start >= actions.len() {
        return Err("The requested playback start is outside this macro.".into());
    }
    if !speed.is_finite() || speed <= 0.0 {
        return Err("Playback speed must be a positive number.".into());
    }
    state.automation.begin_playback()?;
    let automation = Arc::clone(&state.automation);
    let playback_app = app.clone();
    let _ = app.emit("playback-state", true);
    std::thread::spawn(move || {
        let result = automation.play(
            &actions,
            start,
            speed,
            pointer_hz.unwrap_or(120),
            |progress: PlaybackProgress| {
                let _ = playback_app.emit("playback-progress", progress);
            },
        );
        if let Err(error) = result {
            let _ = playback_app.emit("playback-error", error);
        }
        automation.complete_playback();
        let _ = playback_app.emit("playback-state", false);
    });
    Ok(())
}

#[tauri::command]
fn stop_playback(app: AppHandle, state: State<'_, AppState>) {
    state.automation.stop_playback();
    let _ = app.emit("playback-state", false);
}

#[tauri::command]
fn save_macro(
    state: State<'_, AppState>,
    document: MacroDocument,
) -> Result<MacroDocument, String> {
    storage::save(&document)?;
    let mut docs = state
        .documents
        .lock()
        .map_err(|_| "Macro library is busy")?;
    if let Some(existing) = docs.iter_mut().find(|item| item.id == document.id) {
        *existing = document.clone();
    } else {
        docs.push(document.clone());
    }
    Ok(document)
}

#[tauri::command]
fn load_macros(state: State<'_, AppState>) -> Result<MacroLibrary, String> {
    let (documents, warnings) = storage::load_all()?;
    *state
        .documents
        .lock()
        .map_err(|_| "Macro library is busy")? = documents.clone();
    Ok(MacroLibrary {
        documents,
        warnings,
    })
}

#[tauri::command]
fn delete_macro(state: State<'_, AppState>, id: String) -> Result<(), String> {
    storage::delete(&id)?;
    state
        .documents
        .lock()
        .map_err(|_| "Macro library is busy")?
        .retain(|item| item.id != id);
    Ok(())
}

#[tauri::command]
fn export_macro(document: MacroDocument, path: PathBuf) -> Result<(), String> {
    storage::export(&document, path)
}

#[tauri::command]
fn import_macro(path: PathBuf) -> Result<MacroDocument, String> {
    storage::import(path)
}

pub fn run() {
    let automation = Arc::new(MacAutomation::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            automation,
            documents: Mutex::new(Vec::new()),
            emergency_stop_available: AtomicBool::new(false),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Escape);
            let registration =
                app.global_shortcut()
                    .on_shortcut(shortcut, move |_app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            app_handle.state::<AppState>().automation.stop_playback();
                            let _ = app_handle.emit("playback-state", false);
                        }
                    });
            match registration {
                Ok(()) => app
                    .state::<AppState>()
                    .emergency_stop_available
                    .store(true, Ordering::SeqCst),
                Err(error) => eprintln!("Could not register emergency stop: {error}"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            permission_status,
            runtime_status,
            request_accessibility,
            request_input_monitoring,
            request_screen_recording,
            start_recording,
            stop_recording,
            playback,
            stop_playback,
            save_macro,
            load_macros,
            delete_macro,
            export_macro,
            import_macro
        ])
        .run(tauri::generate_context!())
        .expect("error while running BetterMacro");
}
