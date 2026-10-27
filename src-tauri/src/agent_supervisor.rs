use crate::agent::{
    collector_loop, database_path, dispatch_command, global_aram_database_path,
    initialize_database, initialize_global_aram_database, websocket_loop, RuntimeState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AgentSupervisor {
    path: PathBuf,
    runtime: Arc<RwLock<RuntimeState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub running: bool,
    pub reachable: bool,
    pub port: Option<u16>,
    pub collector_state: Option<String>,
    pub database_ready: bool,
    pub error: Option<String>,
}

impl AgentSupervisor {
    pub fn start() -> Result<Self, String> {
        let path = database_path();
        initialize_database(&path)?;
        let global_path = global_aram_database_path();
        initialize_global_aram_database(&global_path)?;

        let runtime = Arc::new(RwLock::new(RuntimeState {
            collector_state: "sleeping".to_string(),
            ..RuntimeState::default()
        }));

        tauri::async_runtime::spawn(collector_loop(path.clone(), runtime.clone()));
        tauri::async_runtime::spawn(websocket_loop(runtime.clone()));

        crate::logger::info(
            "tauri.agent.supervisor",
            &format!(
                "In-process companion agent started with database {:?}",
                path
            ),
        );

        Ok(Self { path, runtime })
    }

    pub async fn status(&self) -> AgentStatus {
        let state = self.runtime.read().await;
        AgentStatus {
            running: true,
            reachable: true,
            port: None,
            collector_state: Some(state.collector_state.clone()),
            database_ready: self.path.exists(),
            error: None,
        }
    }

    pub async fn request(&self, command: &str) -> Result<Value, String> {
        self.request_with_args(command, Value::Null).await
    }

    pub async fn request_with_args(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_command(command, args, &self.path, &self.runtime).await
    }
}
