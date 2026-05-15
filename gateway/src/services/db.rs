use sqlx::PgPool;
use crate::Models::events::AnalysisResult;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }

    pub async fn insert_job(&self, job_id: &str, image_ref: Option<&str>) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO meme_jobs (job_id, status, image_ref) VALUES ($1, 'queued', $2) \
             ON CONFLICT (job_id) DO NOTHING",
        )
        .bind(job_id)
        .bind(image_ref)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub async fn complete_job(&self, job_id: &str, result: &AnalysisResult) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE meme_jobs SET status = 'completed' WHERE job_id = $1")
            .bind(job_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO meme_results (job_id, ocr_text, tags, emotion) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (job_id) DO NOTHING",
        )
        .bind(job_id)
        .bind(&result.ocr_text)
        .bind(&result.tags)
        .bind(&result.emotion)
        .execute(&mut *tx)
        .await?;

        tx.commit().await
    }

    pub async fn fail_job(&self, job_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE meme_jobs SET status = 'failed' WHERE job_id = $1")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}
