use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use crate::Models::events::AnalysisResult;

pub type JobStore = Arc<RwLock<HashMap<String, JobRecord>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub status: JobStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<AnalysisResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Completed,
    Failed,
}

impl JobRecord {
    pub fn queued(job_id: String) -> Self {
        Self {
            job_id,
            status: JobStatus::Queued,
            result: None,
            error: None,
        }
    }

    pub fn completed(job_id: String, result: AnalysisResult) -> Self {
        Self {
            job_id,
            status: JobStatus::Completed,
            result: Some(result),
            error: None,
        }
    }

    pub fn failed(job_id: String, error: String) -> Self {
        Self {
            job_id,
            status: JobStatus::Failed,
            result: None,
            error: Some(error),
        }
    }
}
