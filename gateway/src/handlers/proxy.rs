use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::response::IntoResponse;
use crate::Models::AppState::AppState;

pub async fn proxy_ai(
    State(state): State<AppState>,
    Path(path): Path<String>,
    req: Request<Body>,
) -> impl IntoResponse {
    state.proxy.forward(path, req).await
}