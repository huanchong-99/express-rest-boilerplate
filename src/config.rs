use std::env;

/// Application configuration loaded from environment variables.
/// Mirrors the original Express.js config from src/config/app.js:
///   NODE_ENV, PORT, JWT_SECRET, JWT_EXPIRATION_MINUTES, MONGO_URI
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_minutes: i64,
    pub port: u16,
    pub host: String,
    pub env: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// Required env vars:
    ///   DATABASE_URL       – PostgreSQL connection string (replaces MONGO_URI)
    ///   JWT_SECRET         – Secret for signing JWT tokens
    ///   JWT_EXPIRATION_MINUTES – Token lifetime in minutes (default: 15)
    ///   PORT               – Server port (default: 3000)
    ///   HOST               – Server host (default: 0.0.0.0)
    ///   NODE_ENV           – Environment name (default: development)
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            jwt_expiration_minutes: env::var("JWT_EXPIRATION_MINUTES")
                .unwrap_or_else(|_| "15".into())
                .parse()
                .expect("JWT_EXPIRATION_MINUTES must be a number"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("PORT must be a number"),
            host: env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".into()),
            env: env::var("NODE_ENV")
                .unwrap_or_else(|_| "development".into()),
        }
    }
}
