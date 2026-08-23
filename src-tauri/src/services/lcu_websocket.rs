//! # LCU WebSocket Watcher Service
//!
//! Subscribes to the League Client WAMP WebSocket (`wss://127.0.0.1:<port>/`),
//! listens for real-time client events (login, champion select, rune page updates),
//! and ensures the `[LCU COMPANION SLOT]` is automatically created/maintained.

use crate::services::lcu_client::{LcuClient, LcuCredentials};
use crate::services::lcu_perks::ensure_companion_slot;
use futures_util::{SinkExt, StreamExt};
use native_tls::TlsConnector;
use serde_json::Value;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;

pub struct LcuWsWatcher;

impl LcuWsWatcher {
    /// Spawns the resilient background LCU WebSocket watcher using Tauri's async runtime.
    pub fn start(app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            Self::run_loop(app).await;
        });
    }

    async fn run_loop(app: AppHandle) {
        let mut was_connected = false;
        loop {
            if let Some(creds) = LcuClient::find_credentials() {
                if let Err(e) = Self::connect_and_listen(&app, &creds, &mut was_connected).await {
                    eprintln!("[LCU WS] Connection closed or error: {}", e);
                }
                if was_connected {
                    was_connected = false;
                    let _ = app.emit("lcu:disconnected", ());
                }
            } else if was_connected {
                was_connected = false;
                let _ = app.emit("lcu:disconnected", ());
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn connect_and_listen(
        app: &AppHandle,
        creds: &LcuCredentials,
        was_connected: &mut bool,
    ) -> Result<(), String> {
        let ws_url = format!("wss://127.0.0.1:{}/", creds.port);
        let mut request = ws_url
            .clone()
            .into_client_request()
            .map_err(|e| format!("Invalid WS request: {}", e))?;

        let auth_val = HeaderValue::from_str(&creds.basic_auth_header())
            .map_err(|e| format!("Invalid auth header: {}", e))?;
        request.headers_mut().insert("Authorization", auth_val);

        let tls_builder = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("Failed to build TlsConnector: {}", e))?;
        let connector = Connector::NativeTls(tls_builder);

        let (ws_stream, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(connector),
        )
        .await
        .map_err(|e| format!("Failed to connect to LCU WS at {}: {}", ws_url, e))?;

        println!(
            "[LCU WS] Successfully connected to League Client WebSocket on port {}",
            creds.port
        );

        *was_connected = true;

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to all LCU events via WAMP opcode 5
        let sub_msg = Message::Text(r#"[5, "OnJsonApiEvent"]"#.to_string().into());
        write
            .send(sub_msg)
            .await
            .map_err(|e| format!("Failed to send subscription message: {}", e))?;

        // On initial connection, notify frontend and ensure companion slot exists
        let _ = app.emit("lcu:connected", ());
        let _ = app.emit("lcu:summoner-changed", ());
        let _ = app.emit("lcu:runes-changed", ());

        if let Ok(client) = LcuClient::new(creds.clone()) {
            let _ = ensure_companion_slot(&client).await;
        }

        while let Some(msg_result) = read.next().await {
            let msg = msg_result.map_err(|e| format!("WS read error: {}", e))?;
            match msg {
                Message::Text(text) => {
                    Self::handle_event(app, &text, creds).await;
                }
                Message::Close(_) => {
                    println!("[LCU WS] Received Close frame from server.");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_event(app: &AppHandle, text: &str, creds: &LcuCredentials) {
        let parsed: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return,
        };

        if let Some(arr) = parsed.as_array() {
            if arr.len() >= 3 && arr[0] == 8 {
                let payload = &arr[2];
                let uri = payload.get("uri").and_then(|v| v.as_str()).unwrap_or("");

                // When user logs in or profile changes, ensure companion slot is ready
                if uri == "/lol-login/v1/session"
                    || uri == "/lol-summoner/v1/current-summoner"
                    || uri == "/lol-gameflow/v1/gameflow-phase"
                {
                    if let Ok(client) = LcuClient::new(creds.clone()) {
                        let _ = ensure_companion_slot(&client).await;
                    }
                }

                // Check for summoner / login / regalia / challenge events
                if uri == "/lol-login/v1/session"
                    || uri == "/lol-summoner/v1/current-summoner"
                    || uri == "/lol-regalia/v2/current-summoner/regalia"
                    || uri == "/lol-challenges/v1/summary-player-data/local-player"
                {
                    let _ = app.emit("lcu:summoner-changed", ());
                }

                // Check for rune perks / pages events
                if uri == "/lol-perks/v1/pages"
                    || uri == "/lol-perks/v1/currentpage"
                {
                    let _ = app.emit("lcu:runes-changed", ());
                }
            }
        }
    }
}
