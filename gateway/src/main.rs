use reqwest::Client;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use crate::app::app::create_app;
use crate::Models::AppState::AppState;
use crate::services::proxy::ProxyService;

mod Models;
mod observability;
mod config;
mod services;
mod handlers;
mod middleware;
mod app;

#[tokio::main]
async fn main() {
    let _guard = observability::logging::init_logging();
    let metrics = observability::metrics::init_metrics();

    let config = config::Config::from_env();
    let client = Client::new();

    let proxy = ProxyService::new(client, config.ai_base);

    let state = AppState {
        proxy,
        metrics
    };

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}
