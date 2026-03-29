use std::time::Duration;
use axum::{extract::MatchedPath, middleware, Router};
use axum::http::Request;
use axum::routing::{any, get};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnFailure, TraceLayer};
use tracing::{Level, Span, field};
use crate::handlers::metrics::metrics;
use crate::handlers::proxy::proxy_ai;
use crate::middleware::metrics::metrics_middleware;
use crate::Models::AppState::AppState;

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(metrics))
        .route("/ai/*path", any(proxy_ai))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str)
                        .unwrap_or_else(|| request.uri().path());
                    let request_id = request
                        .headers()
                        .get(REQUEST_ID_HEADER)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("unknown");

                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        matched_path = %matched_path,
                        request_id = %request_id,
                        status_code = field::Empty,
                        latency_ms = field::Empty,
                    )
                })
                .on_request(())
                .on_response(|response: &axum::http::Response<_>, latency: Duration, span: &Span| {
                    span.record("status_code", response.status().as_u16());
                    span.record("latency_ms", latency.as_millis() as u64);

                    tracing::event!(
                        parent: span,
                        Level::INFO,
                        status_code = response.status().as_u16(),
                        latency_ms = latency.as_millis() as u64,
                        "request completed"
                    );
                })
                .on_failure(
                    DefaultOnFailure::new()
                        .level(Level::ERROR)
                )
        )
        .layer(PropagateRequestIdLayer::new(http::HeaderName::from_static(REQUEST_ID_HEADER)))
        .layer(SetRequestIdLayer::new(
            http::HeaderName::from_static(REQUEST_ID_HEADER),
            MakeRequestUuid,
        ))
        .layer(middleware::from_fn(metrics_middleware))
        .with_state(state)
}
