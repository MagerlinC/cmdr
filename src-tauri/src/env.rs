use std::sync::OnceLock;

static SHELL_PATH: OnceLock<String> = OnceLock::new();

/// Resolve the user's login shell PATH by asking their shell.
/// Falls back to the current PATH if anything goes wrong.
pub fn shell_path() -> &'static str {
    SHELL_PATH.get_or_init(|| {
        let current = std::env::var("PATH").unwrap_or_default();

        // Detect the user's shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());

        // Ask the shell for its PATH via a login-interactive invocation
        let result = std::process::Command::new(&shell)
            .args(["-l", "-c", "echo $PATH"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path.is_empty() {
                    log::warn!("Shell returned empty PATH, using current");
                    current
                } else {
                    log::info!("Resolved shell PATH: {}", path);
                    path
                }
            }
            Ok(output) => {
                log::warn!("Shell exited with {}, using current PATH", output.status);
                current
            }
            Err(e) => {
                log::warn!("Failed to invoke shell for PATH: {}, using current", e);
                current
            }
        }
    })
}
