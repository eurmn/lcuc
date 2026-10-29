//! # LCU Client Service
//!
//! Provides connection discovery, authentication, and HTTP request handling
//! for the League of Legends Client Update (LCU) local HTTPS API.
//!
//! ## Connection Strategy
//! 1. **Lockfile Discovery**: Reads `C:\Riot Games\League of Legends\lockfile` or process directories.
//! 2. **Process Scanning**: Uses `sysinfo` with full command-line refresh to inspect `--app-port` and `--remoting-auth-token`.
//! 3. **Unverified TLS**: Bypasses self-signed certificate validation for localhost HTTPS communication.

use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

#[derive(Debug, Clone)]
pub struct LcuResponseError {
    pub status: Option<u16>,
    pub retry_after: Option<Duration>,
    pub message: String,
}

impl std::fmt::Display for LcuResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status) = self.status {
            if let Some(dur) = self.retry_after {
                write!(
                    f,
                    "LCU request failed with status {} (retry-after: {}s): {}",
                    status,
                    dur.as_secs(),
                    self.message
                )
            } else {
                write!(
                    f,
                    "LCU request failed with status {}: {}",
                    status, self.message
                )
            }
        } else {
            write!(f, "LCU network/client error: {}", self.message)
        }
    }
}

impl std::error::Error for LcuResponseError {}

/// Credentials required to authenticate with the local League Client Update (LCU) API.
#[derive(Debug, Clone)]
pub struct LcuCredentials {
    /// Port on localhost where the LCU HTTPS server is listening (e.g. `58652`).
    pub port: u16,
    /// Dynamic base64 remoting authentication token generated on client launch.
    pub auth_token: String,
    /// Process ID (PID) of the running League Client.
    pub pid: u32,
}

impl LcuCredentials {
    /// Returns the base HTTPS URL for the local LCU instance.
    ///
    /// # Example
    /// ```rust,ignore
    /// let url = credentials.base_url(); // "https://127.0.0.1:58652"
    /// ```
    pub fn base_url(&self) -> String {
        format!("https://127.0.0.1:{}", self.port)
    }

    /// Formats the HTTP Basic Authentication header for the LCU API (`riot:<token>`).
    pub fn basic_auth_header(&self) -> String {
        let auth_str = format!("riot:{}", self.auth_token);
        format!("Basic {}", BASE64_STANDARD.encode(auth_str.as_bytes()))
    }
}

/// Asynchronous HTTP client configured for communicating with the local League of Legends client.
#[derive(Clone)]
pub struct LcuClient {
    /// Active credentials for the current client session.
    pub credentials: LcuCredentials,
    client: Client,
}

impl LcuClient {
    /// Constructs a new `LcuClient` with unverified TLS and Basic Auth headers.
    ///
    /// # Arguments
    /// * `credentials` - The port, token, and PID of the running client.
    ///
    /// # Errors
    /// Returns `Err(String)` if the HTTP client could not be constructed.
    pub fn new(credentials: LcuCredentials) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let auth_val = HeaderValue::from_str(&credentials.basic_auth_header())
            .map_err(|e| format!("Invalid auth header: {}", e))?;
        headers.insert(AUTHORIZATION, auth_val);

        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build reqwest client: {}", e))?;

        Ok(Self {
            credentials,
            client,
        })
    }

    /// Parses a standard League of Legends lockfile content string.
    ///
    /// # Format
    /// `process_name:pid:port:auth_token:protocol` (e.g. `LeagueClient:25752:58652:KvCzdDzcv0Wg2faERY8OSg:https`)
    fn parse_lockfile_content(content: &str) -> Option<LcuCredentials> {
        let parts: Vec<&str> = content.trim().split(':').collect();
        if parts.len() >= 5 {
            let pid = parts[1].parse::<u32>().ok()?;
            let port = parts[2].parse::<u16>().ok()?;
            let auth_token = parts[3].to_string();
            if port > 0 && !auth_token.is_empty() {
                return Some(LcuCredentials {
                    port,
                    auth_token,
                    pid,
                });
            }
        }
        None
    }

    /// Attempts to read the LCU lockfile from a specific file path.
    fn try_read_lockfile(path: &Path) -> Option<LcuCredentials> {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Some(creds) = Self::parse_lockfile_content(&content) {
                    return Some(creds);
                }
            }
        }
        None
    }

    /// Discovers LCU credentials via standard installation directory lockfiles.
    fn find_credentials_from_known_lockfiles() -> Option<LcuCredentials> {
        let common_paths = [
            r"C:\Riot Games\League of Legends\lockfile",
            r"D:\Riot Games\League of Legends\lockfile",
            r"E:\Riot Games\League of Legends\lockfile",
            r"F:\Riot Games\League of Legends\lockfile",
            r"C:\Program Files\Riot Games\League of Legends\lockfile",
            r"C:\Program Files (x86)\Riot Games\League of Legends\lockfile",
        ];

        for path_str in &common_paths {
            if let Some(creds) = Self::try_read_lockfile(Path::new(path_str)) {
                return Some(creds);
            }
        }
        None
    }

    /// Discovers LCU credentials by scanning running processes using `sysinfo`.
    ///
    /// Inspects processes starting with `leagueclient` and checks both process directory lockfiles
    /// and command-line arguments (`--app-port` and `--remoting-auth-token`).
    pub fn find_credentials() -> Option<LcuCredentials> {
        // 1. First priority: Check common install paths for lockfile
        if let Some(creds) = Self::find_credentials_from_known_lockfiles() {
            return Some(creds);
        }

        // Throttle full process scanning when lockfile is absent to avoid CPU/IO spikes
        static LAST_SYSINFO_SCAN: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
        {
            if let Ok(mut last) = LAST_SYSINFO_SCAN.lock() {
                if let Some(instant) = *last {
                    if instant.elapsed() < Duration::from_secs(5) {
                        return None;
                    }
                }
                *last = Some(Instant::now());
            }
        }

        // 2. Second priority: Process scanning with full command-line refresh
        let refresh_kind = RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::everything().with_cmd(UpdateKind::Always));
        let mut sys = System::new_with_specifics(refresh_kind);
        sys.refresh_processes(ProcessesToUpdate::All, true);

        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            if name.starts_with("leagueclient") {
                // Check lockfile in process executable parent directory
                if let Some(exe_path) = process.exe() {
                    if let Some(parent) = exe_path.parent() {
                        let lockfile_path: PathBuf = parent.join("lockfile");
                        if let Some(creds) = Self::try_read_lockfile(&lockfile_path) {
                            return Some(creds);
                        }
                    }
                }

                // Check command-line arguments
                let cmdline = process.cmd();
                let mut app_port: Option<u16> = None;
                let mut auth_token: Option<String> = None;

                for arg_os in cmdline {
                    let arg = arg_os.to_string_lossy();
                    if let Some(port_str) = arg.strip_prefix("--app-port=") {
                        if let Ok(p) = port_str.parse::<u16>() {
                            app_port = Some(p);
                        }
                    } else if let Some(token) = arg.strip_prefix("--remoting-auth-token=") {
                        let clean_token = token.trim_matches('"').trim_matches('\'').to_string();
                        auth_token = Some(clean_token);
                    }
                }

                if let (Some(port), Some(token)) = (app_port, auth_token) {
                    return Some(LcuCredentials {
                        port,
                        auth_token: token,
                        pid: pid.as_u32(),
                    });
                }
            }
        }

        None
    }

    /// Connects to the local League Client, discovering credentials automatically.
    ///
    /// # Errors
    /// Returns `Err(String)` if League of Legends is not running or credentials cannot be found.
    pub fn connect() -> Result<Self, String> {
        let creds = Self::find_credentials().ok_or_else(|| {
            "League of Legends client is not running or lockfile could not be found.".to_string()
        })?;
        Self::new(creds)
    }

    /// Performs an authenticated HTTP GET request against the local LCU API returning detailed error metadata.
    pub async fn get_json_response<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, LcuResponseError> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            let is_timeout = e.is_timeout();
            LcuResponseError {
                status: None,
                retry_after: None,
                message: if is_timeout {
                    format!("LCU GET {path} timed out: {e}")
                } else {
                    format!("LCU GET {path} failed: {e}")
                },
            }
        })?;

        let status = resp.status();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        if !status.is_success() {
            return Err(LcuResponseError {
                status: Some(status.as_u16()),
                retry_after,
                message: format!("LCU GET {path} returned HTTP {status}"),
            });
        }

        resp.json::<T>().await.map_err(|e| LcuResponseError {
            status: Some(status.as_u16()),
            retry_after: None,
            message: format!("Failed deserializing LCU JSON for {path}: {e}"),
        })
    }

    /// Performs an authenticated HTTP GET request against the local LCU API and deserializes the JSON response.
    ///
    /// # Arguments
    /// * `path` - The API endpoint path (e.g. `/lol-summoner/v1/current-summoner`).
    ///
    /// # Errors
    /// Returns `Err(String)` on network error, non-2xx HTTP status, or JSON deserialization failure.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        self.get_json_response(path)
            .await
            .map_err(|e| e.to_string())
    }

    /// Performs an authenticated HTTP POST request with a JSON payload against the local LCU API.
    ///
    /// # Arguments
    /// * `path` - The API endpoint path.
    /// * `body` - The JSON body to send.
    ///
    /// # Errors
    /// Returns `Err(String)` on network failure or error status code.
    pub async fn post_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU POST {} failed: {}", path, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "LCU POST {} returned error status: {}",
                path,
                resp.status()
            ));
        }

        resp.json::<T>()
            .await
            .map_err(|e| format!("Failed deserializing LCU POST JSON for {}: {}", path, e))
    }

    /// Performs an authenticated HTTP POST request with a JSON payload and returns the HTTP status code.
    pub async fn post_json_status(&self, path: &str, body: &Value) -> Result<u16, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU POST {} failed: {}", path, e))?;

        Ok(resp.status().as_u16())
    }

    /// Performs an authenticated HTTP PUT request with a JSON payload against the local LCU API.
    pub async fn put_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .put(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU PUT {} failed: {}", path, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "LCU PUT {} returned error status: {}",
                path,
                resp.status()
            ));
        }

        resp.json::<T>()
            .await
            .map_err(|e| format!("Failed deserializing LCU PUT JSON for {}: {}", path, e))
    }

    /// Performs an authenticated HTTP PUT request with a JSON payload and returns the HTTP status code.
    pub async fn put_json_status(&self, path: &str, body: &Value) -> Result<u16, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .put(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU PUT {} failed: {}", path, e))?;

        Ok(resp.status().as_u16())
    }

    /// Performs an authenticated HTTP PATCH request with a JSON payload against the local LCU API.
    pub async fn patch_json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .patch(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU PATCH {} failed: {}", path, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "LCU PATCH {} returned error status: {}",
                path,
                resp.status()
            ));
        }

        resp.json::<T>()
            .await
            .map_err(|e| format!("Failed deserializing LCU PATCH JSON for {}: {}", path, e))
    }

    /// Performs an authenticated HTTP PATCH request with a JSON payload and returns the HTTP status code.
    pub async fn patch_json_status(&self, path: &str, body: &Value) -> Result<u16, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .patch(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| format!("LCU PATCH {} failed: {}", path, e))?;

        Ok(resp.status().as_u16())
    }

    /// Performs an authenticated HTTP DELETE request and returns the HTTP status code.
    ///
    /// # Arguments
    /// * `path` - The API endpoint path.
    pub async fn delete_status(&self, path: &str) -> Result<u16, String> {
        let url = format!("{}{}", self.credentials.base_url(), path);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("LCU DELETE {} failed: {}", path, e))?;

        Ok(resp.status().as_u16())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger;

    #[tokio::test]
    async fn test_lcu_discovery() {
        let creds = LcuClient::find_credentials();
        logger::debug(
            "test.lcu-client",
            &format!("LCU credentials discovered: {}", creds.is_some()),
        );
        assert!(
            creds.is_some(),
            "Expected to find League Client credentials from running client or lockfile"
        );
        let client = LcuClient::connect().expect("Failed connecting to LCU");
        let resp: Value = client
            .get_json("/riotclient/region-locale")
            .await
            .expect("Failed calling region-locale");
        logger::debug(
            "test.lcu-client",
            &format!("Region locale response received: {}", resp.is_object()),
        );
    }
}
