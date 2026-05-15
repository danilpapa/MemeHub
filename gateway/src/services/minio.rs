use std::sync::Arc;
use std::time::Duration;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Builder as S3Builder, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;

#[derive(Clone)]
pub struct MinioService {
    client: Arc<Client>,
    bucket: String,
}

impl MinioService {
    pub fn new(endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> Self {
        let creds = Credentials::new(access_key, secret_key, None, None, "minio");
        let config = S3Builder::new()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(creds)
            .force_path_style(true)
            .build();
        Self {
            client: Arc::new(Client::from_conf(config)),
            bucket: bucket.to_owned(),
        }
    }

    pub async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<(), String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub async fn presigned_url(&self, key: &str, expiry_secs: u64) -> Result<String, String> {
        let cfg = PresigningConfig::expires_in(Duration::from_secs(expiry_secs))
            .map_err(|e| e.to_string())?;
        let presigned = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(cfg)
            .await
            .map_err(|e| e.to_string())?;
        Ok(presigned.uri().to_string())
    }
}
