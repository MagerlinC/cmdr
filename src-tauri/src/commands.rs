use crate::controller::StackController;
use crate::state::LayerStatus;
use std::sync::Arc;
use std::path::PathBuf;
use tauri::State;
use tokio::sync::Mutex;
use crate::load_config_into;

pub type AppState = Arc<Mutex<Option<StackController>>>;

#[tauri::command]
pub async fn get_layers(state: State<'_, AppState>) -> Result<Vec<LayerStatus>, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    Ok(controller.all_statuses().await)
}

#[tauri::command]
pub async fn start_layer(
    state: State<'_, AppState>,
    layer_name: String,
    runtime_name: Option<String>,
) -> Result<LayerStatus, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    let layer = controller
        .find_layer(&layer_name)
        .ok_or_else(|| format!("Layer '{}' not found", layer_name))?;

    let layer = Arc::clone(layer);
    drop(lock);

    layer.start(runtime_name.as_deref()).await?;
    Ok(layer.status().await)
}

#[tauri::command]
pub async fn stop_layer(
    state: State<'_, AppState>,
    layer_name: String,
) -> Result<LayerStatus, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    let layer = controller
        .find_layer(&layer_name)
        .ok_or_else(|| format!("Layer '{}' not found", layer_name))?;

    let layer = Arc::clone(layer);
    drop(lock);

    layer.stop().await?;
    Ok(layer.status().await)
}

#[tauri::command]
pub async fn restart_layer(
    state: State<'_, AppState>,
    layer_name: String,
) -> Result<LayerStatus, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    let layer = controller
        .find_layer(&layer_name)
        .ok_or_else(|| format!("Layer '{}' not found", layer_name))?;

    let layer = Arc::clone(layer);
    drop(lock);

    layer.restart().await?;
    Ok(layer.status().await)
}

#[tauri::command]
pub async fn build_layer(
    state: State<'_, AppState>,
    layer_name: String,
) -> Result<LayerStatus, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    let layer = controller
        .find_layer(&layer_name)
        .ok_or_else(|| format!("Layer '{}' not found", layer_name))?;

    let layer = Arc::clone(layer);
    drop(lock);

    layer.build().await?;
    Ok(layer.status().await)
}

#[tauri::command]
pub async fn switch_runtime(
    state: State<'_, AppState>,
    layer_name: String,
    runtime_name: String,
) -> Result<LayerStatus, String> {
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    let layer = controller
        .find_layer(&layer_name)
        .ok_or_else(|| format!("Layer '{}' not found", layer_name))?;

    let layer = Arc::clone(layer);
    drop(lock);

    layer.switch_runtime(&runtime_name).await?;
    Ok(layer.status().await)
}

fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/cmdr")
}

#[tauri::command]
pub async fn open_config_dir() -> Result<String, String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    std::process::Command::new("open")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("Failed to open directory: {}", e))?;

    Ok(dir.display().to_string())
}

const SAMPLE_CONFIG: &str = r#"layers:
  - name: Database
    runtimes:
      - name: Docker
        type: docker
        cwd: /path/to/project
        up: docker compose up -d postgres
        down: docker compose stop postgres

  - name: API
    runtimes:
      - name: Docker
        type: docker
        cwd: /path/to/project
        build: docker compose build api
        up: docker compose up -d api
        down: docker compose stop api
      - name: Terminal
        type: terminal
        cwd: /path/to/project/api
        up: pnpm dev

  - name: Frontend
    runtimes:
      - name: Docker
        type: docker
        cwd: /path/to/project
        up: docker compose up -d frontend
        down: docker compose stop frontend
      - name: Terminal
        type: terminal
        cwd: /path/to/project/frontend
        up: pnpm dev
"#;

#[tauri::command]
pub async fn create_sample_config() -> Result<String, String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let path = dir.join("cmdr.yaml");
    if path.exists() {
        return Err("cmdr.yaml already exists — not overwriting".into());
    }

    std::fs::write(&path, SAMPLE_CONFIG)
        .map_err(|e| format!("Failed to write sample config: {}", e))?;

    // Open the folder so the user can see the file
    std::process::Command::new("open")
        .arg(&dir)
        .spawn()
        .ok();

    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn reload_config(state: State<'_, AppState>) -> Result<Vec<LayerStatus>, String> {
    load_config_into(&state).await?;
    let lock = state.lock().await;
    let controller = lock
        .as_ref()
        .ok_or_else(|| "Stack not initialized".to_string())?;
    Ok(controller.all_statuses().await)
}
