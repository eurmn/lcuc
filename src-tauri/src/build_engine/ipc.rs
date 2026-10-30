use std::io::{self, BufRead, Write};

use crate::build_engine::bitset_miner::compute_build_recommendation;
use crate::build_engine::models::{BuildEngineRequest, BuildEngineResponse};

pub fn run_ipc_stdin_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();

    let mut line_buffer = String::new();
    while stdin_lock.read_line(&mut line_buffer)? > 0 {
        let trimmed = line_buffer.trim();
        if trimmed.is_empty() {
            line_buffer.clear();
            continue;
        }

        let response = match serde_json::from_str::<BuildEngineRequest>(trimmed) {
            Ok(req) => {
                let req_id = req.id;
                match compute_build_recommendation(req) {
                    Ok(result) => BuildEngineResponse {
                        id: req_id,
                        result: Some(result),
                        error: None,
                    },
                    Err(err) => BuildEngineResponse {
                        id: req_id,
                        result: None,
                        error: Some(err),
                    },
                }
            }
            Err(err) => BuildEngineResponse {
                id: 0,
                result: None,
                error: Some(format!("Invalid BuildEngineRequest JSON: {}", err)),
            },
        };

        if let Ok(json_out) = serde_json::to_string(&response) {
            stdout_lock.write_all(json_out.as_bytes())?;
            stdout_lock.write_all(b"\n")?;
            stdout_lock.flush()?;
        }

        line_buffer.clear();
    }

    Ok(())
}
