use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequestedEvent {
    pub job_id: String,
    pub image: ImageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    File {
        filename: Option<String>,
        content_type: Option<String>,
        data_base64: String,
    },
    Url {
        image_url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisCompletedEvent {
    pub job_id: String,
    pub status: AnalysisEventStatus,
    pub result: Option<AnalysisResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisEventStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub ocr_text: String,
    pub tags: Vec<String>,
    pub emotion: String,
}
