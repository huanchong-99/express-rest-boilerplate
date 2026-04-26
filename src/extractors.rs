//! Custom extractors for Axum.
//!
//! ValidatedJson: Deserializes JSON body and validates using the validator crate.
//! ValidatedQuery: Deserializes query params and validates using the validator crate.

use axum::extract::FromRequest;
use axum::http::Request;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::errors::{AppError, FieldError};

/// A JSON extractor that also validates the deserialized value.
pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(request, state)
            .await
            .map_err(|e| AppError::BadRequest(e.body_text()))?;

        value.validate().map_err(validation_errors_to_app_error)?;

        Ok(ValidatedJson(value))
    }
}

/// Validate a struct that implements `Validate` and convert errors to `AppError`.
/// This is used by handlers that receive raw `Json<T>` and then validate manually.
pub fn validate_to_app_error<T: Validate>(value: &T) -> Result<(), AppError> {
    value.validate().map_err(validation_errors_to_app_error)
}

fn validation_errors_to_app_error(err: validator::ValidationErrors) -> AppError {
    let field_errors: Vec<FieldError> = err
        .field_errors()
        .into_iter()
        .map(|(field, errors)| {
            let messages: Vec<String> = errors
                .iter()
                .map(|e| {
                    let code = e.code.as_ref();
                    match code {
                        "email" => format!("\"{}\" must be a valid email", field),
                        "length" => {
                            let min = e.params.get("min").and_then(|v| v.as_u64());
                            let max = e.params.get("max").and_then(|v| v.as_u64());
                            match (min, max) {
                                (Some(min), None) => {
                                    format!("\"{}\" must be at least {} characters", field, min)
                                }
                                (None, Some(max)) => {
                                    format!("\"{}\" must be at most {} characters", field, max)
                                }
                                (Some(min), Some(max)) => {
                                    format!(
                                        "\"{}\" length must be between {} and {} characters",
                                        field, min, max
                                    )
                                }
                                _ => format!("\"{}\" has invalid length", field),
                            }
                        }
                        "required" => format!("\"{}\" is required", field),
                        _ => format!(
                            "\"{}\" {}",
                            field,
                            e.message.clone().unwrap_or_else(|| "is invalid".into())
                        ),
                    }
                })
                .collect();
            FieldError::new(field, "body", messages)
        })
        .collect();
    AppError::Validation { errors: field_errors }
}

/// A query parameter extractor that also validates the deserialized value.
pub struct ValidatedQuery<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(request: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) = axum::extract::Query::<T>::from_request(request, state)
            .await
            .map_err(|e| AppError::Validation {
                errors: vec![FieldError::new("query", "query", vec![e.body_text()])],
            })?;

        value.validate().map_err(query_validation_errors_to_app_error)?;

        Ok(ValidatedQuery(value))
    }
}

fn query_validation_errors_to_app_error(err: validator::ValidationErrors) -> AppError {
    let field_errors: Vec<FieldError> = err
        .field_errors()
        .into_iter()
        .map(|(field, errors)| {
            let messages: Vec<String> = errors
                .iter()
                .map(|e| {
                    let code = e.code.as_ref();
                    match code {
                        "range" => format!("\"{}\" must be a number", field),
                        _ => format!("\"{}\" is invalid", field),
                    }
                })
                .collect();
            FieldError::new(field, "query", messages)
        })
        .collect();
    AppError::Validation { errors: field_errors }
}
