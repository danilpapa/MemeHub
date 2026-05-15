CREATE TABLE IF NOT EXISTS meme_jobs (
    job_id     TEXT        PRIMARY KEY,
    status     TEXT        NOT NULL,
    image_ref  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS meme_results (
    job_id       TEXT        PRIMARY KEY REFERENCES meme_jobs (job_id),
    ocr_text     TEXT        NOT NULL,
    tags         TEXT[]      NOT NULL,
    emotion      TEXT        NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
