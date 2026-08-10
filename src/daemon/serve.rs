use axum::extract::{Query, State};
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use std::sync::Arc;

use crate::config::config::Config;
use crate::download::engine::TorrentEngine;
use crate::download::queue::{AddInput, DownloadQueue};
use crate::sources::parse_input;

pub async fn run_serve(
    port: u16,
    dir: String,
    token: Option<String>,
    config: Config,
) -> anyhow::Result<()> {
    let engine = Arc::new(TorrentEngine::new().await?);
    let queue = Arc::new(DownloadQueue::new(engine));

    let trackers = crate::config::trackers::parse_trackers(&config.trackers.join(","));
    queue.set_trackers(trackers);

    let state = Arc::new(ServeState {
        queue,
        dir,
        token,
        config,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/search", get(search))
        .route("/download", post(download))
        .route("/downloads", get(downloads))
        .route("/seeds", get(seeds))
        .route("/pause/{id}", post(pause))
        .route("/resume/{id}", post(resume))
        .route("/cancel/{id}", post(cancel))
        .route("/remove/{id}", post(remove))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("serve API on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

struct ServeState {
    queue: Arc<DownloadQueue>,
    dir: String,
    token: Option<String>,
    config: Config,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

fn check_auth(state: &ServeState, token: &Option<String>) -> bool {
    match &state.token {
        None => true,
        Some(expected) => token.as_deref() == Some(expected.as_str()),
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    token: Option<String>,
}

async fn search(
    State(state): State<Arc<ServeState>>,
    Query(q): Query<SearchQuery>,
) -> Response {
    if !check_auth(&state, &q.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let sources = crate::sources::all_sources();
    let mut results = vec![];
    let client = crate::util::net::build_client();
    for source in sources {
        if let Ok(r) = source.search(&q.q, &client, None).await {
            results.extend(r);
        }
    }

    let json: Vec<serde_json::Value> = results
        .iter()
        .map(|r| serde_json::json!({
            "name": r.name,
            "source": r.source.tag(),
            "seeders": r.seeders,
            "leechers": r.leechers,
            "size_bytes": r.size_bytes,
            "magnet": r.magnet(),
            "id": r.id(),
        }))
        .collect();

    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], serde_json::to_string(&json).unwrap_or_default()).into_response()
}

#[derive(Deserialize)]
struct DownloadBody {
    input: String,
    name: Option<String>,
}

async fn download(
    State(state): State<Arc<ServeState>>,
    Query(token): Query<TokenQuery>,
    axum::Json(body): axum::Json<DownloadBody>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let Some(parsed) = parse_input(&body.input) else {
        return (StatusCode::BAD_REQUEST, "invalid magnet or torrent file").into_response();
    };

    let name = body.name.unwrap_or(parsed.name.clone());
    state.queue.add(
        AddInput {
            id: parsed.info_hash,
            name,
            magnet: parsed.magnet,
            source: None,
            size_bytes: None,
        },
        &state.dir,
    ).await;

    (StatusCode::OK, "added").into_response()
}

async fn downloads(
    State(state): State<Arc<ServeState>>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let items = state.queue.get_items().await;
    let json = serde_json::to_string(&items).unwrap_or_default();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], json).into_response()
}

async fn seeds(
    State(state): State<Arc<ServeState>>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let seeds = state.queue.get_seeds().await;
    let json = serde_json::to_string(&seeds).unwrap_or_default();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], json).into_response()
}

async fn pause(
    State(state): State<Arc<ServeState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    state.queue.toggle_pause(&id).await;
    (StatusCode::OK, "ok").into_response()
}

async fn resume(
    State(state): State<Arc<ServeState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    state.queue.resume(&id).await;
    (StatusCode::OK, "ok").into_response()
}

async fn cancel(
    State(state): State<Arc<ServeState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    state.queue.cancel(&id).await;
    (StatusCode::OK, "ok").into_response()
}

async fn remove(
    State(state): State<Arc<ServeState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Query(token): Query<TokenQuery>,
) -> Response {
    if !check_auth(&state, &token.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    state.queue.remove(&id, false).await;
    (StatusCode::OK, "ok").into_response()
}
