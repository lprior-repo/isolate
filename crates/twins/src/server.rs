#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! HTTP server module for twin runtime
//!
//! Provides an axum-based HTTP server that serves twin endpoints.

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header::HeaderName, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, delete, get, head, options, patch, post, put},
    Router,
};
use thiserror::Error;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

use crate::{
    definition::{Endpoint, HttpMethod, TwinDefinition},
    state::{InMemoryTwinState, RequestRecord, TwinState},
};

/// Errors that can occur in the server
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to parse request body: {0}")]
    BodyParseError(String),
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),
    #[error("Failed to start server: {0}")]
    StartupError(String),
    #[error("Invalid state: {0}")]
    StateError(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let body = self.to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Twin definition
    pub definition: TwinDefinition,
    /// Request/response state
    pub state: Arc<RwLock<InMemoryTwinState>>,
}

impl AppState {
    /// Create new application state
    #[must_use]
    pub fn new(definition: TwinDefinition) -> Self {
        Self {
            definition,
            state: Arc::new(RwLock::new(InMemoryTwinState::new())),
        }
    }

    /// Find matching endpoint for request
    #[must_use]
    pub fn find_endpoint(&self, method: &Method, path: &str) -> Option<&Endpoint> {
        let http_method = match method.as_str() {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            _ => return None,
        };

        self.definition
            .endpoints
            .iter()
            .find(|e| e.method == http_method && e.path == path)
    }
}

/// Handler for twin endpoints
async fn twin_handler(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    // Get path from request URI
    let path = request.uri().path().to_string();

    // Find matching endpoint
    let Some(endpoint) = state.find_endpoint(&method, &path) else {
        return (
            StatusCode::NOT_FOUND,
            format!("No endpoint found for {method} {path}"),
        )
            .into_response();
    };

    // Extract request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {e}"),
            )
                .into_response();
        }
    };

    let request_body_str = if body_bytes.is_empty() {
        None
    } else {
        String::from_utf8(body_bytes.to_vec()).ok()
    };

    // Convert headers to HashMap
    let request_headers: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Build response
    let response = &endpoint.response;
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK);

    let mut builder = Response::builder().status(status);

    // Add response headers
    for (key, value) in &response.headers {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            builder = builder.header(&name, value.as_str());
        }
    }

    // Add response body
    let response_body = serde_json::to_string(&response.body).ok();
    if response_body.is_some() {
        builder = builder.header("content-type", "application/json");
    }

    // Record the request
    let record = RequestRecord::new(
        method.to_string(),
        path,
        request_headers,
        request_body_str,
        response.status,
        response.headers.clone(),
        response_body.clone(),
    );

    // Update state
    let mut state_guard = state.state.write().await;
    let new_state = state_guard.add_record(record);
    *state_guard = new_state;
    drop(state_guard);

    // Return response
    builder
        .body(Body::from(response_body.unwrap_or_default()))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response())
}

/// Handler for 404 Not Found
async fn not_found_handler(method: Method, Path(path): Path<String>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("No endpoint found for {method} {path}"),
    )
}

/// Handler for inspection endpoint - GET /_inspect/state
async fn inspect_state(State(state): State<AppState>) -> impl IntoResponse {
    let records;
    let count;
    {
        let state_guard = state.state.read().await;
        records = state_guard.get_records();
        count = state_guard.record_count();
    }

    let response = serde_json::json!({
        "twin": state.definition.name,
        "port": state.definition.port,
        "request_count": count,
        "requests": records
    });

    (
        StatusCode::OK,
        serde_json::to_string(&response).unwrap_or_default(),
    )
}

/// Handler for inspection endpoint - GET /_inspect/requests
async fn inspect_requests(State(state): State<AppState>) -> impl IntoResponse {
    let records;
    {
        let state_guard = state.state.read().await;
        records = state_guard.get_records();
    }
    let records_vec: Vec<_> = records.into_iter().collect();

    let response = serde_json::json!({
        "requests": records_vec
    });

    (
        StatusCode::OK,
        serde_json::to_string(&response).unwrap_or_default(),
    )
}

/// Handler for clearing state - POST /_inspect/clear
async fn clear_state(State(state): State<AppState>) -> impl IntoResponse {
    let mut state_guard = state.state.write().await;
    *state_guard = InMemoryTwinState::new();
    drop(state_guard);

    (StatusCode::OK, r#"{"status":"cleared"}"#)
}

/// Build the router for the twin server
pub fn build_router(definition: TwinDefinition) -> Router {
    let app_state = AppState::new(definition);

    // Build endpoint routes dynamically
    let mut router = Router::new()
        // Inspection endpoints
        .route("/_inspect/state", get(inspect_state))
        .route("/_inspect/requests", get(inspect_requests))
        .route("/_inspect/clear", post(clear_state));

    // Add twin endpoints
    for endpoint in &app_state.definition.endpoints {
        let path = endpoint.path.clone();
        let method = endpoint.method;

        router = match method {
            HttpMethod::GET => router.route(&path, get(twin_handler)),
            HttpMethod::POST => router.route(&path, post(twin_handler)),
            HttpMethod::PUT => router.route(&path, put(twin_handler)),
            HttpMethod::DELETE => router.route(&path, delete(twin_handler)),
            HttpMethod::PATCH => router.route(&path, patch(twin_handler)),
            HttpMethod::OPTIONS => router.route(&path, options(twin_handler)),
            HttpMethod::HEAD => router.route(&path, head(twin_handler)),
        };
    }

    // Add fallback for 404
    router
        .fallback(any(not_found_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

/// Start the twin server
///
/// # Errors
/// Returns `ServerError` if the server fails to start.
pub async fn start_server(definition: TwinDefinition) -> Result<(), ServerError> {
    let port = definition.port;
    let router = build_router(definition);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ServerError::StartupError(e.to_string()))?;

    tracing::info!("Starting twin server on http://{addr}");

    axum::serve(listener, router)
        .await
        .map_err(|e| ServerError::StartupError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_YAML: &str = r"
name: test-twin
port: 3002
endpoints:
  - path: /api/test
    method: GET
    response:
      status: 200
      body:
        message: 'test response'
  - path: /api/test
    method: POST
    response:
      status: 201
      body:
        created: true
";

    #[test]
    fn test_build_router() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("Should parse");
        let _router = build_router(definition);
        // Router is built successfully
    }

    #[tokio::test]
    async fn test_find_endpoint() {
        let definition = TwinDefinition::from_yaml(TEST_YAML).expect("Should parse");
        let state = AppState::new(definition);

        let endpoint = state.find_endpoint(&Method::GET, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::POST, "/api/test");
        assert!(endpoint.is_some());

        let endpoint = state.find_endpoint(&Method::GET, "/nonexistent");
        assert!(endpoint.is_none());
    }
}
