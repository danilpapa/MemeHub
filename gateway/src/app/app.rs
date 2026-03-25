use axum::{middleware, Router};
use axum::routing::{any, get};
use tower_http::trace::TraceLayer;
use crate::handlers::metrics::metrics;
use crate::handlers::proxy::proxy_ai;
use crate::middleware::metrics::metrics_middleware;
use crate::Models::AppState::AppState;

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(metrics))
        .route("/ai/*path", any(proxy_ai))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(metrics_middleware))
        .with_state(state)
}