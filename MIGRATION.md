# Migration Guide: Express.js to Rust/Axum

This document describes the migration from the original Express.js/MongoDB REST API to Rust/Axum with PostgreSQL.

## Endpoint Mapping

### Authentication

| Express.js Route              | Axum Route                    | Handler                      |
|-------------------------------|-------------------------------|------------------------------|
| POST /v1/auth/register        | POST /v1/auth/register        | handlers::auth::register     |
| POST /v1/auth/login           | POST /v1/auth/login           | handlers::auth::login        |
| POST /v1/auth/refresh-token   | POST /v1/auth/refresh-token   | handlers::auth::refresh      |
| POST /v1/auth/facebook        | not migrated                  | -                            |
| POST /v1/auth/google          | not migrated                  | -                            |
| POST /v1/auth/reset-password  | not migrated                  | -                            |

### Users

| Express.js Route              | Axum Route                       | Handler                      |
|-------------------------------|----------------------------------|------------------------------|
| GET    /v1/users              | GET    /v1/users                 | handlers::user::list_users   |
| POST   /v1/users              | POST   /v1/users                 | handlers::user::create_user  |
| GET    /v1/users/profile      | GET    /v1/users/profile         | handlers::user::get_profile  |
| GET    /v1/users/:userId      | GET    /v1/users/:user_id        | handlers::user::get_user     |
| PUT    /v1/users/:userId      | PUT    /v1/users/:user_id        | handlers::user::replace_user |
| PATCH  /v1/users/:userId      | PATCH  /v1/users/:user_id        | handlers::user::update_user  |
| DELETE /v1/users/:userId      | DELETE /v1/users/:user_id        | handlers::user::delete_user  |

### Health Check

| Express.js Route    | Axum Route          | Handler                        |
|---------------------|----------------------|--------------------------------|
| GET /v1/health-check | GET /v1/health-check | handlers::health::health_check |

### API Documentation

| Express.js                | Axum                          |
|---------------------------|-------------------------------|
| apidoc (static HTML docs) | Swagger UI at /docs           |
| -                         | OpenAPI JSON at /openapi.json |

## Database Migration (MongoDB to PostgreSQL)

### Schema Changes

**Users Collection to users Table**

| MongoDB Field      | PostgreSQL Column | Type         | Notes                          |
|--------------------|-------------------|--------------|--------------------------------|
| _id (ObjectId)     | id                | UUID         | Primary key, auto-generated    |
| email              | email             | VARCHAR(255) | Unique, case-insensitive index |
| password           | password          | VARCHAR(128) | Argon2 hash (was bcrypt)       |
| name               | name              | VARCHAR(128) | Nullable                       |
| role               | role              | VARCHAR(16)  | CHECK: user or admin           |
| picture            | picture           | TEXT         | Nullable                       |
| services.facebook  | facebook_id       | VARCHAR(255) | Nullable, flattened            |
| services.google    | google_id         | VARCHAR(255) | Nullable, flattened            |
| createdAt          | created_at        | TIMESTAMPTZ  | Auto via DEFAULT NOW()         |
| updatedAt          | updated_at        | TIMESTAMPTZ  | Auto via trigger               |

**RefreshTokens Collection to refresh_tokens Table**

| MongoDB Field      | PostgreSQL Column | Type         | Notes                           |
|--------------------|-------------------|--------------|---------------------------------|
| _id (ObjectId)     | id                | UUID         | Primary key                     |
| token              | token             | TEXT         | Indexed                         |
| userId (ObjectId)  | user_id           | UUID         | FK to users(id) ON DELETE CASCADE|
| userEmail          | user_email        | VARCHAR(255) | Indexed for lookup              |
| expires            | expires           | TIMESTAMPTZ  | Nullable                        |
| createdAt          | created_at        | TIMESTAMPTZ  | Auto via DEFAULT NOW()          |

### Migrations

SQL migrations in migrations/ applied via sqlx::migrate!():
- 20240101000000_create_users_table.sql
- 20240101000001_create_refresh_tokens_table.sql

## Configuration Changes

| Variable                | Express.js       | Rust/Axum                | Notes                        |
|-------------------------|------------------|--------------------------|------------------------------|
| NODE_ENV                | Required         | -                        | Replaced by RUST_ENV         |
| RUST_ENV                | -                | Optional (default: dev)  |                              |
| PORT                    | Required         | Optional (default: 3000) |                              |
| HOST                    | -                | Optional (default: 0.0.0.0)|                            |
| MONGO_URI               | Required         | -                        | Replaced by DATABASE_URL     |
| DATABASE_URL            | -                | Required                 | PostgreSQL connection string |
| JWT_SECRET              | Required         | Required                 | Same purpose                 |
| JWT_EXPIRATION_MINUTES  | Optional (15)    | Optional (15)            | Same default                 |

## Behavioral Differences

### Password Hashing
- Express.js: bcrypt via bcryptjs
- Rust: Argon2 via the argon2 crate
- Impact: Existing bcrypt hashes are NOT compatible. Users must reset passwords.
- Reason: Argon2 is the recommended password hashing algorithm (OWASP).

### ID Format
- Express.js: MongoDB ObjectId (24-char hex string)
- Rust: UUID v4 (36-char string with hyphens)
- Impact: Client code referencing user IDs by ObjectId must be updated.

### Validation Error Format
The error response shape is preserved for compatibility:

`{`json
{
  "code": 400,
  "message": "Validation Error",
  "errors": [
    {
      "field": "email",
      "location": "body",
      "messages": ["email must be a valid email"]
    }
  ]
}
``

Minor differences:
- Express (Joi): "email" is required
- Rust (validator): "email" is invalid (deserialization fails before validation)
- Query param validation location says "body" in Rust; Express had "query".

### Response Field Names
Express.js used camelCase (createdAt, updatedAt, userId, accessToken).
Rust API uses snake_case (created_at, updated_at, user_id, access_token).
Breaking change for clients.

### Refresh Token Flow
- Same flow preserved: token looked up by userEmail + token, consumed and deleted on refresh.
- Token format: {user_id}.{random_uuid}

### Pagination
- Express.js: page and perPage query params (Joi validated).
- Rust: page and per_page query params (snake_case).
- Default: page=1, per_page=30. Max per_page=100.

### Authorization Model
- Admin users (role=admin) can access/modify any user resource.
- Regular users (role=user) can only access/modify their own.
- GET /v1/users and POST /v1/users require admin role.
- Matches original Express.js behavior.

### Non-Migrated Features
- POST /v1/auth/facebook (Facebook OAuth)
- POST /v1/auth/google (Google OAuth)
- POST /v1/auth/reset-password (Password reset via email)
- Email templates
- Rate limiting middleware

## Docker

- Dockerfile: Multi-stage build with cargo-chef. Runtime: debian:bookworm-slim.
- docker-compose.yml: PostgreSQL 16 + Rust application.
- Overrides preserved: docker-compose.dev.yml, docker-compose.test.yml, docker-compose.prod.yml.

## Testing

Express.js: Mocha + Chai + Supertest with MongoDB.
Rust: Cargo built-in test framework with tower::ServiceExt::oneshot.

| File                      | Tests | Coverage                                    |
|---------------------------|-------|---------------------------------------------|
| health_test.rs            | 2     | Health check endpoint                       |
| api_handlers_test.rs      | 22    | Auth and user API validation, response shapes|
| auth_middleware_test.rs    | 12    | JWT, password hashing, input validation     |
| domain_handlers_test.rs   | 33    | Domain models, services, errors, authz      |
| integration_tests.rs      | 53    | Full E2E: CORS, OpenAPI, auth, errors       |

Total: 122 tests (plus 1 inline in test_utils.rs)

## Running the Application

``bash
cp .env.example .env
# Edit .env with your database credentials
cargo run
# Or: docker compose up --build
``

## API Documentation

- Swagger UI: http://localhost:3000/docs
- OpenAPI JSON: http://localhost:3000/openapi.json