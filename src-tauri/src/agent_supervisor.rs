use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct AgentSupervisor {
    state: Arc<Mutex<AgentProcessState>>,
}

#[derive(Default)]
struct AgentProcessState {
    child: Option<Child>,
    port: Option<u16>,
    token: Option<String>,
    startup_error: Option<String>,
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
        let supervisor = Self {
            state: Arc::new(Mutex::new(AgentProcessState::default())),
        };
        if let Err(error) = supervisor.spawn_agent() {
            let mut state = supervisor
                .state
                .lock()
                .map_err(|_| "agent supervisor state lock poisoned".to_string())?;
            state.startup_error = Some(error);
        }
        Ok(supervisor)
    }

    fn agent_binary() -> PathBuf {
        if let Ok(path) = std::env::var("LCU_COMPANION_AGENT_BIN") {
            return PathBuf::from(path);
        }

        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("lcu-companion-agent.exe")))
            .unwrap_or_else(|| PathBuf::from("lcu-companion-agent.exe"))
    }

    fn generate_token() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("{nanos:x}-{}", std::process::id())
    }

    fn reserve_port() -> Result<u16, String> {
        let listener = StdTcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("failed reserving agent port: {error}"))?;
        listener
            .local_addr()
            .map(|address| address.port())
            .map_err(|error| format!("failed reading reserved agent port: {error}"))
    }

    fn spawn_agent(&self) -> Result<(), String> {
        let port = Self::reserve_port()?;
        let token = Self::generate_token();
        let binary = Self::agent_binary();

        let (program, arguments) = if binary.exists() {
            (
                binary.to_string_lossy().to_string(),
                vec![
                    "--port".to_string(),
                    port.to_string(),
                    "--token".to_string(),
                    token.clone(),
                ],
            )
        } else {
            let manifest = std::env::current_dir()
                .map_err(|error| format!("failed finding agent manifest: {error}"))?
                .join("src-tauri")
                .join("Cargo.toml");
            (
                "cargo".to_string(),
                vec![
                    "run".to_string(),
                    "--quiet".to_string(),
                    "--manifest-path".to_string(),
                    manifest.to_string_lossy().to_string(),
                    "--bin".to_string(),
                    "lcu-companion-agent".to_string(),
                    "--".to_string(),
                    "--port".to_string(),
                    port.to_string(),
                    "--token".to_string(),
                    token.clone(),
                ],
            )
        };

        let child = Command::new(&program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                format!(
                    "failed starting agent with {program}: {error}; set LCU_COMPANION_AGENT_BIN to override"
                )
            })?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| "agent supervisor state lock poisoned".to_string())?;
        state.child = Some(child);
        state.port = Some(port);
        state.token = Some(token);
        Ok(())
    }

    pub async fn status(&self) -> AgentStatus {
        let (port, token, running) = match self.state.lock() {
            Ok(mut state) => {
                let running = state
                    .child
                    .as_mut()
                    .and_then(|child| child.try_wait().ok())
                    .flatten()
                    .is_none();
                (state.port, state.token.clone(), running)
            }
            Err(_) => {
                return AgentStatus {
                    running: false,
                    reachable: false,
                    port: None,
                    collector_state: None,
                    database_ready: false,
                    error: Some("agent supervisor state lock poisoned".to_string()),
                };
            }
        };

        let Some(port) = port else {
            return AgentStatus {
                running,
                reachable: false,
                port: None,
                collector_state: None,
                database_ready: false,
                error: Some(
                    self.state
                        .lock()
                        .ok()
                        .and_then(|state| state.startup_error.clone())
                        .unwrap_or_else(|| "agent port is unavailable".to_string()),
                ),
            };
        };
        let Some(token) = token else {
            return AgentStatus {
                running,
                reachable: false,
                port: Some(port),
                collector_state: None,
                database_ready: false,
                error: Some("agent token is unavailable".to_string()),
            };
        };

        let request = serde_json::json!({
            "token": token,
            "command": "health",
        });
        let encoded = match serde_json::to_vec(&request) {
            Ok(mut bytes) => {
                bytes.push(b'\n');
                bytes
            }
            Err(error) => {
                return AgentStatus {
                    running,
                    reachable: false,
                    port: Some(port),
                    collector_state: None,
                    database_ready: false,
                    error: Some(error.to_string()),
                };
            }
        };

        let mut stream = match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(stream) => stream,
            Err(error) => {
                return AgentStatus {
                    running,
                    reachable: false,
                    port: Some(port),
                    collector_state: None,
                    database_ready: false,
                    error: Some(error.to_string()),
                };
            }
        };
        if let Err(error) = stream.write_all(&encoded).await {
            return AgentStatus {
                running,
                reachable: false,
                port: Some(port),
                collector_state: None,
                database_ready: false,
                error: Some(error.to_string()),
            };
        }

        let mut line = String::new();
        let mut reader = BufReader::new(&mut stream);
        if let Err(error) = reader.read_line(&mut line).await {
            return AgentStatus {
                running,
                reachable: false,
                port: Some(port),
                collector_state: None,
                database_ready: false,
                error: Some(error.to_string()),
            };
        }

        let response: serde_json::Value = match serde_json::from_str(&line) {
            Ok(response) => response,
            Err(error) => {
                return AgentStatus {
                    running,
                    reachable: false,
                    port: Some(port),
                    collector_state: None,
                    database_ready: false,
                    error: Some(error.to_string()),
                };
            }
        };

        AgentStatus {
            running,
            reachable: response.get("ok").and_then(|value| value.as_bool()).unwrap_or(false),
            port: Some(port),
            collector_state: response
                .pointer("/data/collector_state")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            database_ready: response
                .pointer("/data/database_ready")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            error: response
                .get("error")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        }
    }

    pub async fn request(&self, command: &str) -> Result<Value, String> {
            let (port, token) = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "agent supervisor state lock poisoned".to_string())?;
                (
                    state.port.ok_or_else(|| "agent port is unavailable".to_string())?,
                    state.token.clone().ok_or_else(|| "agent token is unavailable".to_string())?,
                )
            };
            let mut stream = TcpStream::connect(("127.0.0.1", port))
                .await
                .map_err(|error| format!("agent connection failed: {error}"))?;
            let mut request = serde_json::to_vec(&serde_json::json!({
                "token": token,
                "command": command,
            }))
            .map_err(|error| format!("agent request encoding failed: {error}"))?;
            request.push(b'\n');
            stream
                .write_all(&request)
                .await
                .map_err(|error| format!("agent request failed: {error}"))?;
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .map_err(|error| format!("agent response failed: {error}"))?;
            let response: Value = serde_json::from_str(&line)
                .map_err(|error| format!("agent response parsing failed: {error}"))?;
            if !response.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                return Err(response
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("agent request failed")
                    .to_string());
            }
            Ok(response.get("data").cloned().unwrap_or(Value::Null))
    }
}

impl Drop for AgentSupervisor {
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) > 1 {
            return;
        }
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let Some(mut child) = state.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
