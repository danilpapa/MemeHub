pub struct Config {
    pub ai_base: String,
    pub otlp_endpoint: String,
    pub service_name: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ai_base: std::env::var("AI_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://jaeger:4317".to_string()),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "gateway".to_string()),
        }
    }
}
