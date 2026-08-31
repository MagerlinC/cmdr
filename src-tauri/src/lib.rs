mod commands;
mod config;
mod controller;
pub mod env;
mod runtime;
mod state;

use commands::AppState;
use config::Config;
use controller::StackController;
use std::path::PathBuf;
use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn find_config() -> Option<PathBuf> {
    // Look for cmdr.yaml in the current directory, then in common locations
    let candidates = vec![
        std::env::current_dir().ok().map(|d| d.join("cmdr.yaml")),
        std::env::current_dir().ok().map(|d| d.join("cmdr.yml")),
        dirs::home_dir().map(|d| d.join(".config/cmdr/cmdr.yaml")),
        dirs::home_dir().map(|d| d.join(".config/cmdr/cmdr.yml")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

pub async fn load_config_into(state: &AppState) -> Result<(), String> {
    let config_path = find_config().ok_or_else(|| "No cmdr.yaml found".to_string())?;
    log::info!("Loading config from: {}", config_path.display());

    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    let config = Config::load(&config_path)?;
    let controller = StackController::from_config(config, config_dir);
    controller.discover_all().await;
    *state.lock().await = Some(controller);
    log::info!("Stack controller initialized");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(None::<StackController>)) as AppState)
        .invoke_handler(tauri::generate_handler![
            commands::get_layers,
            commands::start_layer,
            commands::stop_layer,
            commands::restart_layer,
            commands::build_layer,
            commands::switch_runtime,
            commands::open_config_dir,
            commands::create_sample_config,
            commands::reload_config,
        ])
        .setup(|_app| {
            let state: AppState =
                Arc::clone(&_app.state::<Arc<Mutex<Option<StackController>>>>());

            tauri::async_runtime::spawn(async move {
                if let Err(e) = load_config_into(&state).await {
                    log::warn!("Initial config load: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
