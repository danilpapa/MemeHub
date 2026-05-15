use std::time::Instant;
use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;

pub async fn metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method.clone(),
        "path"   => path.clone(),
        "status" => status.clone(),
    )
    .record(elapsed);

    metrics::counter!(
        "http_requests_total",
        "method" => method,
        "path"   => path,
        "status" => status,
    )
    .increment(1);

    response
}
