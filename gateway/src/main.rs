use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{any, get};
use axum::Router;
use metrics_exporter_prometheus::PrometheusHandle;
use reqwest::Client;
use std::fs;
use std::time::Instant;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Clone)]
struct AppState {
    client: Client,
    ai_base: String,
    metrics_handle: PrometheusHandle,
}

#[tokio::main]
async fn main() {
    fs::create_dir_all("logs").expect("create logs dir");
    let file_appender = tracing_appender::rolling::never("logs", "gateway.jsonl");
    let (non_blocking, _guard): (_, WorkerGuard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with_writer(non_blocking)
        .json()
        .init();

    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder");

    let ai_base = std::env::var("AI_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let state = AppState {
        client: Client::new(),
        ai_base: ai_base,
        metrics_handle,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/ai/*path", any(proxy_ai))
        .layer(middleware::from_fn(metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn health() -> impl IntoResponse {
    "ok"
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    state.metrics_handle.render()
}

async fn metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();

    tracing::info!(
        method = %method,
        path = %path,
        status = %status,
        elapsed_sec = elapsed,
        "request"
    );

    let labels = [
        ("method", method.clone()),
        ("path", path.clone()),
        ("status", status.clone()),
    ];
    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_request_duration_seconds", &labels).record(elapsed);

    response
}

async fn proxy_ai(
    State(state): State<AppState>,
    Path(path): Path<String>,
    req: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let headers = parts.headers;
    let query = parts.uri.query().map(|q| format!("?{q}")).unwrap_or_default();

    let target = format!("{}/{}{}", state.ai_base, path, query);

    let mut out = state.client.request(method, target);

    for (name, val) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host") {
            continue;
        }
        out = out.header(name, val);
    }

    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();
    let res = out.body(bytes).send().await;

    match res {
        Ok(resp) => {
            let status = resp.status();
            let mut out_headers = HeaderMap::new();
            for (k, v) in resp.headers().iter() {
                out_headers.insert(k, v.clone());
            }
            let body = resp.bytes().await.unwrap_or_default();
            (status, out_headers, body).into_response()
        }
        Err(_) => (StatusCode::BAD_GATEWAY, "upstream error").into_response(),
    }
}
