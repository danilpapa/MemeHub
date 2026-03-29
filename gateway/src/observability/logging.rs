use tracing_appender::non_blocking::WorkerGuard;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use tracing_appender::non_blocking;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging(service_name: &str, otlp_endpoint: &str) -> WorkerGuard {
    std::fs::create_dir_all("./logs")
        .expect("Failed to create logging folder");
    let file_appender = tracing_appender::rolling::never(
        "logs",
        "gateway.jsonl"
    );
    let (non_blocking, _guard) = non_blocking(file_appender);
    let env_filter = EnvFilter::from_default_env()
        .add_directive(
            "info"
                .parse()
                .unwrap()
        );
    let tracer_provider = build_tracer_provider(service_name, otlp_endpoint);
    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer(service_name.to_string());
    let fmt_layer = fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_current_span(true)
        .with_span_list(true);
    let otel_layer = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();
    
    _guard
}

fn build_tracer_provider(service_name: &str, otlp_endpoint: &str) -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .expect("Failed to create OTLP exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter, Tokio)
        .with_resource(Resource::new([
            KeyValue::new("service.name", service_name.to_string()),
        ]))
        .build()
}
