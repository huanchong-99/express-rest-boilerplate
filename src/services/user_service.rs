//! User service – mirrors User static methods from user.model.js

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::user::{NewUser, UpdateUser, User};

/// Get a single user by ID.
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::UserNotFound)
}

/// List users with optional filtering and pagination.
/// Default: page=1, perPage=30, sorted by createdAt descending.
pub async fn list_users(
    pool: &PgPool,
    page: i64,
    per_page: i64,
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
) -> Result<Vec<User>, AppError> {
    let page = page.max(1);
    let per_page = per_page.clamp(1, 100);
    let offset = per_page.saturating_mul(page.saturating_sub(1));

    let mut query = String::from("SELECT * FROM users WHERE 1=1");
    let mut param_idx: u32 = 1;

    if name.is_some() {
        query.push_str(&format!(" AND name ILIKE ${param_idx}"));
        param_idx += 1;
    }
    if email.is_some() {
        query.push_str(&format!(" AND email ILIKE ${param_idx}"));
        param_idx += 1;
    }
    if role.is_some() {
        query.push_str(&format!(" AND role = ${param_idx}"));
        param_idx += 1;
    }

    query.push_str(&format!(
        " ORDER BY created_at DESC OFFSET ${param_idx} LIMIT ${}",
        param_idx + 1
    ));

    let mut q = sqlx::query_as::<_, User>(&query);

    if let Some(n) = &name {
        q = q.bind(format!("%{}%", n));
    }
    if let Some(e) = &email {
        q = q.bind(format!("%{}%", e));
    }
    if let Some(r) = &role {
        q = q.bind(r);
    }

    q = q.bind(offset).bind(per_page);

    let rows = q.fetch_all(pool).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(rows)
}

/// Create a new user. Hashes password with bcrypt before storage.
pub async fn create_user(pool: &PgPool, new_user: NewUser) -> Result<User, AppError> {
    let hash = bcrypt::hash(&new_user.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let role = new_user.role.as_deref().unwrap_or("user");

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password, name, role) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(new_user.email.to_lowercase())
    .bind(&hash)
    .bind(&new_user.name)
    .bind(role)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::DuplicateEmail;
            }
        }
        AppError::Internal(e.to_string())
    })?;

    Ok(user)
}

/// Replace a user entirely (PUT).
pub async fn replace_user(
    pool: &PgPool,
    id: Uuid,
    new_user: NewUser,
    is_admin: bool,
) -> Result<User, AppError> {
    let existing = get_user(pool, id).await?;

    let new_role = if is_admin {
        new_user.role.as_deref().unwrap_or(&existing.role)
    } else {
        &existing.role
    };

    let hash = bcrypt::hash(&new_user.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET email = $1, password = $2, name = $3, role = $4, updated_at = NOW() WHERE id = $5 RETURNING *",
    )
    .bind(new_user.email.to_lowercase())
    .bind(&hash)
    .bind(&new_user.name)
    .bind(new_role)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::DuplicateEmail;
            }
        }
        AppError::Internal(e.to_string())
    })?;

    Ok(user)
}

/// Update a user partially (PATCH).
pub async fn update_user(
    pool: &PgPool,
    id: Uuid,
    update: UpdateUser,
    is_admin: bool,
) -> Result<User, AppError> {
    let existing = get_user(pool, id).await?;

    let final_email = update
        .email
        .map(|e| e.to_lowercase())
        .unwrap_or(existing.email);
    let final_name = update.name.or(existing.name);
    let final_picture = update.picture.or(existing.picture);

    let final_role = if is_admin {
        update.role.as_deref().unwrap_or(&existing.role).to_string()
    } else {
        existing.role.clone()
    };

    let final_password = if let Some(ref pw) = update.password {
        bcrypt::hash(pw, bcrypt::DEFAULT_COST).map_err(|e| AppError::Internal(e.to_string()))?
    } else {
        existing.password.clone()
    };

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET email = $1, password = $2, name = $3, role = $4, picture = $5, updated_at = NOW() WHERE id = $6 RETURNING *",
    )
    .bind(&final_email)
    .bind(&final_password)
    .bind(&final_name)
    .bind(&final_role)
    .bind(&final_picture)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::DuplicateEmail;
            }
        }
        AppError::Internal(e.to_string())
    })?;

    Ok(user)
}

/// Delete a user.
pub async fn delete_user(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(AppError::UserNotFound);
    }

    Ok(())
}

/// Find a user by email address.
#[allow(dead_code)]
pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.to_lowercase())
        .fetch_optional(pool)
        .await?;

    Ok(user)
}
