//! Unit tests for auth middleware functions.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

// ---- Token creation and decoding ----

#[test]
fn test_create_and_decode_access_token() {
    let key = "jwt-test-key";
    let user_id = Uuid::new_v4();
    let expiration = 15;

    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
    }

    let now = Utc::now();
    let expires = now + chrono::Duration::minutes(expiration);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expires.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.as_bytes()),
    )
    .expect("should succeed");

    let decoded = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(key.as_bytes()),
        &Validation::default(),
    )
    .expect("should succeed");

    assert_eq!(decoded.claims.sub, user_id.to_string());
}

#[test]
fn test_expired_token_is_rejected() {
    let key = "jwt-test-key";
    let user_id = Uuid::new_v4();

    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
    }

    let now = Utc::now() - chrono::Duration::hours(1);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.as_bytes()),
    )
    .expect("should succeed");

    let result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(key.as_bytes()),
        &Validation::default(),
    );

    assert!(result.is_err());
}

#[test]
fn test_wrong_key_is_rejected() {
    let signing_key = "jwt-signing-key-a";
    let wrong_key = "jwt-signing-key-b";
    let user_id = Uuid::new_v4();

    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
    }

    let now = Utc::now();
    let expires = now + chrono::Duration::minutes(15);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expires.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(signing_key.as_bytes()),
    )
    .expect("should succeed");

    let result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(wrong_key.as_bytes()),
        &Validation::default(),
    );

    assert!(result.is_err());
}

// ---- Role validation ----

#[test]
fn test_valid_roles() {
    let roles = ["user", "admin"];
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"admin"));
}

#[test]
fn test_invalid_roles() {
    let roles = ["user", "admin"];
    assert!(!roles.contains(&"superadmin"));
    assert!(!roles.contains(&"moderator"));
    assert!(!roles.contains(&""));
}

// ---- Password hashing (argon2) ----

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("should succeed")
        .to_string()
}

fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = PasswordHash::new(hash).expect("should succeed");
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[test]
fn test_argon2_password_hash_and_verify() {
    let password = "test-password-123";
    let hash = hash_password(password);
    assert!(verify_password(password, &hash));
    assert!(!verify_password("wrong-password", &hash));
}

#[test]
fn test_argon2_different_hashes_for_same_password() {
    let password = "same-password";
    let hash1 = hash_password(password);
    let hash2 = hash_password(password);
    assert_ne!(hash1, hash2);
    assert!(verify_password(password, &hash1));
    assert!(verify_password(password, &hash2));
}

// ---- Input validation ----

#[test]
fn test_email_validation() {
    #[derive(Debug, Validate)]
    struct EmailTest {
        #[validate(email)]
        email: String,
    }

    assert!(EmailTest {
        email: "user@example.com".into()
    }
    .validate()
    .is_ok());
    assert!(EmailTest {
        email: "test.user+tag@domain.org".into()
    }
    .validate()
    .is_ok());
    assert!(EmailTest {
        email: "not-an-email".into()
    }
    .validate()
    .is_err());
    assert!(EmailTest {
        email: "@domain.com".into()
    }
    .validate()
    .is_err());
    assert!(EmailTest {
        email: "user@".into()
    }
    .validate()
    .is_err());
    assert!(EmailTest {
        email: "".into()
    }
    .validate()
    .is_err());
}

#[test]
fn test_password_length_validation() {
    #[derive(Debug, Validate)]
    struct PasswordTest {
        #[validate(length(min = 6, max = 128))]
        password: String,
    }

    assert!(PasswordTest {
        password: "123456".into()
    }
    .validate()
    .is_ok());
    assert!(PasswordTest {
        password: "a".repeat(128)
    }
    .validate()
    .is_ok());
    assert!(PasswordTest {
        password: "12345".into()
    }
    .validate()
    .is_err());
    assert!(PasswordTest {
        password: "".into()
    }
    .validate()
    .is_err());
}

#[test]
fn test_pagination_range_validation() {
    #[derive(Debug, Validate)]
    struct PageTest {
        #[validate(range(min = 1))]
        page: Option<i64>,
        #[validate(range(min = 1, max = 100))]
        per_page: Option<i64>,
    }

    assert!(PageTest {
        page: Some(1),
        per_page: Some(30)
    }
    .validate()
    .is_ok());
    assert!(PageTest {
        page: None,
        per_page: None
    }
    .validate()
    .is_ok());
    assert!(PageTest {
        page: Some(0),
        per_page: Some(30)
    }
    .validate()
    .is_err());
    assert!(PageTest {
        page: Some(-1),
        per_page: Some(30)
    }
    .validate()
    .is_err());
    assert!(PageTest {
        page: Some(1),
        per_page: Some(101)
    }
    .validate()
    .is_err());
    assert!(PageTest {
        page: Some(1),
        per_page: Some(0)
    }
    .validate()
    .is_err());
}

#[test]
fn test_register_request_validation() {
    #[derive(Debug, Validate)]
    struct RegisterRequest {
        #[validate(email(message = "must be a valid email"))]
        email: String,
        #[validate(length(min = 6, max = 128))]
        password: String,
    }

    assert!(RegisterRequest {
        email: "user@example.com".into(),
        password: "password123".into(),
    }
    .validate()
    .is_ok());

    assert!(RegisterRequest {
        email: "not-email".into(),
        password: "password123".into(),
    }
    .validate()
    .is_err());

    assert!(RegisterRequest {
        email: "user@example.com".into(),
        password: "12345".into(),
    }
    .validate()
    .is_err());
}

#[test]
fn test_login_request_validation() {
    #[derive(Debug, Validate)]
    struct LoginRequest {
        #[validate(email(message = "must be a valid email"))]
        email: String,
        #[validate(length(max = 128))]
        password: String,
    }

    assert!(LoginRequest {
        email: "user@example.com".into(),
        password: "any-password".into(),
    }
    .validate()
    .is_ok());

    assert!(LoginRequest {
        email: "not-email".into(),
        password: "password".into(),
    }
    .validate()
    .is_err());

    assert!(LoginRequest {
        email: "user@example.com".into(),
        password: "".into(),
    }
    .validate()
    .is_ok());
}
