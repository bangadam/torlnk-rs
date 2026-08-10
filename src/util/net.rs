use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

pub const USER_AGENT: &str = "torlnk-rs (+https://github.com/baairon/torlnk-rs)";

#[derive(Debug, thiserror::Error)]
#[error("HTTP {status}: {message}")]
pub struct HttpError {
    pub status: u16,
    pub message: String,
}

impl HttpError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

const RETRY_STATUS: &[u16] = &[408, 425, 429, 500, 502, 503, 504];
const DEFAULT_RETRIES: u32 = 5;
const DEFAULT_BASE_MS: u64 = 500;
const DEFAULT_CAP_MS: u64 = 20_000;

fn backoff_delay(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    let exp = (cap_ms as f64).min(base_ms as f64 * 2f64.powi(attempt as i32));
    let jittered = (rand_jitter() * exp) as u64;
    jittered
}

fn rand_jitter() -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    (h.finish() as f64) / (u64::MAX as f64)
}

/// Build a reqwest client with sensible defaults for source scraping.
pub fn build_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .gzip(true)
        .brotli(true)
        .build()
        .expect("failed to build HTTP client")
}

/// Fetch a URL with retry + exponential backoff. Retries on network errors and
/// retryable HTTP status codes (408, 425, 429, 500, 502, 503, 504).
/// Returns the Response on success, or the last error.
pub async fn fetch_resilient(
    client: &Client,
    url: &str,
    retries: u32,
) -> Result<Response, HttpError> {
    fetch_resilient_with(client, url, retries, None).await
}

/// Fetch with an optional cancellation token.
pub async fn fetch_resilient_with(
    client: &Client,
    url: &str,
    retries: u32,
    cancel: Option<&tokio_util::sync::CancellationToken>,
) -> Result<Response, HttpError> {
    let retries = if retries == 0 { DEFAULT_RETRIES } else { retries };

    let mut last_error = HttpError::new(0, "no request attempted");

    for attempt in 0..=retries {
        if let Some(token) = cancel {
            if token.is_cancelled() {
                return Err(HttpError::new(0, "aborted"));
            }
        }

        let result = client.get(url).send().await;
        match result {
            Ok(res) => {
                let status = res.status().as_u16();
                if !RETRY_STATUS.contains(&status) {
                    return Ok(res);
                }

                // Check for DDoS-Guard / Cloudflare on 503
                if status == 503 {
                    if let Some(server) = res.headers().get("server") {
                        let server = server.to_str().unwrap_or("").to_lowercase();
                        if server.contains("ddos-guard") || server.contains("cloudflare") {
                            return Err(HttpError::new(
                                status,
                                format!("Request to {} blocked by {} (HTTP {})", url, server, status),
                            ));
                        }
                    }
                }

                if attempt >= retries {
                    return Err(HttpError::new(
                        status,
                        format!(
                            "Request to {} failed after {} retries (HTTP {})",
                            url, retries, status
                        ),
                    ));
                }

                last_error = HttpError::new(status, format!("HTTP {}", status));
            }
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    last_error = HttpError::new(0, e.to_string());
                } else {
                    return Err(HttpError::new(0, e.to_string()));
                }
                if attempt >= retries {
                    return Err(HttpError::new(0, e.to_string()));
                }
            }
        }

        let delay = backoff_delay(attempt, DEFAULT_BASE_MS, DEFAULT_CAP_MS);
        tokio::select! {
            _ = sleep(Duration::from_millis(delay)) => {}
            _ = tokio::signal::ctrl_c() => return Err(HttpError::new(0, "aborted")),
        }
    }

    Err(last_error)
}

/// Helper: fetch URL and return text, with retry.
pub async fn fetch_text(
    client: &Client,
    url: &str,
    retries: u32,
) -> Result<String, HttpError> {
    let res = fetch_resilient(client, url, retries).await?;
    let status = res.status();
    if !status.is_success() {
        return Err(HttpError::new(
            status.as_u16(),
            format!("HTTP {}", status),
        ));
    }
    res.text()
        .await
        .map_err(|e| HttpError::new(0, e.to_string()))
}

/// Helper: fetch URL and return JSON, with retry.
pub async fn fetch_json<T: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    retries: u32,
) -> Result<T, HttpError> {
    let res = fetch_resilient(client, url, retries).await?;
    let status = res.status();
    if !status.is_success() {
        return Err(HttpError::new(
            status.as_u16(),
            format!("HTTP {}", status),
        ));
    }
    res.json::<T>()
        .await
        .map_err(|e| HttpError::new(0, e.to_string()))
}
