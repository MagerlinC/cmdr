use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub layers: Vec<LayerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub name: String,
    pub runtimes: Vec<RuntimeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub runtime_type: String,
    #[serde(default)]
    pub cwd: Option<String>,
    pub up: String,
    #[serde(default)]
    pub down: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {}", path.display(), e))?;
        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() {
            return Err("Configuration must have at least one layer".into());
        }
        for layer in &self.layers {
            if layer.runtimes.is_empty() {
                return Err(format!("Layer '{}' must have at least one runtime", layer.name));
            }
        }
        Ok(())
    }
}

impl RuntimeConfig {
    pub fn resolve_cwd(&self, config_dir: &Path) -> PathBuf {
        match &self.cwd {
            Some(cwd) => {
                let p = PathBuf::from(cwd);
                if p.is_absolute() {
                    p
                } else {
                    config_dir.join(p)
                }
            }
            None => config_dir.to_path_buf(),
        }
    }
}
