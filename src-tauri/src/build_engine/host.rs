use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

use crate::build_engine::bitset_miner::compute_build_recommendation;
use crate::build_engine::models::{
    BuildEngineRequest, BuildEngineResponse, BuildRecommendationResult,
};

struct ChildHandle {
    child: Child,
    stdin: ChildStdin,
}

struct EngineSupervisor {
    child_handle: Option<ChildHandle>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<BuildEngineResponse>>>>,
}

lazy_static::lazy_static! {
    static ref SUPERVISOR: Mutex<Option<EngineSupervisor>> = Mutex::new(None);
    static ref NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);
}

fn resolve_build_engine_binary_path() -> Option<PathBuf> {
    let binary_name = if cfg!(windows) {
        "lcu-build-engine.exe"
    } else {
        "lcu-build-engine"
    };

    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(binary_name);
        if sibling.exists() {
            return Some(sibling);
        }
        if let Some(target_dir) = exe.parent().and_then(|p| p.parent()) {
            let release_candidate = target_dir.join("release").join(binary_name);
            if release_candidate.exists() {
                return Some(release_candidate);
            }
            let debug_candidate = target_dir.join("debug").join(binary_name);
            if debug_candidate.exists() {
                return Some(debug_candidate);
            }
        }
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            let candidate = parent.join(binary_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    let current_dir_candidate = PathBuf::from(binary_name);
    if current_dir_candidate.exists() {
        return Some(current_dir_candidate);
    }

    None
}

pub fn ensure_build_engine_process_started() -> Result<(), String> {
    let mut lock = SUPERVISOR.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut sup) = *lock {
        if let Some(ref mut handle) = sup.child_handle {
            if let Ok(None) = handle.child.try_wait() {
                return Ok(());
            }
        }
    }

    let Some(binary_path) = resolve_build_engine_binary_path() else {
        // Binary not yet built or available on disk; host will use in-process fallback
        return Ok(());
    };

    let mut cmd = Command::new(&binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Err(format!(
                "Failed to spawn build engine binary {:?}: {}",
                binary_path, e
            ));
        }
    };

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to capture stdin for build engine".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout for build engine".to_string())?;

    let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<BuildEngineResponse>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let pending_clone = Arc::clone(&pending);

    let _ = std::thread::Builder::new()
        .name("build-engine-stdout-reader".to_string())
        .spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line_str) = line else {
                    break;
                };
                let trimmed = line_str.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<BuildEngineResponse>(trimmed) {
                    if let Ok(mut map) = pending_clone.lock() {
                        if let Some(sender) = map.remove(&resp.id) {
                            let _ = sender.send(resp);
                        }
                    }
                }
            }
        });

    *lock = Some(EngineSupervisor {
        child_handle: Some(ChildHandle { child, stdin }),
        pending_requests: pending,
    });

    Ok(())
}

pub async fn request_build_recommendation(
    champion_id: u32,
    owned_items: Vec<u32>,
    owned_augments: Vec<String>,
) -> Result<BuildRecommendationResult, String> {
    let req_id = NEXT_REQ_ID.fetch_add(1, Ordering::Relaxed);
    let req = BuildEngineRequest {
        id: req_id,
        champion_id,
        owned_items,
        owned_augments,
    };

    let (tx, rx) = oneshot::channel();
    let mut sent = false;

    {
        if let Ok(mut lock) = SUPERVISOR.lock() {
            if let Some(ref mut sup) = *lock {
                if let Some(ref mut handle) = sup.child_handle {
                    if let Ok(None) = handle.child.try_wait() {
                        if let Ok(json_line) = serde_json::to_string(&req) {
                            if handle.stdin.write_all(json_line.as_bytes()).is_ok()
                                && handle.stdin.write_all(b"\n").is_ok()
                                && handle.stdin.flush().is_ok()
                            {
                                if let Ok(mut pending) = sup.pending_requests.lock() {
                                    pending.insert(req_id, tx);
                                    sent = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if sent {
        match tokio::time::timeout(Duration::from_millis(1500), rx).await {
            Ok(Ok(resp)) => {
                if let Some(result) = resp.result {
                    return Ok(result);
                }
                if let Some(err) = resp.error {
                    return Err(err);
                }
            }
            Ok(Err(_oneshot_err)) => {
                // Sender dropped, child may have terminated; fallback below
            }
            Err(_timeout) => {
                // Timeout, clean pending entry and fallback
                if let Ok(lock) = SUPERVISOR.lock() {
                    if let Some(ref sup) = *lock {
                        if let Ok(mut pending) = sup.pending_requests.lock() {
                            pending.remove(&req_id);
                        }
                    }
                }
            }
        }
    }

    // In-process fallback if process not running, crashed, or timed out
    compute_build_recommendation(req)
}
