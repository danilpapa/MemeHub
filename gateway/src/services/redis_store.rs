use redis::{AsyncCommands, Client, aio::ConnectionManager};
use tracing::error;
use crate::Models::events::AnalysisResult;
use crate::Models::jobs::JobRecord;

const JOB_TTL_SECS: u64 = 86_400; // 24h

#[derive(Clone)]
pub struct RedisJobStore {
    conn: ConnectionManager,
}

impl RedisJobStore {
    pub async fn new(url: &str) -> Self {
        let client = Client::open(url).expect("invalid redis URL");
        let conn = ConnectionManager::new(client)
            .await
            .expect("connect to redis");
        Self { conn }
    }

    pub async fn set_queued(&self, job_id: &str) {
        self.write(job_id, &JobRecord::queued(job_id.to_owned())).await;
    }

    pub async fn set_completed(&self, job_id: &str, result: AnalysisResult) {
        self.write(job_id, &JobRecord::completed(job_id.to_owned(), result)).await;
    }

    pub async fn set_failed(&self, job_id: &str, err: String) {
        self.write(job_id, &JobRecord::failed(job_id.to_owned(), err)).await;
    }

    pub async fn get(&self, job_id: &str) -> Option<JobRecord> {
        let mut conn = self.conn.clone();
        let raw: Option<String> = conn.get(job_id).await.unwrap_or(None);
        raw.and_then(|s| serde_json::from_str(&s).ok())
    }

    async fn write(&self, job_id: &str, record: &JobRecord) {
        let json = match serde_json::to_string(record) {
            Ok(v) => v,
            Err(e) => {
                error!(error = %e, "serialize job record");
                return;
            }
        };
        let mut conn = self.conn.clone();
        if let Err(e) = conn.set_ex::<_, _, ()>(job_id, json, JOB_TTL_SECS).await {
            error!(error = %e, job_id = %job_id, "redis set_ex failed");
        }
    }
}
