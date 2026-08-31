use crate::config::RuntimeConfig;
use crate::env;
use crate::state::RuntimeState;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Create a Command with the user's shell PATH injected.
fn cmd(program: &str) -> Command {
    let mut c = Command::new(program);
    c.env("PATH", env::shell_path());
    c
}

pub struct ManagedRuntime {
    pub config: RuntimeConfig,
    pub config_dir: PathBuf,
    pub state: Mutex<RuntimeState>,
    pub error: Mutex<Option<String>>,
    pub child: Mutex<Option<Child>>,
}

impl ManagedRuntime {
    pub fn new(config: RuntimeConfig, config_dir: PathBuf) -> Self {
        Self {
            config,
            config_dir,
            state: Mutex::new(RuntimeState::Unknown),
            error: Mutex::new(None),
            child: Mutex::new(None),
        }
    }

    pub fn cwd(&self) -> PathBuf {
        self.config.resolve_cwd(&self.config_dir)
    }

    fn parse_command(cmd: &str) -> (&str, Vec<&str>) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            ("", vec![])
        } else {
            (parts[0], parts[1..].to_vec())
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        {
            let state = self.state.lock().await;
            if state.is_active() {
                return Err(format!("Runtime '{}' is already active", self.config.name));
            }
            if state.is_transitioning() {
                return Err(format!("Runtime '{}' is already transitioning", self.config.name));
            }
        }

        *self.state.lock().await = RuntimeState::Starting;
        *self.error.lock().await = None;

        let cwd = self.cwd();
        let (program, args) = Self::parse_command(&self.config.up);

        log::info!(
            "Starting runtime '{}': {} (cwd: {})",
            self.config.name,
            self.config.up,
            cwd.display()
        );

        match self.config.runtime_type.as_str() {
            "docker" => {
                // Docker commands are fire-and-forget (e.g., docker compose up -d)
                let output = cmd(program)
                    .args(&args)
                    .current_dir(&cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await
                    .map_err(|e| {
                        let msg = format!("Failed to execute '{}': {}", self.config.up, e);
                        log::error!("{}", msg);
                        msg
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let msg = format!("Command '{}' failed: {}", self.config.up, stderr);
                    log::error!("{}", msg);
                    *self.state.lock().await = RuntimeState::Error;
                    *self.error.lock().await = Some(msg.clone());
                    return Err(msg);
                }

                *self.state.lock().await = RuntimeState::Running;
                Ok(())
            }
            "terminal" => {
                // Terminal commands are long-running processes we need to track
                let child = cmd(program)
                    .args(&args)
                    .current_dir(&cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(false)
                    .spawn()
                    .map_err(|e| {
                        let msg = format!("Failed to spawn '{}': {}", self.config.up, e);
                        log::error!("{}", msg);
                        msg
                    })?;

                log::info!(
                    "Spawned terminal process for '{}', pid: {:?}",
                    self.config.name,
                    child.id()
                );

                *self.child.lock().await = Some(child);
                *self.state.lock().await = RuntimeState::Running;
                Ok(())
            }
            other => {
                let msg = format!("Unknown runtime type: {}", other);
                *self.state.lock().await = RuntimeState::Error;
                *self.error.lock().await = Some(msg.clone());
                Err(msg)
            }
        }
    }

    pub async fn stop(&self) -> Result<(), String> {
        let current_state = *self.state.lock().await;
        if current_state == RuntimeState::Stopped {
            return Ok(());
        }

        *self.state.lock().await = RuntimeState::Stopping;
        *self.error.lock().await = None;

        log::info!("Stopping runtime '{}'", self.config.name);

        match self.config.runtime_type.as_str() {
            "docker" => {
                if let Some(down_cmd) = &self.config.down {
                    let cwd = self.cwd();
                    let (program, args) = Self::parse_command(down_cmd);

                    let output = cmd(program)
                        .args(&args)
                        .current_dir(&cwd)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .output()
                        .await
                        .map_err(|e| {
                            let msg = format!("Failed to execute '{}': {}", down_cmd, e);
                            log::error!("{}", msg);
                            msg
                        })?;

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let msg = format!("Command '{}' failed: {}", down_cmd, stderr);
                        log::error!("{}", msg);
                        *self.state.lock().await = RuntimeState::Error;
                        *self.error.lock().await = Some(msg.clone());
                        return Err(msg);
                    }
                }

                *self.state.lock().await = RuntimeState::Stopped;
                Ok(())
            }
            "terminal" => {
                let mut child_lock = self.child.lock().await;

                if let Some(ref mut child) = *child_lock {
                    // If a down command is configured, use it
                    if let Some(down_cmd) = &self.config.down {
                        let cwd = self.cwd();
                        let (program, args) = Self::parse_command(down_cmd);

                        let output = cmd(program)
                            .args(&args)
                            .current_dir(&cwd)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .output()
                            .await
                            .map_err(|e| format!("Failed to execute '{}': {}", down_cmd, e))?;

                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            let msg = format!("Down command '{}' failed: {}", down_cmd, stderr);
                            log::error!("{}", msg);
                            *self.state.lock().await = RuntimeState::Error;
                            *self.error.lock().await = Some(msg.clone());
                            return Err(msg);
                        }
                    } else {
                        // No down command: send SIGTERM to the process group
                        if let Some(pid) = child.id() {
                            log::info!("Sending SIGTERM to process group {}", pid);
                            // Kill the process group (negative pid)
                            unsafe {
                                libc::kill(-(pid as i32), libc::SIGTERM);
                            }

                            // Wait for the process to exit
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                child.wait(),
                            )
                            .await
                            {
                                Ok(Ok(_)) => {
                                    log::info!("Process exited cleanly");
                                }
                                Ok(Err(e)) => {
                                    log::warn!("Error waiting for process: {}", e);
                                }
                                Err(_) => {
                                    log::warn!("Timeout waiting for process, sending SIGKILL");
                                    unsafe {
                                        libc::kill(-(pid as i32), libc::SIGKILL);
                                    }
                                    let _ = child.wait().await;
                                }
                            }
                        } else {
                            // Process already exited
                            log::info!("Process already exited");
                        }
                    }
                }

                *child_lock = None;
                drop(child_lock);

                *self.state.lock().await = RuntimeState::Stopped;
                Ok(())
            }
            _ => {
                *self.state.lock().await = RuntimeState::Error;
                Err(format!("Unknown runtime type: {}", self.config.runtime_type))
            }
        }
    }

    pub async fn check_state(&self) -> RuntimeState {
        match self.config.runtime_type.as_str() {
            "docker" => {
                self.check_docker_state().await
            }
            "terminal" => {
                self.check_terminal_state().await
            }
            _ => RuntimeState::Unknown,
        }
    }

    async fn check_docker_state(&self) -> RuntimeState {
        // Try to determine Docker container state from the up command
        // Parse container/service names from the up command
        let up_cmd = &self.config.up;

        // Check if this looks like a docker compose command
        if up_cmd.contains("docker compose") || up_cmd.contains("docker-compose") {
            // Extract service names (everything after the flags like -d)
            let parts: Vec<&str> = up_cmd.split_whitespace().collect();
            // Find the subcommand (up/start) and get services after it
            let mut services = Vec::new();
            let mut found_action = false;
            for part in &parts {
                if found_action && !part.starts_with('-') {
                    services.push(*part);
                }
                if *part == "up" || *part == "start" || *part == "run" {
                    found_action = true;
                }
            }

            if services.is_empty() {
                return RuntimeState::Unknown;
            }

            let cwd = self.cwd();

            // Use docker compose ps to check service state
            for service in &services {
                let output = cmd("docker")
                    .args(["compose", "ps", "--format", "{{.State}}", service])
                    .current_dir(&cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                match output {
                    Ok(output) if output.status.success() => {
                        let state_str = String::from_utf8_lossy(&output.stdout)
                            .trim()
                            .to_lowercase();
                        if state_str.is_empty() {
                            return RuntimeState::Stopped;
                        }
                        match state_str.as_str() {
                            "running" => {}
                            "exited" | "dead" | "removing" => return RuntimeState::Stopped,
                            "created" | "restarting" => return RuntimeState::Starting,
                            _ => return RuntimeState::Unknown,
                        }
                    }
                    _ => return RuntimeState::Unknown,
                }
            }

            // All services are running
            RuntimeState::Running
        } else {
            RuntimeState::Unknown
        }
    }

    async fn check_terminal_state(&self) -> RuntimeState {
        let child_lock = self.child.lock().await;
        match &*child_lock {
            Some(child) => {
                match child.id() {
                    Some(pid) => {
                        // Check if process is still alive
                        let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
                        if alive {
                            RuntimeState::Running
                        } else {
                            RuntimeState::Crashed
                        }
                    }
                    None => {
                        // Process has already exited
                        RuntimeState::Crashed
                    }
                }
            }
            None => RuntimeState::Stopped,
        }
    }

    pub async fn build(&self) -> Result<(), String> {
        let build_cmd = self
            .config
            .build
            .as_ref()
            .ok_or_else(|| format!("Runtime '{}' has no build command", self.config.name))?;

        {
            let state = self.state.lock().await;
            if state.is_transitioning() {
                return Err(format!(
                    "Runtime '{}' is already transitioning",
                    self.config.name
                ));
            }
        }

        let prev_state = *self.state.lock().await;
        *self.state.lock().await = RuntimeState::Building;
        *self.error.lock().await = None;

        let cwd = self.cwd();
        let (program, args) = Self::parse_command(build_cmd);

        log::info!(
            "Building runtime '{}': {} (cwd: {})",
            self.config.name,
            build_cmd,
            cwd.display()
        );

        let output = cmd(program)
            .args(&args)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                let msg = format!("Failed to execute '{}': {}", build_cmd, e);
                log::error!("{}", msg);
                msg
            });

        match output {
            Ok(output) if output.status.success() => {
                *self.state.lock().await = prev_state;
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let msg = format!("Build command '{}' failed: {}", build_cmd, stderr);
                log::error!("{}", msg);
                *self.state.lock().await = RuntimeState::Error;
                *self.error.lock().await = Some(msg.clone());
                Err(msg)
            }
            Err(msg) => {
                *self.state.lock().await = RuntimeState::Error;
                *self.error.lock().await = Some(msg.clone());
                Err(msg)
            }
        }
    }

    pub async fn get_pid(&self) -> Option<u32> {
        let child_lock = self.child.lock().await;
        child_lock.as_ref().and_then(|c| c.id())
    }
}
