//! Custom extractors for validated JSON body and query parameters.

use axum::async_trait;
use axum::extract::FromRequest;
use axum::Json;
use validator::Validate;

use crate::errors::{AppError, FieldError};

/// A JSON body extractor that runs validator::Validate on the deserialized type.
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: Validate + serde::de::DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|rejection| {
                let msg = rejection.body_text();
                AppError::BadRequest(msg)
            })?;

        value.validate().map_err(|e| {
            let errors = validation_errors_to_field_errors(e);
            AppError::Validation { errors }
        })?;

        Ok(ValidatedJson(value))
    }
}

/// A query parameter extractor that runs validator::Validate on the deserialized type.
#[allow(dead_code)]
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: Validate + serde::de::DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) = axum::extract::Query::<T>::from_request(req, state)
            .await
            .map_err(|rejection| {
                let msg = rejection.body_text();
                AppError::BadRequest(msg)
            })?;

        value.validate().map_err(|e| {
            let errors = validation_errors_to_field_errors(e);
            AppError::Validation { errors }
        })?;

        Ok(ValidatedQuery(value))
    }
}

/// Convert validator::ValidationErrors into our FieldError format.
fn validation_errors_to_field_errors(
    ve: validator::ValidationErrors,
) -> Vec<FieldError> {
    let mut field_errors = Vec::new();
    for (field_name, field_errs) in ve.field_errors() {
        let messages: Vec<String> = field_errs
            .iter()
            .map(|e| {
                if let Some(msg) = e.message.as_ref() {
                    format!("\"{}\" {}", field_name, msg)
                } else {
                    match e.code.as_ref() {
                        "email" => format!("\"{}\" must be a valid email", field_name),
                        "length" => {
                            let min = e.params.get("min").and_then(|v| v.as_u64());
                            let max = e.params.get("max").and_then(|v| v.as_u64());
                            match (min, max) {
                                (Some(min), Some(max)) => format!(
                                    "\"{}\" length must be {}-{} characters long",
                                    field_name, min, max
                                ),
                                (Some(min), None) => format!(
                                    "\"{}\" must be at least {} characters long",
                                    field_name, min
                                ),
                                (None, Some(max)) => format!(
                                    "\"{}\" must be at most {} characters long",
                                    field_name, max
                                ),
                                _ => format!("\"{}\" has invalid length", field_name),
                            }
                        }
                        "range" => {
                            let min = e.params.get("min").and_then(|v| v.as_f64());
                            let max = e.params.get("max").and_then(|v| v.as_f64());
                            match (min, max) {
                                (Some(min), Some(max)) => format!(
                                    "\"{}\" must be between {} and {}",
                                    field_name, min, max
                                ),
                                _ => format!("\"{}\" is out of range", field_name),
                            }
                        }
                        _ => format!("\"{}\" is invalid", field_name),
                    }
                }
            })
            .collect();
        field_errors.push(FieldError::new(field_name, "body", messages));
    }
    field_errors
}

/// Validate a struct that implements `Validate` and convert errors to `AppError`.
/// Used by handlers that receive raw `Json<T>` and then validate manually.
#[allow(dead_code)]
pub fn validate_to_app_error<T: Validate>(value: &T) -> Result<(), AppError> {
    value.validate().map_err(|e| {
        let errors = validation_errors_to_field_errors(e);
        AppError::Validation { errors }
    })
}
