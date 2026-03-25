use metrics_exporter_prometheus::PrometheusHandle;

pub fn init_metrics() -> PrometheusHandle {
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder");
    
    metrics_handle
}