//! OpenAPI documentation configuration using utoipa.
//!
//! Provides the `OpenApi` specification and Swagger UI mount.

use utoipa::openapi::security::SecurityScheme;
use utoipa::{Modify, OpenApi};

use crate::handlers::auth::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest, TokenResponse};
use crate::handlers::user::ListUsersQuery;
use crate::models::refresh_token::{NewRefreshToken, RefreshToken};
use crate::models::user::{NewUser, UpdateUser, User, UserResponse};

/// Main OpenAPI specification for the API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Express REST Boilerplate (Rust)",
        version = "1.0.0",
        description = "A REST API boilerplate migrated from Express.js to Rust/Axum.",
    ),
    paths(
        crate::handlers::health::health_check,
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::refresh,
        crate::handlers::user::list_users,
        crate::handlers::user::create_user,
        crate::handlers::user::get_profile,
        crate::handlers::user::get_user,
        crate::handlers::user::replace_user,
        crate::handlers::user::update_user,
        crate::handlers::user::delete_user,
    ),
    components(
        schemas(
            User,
            UserResponse,
            NewUser,
            UpdateUser,
            RefreshToken,
            NewRefreshToken,
            AuthResponse,
            TokenResponse,
            RegisterRequest,
            LoginRequest,
            RefreshRequest,
            ListUsersQuery,
            crate::errors::ErrorBody,
            crate::errors::FieldError,
        )
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Adds Bearer JWT security scheme to the OpenAPI spec.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::http(utoipa::openapi::security::Http::new(
                    utoipa::openapi::security::HttpAuthScheme::Bearer,
                )),
            );
        }
    }
}
