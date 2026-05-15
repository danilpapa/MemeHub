use reqwest::Client;
use crate::app::app::create_app;
use crate::Models::AppState::AppState;
use crate::services::db::Database;
use crate::services::kafka::{spawn_analysis_completed_consumer, KafkaService, KafkaTopics};
use crate::services::minio::MinioService;
use crate::services::proxy::ProxyService;
use crate::services::redis_store::RedisJobStore;

mod Models;
mod observability;
mod config;
mod services;
mod handlers;
mod middleware;
mod app;

#[tokio::main]
async fn main() {
    let config = config::Config::from_env();
    let _guard = observability::logging::init_logging(
        &config.service_name,
        &config.otlp_endpoint,
    );
    let metrics = observability::metrics::init_metrics();
    let client = Client::new();

    let proxy = ProxyService::new(client, config.ai_base);
    let topics = KafkaTopics {
        analysis_requested: config.analysis_requested_topic.clone(),
        analysis_completed: config.analysis_completed_topic.clone(),
    };
    let kafka = KafkaService::new(&config.kafka_bootstrap_servers, topics.clone());
    let redis = RedisJobStore::new(&config.redis_url).await;

    let db = if let Some(ref url) = config.database_url {
        match Database::new(url).await {
            Ok(db) => {
                db.run_migrations().await.expect("run postgres migrations");
                Some(db)
            }
            Err(e) => {
                tracing::warn!(error = %e, "postgres unavailable — db disabled");
                None
            }
        }
    } else {
        None
    };

    let minio = config.minio_endpoint.as_deref().map(|endpoint| {
        MinioService::new(
            endpoint,
            &config.minio_access_key,
            &config.minio_secret_key,
            &config.minio_bucket,
        )
    });

    spawn_analysis_completed_consumer(
        config.kafka_bootstrap_servers,
        topics.analysis_completed,
        redis.clone(),
        db.clone(),
    );

    let state = AppState {
        proxy,
        metrics,
        kafka,
        redis,
        minio,
        db,
    };

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}
