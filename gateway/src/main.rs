use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::any;
use reqwest::Client;

#[derive(Clone)]
struct AppState {
    client: Client,
    ai_base: String,
}

#[tokio::main]
async fn main() {
    let ai_base = std::env::var("AI_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let state = AppState {
        client: Client::new(),
        ai_base: ai_base,
    };

    let app = Router::new()
        .route("/ai/*path", any(proxy_ai))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
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