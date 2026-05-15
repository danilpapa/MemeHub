use std::time::Duration;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::{error, info, warn};
use crate::Models::events::{AnalysisCompletedEvent, AnalysisEventStatus, AnalysisRequestedEvent};
use crate::services::db::Database;
use crate::services::redis_store::RedisJobStore;

#[derive(Clone)]
pub struct KafkaTopics {
    pub analysis_requested: String,
    pub analysis_completed: String,
}

#[derive(Clone)]
pub struct KafkaService {
    producer: FutureProducer,
    topics: KafkaTopics,
}

impl KafkaService {
    pub fn new(bootstrap_servers: &str, topics: KafkaTopics) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("create kafka producer");

        Self { producer, topics }
    }

    pub async fn publish_analysis_requested(
        &self,
        event: &AnalysisRequestedEvent,
    ) -> Result<(), String> {
        let payload = serde_json::to_string(event)
            .map_err(|error| error.to_string())?;

        self.producer
            .send(
                FutureRecord::to(&self.topics.analysis_requested)
                    .key(&event.job_id)
                    .payload(&payload),
                Duration::from_secs(5),
            )
            .await
            .map(|_| ())
            .map_err(|(error, _)| error.to_string())
    }
}

pub fn spawn_analysis_completed_consumer(
    bootstrap_servers: String,
    topic: String,
    redis: RedisJobStore,
    db: Option<Database>,
) {
    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &bootstrap_servers)
            .set("group.id", "gateway-analysis-completed")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("create kafka consumer");

        consumer
            .subscribe(&[&topic])
            .expect("subscribe to analysis completed topic");

        info!(topic = %topic, "gateway kafka consumer started");

        loop {
            match consumer.recv().await {
                Ok(message) => {
                    let Some(payload) = message.payload() else {
                        warn!("received kafka message without payload");
                        continue;
                    };

                    match serde_json::from_slice::<AnalysisCompletedEvent>(payload) {
                        Ok(event) => {
                            match event.status {
                                AnalysisEventStatus::Completed => {
                                    match event.result {
                                        Some(result) => {
                                            redis.set_completed(&event.job_id, result.clone()).await;
                                            if let Some(ref db) = db {
                                                if let Err(e) = db.complete_job(&event.job_id, &result).await {
                                                    warn!(error = %e, "db complete_job failed");
                                                }
                                            }
                                        }
                                        None => {
                                            let err = "completed event has no result".to_string();
                                            redis.set_failed(&event.job_id, err.clone()).await;
                                            if let Some(ref db) = db {
                                                if let Err(e) = db.fail_job(&event.job_id).await {
                                                    warn!(error = %e, "db fail_job failed");
                                                }
                                            }
                                        }
                                    }
                                }
                                AnalysisEventStatus::Failed => {
                                    let err = event.error
                                        .unwrap_or_else(|| "analysis failed".to_string());
                                    redis.set_failed(&event.job_id, err.clone()).await;
                                    if let Some(ref db) = db {
                                        if let Err(e) = db.fail_job(&event.job_id).await {
                                            warn!(error = %e, "db fail_job failed");
                                        }
                                    }
                                }
                            }

                            if let Err(e) = consumer.commit_message(&message, CommitMode::Async) {
                                warn!(error = %e, "failed to commit kafka message");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "failed to parse analysis completed event");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "gateway kafka consumer error");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });
}
