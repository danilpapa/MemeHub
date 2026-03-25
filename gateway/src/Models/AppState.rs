use metrics_exporter_prometheus::PrometheusHandle;
use crate::services::proxy::ProxyService;

#[derive(Clone)]
pub struct AppState {
    pub proxy: ProxyService,
    pub metrics: PrometheusHandle,
}