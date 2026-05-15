use metrics_exporter_prometheus::PrometheusHandle;
use crate::services::db::Database;
use crate::services::kafka::KafkaService;
use crate::services::minio::MinioService;
use crate::services::proxy::ProxyService;
use crate::services::redis_store::RedisJobStore;

#[derive(Clone)]
pub struct AppState {
    pub proxy: ProxyService,
    pub metrics: PrometheusHandle,
    pub kafka: KafkaService,
    pub redis: RedisJobStore,
    pub minio: Option<MinioService>,
    pub db: Option<Database>,
}
