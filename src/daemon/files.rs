use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

pub async fn run_files(port: u16, dir: String, token: Option<String>) -> anyhow::Result<()> {
    let state = Arc::new(FilesState { dir, token });

    let app = Router::new()
        .route("/", get(list_files))
        .route("/{*path}", get(serve_file))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("files server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

struct FilesState {
    dir: String,
    token: Option<String>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

fn check_auth(state: &FilesState, token: &Option<String>) -> bool {
    match &state.token {
        None => true,
        Some(expected) => token.as_deref() == Some(expected.as_str()),
    }
}

async fn list_files(
    State(state): State<Arc<FilesState>>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let root = PathBuf::from(&state.dir);
    let mut entries = vec![];

    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            entries.push(serde_json::json!({
                "name": name,
                "is_dir": is_dir,
            }));
        }
    }

    entries.sort_by(|a, b| {
        let a_dir = a["is_dir"].as_bool().unwrap_or(false);
        let b_dir = b["is_dir"].as_bool().unwrap_or(false);
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                a["name"].as_str().cmp(&b["name"].as_str())
            }
        }
    });

    let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json,
    ).into_response()
}

async fn serve_file(
    State(state): State<Arc<FilesState>>,
    Path(path): Path<String>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let full = PathBuf::from(&state.dir).join(&path);

    // Prevent path traversal
    let canonical = match tokio::fs::canonicalize(&full).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };
    let root = match tokio::fs::canonicalize(&state.dir).await {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "error").into_response(),
    };
    if !canonical.starts_with(&root) {
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }

    if canonical.is_dir() {
        // List directory
        let mut entries = vec![];
        if let Ok(mut rd) = tokio::fs::read_dir(&canonical).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                entries.push(serde_json::json!({ "name": name, "is_dir": is_dir }));
            }
        }
        let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json,
        ).into_response();
    }

    match tokio::fs::read(&canonical).await {
        Ok(data) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/octet-stream")],
            data,
        ).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
