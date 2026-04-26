//! Error types – mirrors src/api/utils/APIError.js and src/api/middlewares/error.js.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Application error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("{0}")]
    BadRequest(String),

    #[error("Validation Error")]
    Validation { errors: Vec<FieldError> },

    #[error("Validation Error")]
    DuplicateEmail,

    #[error("{0}")]
    Internal(String),

    #[error("User does not exist")]
    UserNotFound,

    #[error("Incorrect email or password")]
    IncorrectCredentials,

    #[error("Incorrect email or refreshToken")]
    IncorrectRefreshToken,
}

/// A single field-level validation error.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub location: String,
    pub messages: Vec<String>,
}

impl FieldError {
    pub fn new(field: impl Into<String>, location: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            field: field.into(),
            location: location.into(),
            messages,
        }
    }
}

/// JSON error response body.
#[derive(serde::Serialize)]
struct ErrorBody {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Vec<FieldError>>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, errors) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), None),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, self.to_string(), None),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::IncorrectCredentials => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::IncorrectRefreshToken => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string(), None),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), None),
            AppError::Validation { errors: field_errors } => {
                (StatusCode::BAD_REQUEST, self.to_string(), Some(field_errors.clone()))
            }
            AppError::DuplicateEmail => (
                StatusCode::CONFLICT,
                "Validation Error".into(),
                Some(vec![FieldError::new("email", "body", vec!["\"email\" already exists".into()])]),
            ),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), None),
        };

        let body = ErrorBody {
            code: status.as_u16(),
            message,
            errors,
        };

        (status, Json(body)).into_response()
    }
}

/// Convert sqlx::Error into AppError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::UserNotFound,
            _ => AppError::Internal(err.to_string()),
        }
    }
}
