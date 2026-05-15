pub struct Config {
    pub ai_base: String,
    pub otlp_endpoint: String,
    pub service_name: String,
    pub kafka_bootstrap_servers: String,
    pub analysis_requested_topic: String,
    pub analysis_completed_topic: String,
    pub redis_url: String,
    pub database_url: Option<String>,
    pub minio_endpoint: Option<String>,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub minio_bucket: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ai_base: std::env::var("AI_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://jaeger:4317".to_string()),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "gateway".to_string()),
            kafka_bootstrap_servers: std::env::var("KAFKA_BOOTSTRAP_SERVERS")
                .unwrap_or_else(|_| "localhost:19092".to_string()),
            analysis_requested_topic: std::env::var("KAFKA_ANALYSIS_REQUESTED_TOPIC")
                .unwrap_or_else(|_| "meme.analysis.requested".to_string()),
            analysis_completed_topic: std::env::var("KAFKA_ANALYSIS_COMPLETED_TOPIC")
                .unwrap_or_else(|_| "meme.analysis.completed".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            database_url: std::env::var("DATABASE_URL").ok(),
            minio_endpoint: std::env::var("MINIO_ENDPOINT").ok(),
            minio_access_key: std::env::var("MINIO_ACCESS_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            minio_secret_key: std::env::var("MINIO_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            minio_bucket: std::env::var("MINIO_BUCKET")
                .unwrap_or_else(|_| "memes".to_string()),
        }
    }
}
