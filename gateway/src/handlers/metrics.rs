use axum::{extract::State, response::IntoResponse};
use crate::Models::AppState::AppState;

pub async fn metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    state.metrics.render()
}