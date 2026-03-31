use std::path::PathBuf;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use jsonrpc_core::IoHandler;

pub fn serve(io: IoHandler, web_root: PathBuf, addr: SocketAddr) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async move {
        let state = Arc::new(AppState { io, web_root });
        let app = Router::new()
            .route("/rpc", post(rpc_handler))
            .route("/", get(static_handler))
            .route("/*path", get(static_handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(addr).await
            .expect("failed to bind server");
        axum::serve(listener, app).await.expect("server error");
    });
}

struct AppState {
    io: IoHandler,
    web_root: PathBuf,
}

async fn rpc_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Response {
    match state.io.handle_request_sync(&body) {
        Some(resp) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            resp,
        ).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn static_handler(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let raw = req.uri().path();
    let rel = if raw == "/" { "index.html" } else { raw.trim_start_matches('/') };

    if rel.contains("..") {
        return StatusCode::FORBIDDEN.into_response();
    }

    let path = state.web_root.join(rel);
    match tokio::fs::read(&path).await {
        Ok(data) => {
            let ct = content_type(rel);
            ([(header::CONTENT_TYPE, ct)], data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html")     { "text/html; charset=utf-8" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".js")  { "application/javascript" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else                           { "application/octet-stream" }
}
