use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use base64::{engine::general_purpose, Engine as _};
use multer::Multipart;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Models::AppState::AppState;
use crate::Models::events::{AnalysisRequestedEvent, ImageSource};
use crate::services::minio::MinioService;

const MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct AnalysisUrlRequest {
    image_url: String,
}

#[derive(Debug, Serialize)]
struct EnqueueResponse {
    job_id: String,
    status: String,
}

pub async fn enqueue_analysis(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request<Body>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4().to_string();

    let image = match parse_image_source(headers, req).await {
        Ok(image) => image,
        Err(response) => return response,
    };

    // Upload file to MinIO and replace with presigned URL when MinIO is configured.
    // Destructure without `ref` so fields are owned and can be moved or reconstructed.
    let (image, image_ref) = match image {
        ImageSource::File { filename, content_type, data_base64 } => {
            if let Some(ref minio) = state.minio {
                match upload_to_minio(minio, &job_id, filename.as_deref(), content_type.as_deref(), &data_base64).await {
                    Ok(url) => {
                        let image_ref = url.clone();
                        (ImageSource::Url { image_url: url }, Some(image_ref))
                    }
                    Err(err) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "error": format!("upload failed: {err}") })),
                        ).into_response();
                    }
                }
            } else {
                (ImageSource::File { filename, content_type, data_base64 }, None)
            }
        }
        ImageSource::Url { image_url } => {
            let image_ref = image_url.clone();
            (ImageSource::Url { image_url }, Some(image_ref))
        }
    };

    state.redis.set_queued(&job_id).await;

    if let Some(ref db) = state.db {
        if let Err(e) = db.insert_job(&job_id, image_ref.as_deref()).await {
            tracing::warn!(error = %e, "db insert_job failed");
        }
    }

    let event = AnalysisRequestedEvent {
        job_id: job_id.clone(),
        image,
    };

    if let Err(error) = state.kafka.publish_analysis_requested(&event).await {
        state.redis.set_failed(&job_id, error.clone()).await;
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "job_id": job_id,
                "status": "failed",
                "error": error,
            })),
        ).into_response();
    }

    (
        StatusCode::ACCEPTED,
        Json(EnqueueResponse {
            job_id,
            status: "queued".to_string(),
        }),
    ).into_response()
}

pub async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    match state.redis.get(&job_id).await {
        Some(job) => (StatusCode::OK, Json(job)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "job not found",
                "job_id": job_id,
            })),
        ).into_response(),
    }
}

async fn upload_to_minio(
    minio: &MinioService,
    job_id: &str,
    filename: Option<&str>,
    content_type: Option<&str>,
    data_base64: &str,
) -> Result<String, String> {
    let bytes = general_purpose::STANDARD
        .decode(data_base64)
        .map_err(|e| format!("base64 decode: {e}"))?;
    let name = filename.unwrap_or("image.bin");
    let key = format!("uploads/{job_id}/{name}");
    let ct = content_type.unwrap_or("application/octet-stream");
    minio.upload(&key, bytes, ct).await?;
    minio.presigned_url(&key, 3600).await
}

async fn parse_image_source(
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<ImageSource, axum::response::Response> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let (_, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_UPLOAD_BYTES)
        .await
        .map_err(|_| {
            (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(serde_json::json!({ "error": "request body is too large" })),
            )
                .into_response()
        })?;

    if content_type.starts_with("multipart/form-data") {
        let boundary = multer::parse_boundary(content_type)
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid multipart boundary" })),
                )
                    .into_response()
            })?;
        let stream = futures_util::stream::once(async move {
            Ok::<_, std::io::Error>(bytes)
        });
        let mut multipart = Multipart::new(stream, boundary);

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid multipart body" })),
                )
                    .into_response()
            })?
        {
            if field.name() != Some("file") {
                continue;
            }

            let filename = field.file_name().map(ToString::to_string);
            let content_type = field.content_type().map(ToString::to_string);
            let data = field
                .bytes()
                .await
                .map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": "failed to read uploaded file" })),
                    )
                        .into_response()
                })?;

            return Ok(ImageSource::File {
                filename,
                content_type,
                data_base64: general_purpose::STANDARD.encode(data),
            });
        }

        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "multipart field 'file' is required" })),
        )
            .into_response());
    }

    if content_type.starts_with("application/json") {
        let payload = serde_json::from_slice::<AnalysisUrlRequest>(&bytes)
            .map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid json body" })),
                )
                    .into_response()
            })?;

        return Ok(ImageSource::Url {
            image_url: payload.image_url,
        });
    }

    Err((
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        Json(serde_json::json!({
            "error": "send multipart/form-data with file or application/json with image_url"
        })),
    )
        .into_response())
}
