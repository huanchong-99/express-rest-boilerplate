use std::env;

/// Application configuration loaded from environment variables.
/// Mirrors the original Express.js config from src/config/app.js:
///   NODE_ENV, PORT, JWT_SECRET, JWT_EXPIRATION_MINUTES, MONGO_URI
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub token_signing_key: String,
    pub jwt_expiration_minutes: i64,
    pub port: u16,
    pub host: String,
    pub env: String,
}

/// Error type for configuration loading failures.
#[derive(Debug)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Configuration error: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

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
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError("DATABASE_URL must be set".into()))?;
        let token_signing_key = env::var("JWT_SECRET")
            .map_err(|_| ConfigError("JWT_SECRET must be set".into()))?;
        let jwt_expiration_minutes: i64 = env::var("JWT_EXPIRATION_MINUTES")
            .unwrap_or_else(|_| "15".into())
            .parse()
            .map_err(|_| ConfigError("JWT_EXPIRATION_MINUTES must be a number".into()))?;
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .map_err(|_| ConfigError("PORT must be a number".into()))?;
        let host = env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".into());
        let env_name = env::var("NODE_ENV")
            .unwrap_or_else(|_| "development".into());

        Ok(Self {
            database_url,
            token_signing_key,
            jwt_expiration_minutes,
            port,
            host,
            env: env_name,
        })
    }
}
