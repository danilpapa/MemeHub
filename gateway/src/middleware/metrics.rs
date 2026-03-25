use std::time::Instant;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;

pub async fn metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let start = Instant::now();
    let response = next.run(req)
        .await;
    let elapsed = start.elapsed().as_secs_f64();

    metrics::histogram!("http_request_duration_seconds")
        .record(elapsed);

    response
}