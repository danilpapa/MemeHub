pub struct Config {
    pub ai_base: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ai_base: std::env::var("AI_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }
}