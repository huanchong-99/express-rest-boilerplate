# Express REST Boilerplate — Rust/Axum Migration

REST API boilerplate migrated from Express.js/MongoDB to **Rust/Axum/PostgreSQL**.

## Tech Stack

| Layer         | Original (Express.js) | Migrated (Rust)            |
|---------------|----------------------|----------------------------|
| Runtime       | Node.js              | Tokio (async runtime)      |
| Framework     | Express              | Axum 0.7                   |
| Database      | MongoDB (Mongoose)   | PostgreSQL (SQLx)          |
| Auth          | Passport + jwt-simple| jsonwebtoken + argon2      |
| Validation    | Joi                  | validator crate            |
| Docs          | apidoc               | utoipa + Swagger UI        |
| Logging       | Winston + Morgan     | tracing + tracing-subscriber|

## Requirements

- [Rust](https://rustup.rs/) (latest stable, 1.83+)
- [PostgreSQL](https://www.postgresql.org/) 14+
- [Docker](https://www.docker.com/) (optional)

## Getting Started

```bash
# Clone the repo
git clone <repo-url>
cd express-rest-boilerplate

# Copy environment configuration
cp .env.example .env

# Create the PostgreSQL database
createdb express_rest_boilerplate

# Build and run
cargo run
```

The server starts at `http://localhost:3000` by default.

Swagger UI is available at `http://localhost:3000/docs`.

## Environment Variables

| Variable                | Default       | Description                          |
|-------------------------|---------------|--------------------------------------|
| `RUST_ENV`              | `development` | Environment name                     |
| `HOST`                  | `0.0.0.0`     | Server bind address                  |
| `PORT`                  | `3000`        | Server port                          |
| `JWT_SECRET`            | — (required)  | Secret for signing JWT tokens        |
| `JWT_EXPIRATION_MINUTES`| `15`          | Access token lifetime                |
| `DATABASE_URL`          | — (required)  | PostgreSQL connection string         |

Copy `.env.example` to `.env` and adjust values:

```bash
cp .env.example .env
```

## Project Structure

```
src/
├── main.rs              # Entry point: config, tracing, DB pool, server
├── lib.rs               # Crate root: module declarations, create_app()
├── config.rs            # AppConfig loading from environment
├── db.rs                # PgPool creation and migration runner
├── app_state.rs         # Shared AppState (pool + config)
├── errors.rs            # Unified AppError enum with IntoResponse
├── extractors.rs        # ValidatedJson / ValidatedQuery extractors
├── middleware/
│   ├── mod.rs
│   └── auth.rs          # JWT auth extractors: AuthUser, AdminUser, LoggedUser
├── models/
│   ├── mod.rs
│   ├── user.rs          # User, NewUser, UpdateUser, UserResponse
│   └── refresh_token.rs # RefreshToken, NewRefreshToken
├── handlers/
│   ├── mod.rs
│   ├── health.rs        # GET /v1/health-check
│   ├── auth.rs          # POST register, login, refresh-token
│   └── user.rs          # CRUD + profile endpoints
├── services/
│   ├── mod.rs
│   ├── auth_service.rs  # Registration, login, token refresh logic
│   └── user_service.rs  # User CRUD database operations
├── routes/
│   ├── mod.rs           # Route group definitions
│   ├── auth.rs          # Auth route stubs
│   └── user.rs          # User route stubs
├── docs.rs              # utoipa OpenAPI specification
├── schema.rs            # Re-exports of model types
└── test_utils.rs        # Test helpers (config, app builder, assertions)
migrations/
├── 20240101000000_create_users_table.sql
└── 20240101000001_create_refresh_tokens_table.sql
tests/
├── health_test.rs       # Health-check integration tests
└── auth_middleware_test.rs # Auth, hashing, validation unit tests
```

## API Endpoints

| Method | Path                    | Auth          | Description              |
|--------|-------------------------|---------------|--------------------------|
| GET    | `/v1/health-check`      | Public        | Service health check     |
| POST   | `/v1/auth/register`     | Public        | Register new user        |
| POST   | `/v1/auth/login`        | Public        | Login with credentials   |
| POST   | `/v1/auth/refresh-token`| Public        | Refresh access token     |
| GET    | `/v1/users`             | Admin         | List all users           |
| POST   | `/v1/users`             | Admin         | Create user              |
| GET    | `/v1/users/profile`     | Logged-in     | Current user profile     |
| GET    | `/v1/users/{user_id}`   | Owner/Admin   | Get user by ID           |
| PUT    | `/v1/users/{user_id}`   | Owner/Admin   | Replace user             |
| PATCH  | `/v1/users/{user_id}`   | Owner/Admin   | Update user              |
| DELETE | `/v1/users/{user_id}`   | Owner/Admin   | Delete user              |

## Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run only integration tests
cargo test --test health_test
cargo test --test auth_middleware_test
```

> **Note:** Integration tests that hit the database require a running PostgreSQL instance with the test database set up.

## Docker

```bash
# Development
docker compose -f docker-compose.yml -f docker-compose.dev.yml up

# Production
docker compose -f docker-compose.yml -f docker-compose.prod.yml up

# Test
docker compose -f docker-compose.yml -f docker-compose.test.yml up
```

The Dockerfile uses a multi-stage build with `cargo-chef` for dependency caching, producing a minimal runtime image based on `debian:bookworm-slim`.

## Migration Notes

This project was migrated from the [Express REST ES2017 Boilerplate](https://github.com/danielfsousa/express-rest-es2017-boilerplate). Key migration decisions:

- **MongoDB → PostgreSQL**: Mongoose schemas translated to SQL with proper constraints, foreign keys, and indexes
- **bcrypt → argon2**: Password hashing upgraded to Argon2 for modern security
- **jwt-simple → jsonwebtoken**: JWT handling with proper claims validation
- **Joi → validator**: Request validation using Rust's validator derive macros
- **Winston/Morgan → tracing**: Structured logging with tracing-subscriber
- **apidoc → utoipa**: OpenAPI 3.0 spec auto-generated from handler annotations
- **ObjectID → UUID**: Primary keys use PostgreSQL UUIDs (v4)

The original Express.js source remains in `src/api/` for reference during the migration.

## Deliverables Checklist

This migration provides all 15 required deliverables:

1. ✅ Express.js codebase analysis — all routes, middleware, controllers, models, auth, validation, and Docker mapped
2. ✅ Cargo project with all dependencies (axum 0.7, tokio, sqlx, serde, jsonwebtoken, argon2, validator, thiserror, anyhow, utoipa, dotenvy)
3. ✅ SQLx migration files translating Mongoose schemas to PostgreSQL tables
4. ✅ Shared types/models in `src/models/` with proper derives (Serialize, Deserialize, FromRow, Validate, ToSchema)
5. ✅ AppConfig in `src/config.rs` loading from environment variables
6. ✅ Unified AppError enum in `src/errors.rs` with IntoResponse
7. ✅ Database connection pool in `src/db.rs`
8. ✅ JWT auth infrastructure in `src/middleware/auth.rs` (AuthUser, AdminUser, LoggedUser extractors)
9. ✅ AppState struct with Clone in `src/app_state.rs`
10. ✅ Main entry point with tracing, CORS, Swagger UI
11. ✅ Router skeleton matching Express.js route structure in `src/routes/`
12. ✅ API documentation with utoipa Swagger UI at `/docs`
13. ✅ Docker configuration (multi-stage Rust build + PostgreSQL in docker-compose.yml)
14. ✅ Test infrastructure with helpers (`src/test_utils.rs`) — 70 tests pass
15. ✅ README.md with migration notes

## License

[MIT License](LICENSE) - [Daniel Sousa](https://github.com/danielfsousa)

---METADATA---
workflow_id: 086ffcd4-21b5-4897-8619-ac4362401fc8
task_id: 5e97433d-6397-4a2f-8bb6-72d1fd5dcfc5
terminal_id: ccbe1d90-e93b-4099-bffd-f7fbfd4ce021
terminal_order: 1
status: completed
next_action: handoff
