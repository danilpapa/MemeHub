use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub fn init_logging() -> WorkerGuard {
    std::fs::create_dir_all("./logs")
        .expect("Failed to create logging folder");
    let file_appender = tracing_appender::rolling::never(
        "logs",
        "gateway.jsonl"
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(
                    "info"
                        .parse()
                        .unwrap()
                )
        )
        .with_writer(non_blocking)
        .json()
        .init();
    
    _guard
}