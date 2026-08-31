use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Building,
    Crashed,
    Error,
    Unknown,
}

impl RuntimeState {
    pub fn is_active(self) -> bool {
        matches!(self, RuntimeState::Running | RuntimeState::Starting)
    }

    pub fn is_transitioning(self) -> bool {
        matches!(
            self,
            RuntimeState::Starting | RuntimeState::Stopping | RuntimeState::Building
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub name: String,
    pub runtime_type: String,
    pub state: RuntimeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub has_build: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    pub name: String,
    pub runtimes: Vec<RuntimeStatus>,
    pub active_runtime: Option<String>,
    pub transitioning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_message: Option<String>,
}
