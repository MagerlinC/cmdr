use crate::config::{Config, LayerConfig};
use crate::runtime::ManagedRuntime;
use crate::state::{LayerStatus, RuntimeState, RuntimeStatus};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Layer {
    pub config: LayerConfig,
    pub runtimes: Vec<Arc<ManagedRuntime>>,
    pub active_runtime: Mutex<Option<usize>>,
    pub transitioning: Mutex<bool>,
    pub transition_message: Mutex<Option<String>>,
}

impl Layer {
    fn new(config: LayerConfig, config_dir: &PathBuf) -> Self {
        let runtimes = config
            .runtimes
            .iter()
            .map(|rc| Arc::new(ManagedRuntime::new(rc.clone(), config_dir.clone())))
            .collect();

        Self {
            config,
            runtimes,
            active_runtime: Mutex::new(None),
            transitioning: Mutex::new(false),
            transition_message: Mutex::new(None),
        }
    }

    pub async fn status(&self) -> LayerStatus {
        let mut runtime_statuses = Vec::new();
        for rt in &self.runtimes {
            let state = *rt.state.lock().await;
            let error = rt.error.lock().await.clone();
            let pid = rt.get_pid().await;
            runtime_statuses.push(RuntimeStatus {
                name: rt.config.name.clone(),
                runtime_type: rt.config.runtime_type.clone(),
                state,
                error,
                pid,
                has_build: rt.config.build.is_some(),
            });
        }

        let active = self.active_runtime.lock().await;
        let active_name = active.map(|i| self.runtimes[i].config.name.clone());
        let transitioning = *self.transitioning.lock().await;
        let transition_message = self.transition_message.lock().await.clone();

        LayerStatus {
            name: self.config.name.clone(),
            runtimes: runtime_statuses,
            active_runtime: active_name,
            transitioning,
            transition_message,
        }
    }

    fn find_runtime_index(&self, name: &str) -> Option<usize> {
        self.runtimes
            .iter()
            .position(|r| r.config.name.eq_ignore_ascii_case(name))
    }

    pub async fn start(&self, runtime_name: Option<&str>) -> Result<(), String> {
        if *self.transitioning.lock().await {
            return Err("Layer is currently transitioning".into());
        }

        let idx = match runtime_name {
            Some(name) => self
                .find_runtime_index(name)
                .ok_or_else(|| format!("Runtime '{}' not found", name))?,
            None => {
                // Use active runtime or first runtime
                let active = self.active_runtime.lock().await;
                active.unwrap_or(0)
            }
        };

        *self.transitioning.lock().await = true;
        *self.transition_message.lock().await =
            Some(format!("Starting {}...", self.runtimes[idx].config.name));

        let result = self.runtimes[idx].start().await;

        match &result {
            Ok(()) => {
                *self.active_runtime.lock().await = Some(idx);
            }
            Err(_) => {}
        }

        *self.transitioning.lock().await = false;
        *self.transition_message.lock().await = None;

        result
    }

    pub async fn stop(&self) -> Result<(), String> {
        if *self.transitioning.lock().await {
            return Err("Layer is currently transitioning".into());
        }

        let active_idx = {
            let active = self.active_runtime.lock().await;
            match *active {
                Some(idx) => idx,
                None => return Ok(()), // Nothing to stop
            }
        };

        *self.transitioning.lock().await = true;
        *self.transition_message.lock().await = Some(format!(
            "Stopping {}...",
            self.runtimes[active_idx].config.name
        ));

        let result = self.runtimes[active_idx].stop().await;

        if result.is_ok() {
            *self.active_runtime.lock().await = None;
        }

        *self.transitioning.lock().await = false;
        *self.transition_message.lock().await = None;

        result
    }

    pub async fn restart(&self) -> Result<(), String> {
        if *self.transitioning.lock().await {
            return Err("Layer is currently transitioning".into());
        }

        let active_idx = {
            let active = self.active_runtime.lock().await;
            match *active {
                Some(idx) => idx,
                None => return Err("No active runtime to restart".into()),
            }
        };

        *self.transitioning.lock().await = true;

        // Stop
        *self.transition_message.lock().await = Some(format!(
            "Stopping {}...",
            self.runtimes[active_idx].config.name
        ));

        if let Err(e) = self.runtimes[active_idx].stop().await {
            *self.transitioning.lock().await = false;
            *self.transition_message.lock().await = None;
            return Err(format!("Failed to stop during restart: {}", e));
        }

        // Start
        *self.transition_message.lock().await = Some(format!(
            "Starting {}...",
            self.runtimes[active_idx].config.name
        ));

        let result = self.runtimes[active_idx].start().await;

        match &result {
            Ok(()) => {
                *self.active_runtime.lock().await = Some(active_idx);
            }
            Err(_) => {
                *self.active_runtime.lock().await = None;
            }
        }

        *self.transitioning.lock().await = false;
        *self.transition_message.lock().await = None;

        result
    }

    pub async fn build(&self) -> Result<(), String> {
        if *self.transitioning.lock().await {
            return Err("Layer is currently transitioning".into());
        }

        let idx = {
            let active = self.active_runtime.lock().await;
            active.unwrap_or(0)
        };

        let rt = &self.runtimes[idx];
        if rt.config.build.is_none() {
            return Err(format!(
                "Runtime '{}' has no build command configured",
                rt.config.name
            ));
        }

        *self.transitioning.lock().await = true;
        *self.transition_message.lock().await =
            Some(format!("Building {}...", rt.config.name));

        let result = rt.build().await;

        *self.transitioning.lock().await = false;
        *self.transition_message.lock().await = None;

        result
    }

    pub async fn switch_runtime(&self, target_name: &str) -> Result<(), String> {
        if *self.transitioning.lock().await {
            return Err("Layer is currently transitioning".into());
        }

        let target_idx = self
            .find_runtime_index(target_name)
            .ok_or_else(|| format!("Runtime '{}' not found", target_name))?;

        let current_idx = { *self.active_runtime.lock().await };

        // If the target is already active, nothing to do
        if current_idx == Some(target_idx) {
            return Ok(());
        }

        *self.transitioning.lock().await = true;

        // Stop current runtime if one is active
        if let Some(idx) = current_idx {
            *self.transition_message.lock().await = Some(format!(
                "Stopping {}...",
                self.runtimes[idx].config.name
            ));

            if let Err(e) = self.runtimes[idx].stop().await {
                *self.transitioning.lock().await = false;
                *self.transition_message.lock().await = None;
                return Err(format!(
                    "Failed to stop '{}': {}. Target runtime '{}' was NOT started.",
                    self.runtimes[idx].config.name, e, target_name
                ));
            }

            *self.active_runtime.lock().await = None;
        }

        // Start target runtime
        *self.transition_message.lock().await = Some(format!(
            "Starting {}...",
            self.runtimes[target_idx].config.name
        ));

        let result = self.runtimes[target_idx].start().await;

        match &result {
            Ok(()) => {
                *self.active_runtime.lock().await = Some(target_idx);
            }
            Err(_) => {}
        }

        *self.transitioning.lock().await = false;
        *self.transition_message.lock().await = None;

        result
    }

    pub async fn discover_state(&self) {
        for (idx, rt) in self.runtimes.iter().enumerate() {
            let state = rt.check_state().await;
            *rt.state.lock().await = state;
            if state == RuntimeState::Running {
                *self.active_runtime.lock().await = Some(idx);
                log::info!(
                    "Discovered '{}' runtime '{}' is running",
                    self.config.name,
                    rt.config.name
                );
            }
        }
    }
}

pub struct StackController {
    pub layers: Vec<Arc<Layer>>,
    pub config_dir: PathBuf,
}

impl StackController {
    pub fn from_config(config: Config, config_dir: PathBuf) -> Self {
        let layers = config
            .layers
            .into_iter()
            .map(|lc| Arc::new(Layer::new(lc, &config_dir)))
            .collect();

        Self { layers, config_dir }
    }

    pub async fn discover_all(&self) {
        for layer in &self.layers {
            layer.discover_state().await;
        }
    }

    pub async fn all_statuses(&self) -> Vec<LayerStatus> {
        let mut statuses = Vec::new();
        for layer in &self.layers {
            statuses.push(layer.status().await);
        }
        statuses
    }

    pub fn find_layer(&self, name: &str) -> Option<&Arc<Layer>> {
        self.layers
            .iter()
            .find(|l| l.config.name.eq_ignore_ascii_case(name))
    }
}
