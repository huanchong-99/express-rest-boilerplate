# Migration Notes: Express.js + MongoDB → Rust + Axum + PostgreSQL

## Overview

This document tracks the migration of the Express REST boilerplate from Node.js/MongoDB to Rust/Axum/PostgreSQL.

---

## 1. MongoDB to PostgreSQL Schema Mapping

### Collection: users → Table: users

| MongoDB Field       | PostgreSQL Column | Type / Constraints                                        | Notes                                 |
|---------------------|-------------------|-----------------------------------------------------------|---------------------------------------|
| _id (ObjectId)      | id                | UUID PRIMARY KEY DEFAULT gen_random_uuid()                | Switched from ObjectId to UUID        |
| email               | email             | VARCHAR(255) NOT NULL                                     | Unique index on LOWER(email)          |
| password            | password          | VARCHAR(128) NOT NULL                                     | bcrypt-hashed, 6-128 chars            |
| name                | name              | VARCHAR(128)                                              | Indexed (MongoDB index:true)          |
| services.facebook   | facebook_id       | VARCHAR(255)                                              | Flattened from embedded document      |
| services.google     | google_id         | VARCHAR(255)                                              | Flattened from embedded document      |
| role                | role              | VARCHAR(16) NOT NULL DEFAULT 'user' CHECK IN (user,admin) | Enum enforced by CHECK constraint     |
| picture             | picture           | TEXT                                                      |                                       |
| createdAt           | created_at        | TIMESTAMPTZ NOT NULL DEFAULT NOW()                        | Indexed DESC for sorting              |
| updatedAt           | updated_at        | TIMESTAMPTZ NOT NULL DEFAULT NOW()                        | Auto-updated via trigger              |

**Indexes:**
- UNIQUE on LOWER(email) — matches MongoDB unique:true
- btree on name — matches MongoDB index:true
- btree on created_at DESC — matches .sort({ createdAt: -1 })

**Schema normalizations:**
- The embedded services document was flattened into facebook_id/google_id columns (avoids JSONB for a fixed structure).
- Mongoose timestamps:true replaced by explicit columns + trigger.

### Collection: refreshtokens → Table: refresh_tokens

| MongoDB Field | PostgreSQL Column | Type / Constraints                                    | Notes                          |
|---------------|-------------------|-------------------------------------------------------|--------------------------------|
| _id           | id                | UUID PRIMARY KEY DEFAULT gen_random_uuid()            |                                |
| token         | token             | TEXT NOT NULL                                         | Indexed                        |
| userId        | user_id           | UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE  | Proper FK now                  |
| userEmail     | user_email        | VARCHAR(255) NOT NULL                                 | Denormalized (same as original)|
| expires       | expires           | TIMESTAMPTZ                                           | Nullable                       |
| (auto)        | created_at        | TIMESTAMPTZ NOT NULL DEFAULT NOW()                    | Explicit timestamp             |

---

## 2. All Endpoints from Original Express.js App

### Health Check
- GET /v1/health-check — No auth — returns OK

### Auth Routes (/v1/auth/...)
- POST /v1/auth/register — No auth — Register + return JWT + refresh token
- POST /v1/auth/login — No auth — Login with email/password
- POST /v1/auth/refresh-token — No auth — Exchange refresh token for new JWT

### User Routes (/v1/users/...)
- GET    /v1/users          — ADMIN only — List users (paginated, filtered by name/email/role)
- POST   /v1/users          — ADMIN only — Create user
- GET    /v1/users/profile  — Any logged-in user — Get own profile
- GET    /v1/users/:userId  — LOGGED_USER — Get user by ID
- PUT    /v1/users/:userId  — LOGGED_USER — Replace user entirely
- PATCH  /v1/users/:userId  — LOGGED_USER — Partial update user
- DELETE /v1/users/:userId  — LOGGED_USER — Delete user

Auth model: LOGGED_USER = owner of the resource (userId matches) OR admin

### TODO endpoints (in original comments, not implemented):
- POST /v1/auth/reset-password
- POST /v1/auth/facebook
- POST /v1/auth/google

---

## 3. Environment Variables

| Variable                  | Required | Default       | Description                      |
|---------------------------|----------|---------------|----------------------------------|
| DATABASE_URL              | Yes      | —             | PostgreSQL connection string     |
| JWT_SECRET                | Yes      | —             | Secret for signing JWT tokens    |
| JWT_EXPIRATION_MINUTES    | No       | 15            | Access token lifetime (minutes)  |
| PORT                      | No       | 3000          | Server port                      |
| HOST                      | No       | 0.0.0.0       | Server bind address              |
| NODE_ENV                  | No       | development   | Environment name                 |

Original variables no longer needed: MONGO_URI, MONGO_URI_TESTS

---

## 4. Architecture Mapping

| Express.js Layer           | Rust/Axum Equivalent                |
|----------------------------|--------------------------------------|
| src/index.js               | src/main.rs                          |
| src/config/                | src/config.rs                        |
| src/config/mongoose.js     | src/db.rs (SQLx pool + migrations)   |
| src/api/models/            | src/models/ (SQLx FromRow structs)   |
| src/api/controllers/       | src/handlers/                        |
| src/api/validations/       | validator derive macros on structs   |
| src/api/middlewares/auth.js| src/middleware/auth.rs               |
| src/api/middlewares/error.js| src/errors.rs (thiserror+IntoResponse)|
| src/api/utils/APIError.js  | src/errors.rs (AppError enum)        |
| src/api/routes/v1/         | Router definitions in src/main.rs    |
| passport + jwt-simple      | jsonwebtoken crate                   |
| bcryptjs                   | bcrypt crate                         |
| mongoose statics/methods   | src/services/ layer                  |
| cors middleware             | tower-http CorsLayer                 |
| helmet middleware           | tower-http security headers          |
| morgan + winston            | tracing + tracing-subscriber         |

---

## 5. Key Design Decisions

1. **UUID primary keys** instead of MongoDB ObjectIds.
2. **Flattened services** into separate facebook_id/google_id columns.
3. **Auto-update trigger** for updated_at column.
4. **Service layer** extracted between handlers and database.
5. **Placeholder stubs** — handlers return Not yet implemented, services use todo!().
6. **Refresh token one-time use** preserved — original findOneAndRemove becomes DELETE RETURNING.
